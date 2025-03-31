use std::fmt;

use tonic::{Request, transport::Channel, Status};
use http::uri::InvalidUri;

use confer::v1::confer_service_client::ConferServiceClient;
use confer::v1::{ConfigPath, ConfigValue, SetConfigRequest};

pub mod confer {
    pub mod v1 {
        tonic::include_proto!("confer.v1");
    }
}


#[derive(Debug)]
pub enum ClientError {
    ConnectionError(String),
    GrpcError(Status),
    Other(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::ConnectionError(e) => write!(f, "Connection error: {}", e),
            ClientError::GrpcError(s) => write!(f, "gRPC error: {}", s),
            ClientError::Other(s) => write!(f, "Other error: {}", s),
        }
    }
}

impl std::error::Error for ClientError {}


impl From<tonic::transport::Error> for ClientError {
    fn from(e: tonic::transport::Error) -> Self {
        ClientError::ConnectionError(e.to_string())
    }
}

impl From<tonic::Status> for ClientError {
    fn from(s: tonic::Status) -> Self {
        ClientError::GrpcError(s)
    }
}

impl From<String> for ClientError {
    fn from(s: String) -> Self {
        ClientError::Other(s)
    }
}

impl From<InvalidUri> for ClientError {
    fn from(e: InvalidUri) -> Self {
        ClientError::ConnectionError(e.to_string())
    }
}

// Connect to the Confer server
pub async fn connect(address: String) -> Result<ConferServiceClient<Channel>, ClientError> {
    let channel = Channel::from_shared(address)
        .map_err(ClientError::from)?
        .connect()
        .await
        .map_err(ClientError::from)?;
    let client = ConferServiceClient::new(channel);
    Ok(client)
}

// Set a configuration value
pub async fn set_value(
    client: &mut ConferServiceClient<Channel>,
    path: String,
    value: Vec<u8>,
) -> Result<(), ClientError>
{
    let request = Request::new(SetConfigRequest {
        path: Some(ConfigPath{path}),
        value: Some(ConfigValue{value}),
    });

    let _response = client.set(request).await?;
    Ok(())
}

// Get a configuration value
pub async fn get_value(
    client: &mut ConferServiceClient<Channel>,
    path: String,
) -> Result<Vec<u8>, ClientError>
{
    let request = Request::new(ConfigPath { path, });
    let response = client.get(request).await?;
    let r = response.into_inner();
    Ok(r.value)
}

// Removing a config
pub async fn delete_value(
    client: &mut ConferServiceClient<Channel>,
    path: String,
) -> Result<(), ClientError> {
    let request = Request::new(ConfigPath { path });
    let _response = client.remove(request).await?;
    Ok(())
}

// Get all configurations under the given path
pub async fn list_values(
    client: &mut ConferServiceClient<Channel>,
    path: String,
) -> Result<Vec<String>, ClientError> {
    let request = Request::new(ConfigPath { path });
    let response = client.list(request).await?;
    let r = response.into_inner();
    Ok(r.paths)
}
