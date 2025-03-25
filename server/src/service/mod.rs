use std::sync::Arc;

use tonic::{Request, Response, Status};

use tracing::{debug, instrument, error};
use async_trait::async_trait;

use openraft::Raft;
use crate::proto::confer::v1::confer_service_server::ConferService;
use crate::proto::confer::v1::{Empty, ConfigPath, ConfigValue, ConfigList, SetConfigRequest};
use crate::raft::state_machine::StateMachine;
use crate::raft::operation::Operation;
use crate::raft::config::TypeConfig;
use crate::repository::ConferRepository;

pub struct ConferServiceImpl<CR: ConferRepository> {
    raft: Raft<TypeConfig>,
    state: Arc<StateMachine<CR>>,
}

impl <R: ConferRepository> ConferServiceImpl<R> {
    pub fn new(raft: Raft<TypeConfig>, state: Arc<StateMachine<R>>) -> Self {
        debug!("Creating new ConferServiceImpl");
        ConferServiceImpl { raft, state, }
    }
}

#[async_trait]
impl<CR: ConferRepository + 'static> ConferService for ConferServiceImpl<CR> {
    #[instrument(skip(self, request))]
    async fn get(&self, request: Request<ConfigPath>) -> Result<Response<ConfigValue>, Status> {
        let config_path = request.into_inner();
        debug!("Received get request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let repo = self.state.repository.read().await;
        match repo.get(&config_path).await {
            Ok(value) => {
                debug!("Successfully retrieved value for path: {:?}", &config_path);
                let reply = ConfigValue { value };
                Ok(Response::new(reply))
            }
            Err(error) => {
                error!("Error retrieving value: {:?}", error);
                Err(Status::not_found(format!("Failed to retrieve: {:?}", error.to_string())))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn set(&self, request: Request<SetConfigRequest>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();
        let path = config_path.path.clone().ok_or_else(|| Status::invalid_argument("Path is required"))?;
        let value = config_path.value.clone().ok_or_else(|| Status::invalid_argument("Value is required"))?;

        debug!("Received set request for path: {:?}", &path);
        if path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let set_operation = Operation::Set {
            path: path.clone(),
            value: value.clone(),
        };

        //serialise the operation as a command
        //let set_command = serde_json::to_vec(&set_operation).unwrap();

        debug!("Sending request to raft: {:?}", set_operation);
        let result = self.raft.client_write(set_operation.clone()).await;

        match result {
            Ok(_reponse) => {
                debug!("Successfully set value for path: {:?}", &path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => {
                error!("Error setting value: {:?}", error);
                Err(Status::internal(format!("Failed to Set {:?}. details: {:?}", path, error.to_string())))
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

        let remove_operation = Operation::Remove {
            path: config_path.clone(),
        };

        //serialise the operation as a command
        //let remove_command = serde_json::to_vec(&remove_operation).unwrap();

        debug!("Sending request to raft: {:?}", remove_operation);
        let result = self.raft.client_write(remove_operation.clone()).await;

        match result {
            Ok(_response) => {
                debug!("Successfully removed value for path: {:?}", &config_path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => {
                error!("Error removing value: {:?}", error);
                Err(Status::internal(format!("Failed to remove {:?}. details: {:?}", config_path.path, error.to_string())))
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


        let repo = self.state.repository.read().await;
        match repo.list(&config_path).await {
            Ok(paths) => {
                debug!("Successfully listed paths with prefix: {:?}", &config_path);
                let reply = ConfigList { paths };
                Ok(Response::new(reply))
            }
            Err(error) => {
                error!("Error listing paths: {:?}", error);
                Err(Status::internal(format!("Failed to list {:?}. details: {:?}", config_path.path, error.to_string())))
            }
        }
    }
}


// TODO: Use a trait based approach with trait `ToStatus` and  `fn to_status`
//fn map_confer_error_to_status(error: ConferError) -> Status {
    //let error_details = ErrorDetails {
        //message: error.to_string(), // Use Display implementation
    //};
    //let encoded_details = Bytes::from(error_details.encode_to_vec());

    //match error {
        //ConferError::NotFound { .. } => {
            //Status::with_details(Code::NotFound, "Path not found", encoded_details)
        //}
        //ConferError::InvalidPath { .. } => {
            //Status::with_details(Code::InvalidArgument, "Invalid path", encoded_details)
        //}
        //ConferError::Internal { .. } => {
            //Status::with_details(Code::Internal, "Internal error", encoded_details)
        //}
        //ConferError::RaftError { .. } => {
            //Status::with_details(Code::Internal, "Raft error", encoded_details)
        //}
        //ConferError::SerializationError { .. } => {
            //Status::with_details(Code::Internal, "Serialization error", encoded_details)
        //}
        //ConferError::DeserializationError { .. } => {
            //Status::with_details(Code::Internal, "Deserialization error", encoded_details)
        //}
        //ConferError::StorageError { .. } => {
            //Status::with_details(Code::Internal, "Storage error", encoded_details)
        //}
        //ConferError::UnknownError => {
            //Status::with_details(Code::Internal, "Unknown error", encoded_details)
        //}
    //}
//}

#[cfg(test)]
mod service_tests;

