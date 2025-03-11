use thiserror::Error;

use openraft::error::RaftError;

use openraft::RaftTypeConfig;
use crate::raft::config::TypeConfig;

pub type MyNodeId = <TypeConfig as RaftTypeConfig>::NodeId;
pub type MyNode = <TypeConfig as RaftTypeConfig>::Node;


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
        ConferError::SerializationError {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ConferError {
    fn from(err: std::io::Error) -> Self {
        ConferError::StorageError {
            message: err.to_string(),
        }
    }
}

impl From<RaftError<MyNodeId>> for ConferError {
    fn from(err: RaftError<MyNodeId>) -> Self {
        ConferError::RaftError {
            message: format!("Raft Error: {}", err.to_string()),
        }
    }
}

