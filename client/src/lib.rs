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

use confer::v1::watch_update::Kind;
use confer::v1::confer_service_client::ConferServiceClient;
use confer::v1::{
    ConfigPath,
    ConfigValue,
    SetConfigRequest,
    WatchUpdate,
    Empty,
    LeaderInfo};


/// A client for interacting with the Confer configuration service.
///
/// This client provides methods to retrieve, set, remove, list, and watch configuration data.
/// It uses gRPC for communication and supports secure connections via TLS.
#[derive(Debug)]
pub struct ConferClient {
    client: ConferServiceClient<Channel>,
    watchers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>, // Keep track of watch tasks
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
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = match tls_config {
            Some(config) => {
                info!("Connecting to Confer server with TLS: {}", addr);
                let client_builder = tonic::transport::Channel::from_shared(addr)?;
                client_builder.tls_config(config)?.connect().await?
            }
            None => {
                info!("Connecting to Confer server without TLS: {}", addr);
                tonic::transport::Channel::from_shared(addr)?.connect().await?
            }
        };

        let client = ConferServiceClient::new(channel);
        let watchers = Arc::new(RwLock::new(HashMap::new()));
        Ok(ConferClient { client, watchers })
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
    pub async fn get(&mut self, path: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        info!("Getting config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        let response = self.client.get(request).await?;
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
    pub async fn set(&mut self, path: String, value: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        info!("Setting config for path: {}", path);
        let request = Request::new(SetConfigRequest {
            path: Some(ConfigPath { path }),
            value: Some(ConfigValue { value }),
        });
        self.client.set(request).await?;
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
    pub async fn remove(&mut self, path: String) -> Result<(), Box<dyn std::error::Error>> {
        info!("Removing config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        self.client.remove(request).await?;
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
    async fn list(&mut self, path: String) -> Result<Response<Vec<String>>, Status> {
        let request = Request::new(ConfigPath { path });
        let response = self.client.list(request).await?;
        Ok(Response::new(response.into_inner().paths))
    }

    /// Watches for changes to the configuration value at the given path.
    ///
    /// Returns a stream of `WatchUpdate` messages.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to watch for changes.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Stream` of `WatchUpdate` messages, or an error.
    #[instrument(skip(self), err)]
    pub async fn watch_config(
        &mut self,
        path: String,
    ) -> Result<Response<Streaming<WatchUpdate>>, Box<dyn std::error::Error>> {
        info!("Watching config for path: {}", path);
        let request = Request::new(ConfigPath { path });
        let response = self.client.watch_config(request).await?;
        Ok(response) // No need to convert here
    }


    /// Watches for changes to the configuration value at the given path and calls a callback.
    ///
    /// This method starts a new task that receives `WatchUpdate` messages from the server
    /// and calls the provided `callback` function for each message.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to watch for changes.
    /// * `callback` - A closure to be called when a change occurs.  The closure
    ///              receives a `Result<WatchUpdate, Status>`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure in setting up the watch.
    #[instrument(skip(self, callback))]
    pub async fn watch_config_with_callback<F>(
        &mut self,
        path: String,
        callback: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(Result<WatchUpdate, Status>) + Send + Sync + 'static,
    {
        info!("Watching config with callback for path: {}", path);
        let mut stream = self.watch_config_internal(path.clone()).await?;

        let watchers = Arc::clone(&self.watchers);
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
                        break;
                    }
                    Err(e) => {
                        error!("Error watching path {}: {}", task_key, e);
                        callback(Err(e));
                        break;
                    }
                }
            }
            let mut watchers = watchers.write().await;
            watchers.remove(&task_key);
            info!("Watcher task for path {} removed", task_key);
        });

        let mut watchers = self.watchers.write().await;
        watchers.insert(path, handle);
        Ok(())
    }

    /// Internal helper to get the watch stream.
    #[instrument(skip(self), err)]
    async fn watch_config_internal(
        &mut self,
        path: String,
    ) -> Result<Streaming<WatchUpdate>, Box<dyn std::error::Error>> {
        let request = Request::new(ConfigPath { path });
        let response = self.client.watch_config(request).await?;
        Ok(response.into_inner())
    }

    /// Watches for changes in the Raft leader.
    ///
    /// Returns a stream of `WatchUpdate` messages, where the `kind` field will be
    /// a `LeaderInfo` message.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Stream` of `WatchUpdate` messages, or an error.
    #[instrument(skip(self), err)]
    pub async fn watch_leader(
        &mut self,
    ) -> Result<Response<Streaming<WatchUpdate>>, Box<dyn std::error::Error>> {
        info!("Watching for Raft leader changes.");
        let request = Request::new(Empty {});
        let response = self.client.watch_leader(request).await?;
        Ok(response)
    }

    /// Watches for changes in the Raft leader and calls a callback.
    ///
    /// This method starts a new task that receives `WatchUpdate` messages from the server
    /// and calls the provided `callback` function for each message.  The `callback`
    /// will only be called when the leader changes.
    ///
    /// # Arguments
    ///
    /// * `callback` - A closure to be called when the leader changes. The closure
    ///              receives a `Result<WatchUpdate, Status>`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    #[instrument(skip(self, callback))]
    pub async fn watch_leader_with_callback<F>(
        &mut self,
        callback: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(Result<WatchUpdate, Status>) + Send + Sync + 'static,
    {
        info!("Watching Raft leader with callback.");
        let mut stream = self.client.watch_leader(Request::new(Empty {})).await?.into_inner();

        let watchers = Arc::clone(&self.watchers);
        let task_key = "leader".to_string();
        let handle = tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(update)) => {
                         if let Some(Kind::LeaderChanged(LeaderInfo{..})) = update.kind {
                            info!("Leader changed");
                            callback(Ok(update));
                         }
                         else {
                            warn!("Received non-leader update: {:?}", update);
                         }
                    }
                    Ok(None) => {
                        warn!("Leader watch stream ended");
                        break;
                    }
                    Err(e) => {
                        error!("Error watching leader: {}", e);
                        callback(Err(e));
                        break;
                    }
                }
            }
            let mut watchers = watchers.write().await;
            watchers.remove(&task_key);
            info!("Leader watcher task removed");
        });

        let mut watchers = self.watchers.write().await;
        watchers.insert("leader".to_string(), handle);
        Ok(())
    }

    /// Cancels all active watchers.
    #[instrument(skip(self))]
    pub async fn cancel_watchers(&self) {
        let mut watchers = self.watchers.write().await;
        info!("Cancelling {} watcher tasks", watchers.len());
        for (key, handle) in watchers.iter() {
            handle.abort();
            info!("Cancelled watcher for key: {}", key);
        }
        watchers.clear();
        info!("All watcher tasks cancelled");
    }
}
