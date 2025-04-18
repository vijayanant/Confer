use std::collections::BTreeMap;
use std::sync::Arc;
use std::pin::Pin;

use tokio::sync::RwLock;
use tokio_stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use tonic::transport::Channel;
use tonic::{Request, Response, Status};

use async_trait::async_trait;
use tracing::{debug, error, info, instrument};

use openraft::error::{ClientWriteError::ForwardToLeader, RaftError};
use openraft::Raft;

use crate::proto::confer::v1::{
    confer_service_client::ConferServiceClient,
    confer_service_server::ConferService,
    watch_update::Kind,
    AddLearnerRequest, ChangeMembershipRequest, ClientWriteResponse, ConfigList, ConfigPath,
    ConfigValue, Empty, InitRequest, Membership, Node, NodeIdSet, SetConfigRequest,
    WatchUpdate, LeaderInfo,
};

use crate::raft::{
    config::{NodeId, Node as ConferNode, TypeConfig},
    operation::Operation,
    state_machine::StateMachine,
    network::NetworkFactory,
};

use crate::repository::ConferRepository;
use crate::watchman::WatchMan;

/// The implementation of the Confer service.
///
/// This struct holds the Raft instance and the state machine, and provides
/// the implementation for the Confer service's gRPC methods.
pub struct ConferServiceImpl<CR: ConferRepository> {
    raft          : Raft<TypeConfig>,
    state         : Arc<StateMachine<CR>>,
    network       : Arc<NetworkFactory>,
    watch_manager : Arc<WatchMan>,
    is_leader     : Arc<RwLock<bool>>,
}



impl<R: ConferRepository> ConferServiceImpl<R> {
    /// Creates a new ConferServiceImpl.
    ///
    /// # Arguments
    ///
    /// * `raft`: The Raft instance.
    ///     -  A Raft instance is used for distributed consensus.  It manages
    ///        the replication of configuration data across multiple nodes.
    /// * `state`: The state machine.
    ///     -  The state machine is responsible for applying the configuration changes
    ///        to the underlying data store (the repository).  It also handles
    ///        reading data from the store.
    pub fn new(
        raft: Raft<TypeConfig>, state: Arc<StateMachine<R>>, network: Arc<NetworkFactory>, buffer_size: usize, ) -> Self
    {
        debug!("Creating new ConferServiceImpl");
        let watch_manager = Arc::new(WatchMan::new(buffer_size));
        let is_leader = Arc::new(RwLock::new(false));
        let this = ConferServiceImpl { raft, state, network, watch_manager, is_leader };
        this.clone().spawn_leader_watch_task();
        this
    }

    // To allow cloning and passing to the task
    fn clone(&self) -> Self {
        ConferServiceImpl {
            raft          : self.raft.clone(),
            state         : self.state.clone(),
            network       : self.network.clone(),
            watch_manager : self.watch_manager.clone(),
            is_leader     : self.is_leader.clone(),
        }
    }

    /// Spawns a background task to monitor leadership changes.
    fn spawn_leader_watch_task(self) {
        tokio::spawn(async move {
            let mut metrics = self.raft.server_metrics();
            let watch_manager = self.watch_manager.clone();
            let mut was_leader = self.is_leader.write().await;

            while metrics.changed().await.is_ok() {
                let current_metrics = metrics.borrow().clone();
                let is_leader = current_metrics.state.is_leader();

                if is_leader != *was_leader {
                    // leadership changed
                    *self.is_leader.write().await = is_leader;
                    *was_leader = is_leader;

                    if !is_leader {
                        // Only the previous leader notifies its watchers.
                        if let Some(leader_id) = current_metrics.current_leader {
                            if let Some(leader_node) = current_metrics
                                .membership_config
                                .nodes()
                                .find(|(id, _)| **id == leader_id)
                                .map(|(_, node)| node.clone())
                            {
                                //let message = WatchUpdate::LeaderChanged (leader_node.addr);
                                let message = WatchUpdate {
                                    kind: Some(Kind::LeaderChanged(
                                        LeaderInfo {address: leader_node.addr}
                                    )
                                )};
                                watch_manager.notify_leader_change(message).await;
                                watch_manager.clear().await;
                                watch_manager.clear_leader_watchers().await;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Helper function to create a ConferServiceClient.
    async fn create_confer_client(
        &self,
        address: String,
    ) -> Result<ConferServiceClient<Channel>, Status> {
        let channel = Channel::from_shared(address.clone())
            .map_err(|e| Status::internal(format!("Failed to create channel: {}", e)))?;
        let channel = channel.connect().await.map_err(|e| {
            Status::internal(format!("Failed to connect to {}: {}", address, e))
        })?;
        Ok(ConferServiceClient::new(channel))
    }

    /// Helper function to create a ClientWriteResponse from a MembershipConfig.
    // TODO : This trasformation is required because we created our own proto types and not use
    // the proto types used in openraft. Consider eliminating this.
    fn create_client_write_response(
        &self,
        membership: &openraft::Membership<NodeId, ConferNode>,
    ) -> Result<ClientWriteResponse, Status> {
        let configs: Vec<NodeIdSet> = membership
            .get_joint_config()
            .iter()
            .map(|m| NodeIdSet {
                node_ids: m.iter().map(|key| (*key, Empty {})).collect(),
            })
            .collect();
        let nodes = membership
            .nodes()
            .map(|(nid, n)| {
                (
                    *nid,
                    Node {
                        node_id: *nid,
                        addr: (*n.addr).to_string(),
                    },
                )
            })
            .collect();

        let membership = Membership {
            configs,
            nodes,
        };
        Ok(ClientWriteResponse {
            membership: Some(membership),
        })
    }
}

#[async_trait]
impl<CR: ConferRepository + 'static> ConferService for ConferServiceImpl<CR> {

    /// Watches for changes to the configuration value at the given path.
    type WatchConfigStream =
        Pin<Box<dyn Stream<Item = Result<WatchUpdate, Status>> + Send + 'static>>;

    /// Watches for changes to the status of the raft node.
    type WatchLeaderStream =
        Pin<Box<dyn Stream<Item = Result<WatchUpdate, Status>> + Send + 'static>>;

    /// Retrieves a configuration value by path.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the configuration path.
    ///     -  The `ConfigPath` contains the path of the configuration
    ///        value to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Result<Response<ConfigValue>, Status>`: The result of the operation.
    ///     -  On success, returns the configuration value wrapped in a `Response`.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn get(&self, request: Request<ConfigPath>) -> Result<Response<ConfigValue>, Status> {
        let config_path = request.into_inner(); // Extract the ConfigPath from the Request.
        debug!("Received get request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let repo = self.state.repository.read().await; // Acquire a read lock on the repository.
        match repo.get(&config_path).await {
            Ok(value) => {
                debug!("Successfully retrieved value for path: {:?}", &config_path);
                let reply = ConfigValue { value }; // Wrap the value in a ConfigValue.
                Ok(Response::new(reply)) // Wrap the ConfigValue in a Response.
            }
            Err(error) => {
                error!("Error retrieving value: {:?}", error);
                Err(Status::not_found(format!(
                    "Failed to retrieve: {:?}",
                    error.to_string()
                )))
            }
        }
    }

    /// Sets a configuration value by path.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the configuration path and value.
    ///     -  The `SetConfigRequest` contains the path and the new value
    ///        for the configuration setting.
    ///
    /// # Returns
    ///
    /// * `Result<Response<Empty>, Status>`: The result of the operation.
    ///     -  On success, returns an empty `Response`.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn set(&self, request: Request<SetConfigRequest>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();
        let path = config_path.path.clone()
            .ok_or_else(|| Status::invalid_argument("Path is required"))?;
        let value = config_path.value.clone()
            .ok_or_else(|| Status::invalid_argument("Value is required"))?;

        info!("Received set request for path: {:?}", &path);
        if path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let set_operation = Operation::Set {
            // Create a Set operation.
            path: path.clone(),
            value: value.clone(),
        };

        debug!("Sending request to raft: {:?}", set_operation);
        let result = self.raft.client_write(set_operation.clone()).await; // Send to Raft.

        match result {
            Ok(_reponse) => {
                info!("Successfully set value for path: {:?}", &path);
                // Notify watchers
                let message = WatchUpdate {
                    kind: Some(Kind::UpdatedValue(value.value)
                )};
                self.watch_manager.notify(&path.path, message).await;
                Ok(Response::new(Empty {}))
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    // Handle forwarding.
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding Set request to leader: {:?}", node);
                        let mut client = self.create_confer_client(node.addr.clone()).await?;
                        let res = client
                            .set(Request::new(SetConfigRequest {
                                // Forward the request.
                                path: Some(path),
                                value: Some(value),
                            }))
                            .await?;
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!(
                            "Failed to forward. Leader not known: {}",
                            leader
                        )))
                    }
                }
                e => {
                    error!("Error setting value: {:?}", e);
                    Err(Status::internal(format!(
                        "Failed to Set {:?}. details: {:?}",
                        path,
                        e.to_string()
                    )))
                }
            },
        }
    }

    /// Removes a configuration value by path.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the configuration path.
    ///     - The `ConfigPath` contains the path of the config to remove.
    ///
    /// # Returns
    ///
    /// * `Result<Response<Empty>, Status>`: The result of the operation.
    ///     -  On success, returns an empty `Response`.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn remove(&self, request: Request<ConfigPath>) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner(); // Extract ConfigPath.

        info!("Received remove request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let remove_operation = Operation::Remove {
            // Create Remove operation.
            path: config_path.clone(),
        };

        debug!("Sending request to raft: {:?}", remove_operation);
        let result = self.raft.client_write(remove_operation.clone()).await; // Send to Raft.

        match result {
            Ok(_response) => {
                info!("Successfully removed value for path: {:?}", &config_path);
                // Notify watchers
                let message = WatchUpdate {
                    kind: Some(Kind::Removed(Empty {}))
                };
                self.watch_manager.notify(&config_path.path, message).await;
                Ok(Response::new(Empty {}))
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    // Handle forwarding.
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding Remove request to leader: {:?}", node);
                        let mut client = self.create_confer_client(node.addr.clone()).await?;
                        let res = client.remove(Request::new(config_path)).await?; // Forward.
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!(
                            "Failed to forward. Leader not known: {}",
                            leader
                        )))
                    }
                }
                e => {
                    error!("Error removing value: {:?}", e);
                    Err(Status::internal(format!(
                        "Failed to remove {:?}. details: {:?}",
                        config_path.path,
                        e.to_string()
                    )))
                }
            },
        }
    }

    /// Lists configuration values by path.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the configuration path.
    ///     - The `ConfigPath` specifies the prefix to list configuration
    ///       values under.
    ///
    /// # Returns
    ///
    /// * `Result<Response<ConfigList>, Status>`: The result of the operation.
    ///     -  On success, returns a `ConfigList` wrapped in a `Response`.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn list(&self, request: Request<ConfigPath>) -> Result<Response<ConfigList>, Status> {
        let config_path = request.into_inner(); // Extract ConfigPath

        debug!("Received list request for path: {:?}", &config_path);
        if config_path.path.is_empty() {
            error!("Path can not be empty: {:?}", &config_path);
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let repo = self.state.repository.read().await; // Read from repo.
        match repo.list(&config_path).await {
            Ok(paths) => {
                debug!("Successfully listed paths with prefix: {:?}", &config_path);
                let reply = ConfigList { paths }; // Construct ConfigList.
                Ok(Response::new(reply))
            }
            Err(error) => {
                error!("Error listing paths: {:?}", error);
                Err(Status::internal(format!(
                    "Failed to list {:?}. details: {:?}",
                    config_path.path,
                    error.to_string()
                )))
            }
        }
    }


    #[instrument(skip(self, request))]
    async fn watch_config(
        &self,
        request: Request<ConfigPath>,
    ) -> Result<Response<Self::WatchConfigStream>, Status>
    {
        let config_path = request.into_inner();
        info!("Received watch config request for path: {:?}", &config_path);

        let rx = self.watch_manager.watch(config_path.path.clone()).await.map_err(|e| {
            Status::internal(format!("Failed to subscribe to watcher: {}", e))
        })?;

        let stream = BroadcastStream::new(rx).map(move |result| {
            result.map_err(|e| Status::cancelled(format!("{}",e)))
        });

        let initial_value = self.get(Request::new(config_path.clone())).await.ok().map(|resp| resp.into_inner().value);
        let initial_update = if let Some(value) = initial_value {
            WatchUpdate {
                kind: Some(Kind::UpdatedValue(value)),
            }
        } else {
             WatchUpdate {
                kind: Some(Kind::Removed(Empty {})),
            }
        };

        let stream = tokio_stream::once(Ok(initial_update)).chain(stream);
        Ok(Response::new(Box::pin(stream) as Self::WatchConfigStream))
    }

    #[instrument(skip(self, request))]
    async fn unwatch_config( &self, request: Request<ConfigPath>,) -> Result<Response<Empty>, Status> {
        let config_path = request.into_inner();
        info!("Received unwatch config request for path: {:?}", &config_path);

        let _ = self.watch_manager.unwatch(&config_path.path);

        Ok(Response::new(Empty{}))
    }

    #[instrument(skip(self))]
    async fn watch_leader(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::WatchLeaderStream>, Status>
    {

        info!("Received watch leader request");

        let rx = self.watch_manager.watch_leader().await.map_err(|e| {
            Status::internal(format!("Failed to subscribe to leader: {}", e))
        })?;

        // Convert the receiver into a stream, mapping errors as necessary.
        let stream = BroadcastStream::new(rx).map(move |result| {
            result.map_err(|e| Status::cancelled(format!("{}",e)))
        });


        Ok(Response::new(Box::pin(stream) as Self::WatchConfigStream))

        // Return the stream.
        //Ok(Response::new(Box::pin(stream) as Self::WatchLeaderStream))
    }


    /// Initializes the Raft cluster.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the nodes to initialize the cluster with.
    ///     -  The `InitRequest` contains a list of nodes that will form the
    ///        initial Raft cluster.
    ///
    /// # Returns
    ///
    /// * `Result<Response<Empty>, Status>`: The result of the operation.
    ///     -  On success, returns an empty `Response`.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn init(&self, request: Request<InitRequest>) -> Result<Response<Empty>, Status> {
        debug!("Initializing Raft cluster with request: {:?}", request);
        let request = request.into_inner(); // Extract InitRequest.

        // Convert the node information from the request into a BTreeMap, which is
        // the data structure expected by the Raft::initialize method.  The BTreeMap
        // maps node IDs to ConferNode instances.
        let nodes: BTreeMap<u64, ConferNode> = request
            .nodes
            .into_iter()
            .map(|node| {
                (
                    node.node_id,
                    ConferNode {
                        // Convert to ConferNode.
                        addr: node.addr,
                        node_id: node.node_id,
                        custom_data: "".to_string(),
                    },
                )
            })
            .collect();

        // Initialize the Raft cluster.  This is a critical step that
        // establishes the initial membership of the cluster and elects a leader.
        let result = self
            .raft
            .initialize(nodes) // Initialize Raft.
            .await
            .map_err(|e| {
                error!("Failed to initialize cluster: {:?}", e);
                Status::internal(format!("Failed to initialize cluster {}", e))
            });

        match result {
            Ok(_) => {
                info!("Cluster initialization successful");
                Ok(Response::new(Empty {}))
            }
            Err(e) => Err(e),
        }
    }

    /// Adds a learner to the Raft cluster.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the node to add as a learner.
    ///     -  The `AddLearnerRequest` contains the information about the
    ///        node to be added as a learner.
    ///
    /// # Returns
    ///
    /// * `Result<Response<ClientWriteResponse>, Status>`: The result of the operation.
    ///      - On success, returns the `ClientWriteResponse`  which includes
    ///        the new membership information.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn add_learner(
        &self,
        request: Request<AddLearnerRequest>,
    ) -> Result<Response<ClientWriteResponse>, Status> {
        let req = request.into_inner(); // Extract AddLearnerRequest.

        // Extract the node information from the request.  If the node
        // information is missing, return an error.
        let node = req.node.clone().ok_or_else(|| {
            error!("Node information is required for AddLearnerRequest");
            Status::internal("Node information is required")
        })?;
        debug!("Adding learner node {}", node.node_id);

        // Convert the node information into a ConferNode instance, which
        // is used by the Raft::add_learner method.
        let confer_node = ConferNode {
            addr: node.addr.clone(),
            node_id: node.node_id,
            custom_data: "".to_string(),
        };

        // Add the learner node to the Raft cluster.  Learners receive log
        // entries from the leader but do not participate in voting.
        let result = self
            .raft
            .add_learner(confer_node.node_id, confer_node, true)
            .await;

        match result {
            Ok(response) => {
                info!("Successfully added learner node {}", node.node_id);
                // Construct the response, which includes the updated membership
                // information.  This allows the caller to see the effect of the
                // add_learner operation on the cluster's configuration.
                if let Some(membership) = response.membership() {
                    let res = self.create_client_write_response(membership);
                    Ok(Response::new(res?))
                } else {
                    let msg = "membership not retrieved";
                    error!("{}", msg);
                    Err(Status::internal(msg))
                }
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding AddLearner request to leader: {:?}", node);
                        let mut client = self.create_confer_client(node.addr.clone()).await?;
                        let res = client
                            .add_learner(Request::new(req))
                            .await?;
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!(
                            "Failed to forward. Leader not known: {}",
                            leader
                        )))
                    }
                }
                e => {
                    error!("Failed to add learner node: {:?}", e);
                    Err(Status::internal(format!(
                        "Failed to add learner node: {}",
                        e
                    )))
                }
            },
        }
    }

    /// Changes the membership of the Raft cluster.
    ///
    /// # Arguments
    ///
    /// * `request`: The request containing the new membership configuration.
    ///     -  The `ChangeMembershipRequest` contains the list of members
    ///        to add or remove.
    ///
    /// # Returns
    ///
    /// * `Result<Response<ClientWriteResponse>, Status>`: The result of the operation.
    ///     - On success, returns the `ClientWriteResponse` which includes the
    ///       updated membership information.
    ///     -  On failure, returns a `Status` indicating the error.
    #[instrument(skip(self, request))]
    async fn change_membership(
        &self,
        request: Request<ChangeMembershipRequest>,
    ) -> Result<Response<ClientWriteResponse>, Status> {
        let req = request.into_inner();
        debug!(
            "Changing membership. Members: {:?}, Retain: {}",
            req.members, req.retain
        );

        // Change the membership of the Raft cluster.  This is how you
        // add or remove voting members from the cluster.  The `retain`
        // parameter is used to control whether to use joint consensus.
        let result = self
            .raft
            .change_membership(req.members.clone(), req.retain) // Change membership.
            .await;


        match result {
            Ok(response) => {
                let network_factory = self.network.clone();
                let current_members: Vec<u64> = self.raft.metrics().borrow().clone()
                    .membership_config.membership()
                    .nodes()
                    .map(|(id, _)| *id)
                    .collect();

                tokio::spawn(async move {
                    for member_id in current_members {
                        if !req.members.contains(&member_id) {
                            network_factory.remove_client(member_id);
                            info!("Removed cached grpc client for node {}", member_id);
                        }
                    }
                });
                if let Some(membership) = response.membership() {
                    let res = self.create_client_write_response(membership);
                    Ok(Response::new(res?))
                } else {
                    let msg = "membership not retrieved";
                    error!("{}", msg);
                    Err(Status::internal(msg))
                }
            }
            Err(error) => match error {
                RaftError::APIError(ForwardToLeader(leader)) => {
                    if let Some(node) = leader.leader_node {
                        info!("Forwarding ChangeMembership request to leader: {:?}", node);
                        let mut client = self.create_confer_client(node.addr.clone()).await?;
                        let res = client
                            .change_membership(Request::new(req))
                            .await?;
                        Ok(res)
                    } else {
                        error!("Failed to forward. Leader not known: {:?}", leader);
                        Err(Status::internal(format!(
                            "Failed to forward. Leader not known: {}",
                            leader
                        )))
                    }
                }
                e => {
                    error!("Failed to change membership: {:?}", e);
                    Err(Status::internal(format!(
                        "Failed to change membership: {}",
                        e
                    )))
                }
            },
        }
    }
}

#[cfg(test)]
mod service_tests;
