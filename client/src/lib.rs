mod error;

use crate::error::ClientError;
use tonic::{Request, transport::Channel};
pub mod confer {
    pub mod v1 {
        tonic::include_proto!("confer.v1");
    }
}
use confer::v1::confer_service_client::ConferServiceClient;
use confer::v1::{ConfigPath, ConfigValue, SetConfigRequest};

/// A client for interacting with the Confer configuration service.
///
/// This client provides methods for setting, getting, deleting, and listing configuration values
/// stored in a Confer server.  It uses gRPC for communication with the server.
pub struct ConferClient {
    /// The gRPC client used to communicate with the Confer server.
    client: ConferServiceClient<Channel>,
}

impl ConferClient {
    /// Connects to the Confer server at the specified address.
    ///
    /// This function establishes a gRPC connection to the Confer server and creates a new
    /// `ConferClient` instance.
    ///
    /// # Arguments
    ///
    /// * `address`: The address of the Confer server (e.g., "http://localhost:50051").
    ///
    /// # Returns
    ///
    /// A `Result` containing the new `ConferClient` on success, or a `ClientError` on failure.
    pub async fn connect(address: String) -> Result<Self, ClientError> {
        let channel = Channel::from_shared(address)
            .map_err(ClientError::from)?
            .connect()
            .await
            .map_err(ClientError::from)?;
        let client = ConferServiceClient::new(channel);
        Ok(Self { client })
    }

    /// Sets a configuration value at the specified path.
    ///
    /// This function sends a request to the Confer server to set the configuration value
    /// at the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: The path (key) of the configuration value to set.
    /// * `value`: The value to set, as a byte array.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.  Returns `Ok(())` on success, or a `ClientError`
    /// on failure.
    pub async fn set(&mut self, path: String, value: Vec<u8>) -> Result<(), ClientError> {
        let request = Request::new(SetConfigRequest {
            path: Some(ConfigPath { path }),
            value: Some(ConfigValue { value }),
        });

        let _response = self.client.set(request).await?;
        Ok(())
    }

    /// Gets a configuration value from the specified path.
    ///
    /// This function sends a request to the Confer server to retrieve the configuration value
    /// at the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: The path (key) of the configuration value to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing the configuration value as a byte array on success, or a
    /// `ClientError` on failure.
    pub async fn get(&mut self, path: String) -> Result<Vec<u8>, ClientError> {
        let request = Request::new(ConfigPath { path });
        let response = self.client.get(request).await?;
        let r = response.into_inner();
        Ok(r.value)
    }

    /// Deletes the configuration value at the specified path.
    ///
    /// This function sends a request to the Confer server to delete the configuration value
    /// at the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: The path (key) of the configuration value to delete.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.  Returns `Ok(())` on success, or a `ClientError`
    /// on failure.
    pub async fn delete(&mut self, path: String) -> Result<(), ClientError> {
        let request = Request::new(ConfigPath { path });
        let _response = self.client.remove(request).await?;
        Ok(())
    }

    /// Lists all configuration values under the specified path.
    ///
    /// This function sends a request to the Confer server to retrieve a list of all
    /// configuration values that are children of the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: The parent path (prefix) to list configuration values under.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of configuration paths (keys) on success, or a
    /// `ClientError` on failure.
    pub async fn list(&mut self, path: String) -> Result<Vec<String>, ClientError> {
        let request = Request::new(ConfigPath { path });
        let response = self.client.list(request).await?;
        let r = response.into_inner();
        Ok(r.paths)
    }
}
