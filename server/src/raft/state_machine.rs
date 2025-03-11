use async_trait::async_trait;
use crate::raft::config::TypeConfig;
use crate::raft::operation::{Operation, OperationResponse};
use crate::repository::ConferRepository;
use crate::proto::confer::v1::ConfigPath;
use serde::{Deserialize, Serialize};
use openraft_memstore::MemStore;
use openraft::storage::Adaptor;
use openraft::StorageError;
use openraft::RaftTypeConfig;
use tonic::Response;
use crate::error::ConferError;

type LogEntry = <TypeConfig as RaftTypeConfig>::Entry;
type NodeId = <TypeConfig as RaftTypeConfig>::NodeId;


#[derive(Clone, Debug)]
pub struct ConferRepositoryAdaptor<T: ConferRepository> {
    pub repository: T,
}

impl<T: ConferRepository> ConferRepositoryAdaptor<T> {
    pub fn new(repository: T) -> Self {
        ConferRepositoryAdaptor { repository }
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<Response<TypeConfig>>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = LogEntry> + Send,
    {
        let mut responses = Vec::new();
        for log_entry in entries {
            let response = match &log_entry.payload {
                openraft::entry::EntryPayload::Normal(op) => {
                    match op {
                        Operation::Set { path, value } => {
                            let confer_path = ConfigPath { path: path.path.clone() };
                            self.repository
                                .set(&confer_path, value.value.clone())
                                .await
                                .map_err(|e| StorageError::Other(e.to_string()))?;
                            Ok(Response::new(OperationResponse::Success))
                        }
                        Operation::Remove { path } => {
                            let confer_path = ConfigPath { path: path.path.clone() };
                            self.repository
                                .remove(&confer_path)
                                .await
                                .map_err(|e| StorageErrorr::Other(e.to_string()))?;
                            Ok(Response::new(OperationResponse::Success))
                        }
                    }
                }
                openraft::entry::EntryPayload::Membership { .. } => Ok(Response::new(OperationResponse::Success)),
                openraft::entry::EntryPayload::Blank => Ok(Response::new(OperationResponse::Success)),
            }?;
            responses.push(response);
        }
        Ok(responses)
    }

    //fn apply(&mut self, log_entry: &LogEntry<TypeConfig>) -> Result<Response<TypeConfig>, StorageError> {
        //match &log_entry.payload {
            //openraft::entry::EntryPayload::Normal { data: op } => {
                //match op {
                    //Operation::Set { path, value } => {
                        //let confer_path = ConfigPath { path: path.path.clone() };
                        //self.repository
                            //.set(&confer_path, value.value.clone())
                            //.await
                            //.map_err(|e| StorageError::Other(e.to_string()))?;
                        //Ok(OperationResponse::Success)
                    //}
                    //Operation::Remove { path } => {
                        //let confer_path = ConfigPath { path: path.path.clone() };
                        //self.repository
                            //.remove(&confer_path)
                            //.await
                            //.map_err(|e| StorageError::Other(e.to_string()))?;
                        //Ok(OperationResponse::Success)
                    //}
                //}
            //}
            //openraft::entry::EntryPayload::Membership { .. } => Ok(OperationResponse::Success),
            //openraft::entry::EntryPayload::Blank => Ok(OperationResponse::Success),
        //}
    //}
}

// Create the type alias
type ConferAdaptor = Adaptor<TypeConfig, MemStore>;

