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
use tracing::{debug, error};

type NodeId = <TypeConfig as RaftTypeConfig>::NodeId;
type Node = <TypeConfig as RaftTypeConfig>::Node;

/// Factory for creating Raft network clients.
///
/// This factory is responsible for creating and caching gRPC clients used to communicate with other Raft nodes.
pub struct NetworkFactory {
    /// A cache of gRPC clients, keyed by NodeId.  Uses a Mutex for thread-safe access.
    client_cache: Arc<Mutex<HashMap<NodeId, RaftServiceClient<Channel>>>>,
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
            let cache = self.client_cache.lock().unwrap();
            client_option = cache.get(&target).cloned();
        } // MutexGuard is dropped here

        if let Some(client) = client_option {
            return Ok(client);
        }

        let channel = Channel::from_shared(address.clone())
            .map_err(|e| ConferError::Internal {
                message: format!("Failed to create channel to {}: {}", address, e),
            })?
            .connect()
            .await
            .map_err(|e| ConferError::Internal {
                message: format!("Failed to connect to {}: {}", address, e),
            })?;

        let client = RaftServiceClient::new(channel);
        {
            let mut cache = self.client_cache.lock().unwrap();
            cache.insert(target, client.clone());
        } // MutexGuard dropped here
        Ok(client)
    }
}

/// A Raft network client.
///
/// This client is used to send Raft requests to other nodes using gRPC.
pub struct NetworkClient {
    /// The gRPC client.
    client: RaftServiceClient<Channel>,
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
    /// Sends an AppendEntriesRequest to a remote node.
    ///
    /// This method serializes the `AppendEntriesRequest`, sends it over gRPC,
    /// and deserializes the response.  It handles any errors that occur
    /// during the process.
    ///
    /// # Arguments
    ///
    /// * `request`: The `AppendEntriesRequest` to send.
    /// * `_options`:  (Currently unused)  Options for the RPC call.
    ///
    /// # Returns
    ///
    /// The `AppendEntriesResponse` from the remote node, or an error if the
    /// request fails.
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<TypeConfig>,
        _options: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("Sending AppendEntriesRequest: {:?}", request);
        let serialized_request =
            serde_json::to_vec(&request).map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage {
            data: serialized_request,
        };

        let grpc_response = self
            .client
            .append_entries(Request::new(grpc_request))
            .await
            .map_err(|e| {
                error!("AppendEntries RPC failed: {:?}", e);
                RPCError::Network(NetworkError::new(&e))
            })?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        debug!(
            "Received AppendEntriesResponse: {:?}",
            deserialized_response
        );
        Ok(deserialized_response)
    }

    /// Sends an InstallSnapshotRequest to a remote node.
    ///
    /// This method serializes the `InstallSnapshotRequest`, sends it over gRPC,
    /// and deserializes the response.  It handles any errors that occur
    /// during the process.
    ///
    /// # Arguments
    ///
    /// * `request`: The `InstallSnapshotRequest` to send.
    /// * `_option`: (Currently unused) Options for the RPC call.
    ///
    /// # Returns
    ///
    /// The `InstallSnapshotResponse` from the remote node, or an error if
    /// the request fails.
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
    ///
    /// This method serializes the `VoteRequest`, sends it over gRPC,
    /// and deserializes the response. It handles any errors that occur
    /// during the process.
    ///
    /// # Arguments
    ///
    /// * `request`: The `VoteRequest` to send.
    /// * `_option`: (Currently unused) Options for the RPC call.
    ///
    /// # Returns
    ///
    /// The `VoteResponse` from the remote node, or an error if the
    /// request fails.
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
