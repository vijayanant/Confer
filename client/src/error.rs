use std::fmt;
use http::uri::InvalidUri;
use tonic::Status;

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
