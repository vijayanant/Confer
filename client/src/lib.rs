pub mod error;

use std::sync::Arc;
use std::collections::HashMap;

use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request, Response, Status, Streaming,
};
use tracing::{info, warn, error, instrument};
use tokio::task::JoinHandle;
use tokio::sync::RwLock;

pub mod confer {
    pub mod v1 {
        tonic::include_proto!("confer.v1");
    }
}

use crate::error::ClientError;
use confer::v1::confer_service_client::ConferServiceClient;
use confer::v1::{
    ConfigPath, ConfigValue,
    Empty, SetConfigRequest,
    ConfigUpdate, ClusterUpdate};


/// A client for interacting with the Confer configuration service.
///
/// This client provides methods to retrieve, set, remove, list, and watch configuration data.
/// It uses gRPC for communication and supports secure connections via TLS.
#[derive(Debug)]
pub struct ConferClient {
    client: Arc<RwLock<ConferServiceClient<Channel>>>,
    config_watchers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    cluster_watchers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl ConferClient {
    /// Creates a new ConferClient with a connection to the specified address.
    ///
    /// The address should be a valid URI, including the scheme (e.g., "http://", "https://").
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the Confer server.
    /// * `tls_config` - Optional TLS configuration for secure connection.
    #[instrument(skip(tls_config), err)]
    pub async fn new(
        addr: String,
        tls_config: Option<ClientTlsConfig>,
    ) -> Result<Self, ClientError> {
        let channel = match tls_config {
            Some(config) => {
                info!("Connecting to Confer server with TLS: {}", addr);
                let client_builder = tonic::transport::Channel::from_shared(addr)
                    .map_err(|e| ClientError::ConnectionError(e.to_string()))?;
                client_builder.tls_config(config)
                    .map_err(|e| ClientError::ConnectionError(e.to_string()))?
                    .connect().await
                    .map_err(|e| ClientError::ConnectionError(e.to_string()))?
            }
            None => {
                info!("Connecting to Confer server without TLS: {}", addr);
                tonic::transport::Channel::from_shared(addr)
                    .map_err(|e| ClientError::ConnectionError(e.to_string()))?
                    .connect().await
                    .map_err(|e| ClientError::ConnectionError(e.to_string()))?
            }
        };

        let client = Arc::new(RwLock::new(ConferServiceClient::new(channel)));
        let config_watchers = Arc::new(RwLock::new(HashMap::new()));
        let cluster_watchers = Arc::new(RwLock::new(HashMap::new()));
        Ok(ConferClient { client, config_watchers, cluster_watchers })
    }

    /// Retrieves the configuration value for a given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration value to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing the configuration value as a byte vector, or an error if the operation fails.
    #[instrument(skip(self), err)]
    pub async fn get(&self, path: String) -> Result<Vec<u8>, ClientError> {
        info!("Getting config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        let mut client = self.client.write().await;
        let response = client.get(request).await
            .map_err(ClientError::GrpcError)?;
        Ok(response.into_inner().value)
    }

    /// Sets the configuration value for a given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration value to set.
    /// * `value` - The configuration value as a byte vector.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    #[instrument(skip(self, value), err)]
    pub async fn set(&self, path: String, value: Vec<u8>) -> Result<(), ClientError> {
        info!("Setting config for path: {}", path);
        let request = Request::new(SetConfigRequest {
            path: Some(ConfigPath { path }),
            value: Some(ConfigValue { value }),
        });
        let mut client = self.client.write().await;
        client.set(request).await
            .map_err(ClientError::GrpcError)?;
        Ok(())
    }

    /// Removes the configuration value associated with the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration value to remove.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    #[instrument(skip(self), err)]
    pub async fn remove(&self, path: String) -> Result<(), ClientError> {
        info!("Removing config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        let mut client = self.client.write().await;
        client.remove(request).await
            .map_err(ClientError::GrpcError)?;
        Ok(())
    }

    /// Lists all paths that start with the given path prefix.
    ///
    /// # Arguments
    ///
    /// * `path` - The path prefix to list.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of paths, or an error if the operation fails.
    #[instrument(skip(self), err)]
    async fn list(&self, path: String) -> Result<Response<Vec<String>>, Status> { // Keep as Status for now, as that's what the direct gRPC call returns
        let request = Request::new(ConfigPath { path });
        let mut client = self.client.write().await;
        let response = client.list(request).await?;
        Ok(Response::new(response.into_inner().paths))
    }

    /// Watches for changes to the configuration value at the given path.
    ///
    /// Returns a stream of `ConfigUpdate` messages.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to watch for changes.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Stream` of `ConfigUpdate` messages, or an error.
    #[instrument(skip(self), err)]
    pub async fn watch_config(
        &self,
        path: String,
    ) -> Result<Response<Streaming<ConfigUpdate>>, ClientError> {
        info!("Watching config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        let mut client = self.client.write().await;
        client.watch_config(request).await
            .map_err(ClientError::GrpcError)
    }

    /// Watches for changes to the configuration value at the given path and calls a callback.
    ///
    /// This method starts a new task that receives `ConfigUpdate` messages from the server
    /// and calls the provided `callback` function for each message.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to watch for changes.
    /// * `callback` - A closure to be called when a change occurs.  The closure
    ///                 receives a `Result<ConfigUpdate, ClientError>`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure in setting up the watch.
    #[instrument(skip(self, callback))]
    pub async fn watch_config_with_callback<F>(
        &self,
        path: String,
        callback: F,
    ) -> Result<(), ClientError>
    where
        F: Fn(Result<ConfigUpdate, ClientError>) + Send + Sync + 'static,
    {
        info!("Watching config with callback for path: {}", path);
        let mut stream = self.watch_config_internal(path.clone()).await?;

        let config_watchers = self.config_watchers.clone();
        let task_key = path.clone();
        let handle = tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(update)) => {
                        info!("Config updated for path: {}", task_key);
                        callback(Ok(update));
                    }
                    Ok(None) => {
                        warn!("Stream ended for path: {}", task_key);
                        callback(Err(ClientError::GrpcError(Status::unavailable("Config watch stream ended by server"))));
                        break;
                    }
                    Err(e) => {
                        error!("Error watching path {}: {}", task_key, e);
                        callback(Err(ClientError::GrpcError(e)));
                        break;
                    }
                }
            }
            let mut watchers = config_watchers.write().await;
            watchers.remove(&task_key);
            info!("Watcher task for path {} removed", task_key);
        });

        let mut watchers = self.config_watchers.write().await;
        watchers.insert(path, handle);
        Ok(())
    }

    /// Internal helper to get the watch stream.
    #[instrument(skip(self), err)]
    async fn watch_config_internal(
        &self,
        path: String,
    ) -> Result<Streaming<ConfigUpdate>, ClientError> {
        let request = Request::new(ConfigPath { path });
        let mut client = self.client.write().await;
        let response = client.watch_config(request).await
            .map_err(ClientError::GrpcError)?;
        Ok(response.into_inner())
    }

    /// Watches for cluster changes and calls a callback.
    #[instrument(skip(self, callback))]
    pub async fn watch_cluster<F>(
        &self,
        callback: F,
    ) -> Result<(), ClientError>
    where
        F: Fn(Result<ClusterUpdate, ClientError>) + Send + Sync + 'static,
    {
        info!("Watching for cluster changes.");
        let mut stream = self.watch_cluster_internal().await?;
        let cluster_watchers = Arc::clone(&self.cluster_watchers);
        let task_key = "cluster".to_string();
        let task_key_clone = task_key.clone();
        let handle = tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(update)) => {
                        info!("Cluster update received");
                        callback(Ok(update));
                    }
                    Ok(None) => {
                        warn!("Cluster watch stream ended");
                        callback(Err(ClientError::GrpcError(Status::unavailable("Cluster watch stream ended by server"))));
                        break;
                    }
                    Err(e) => {
                        error!("Error watching cluster: {}", e);
                        callback(Err(ClientError::GrpcError(e)));
                        break;
                    }
                }
            }
            let mut watchers = cluster_watchers.write().await;
            watchers.remove(&task_key_clone);
            info!("Cluster watcher task removed");
        });

        let mut watchers = self.cluster_watchers.write().await;
        watchers.insert(task_key, handle);
        Ok(())
    }

    /// Internal helper to get the watch cluster stream.
    #[instrument(skip(self), err)]
    async fn watch_cluster_internal(
        &self,
    ) -> Result<Streaming<ClusterUpdate>, ClientError> {
        let request = Request::new(Empty {});
        let mut client = self.client.write().await;
        let response = client.watch_cluster(request).await
            .map_err(ClientError::GrpcError)?;
        Ok(response.into_inner())
    }

    /// Cancels all active config watchers.
    #[instrument(skip(self))]
    pub async fn cancel_config_watchers(&self) {
        let mut watchers = self.config_watchers.write().await;
        info!("Cancelling {} config watcher tasks", watchers.len());
        for (key, handle) in watchers.iter() {
            handle.abort();
            info!("Cancelled config watcher for key: {}", key);
        }
        watchers.clear();
        info!("All config watcher tasks cancelled");
    }

    /// Cancels all active cluster watchers.
    #[instrument(skip(self))]
    pub async fn cancel_cluster_watchers(&self) {
        let mut watchers = self.cluster_watchers.write().await;
        info!("Cancelling {} cluster watcher tasks", watchers.len());
        for (key, handle) in watchers.iter() {
            handle.abort();
            info!("Cancelled cluster watcher for key: {}", key);
        }
        watchers.clear();
        info!("All cluster watcher tasks cancelled");
    }

    /// Cancels all active watchers (both config and cluster).
    #[instrument(skip(self))]
    pub async fn cancel_all_watchers(&self) {
        self.cancel_config_watchers().await;
        self.cancel_cluster_watchers().await;
    }
}
