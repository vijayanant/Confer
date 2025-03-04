use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConferError {
    #[error("Path not found: {path}")]
    NotFound { path: String },
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
    // Add more specific error types as needed
    #[error("Raft error: {message}")]
    RaftError { message: String },
}
