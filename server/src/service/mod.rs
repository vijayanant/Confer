use std::sync::Arc;
use std::collections::BTreeMap;

use tonic::{Request, Response, Status};
use tonic::transport::Channel;

use tracing::{info, debug, instrument, error};
use async_trait::async_trait;

use openraft::Raft;
use openraft::error::{RaftError, ClientWriteError::ForwardToLeader};

use crate::proto::confer::v1::{
    confer_service_client::ConferServiceClient,
    confer_service_server::ConferService,
    Empty, ConfigPath, ConfigValue, ConfigList, SetConfigRequest,
    InitRequest, AddLearnerRequest, ChangeMembershipRequest,
    ClientWriteResponse, Membership, NodeIdSet, Node};

use crate::raft::{
    state_machine::StateMachine,
    operation::Operation,
    config:: {TypeConfig, Node as ConferNode}};

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

        info!("Received set request for path: {:?}", &path);
        if path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let set_operation = Operation::Set {
            path: path.clone(),
            value: value.clone(),
        };

        debug!("Sending request to raft: {:?}", set_operation);
        let result = self.raft.client_write(set_operation.clone()).await;

        match result {
            Ok(_reponse) => {
                info!("Successfully set value for path: {:?}", &path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding Set request to leader: {:?}", node);
                        let channel = Channel::from_shared(node.addr.clone())
                            .map_err(|e| Status::invalid_argument(format!("Failed to create channel: {}", e)))?
                            .connect()
                            .await
                            .map_err(|e| Status::unavailable(format!("Failed to connect to leader: {}", e)))?;

                        let mut client  = ConferServiceClient::new(channel);
                        let res = client.set(Request::new(SetConfigRequest{
                            path: Some(path),
                            value: Some(value)
                        })).await?;
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!("Failed to forward. Leader not known: {}", leader)))
                    }
                }
                e => {
                    error!("Error setting value: {:?}", e);
                    Err(Status::internal(format!("Failed to Set {:?}. details: {:?}", path, e.to_string())))
                }
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn remove(&self, request: Request<ConfigPath>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();

        info!("Received remove request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let remove_operation = Operation::Remove {
            path: config_path.clone(),
        };

        debug!("Sending request to raft: {:?}", remove_operation);
        let result = self.raft.client_write(remove_operation.clone()).await;

        match result {
            Ok(_response) => {
                info!("Successfully removed value for path: {:?}", &config_path);
                Ok(Response::new(Empty {}))
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding Remove request to leader: {:?}", node);
                        let channel = Channel::from_shared(node.addr.clone())
                            .map_err(|e| Status::invalid_argument(format!("Failed to create channel: {}", e)))?
                            .connect()
                            .await
                            .map_err(|e| Status::unavailable(format!("Failed to connect to leader: {}", e)))?;

                        let mut client  = ConferServiceClient::new(channel);
                        let res = client.remove(Request::new(config_path)).await?;
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!("Failed to forward. Leader not known: {}", leader)))
                    }
                }
                e => {
                    error!("Error removing value: {:?}", e);
                    Err(Status::internal(format!("Failed to remove {:?}. details: {:?}", config_path.path, e.to_string())))
                }
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

    async fn init(
        &self,
        request: Request<InitRequest>,
    ) -> Result<Response<Empty>, Status> {
        debug!("Initializing Raft cluster");
        let request = request.into_inner();
        let nodes: BTreeMap<u64, ConferNode> = request
            .nodes
            .into_iter()
            .map (|node| {
                (node.node_id, ConferNode {
                    addr : node.addr,
                    node_id: node.node_id,
                    custom_data: "".to_string(),
                })
            })
            .collect();

        let _result = self.raft
            .initialize(nodes)
            .await
            .map_err(|e| Status::internal(format!("Failed to initialize cluster {}", e)));

        debug!("Cluster initialization successful");
        Ok(Response::new(Empty {}))
    }

    async fn add_learner(
        &self,
        request: Request<AddLearnerRequest>,
        ) -> Result<Response<ClientWriteResponse>,Status>
    {
        let req = request.into_inner();
        let node = req.node.ok_or_else(|| Status::internal("Node information is required"))?;
        debug!("Adding learner node {}", node.node_id);

        let confer_node = ConferNode {
            addr: node.addr.clone(),
            node_id: node.node_id,
            custom_data: "".to_string(),
        };

        let result = self
            .raft
            .add_learner(confer_node.node_id, confer_node, true)
            .await
            .map_err(|e| Status::internal(format!("Failed to add learner node: {}", e)))?;

        debug!("Successfully added learner node {}", node.node_id);

        //let log_id = Some(LogId {
            //term : result.log_id.committed_leader_id().term,
            //index: result.log_id.index,
        //});

        if let Some(membership) = result.membership() {
            let configs: Vec<NodeIdSet> = membership
                .get_joint_config()
                .into_iter()
                .map(|m| {
                    NodeIdSet {
                        node_ids: m.iter().map(|key| {
                            (*key, Empty {})
                        }).collect()
                    }
                })
                .collect();
            let nodes = membership
                .nodes()
                .map(|(nid, n)| {
                    (*nid, Node {
                        node_id: *nid,
                        addr:  (*n.addr).to_string(),
                    })
                }).collect();

            let membership = Membership {
                configs: configs,
                nodes: nodes,
            };

            return Ok(Response::new(ClientWriteResponse {
                membership: Some(membership)
            }))
        } else {
            return Err(Status::internal(format!("membership not retrieved")))
        }
    }

    async fn change_membership(
        &self,
        request: Request<ChangeMembershipRequest>,
        ) -> Result<Response<ClientWriteResponse>, Status>
    {
        let req = request.into_inner();
        debug!(
            "Changing membership. Members: {:?}, Retain: {}",
            req.members, req.retain
        );

        let result = self
            .raft
            .change_membership(req.members, req.retain)
            .await
            .map_err(|e| Status::internal(format!("Failed to change membership: {}", e)))?;

        if let Some(membership) = result.membership() {
            let configs: Vec<NodeIdSet> = membership
                .get_joint_config()
                .into_iter()
                .map(|m| {
                    NodeIdSet {
                        node_ids: m.iter().map(|key| {
                            (*key, Empty {})
                        }).collect()
                    }
                })
                .collect();
            let nodes = membership
                .nodes()
                .map(|(nid, n)| {
                    (*nid, Node {
                        node_id: *nid,
                        addr:  (*n.addr).to_string(),
                    })
                }).collect();

            let membership = Membership {
                configs: configs,
                nodes: nodes,
            };

            return Ok(Response::new(ClientWriteResponse {
                membership: Some(membership)
            }))
        } else {
            return Err(Status::internal(format!("membership not retrieved")))
        }
    }

}

#[cfg(test)]
mod service_tests;

