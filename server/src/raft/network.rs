use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use openraft::{RaftNetwork, RaftTypeConfig};
use openraft::error::{NetworkError, RPCError, RaftError};
use openraft::network::{RPCOption, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest,
    AppendEntriesResponse,
    InstallSnapshotRequest,
    InstallSnapshotResponse,
    VoteRequest,
    VoteResponse,
};

use tonic::{transport::{Error, Channel,}, Request,};
use crate::raft::proto::raft_service_client::RaftServiceClient;
use crate::raft::config::TypeConfig;
use crate::raft::proto::GenericMessage;
use crate::error::ConferError;


type NodeId = <TypeConfig as RaftTypeConfig>::NodeId;
type Node = <TypeConfig as RaftTypeConfig>::Node;

pub struct NetworkFactory {
    client_cache: Arc<Mutex<HashMap<NodeId, RaftServiceClient<Channel>>>>,
    _phantom: std::marker::PhantomData<TypeConfig>,
}

impl NetworkFactory {
    pub fn new() -> Self {
        NetworkFactory {
            client_cache: Arc::new(Mutex::new(HashMap::new())),
            _phantom: std::marker::PhantomData,
        }
    }

    async fn get_client(&self, target: NodeId, address: String) -> Result<RaftServiceClient<Channel>, ConferError> {
        let client_option;
        {
            let cache = self.client_cache.lock().unwrap();
            client_option = cache.get(&target).cloned();
        } // MutexGuard is dropped here

        if let Some(client) = client_option {
            return Ok(client);
        }

        let channel = Channel::from_shared(address.clone())
            .map_err(|e| ConferError::Internal{message: e.to_string()})?
            .connect()
            .await
            .map_err(|e| ConferError::Internal{message: e.to_string()})?;

        let client = RaftServiceClient::new(channel);
        {
            let mut cache = self.client_cache.lock().unwrap();
            cache.insert(target, client.clone());
        } // MutexGuard dropped here
        Ok(client)
    }

    //async fn get_client(&self, target: NodeId, address: String) -> Result<RaftServiceClient<Channel>, ConferError> {
        //let mut cache = self.client_cache.lock().unwrap();
        //if let Some(client) = cache.get(&target) {
            //return Ok(client.clone());
        //}

        //let channel = Channel::from_shared(address.clone())
            //.map_err(|e| ConferError::Internal{message: e.to_string()})?
            //.connect()
            //.await
            //.map_err(|e| ConferError::Internal{message: e.to_string()})?;


        //let client = RaftServiceClient::new(channel);
        //cache.insert(target, client.clone());
        //Ok(client)
    //}
}

pub struct NetworkClient {
    client: RaftServiceClient<Channel>,
    _phantom: std::marker::PhantomData<TypeConfig>,
}

impl NetworkClient {
    pub fn new(client: RaftServiceClient<Channel>) -> Self {
        NetworkClient {
            client,
            _phantom: std::marker::PhantomData,
        }
    }
}


impl RaftNetworkFactory<TypeConfig> for Arc<NetworkFactory> {
    type Network = NetworkClient;

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
        options: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {

        let serialized_request = serde_json::to_vec(&request)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage { data: serialized_request };

        let grpc_response = self.client.append_entries(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(deserialized_response)
    }

    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<TypeConfig>,
        option: RPCOption,
    ) -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId, openraft::error::InstallSnapshotError>>> {

        let serialized_request = serde_json::to_vec(&request)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage { data: serialized_request };

        let grpc_response = self.client.install_snapshot(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(deserialized_response)
    }

    async fn vote(
        &mut self,
        request: VoteRequest<NodeId>,
        option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let serialized_request = serde_json::to_vec(&request)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let grpc_request = GenericMessage { data: serialized_request };

        let grpc_response = self.client.vote(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let generic_message = grpc_response.into_inner();
        let deserialized_response = serde_json::from_slice(&generic_message.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(deserialized_response)
    }
}


