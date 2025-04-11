use crate::raft::config::NodeId;
use openraft::error::RaftError;
use thiserror::Error;
use tracing::error;

/// Custom error type for the Confer application.
///
/// This enum defines the various error conditions that can occur within the Confer
/// application.  It uses the `thiserror` crate to automatically generate
/// implementations for the `Error` trait.
#[derive(Error, Default, Debug)]
pub enum ConferError {
    #[default]
    #[error("Unknown error")]
    UnknownError,

    #[error("Path not found: {path}")]
    NotFound { path: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Deserialization error: {message}")]
    DeserializationError { message: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Raft error: {message}")]
    RaftError { message: String },
}

impl From<serde_json::Error> for ConferError {
    fn from(err: serde_json::Error) -> Self {
        let msg = err.to_string();
        error!("Serialization error: {}", msg);
        ConferError::SerializationError { message: msg }
    }
}

impl From<std::io::Error> for ConferError {
    fn from(err: std::io::Error) -> Self {
        let msg = err.to_string();
        error!("Storage error: {}", msg);
        ConferError::StorageError { message: msg }
    }
}

impl From<RaftError<NodeId>> for ConferError {
    fn from(err: RaftError<NodeId>) -> Self {
        let msg = format!("Raft Error: {}", err);
        error!("Raft error: {}", msg);
        ConferError::RaftError { message: msg }
    }
}
