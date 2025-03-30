use tracing::debug;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use openraft::Raft;

use crate::raft::config::TypeConfig;
use crate::raft::proto::raft_service_server::RaftService;
use crate::raft::proto::GenericMessage;

pub struct RaftServiceImpl {
    raft: Raft<TypeConfig>,
}

impl RaftServiceImpl {
    pub fn new(raft: Raft<TypeConfig>) -> Self {
        RaftServiceImpl { raft }
    }
}

#[tonic::async_trait]
impl RaftService for RaftServiceImpl {
    async fn vote(&self, request: Request<GenericMessage>) -> Result<Response<GenericMessage>, Status> {
        let generic_message = request.into_inner();
        let deserialized = serde_json::from_slice(&generic_message.data)
            .map_err(|e| Status::internal(format!("deserialization error: {:?}",e)))?;

        debug!("Received Vote request: {:?}", deserialized);

        let resp = self.raft
            .vote(deserialized)
            .await;

        match resp {
            Err(e) => {
                debug!("vote operation failed: {:?},", e);
                return  Err(Status::internal(format!("vote operation failed: {:?}", e)));
            }
            Ok(r) => {
                debug!("vote response: {:?}", r);
                let serialized = serde_json::to_vec(&r)
                    .map_err(|e| Status::internal(format!("serialization error: {:?}",e)))?;
                let generic_message = GenericMessage { data : serialized};
                return Ok(Response::new(generic_message));
            }
        }
    }

    async fn append_entries(&self, request: Request<GenericMessage>, ) -> Result<Response<GenericMessage>, Status> {
        let generic_message = request.into_inner();
        let deserialized = serde_json::from_slice(&generic_message.data)
            .map_err(|e| Status::internal(format!("deserialization error: {:?}",e)))?;

        debug!("Received AppendEntries request: {:?}", deserialized);

        let resp = self.raft
            .append_entries(deserialized)
            .await;

        match resp {
            Err(e) => {
                debug!("append_entries operation failed: {:?},", e);
                return  Err(Status::internal(format!("append_entries operation failed: {:?}", e)));
            }
            Ok(r) => {
                debug!("append_entries response: {:?}", r);
                let serialized = serde_json::to_vec(&r)
                    .map_err(|e| Status::internal(format!("serialization error: {:?}",e)))?;
                let generic_message = GenericMessage { data : serialized};
                return Ok(Response::new(generic_message));
            }
        }
    }

    async fn install_snapshot(&self, request: Request<GenericMessage>, ) -> Result<Response<GenericMessage>, Status> {
        let generic_message = request.into_inner();
        let deserialized = serde_json::from_slice(&generic_message.data)
            .map_err(|e| Status::internal(format!("deserialization error: {:?}",e)))?;

        debug!("Received InstallSnapsot request: {:?}", deserialized);

        let resp = self.raft
            .install_snapshot(deserialized)
            .await;

        match resp {
            Err(e) => {
                debug!("AppendEntries operation failed: {:?},", e);
                return  Err(Status::internal(format!("AppendEntries failed: {:?}", e)));
            }
            Ok(r) => {
                debug!("AppendEntries response: {:?}", r);
                let serialized = serde_json::to_vec(&r)
                    .map_err(|e| Status::internal(format!("serialization error: {:?}",e)))?;
                let generic_message = GenericMessage { data : serialized};
                return Ok(Response::new(generic_message));
            }
        }
    }
}
