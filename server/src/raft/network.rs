use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
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

use prost::Message;
use tonic::{transport::{Error, Channel}, Request};
use crate::raft::proto::raft_service_client::RaftServiceClient;
use crate::raft::config::TypeConfig;
use crate::raft::proto::GenericMessage;


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

    async fn get_client(&self, target: NodeId, address: String) -> Result<RaftServiceClient<Channel>, Error> {
        let mut cache = self.client_cache.lock().unwrap();
        if let Some(client) = cache.get(&target) {
            return Ok(client.clone());
        }

        let channel = Channel::from_shared(address.clone())
            .map_err(|e| Error::from_source(e))?
            .connect()
            .await?;

        let client = RaftServiceClient::new(channel);
        cache.insert(target, client.clone());
        Ok(client)
    }
}

#[async_trait]
impl RaftNetworkFactory<TypeConfig> for Arc<NetworkFactory> {
    type Network = NetworkClient;

    async fn new_client(&mut self, target: NodeId, node: &Node) -> Self::Network {
        //let address = node.addr;
        //let client = self.get_client(target, address.to_string()).await.unwrap();

         //let channel = Channel::from_shared(address.clone())
            //.map_err(|e| Error::from_source(e))?
            //.connect()
            //.await.unwrap();

        //let client = RaftServiceClient::new(channel.clone());

        //Network::new(client.clone())
        NetworkClient {
            target: target.clone(),
            node: node.clone(),
        }
    }
}

pub struct NetworkClient {
    target: NodeId,
    node: Node,
}

#[async_trait]
impl RaftNetwork<TypeConfig> for NetworkClient {
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<TypeConfig>,
        options: RPCOption,
    //) -> Result<AppendEntriesResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {

        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.append_entries(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = AppendEntriesResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }

    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<TypeConfig>,
        option: RPCOption,
    //) -> Result<InstallSnapshotResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId, openraft::error::InstallSnapshotError>>> {

        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.install_snapshot(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = InstallSnapshotResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }

    async fn vote(
        &mut self,
        request: VoteRequest<TypeConfig>,
        option: RPCOption,
    //) -> Result<VoteResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.vote(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = VoteResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }
}

pub struct Network {
    client: RaftServiceClient<Channel>,
    _phantom: std::marker::PhantomData<TypeConfig>,
}

impl Network {
    pub fn new(client: RaftServiceClient<Channel>) -> Self {
        Network {
            client,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl RaftNetwork<TypeConfig> for Network {
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<TypeConfig>,
        options: RPCOption,
    //) -> Result<AppendEntriesResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {

        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.append_entries(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = AppendEntriesResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }

    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<TypeConfig>,
        option: RPCOption,
    //) -> Result<InstallSnapshotResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId, openraft::error::InstallSnapshotError>>> {

        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.install_snapshot(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = InstallSnapshotResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }

    async fn vote(
        &mut self,
        request: VoteRequest<TypeConfig>,
        option: RPCOption,
    //) -> Result<VoteResponse<TypeConfig>, RPCError<TypeConfig>> {
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let encoded_request = request.encode_to_vec();
        let grpc_request = GenericMessage { data: encoded_request };

        let grpc_response = self.client.vote(Request::new(grpc_request))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        let decoded_response = VoteResponse::<TypeConfig>::decode(grpc_response.into_inner().data.as_slice())
            .map_err(|e| RPCError::Network(NetworkError::new(e.to_string())))?;

        Ok(decoded_response)
    }
}
