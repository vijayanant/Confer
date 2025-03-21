use bytes::Bytes;
use prost::Message;

use tonic::{Request, Response, Status, Code};

use tracing::{debug, instrument, error};
use async_trait::async_trait;

use crate::proto::confer::v1::confer_service_server::ConferService;
use crate::proto::confer::v1::{Empty, ConfigPath, ConfigValue, ConfigList, SetConfigRequest, ErrorDetails};

use crate::repository::ConferRepository;
use crate::error::ConferError;

pub struct ConferServiceImpl<T: ConferRepository> {
    repository: T, // Box<dyn ConferRepository>,
}

impl <T: ConferRepository> ConferServiceImpl<T> {
    pub fn new(repository: T) -> Self {
        debug!("Creating new ConferServiceImpl");
        ConferServiceImpl { repository }
    }
}

#[async_trait]
impl <T: ConferRepository + 'static> ConferService for ConferServiceImpl<T> {
    #[instrument(skip(self, request))]
    async fn get(&self, request: Request<ConfigPath>) -> Result<Response<ConfigValue>, Status> {
        let config_path = request.into_inner();
        debug!("Received get request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }
        match self.repository.get(&config_path).await {
            Ok(value) => {
                debug!("Successfully retrieved value for path: {:?}", &config_path);
                let reply = ConfigValue { value };
                Ok(Response::new(reply))
            }
            Err(error) => {
                error!("Error retrieving value: {:?}", error);
                Err(map_confer_error_to_status(error))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn set(&self, request: Request<SetConfigRequest>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();
        let path = config_path.path.ok_or_else(|| Status::invalid_argument("Path is required"))?;
        let value = config_path.value.ok_or_else(|| Status::invalid_argument("Value is required"))?;

        debug!("Received set request for path: {:?}", &path);
        if path.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }
        match self.repository.set(&path, value.value).await {
            Ok(_) => {
                debug!("Successfully set value for path: {:?}", &path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => {
                error!("Error setting value: {:?}", error);
                Err(map_confer_error_to_status(error))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn remove(&self, request: Request<ConfigPath>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();

        debug!("Received remove request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }
        match self.repository.remove(&config_path).await {
            Ok(_) => {
                debug!("Successfully removed value for path: {:?}", &config_path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => {
                error!("Error removing value: {:?}", error);
                Err(map_confer_error_to_status(error))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn list(&self, request: Request<ConfigPath>) -> Result<Response<ConfigList>, Status> {
        let config_path = request.into_inner();

        debug!("Received list request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }
        match self.repository.list(&config_path).await {
            Ok(paths) => {
                debug!("Successfully listed paths with prefix: {:?}", &config_path);
                let reply = ConfigList { paths };
                Ok(Response::new(reply))
            }
            Err(error) => {
                error!("Error listing paths: {:?}", error);
                Err(map_confer_error_to_status(error))
            }
        }
    }
}


// TODO: Use a trait based approach with trait `ToStatus` and  `fn to_status`
fn map_confer_error_to_status(error: ConferError) -> Status {
    let error_details = ErrorDetails {
        message: error.to_string(), // Use Display implementation
    };
    let encoded_details = Bytes::from(error_details.encode_to_vec());

    match error {
        ConferError::NotFound { .. } => {
            Status::with_details(Code::NotFound, "Path not found", encoded_details)
        }
        ConferError::InvalidPath { .. } => {
            Status::with_details(Code::InvalidArgument, "Invalid path", encoded_details)
        }
        ConferError::Internal { .. } => {
            Status::with_details(Code::Internal, "Internal error", encoded_details)
        }
        ConferError::RaftError { .. } => {
            Status::with_details(Code::Internal, "Raft error", encoded_details)
        }
        ConferError::SerializationError { .. } => {
            Status::with_details(Code::Internal, "Serialization error", encoded_details)
        }
        ConferError::DeserializationError { .. } => {
            Status::with_details(Code::Internal, "Deserialization error", encoded_details)
        }
        ConferError::StorageError { .. } => {
            Status::with_details(Code::Internal, "Storage error", encoded_details)
        }
        ConferError::UnknownError => {
            Status::with_details(Code::Internal, "Unknown error", encoded_details)
        }
    }
}

#[cfg(test)]
mod service_tests;

