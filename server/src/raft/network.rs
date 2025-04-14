use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use openraft::error::{NetworkError, RPCError, RaftError};
use openraft::network::{RPCOption, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::{RaftNetwork, RaftTypeConfig};

use crate::error::ConferError;
use crate::raft::config::TypeConfig;
use crate::raft::proto::raft_service_client::RaftServiceClient;
use crate::raft::proto::GenericMessage;
use tonic::{transport::Channel, Request};
use tracing::{debug, error, info};

type NodeId = <TypeConfig as RaftTypeConfig>::NodeId;
type Node = <TypeConfig as RaftTypeConfig>::Node;

/// Factory for creating Raft network clients.
///
/// This factory is responsible for creating and caching gRPC clients used to communicate with other Raft nodes.
pub struct NetworkFactory {
    /// A cache of gRPC clients, keyed by NodeId.
    ///
    /// The `Arc` allows shared ownership across threads, and the `Mutex` provides
    /// synchronized access to the `HashMap` for thread safety.
    client_cache: Arc<Mutex<HashMap<NodeId, RaftServiceClient<Channel>>>>,
    /// Phantom data to associate this factory with a specific Raft type configuration.
    ///
    /// This field is necessary because `RaftNetworkFactory` is generic over `TypeConfig`,
    /// and we need to ensure that this factory is used with the correct configuration.
    /// It doesn't store any data at runtime.
    _phantom: std::marker::PhantomData<TypeConfig>,
}

impl Default for NetworkFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkFactory {
    /// Creates a new `NetworkFactory`.
    pub fn new() -> Self {
        NetworkFactory {
            client_cache: Arc::new(Mutex::new(HashMap::new())),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Retrieves a gRPC client for the specified node.
    ///
    /// This method first checks the client cache. If a client is found, it is returned.
    /// Otherwise, a new client is created, stored in the cache, and returned.
    ///
    /// # Arguments
    ///
    /// * `target`: The NodeId of the target node.
    /// * `address`: The address of the target node.
    async fn get_client(
        &self,
        target: NodeId,
        address: String,
    ) -> Result<RaftServiceClient<Channel>, ConferError> {
        let client_option;
        {
            // Acquire a lock on the client cache.  This ensures that only one thread
            // can access the cache at a time, preventing data corruption.
            let cache = self.client_cache.lock().unwrap();
            // Try to get a client from the cache.  `cloned()` creates a new `Arc`
            // pointing to the same underlying `RaftServiceClient`, so we don't
            // take ownership of the client in the cache.
            client_option = cache.get(&target).cloned();
        } // The lock is released here when the `MutexGuard` goes out of scope.  This is
          // crucial for performance, as it allows other threads to access the cache.

        if let Some(client) = client_option {
            info!("Using cached client for node {}", target);
            return Ok(client);
        }

        info!("Creating new client for node {} at address {}", target, address);
        // Attempt to create a gRPC channel to the target address.
        let channel = Channel::from_shared(address.clone())
            .map_err(|e| ConferError::Internal {
                message: format!("Failed to create channel to {}: {}", address, e),
            })?;
        // Attempt to connect to the gRPC server.
        let channel = channel.connect().await.map_err(|e| ConferError::Internal {
            message: format!("Failed to connect to {}: {}", address, e),
        })?;

        let client = RaftServiceClient::new(channel);
        {
            // Acquire a lock to modify the client cache.
            let mut cache = self.client_cache.lock().unwrap();
            // Insert the new client into the cache.  We clone the `Arc` so that the
            // cache holds a separate reference to the client.
            cache.insert(target, client.clone());
        } // The lock is released here.
        Ok(client)
    }

    /// Removes a client from the cache.
    ///
    /// This method is used to remove a client from the cache when a node is removed
    /// from the Raft cluster.  This ensures that stale connections are not used.
    ///
    /// # Arguments
    ///
    /// * `target`: The NodeId of the node to remove from the cache.
    pub fn remove_client(&self, target: NodeId) {
        {
            let mut cache = self.client_cache.lock().unwrap();
            cache.remove(&target);
        }
        info!("Removed client for node {} from cache", target);
    }
}

/// A Raft network client.
///
/// This client is used to send Raft requests to other nodes using gRPC.  It wraps
/// a `RaftServiceClient` and provides methods for sending specific Raft RPCs.
pub struct NetworkClient {
    /// The gRPC client.
    client: RaftServiceClient<Channel>,
    /// Phantom data to associate this client with a specific Raft type configuration.
    _phantom: std::marker::PhantomData<TypeConfig>,
}

impl NetworkClient {
    /// Creates a new `NetworkClient` with the given gRPC client.
    pub fn new(client: RaftServiceClient<Channel>) -> Self {
        NetworkClient {
            client,
            _phantom: std::marker::PhantomData,
        }
    }


    /// Sends an AppendEntriesRequest to a remote node.
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<TypeConfig>,
        _options: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!(
            "Sending AppendEntriesRequest to node {}: {:?}",
            request.vote.leader_id.node_id, request
        );
        let serialized_request =
            serde_json::to_vec(&request).map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage {
            data: serialized_request,
        };

        let grpc_response = self.client.append_entries(Request::new(grpc_request)).await.map_err(|e| {
            error!("AppendEntries RPC failed: {:?}", e);
            RPCError::Network(NetworkError::new(&e))
        })?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response: AppendEntriesResponse<NodeId>  = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        debug!(
            "Received AppendEntriesResponse from node {}: {:?}",
            request.vote.leader_id.node_id,
            deserialized_response
        );
        Ok(deserialized_response)
    }

    /// Sends an InstallSnapshotRequest to a remote node.
    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, openraft::error::InstallSnapshotError>>,
    > {
        debug!("Sending InstallSnapshotRequest: {:?}", request);
        let serialized_request =
            serde_json::to_vec(&request).map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage {
            data: serialized_request,
        };

        let grpc_response = self
            .client
            .install_snapshot(Request::new(grpc_request))
            .await
            .map_err(|e| {
                error!("InstallSnapshot RPC failed: {:?}", e);
                RPCError::Network(NetworkError::new(&e))
            })?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        debug!(
            "Received InstallSnapshotResponse: {:?}",
            deserialized_response
        );
        Ok(deserialized_response)
    }

    /// Sends a VoteRequest to a remote node.
    async fn vote(
        &mut self,
        request: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("Sending VoteRequest: {:?}", request);
        let serialized_request =
            serde_json::to_vec(&request).map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage {
            data: serialized_request,
        };

        let grpc_response = self
            .client
            .vote(Request::new(grpc_request))
            .await
            .map_err(|e| {
                error!("Vote RPC failed: {:?}", e);
                RPCError::Network(NetworkError::new(&e))
            })?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        debug!("Received VoteResponse: {:?}", deserialized_response);
        Ok(deserialized_response)
    }
}

impl RaftNetworkFactory<TypeConfig> for Arc<NetworkFactory> {
    type Network = NetworkClient;

    /// Creates a new Raft network client for the specified node.
    ///
    /// This method retrieves a gRPC client using the `get_client` method
    /// of the `NetworkFactory` and wraps it in a `NetworkClient`.
    ///
    /// # Arguments
    ///
    /// * `target`: The NodeId of the target node.
    /// * `node`: The `Node` struct containing the address of the target node.
    async fn new_client(&mut self, target: NodeId, node: &Node) -> Self::Network {
        let address = &node.addr;
        let client = self.get_client(target, address.to_string()).await.unwrap();
        NetworkClient::new(client.clone())
    }
}

impl RaftNetwork<TypeConfig> for NetworkClient {
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<TypeConfig>,
        _options: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        self.append_entries(request, _options).await
    }

    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, openraft::error::InstallSnapshotError>>,
    > {
        self.install_snapshot(request, _option).await
    }

    async fn vote(
        &mut self,
        request: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        self.vote(request, _option).await
    }
}

