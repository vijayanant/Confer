use bytes::Bytes;
use prost::Message;

use tonic::{Request, Response, Status, Code};

use tracing::{debug, instrument, error};
use async_trait::async_trait;

use crate::proto::confer::v1::confer_service_server::ConferService;
use crate::proto::confer::v1::{Empty, ConfigPath, ConfigValue, ConfigList, SetConfigRequest, ErrorDetails};

use crate::state_machine::StateMachine;
use crate::error::ConferError;

pub struct ConferServiceImpl {
    state_machine: Box<dyn StateMachine>,
}

impl ConferServiceImpl {
    pub fn new(state_machine: Box<dyn StateMachine>) -> Self {
        debug!("Creating new ConferServiceImpl");
        ConferServiceImpl { state_machine }
    }
}

fn map_confer_error_to_status(error: ConferError) -> Status {
    match error {
        ConferError::NotFound { path } => {
            let error_details = ErrorDetails {
                message: format!("Path not found: {}", path),
            };
            let encoded_details = Bytes::from(error_details.encode_to_vec());
            Status::with_details(Code::NotFound, "Path not found", encoded_details)
        }
        ConferError::InvalidPath { path } => {
            let error_details = ErrorDetails {
                message: format!("Invalid path: {}", path),
            };
            let encoded_details = Bytes::from(error_details.encode_to_vec());
            Status::with_details(Code::InvalidArgument, "Invalid path", encoded_details)
        }
        ConferError::Internal { message } => {
            let error_details = ErrorDetails {
                message: format!("Internal error: {}", message),
            };
            let encoded_details = Bytes::from(error_details.encode_to_vec());
            Status::with_details(Code::Internal, "Internal error", encoded_details)
        }
        ConferError::RaftError { message } => {
            let error_details = ErrorDetails {
                message: format!("Raft error: {}", message),
            };
            let encoded_details = Bytes::from(error_details.encode_to_vec());
            Status::with_details(Code::Internal, "Raft error", encoded_details)
        }
    }
}

#[async_trait]
impl ConferService for ConferServiceImpl {
    #[instrument(skip(self, request))]
    async fn get(&self, request: Request<ConfigPath>) -> Result<Response<ConfigValue>, Status> {
        let config_path = request.into_inner();

        debug!("Received get request for path: {:?}", &config_path);
        match self.state_machine.get(&config_path).await {
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
        match self.state_machine.set(&path, value.value).await {
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
        match self.state_machine.remove(&config_path).await {
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
        match self.state_machine.list(&config_path).await {
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
