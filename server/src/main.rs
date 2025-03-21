mod proto;
//mod service;
mod repository;
mod raft;
mod error;

use tonic::transport::Server;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::proto::confer::v1::confer_service_server::ConferServiceServer;
//use crate::service::ConferServiceImpl;

use std::sync::Arc;
use openraft::Raft;
use openraft::Config;
use openraft::SnapshotPolicy;
use crate::repository::ConferRepository;
use crate::repository::HashMapConferRepository;
use crate::raft::config::{TypeConfig, NodeId};
use crate::raft::state_machine::StateMachine;
use crate::raft::log_storage::ConferLogStore;
use crate::raft::network::NetworkFactory;

use crate::raft::operation::{Operation, OperationResponse};
use crate::proto::confer::v1::{ConfigPath, ConfigValue};
use openraft::BasicNode;
use std::collections::BTreeMap;

fn init_tracing() {
    if let Err(e) = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "confer=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .try_init()
    {
        error!("Failed to initialize tracing: {}", e);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let node_id: NodeId  = 1;
    let config        = sensible_config();
    let repository    = HashMapConferRepository::new();
    let state_machine = Arc::new(StateMachine::new(repository));
    let log_store     = Arc::new(ConferLogStore::new());
    let network       = Arc::new(NetworkFactory::new());

    //create a raft instance
    let raft = Raft::new(
        node_id,
        config.clone(),
        network.clone(),
        log_store.clone(),
        state_machine.clone()
    ).await.unwrap();

    // Initialize the Raft node.
    let mut nodes = BTreeMap::new();
    let address = "http://127.0.0.1:67891".to_string();
    nodes.insert(node_id, BasicNode { addr: address.clone(),});
    raft.initialize(nodes).await.unwrap();

    // Simulate a client request (sending an Operation).
    let set_operation = Operation::Set {
        path: ConfigPath { path: "key1".to_string() },
        value: ConfigValue { value: "value1".as_bytes().to_vec() },
    };

    //serialise the operation as a command
    let set_command = serde_json::to_vec(&set_operation).unwrap();

    tracing::info!("Sending command: {:?}", set_operation);
    let set_client_result = raft.client_write(set_operation.clone()).await;

    match set_client_result {
        Ok(response) => {
            tracing::info!("SET Command succeeded: {:?}", response);
        }
        Err(err) => {
            tracing::error!("SET Command failed: {:?}", err);
        }
    }

    // Directly get the value from the state machine.
    let path = ConfigPath { path: "key1".to_string() };
    let repo = state_machine.repository.read().await;
    let retrieved_value_option = repo.get(&path).await;

    match retrieved_value_option {
        Ok(config_value) => {
            let retrieved_value = String::from_utf8(config_value).unwrap();
            tracing::info!("Retrieved value: {}", retrieved_value);
            assert_eq!(retrieved_value, "value1"); // Verify the value.
            tracing::info!("Value verification successful!");
        }
        Err(err) => {
            tracing::error!("Error retrieving value: {:?}", err);
            return Err(err)?;
        }
    };


    Ok(())
}

fn sensible_config() -> Arc<Config> {
    let mut config = Config::default();
    config.election_timeout_min = 1000;
    config.election_timeout_max = 1500;
    config.heartbeat_interval = 500;
    config.snapshot_policy = SnapshotPolicy::LogsSinceLast(500);
    config.cluster_name = "confer_cluster".to_string();
    Arc::new(config)
}
