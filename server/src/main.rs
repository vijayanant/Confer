mod proto;
mod service;
mod repository;
mod raft;
mod error;

use clap::Parser;
use tracing::{info, debug, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::BTreeMap;
use std::ops::Deref;

use tonic::transport::Server;

use openraft::Raft;
use openraft::BasicNode;
use openraft::Config;
use openraft::SnapshotPolicy;

use crate::repository::HashMapConferRepository;
use crate::service::ConferServiceImpl;
use crate::raft::state_machine::StateMachine;
use crate::raft::log_storage::ConferLogStore;
use crate::raft::network::NetworkFactory;
use crate::raft::raft_server::RaftServiceImpl;
use crate::proto::confer::v1::confer_service_server::ConferServiceServer;
use crate::raft::proto::raft_service_server::RaftServiceServer;

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


#[derive(Parser, Debug)]
#[clap(
    name = "Raft Node",
    version = "0.1.0",
    author = "Your Name",
    about = "Runs a node for a distributed Confer system."
)]
struct Args {
    #[clap(
        short = 'm',
        long = "mode",
        value_parser = clap::builder::PossibleValuesParser::new(["new", "join"]),
        help = "Mode of operation: 'new' to start a new cluster, 'join' to join an existing cluster."
    )]
    mode: String,

    #[clap(short = 'i', long = "id", help = "Unique ID for this node (e.g., 'node1').")]
    node_id: String,

    #[clap(short = 's', long = "server", help = "Address for the App to listen on (e.g., '127.0.0.1:45671').")]
    server_address: String,

    #[clap(short = 'r', long = "raft", help = "Address for Raft Node to listen on (e.g., '127.0.0.1:56781').")]
    raft_address: String,

    #[clap(long = "peer", help = "Addresses of other nodes in the cluster (for 'join' mode).")]
    peers: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    init_tracing();

    let args = Args::parse();
    debug!("{:?}", args);
    let mode           = &args.mode;
    let node_id        = args.node_id.parse()?;
    let server_address = args.server_address;
    let raft_address   = args.raft_address;
    let peers          = &args.peers;

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

    info!("Raft Node created!");

    //let raft = Arc::new(raft);

    if mode == "new" { // Initialize the Raft cluster.
        let mut nodes = BTreeMap::new();
        //let server_address  = server_address.parse()?; //"http://127.0.0.1:67891".to_string();
        nodes.insert(node_id.clone(), BasicNode { addr: "127.0.0.1:45671".to_string()});
        nodes.insert(node_id.clone(), BasicNode { addr: "127.0.0.1:45672".to_string()});
        raft.initialize(nodes).await.unwrap();
        info!("Raft Node {} initialised!", node_id.clone());
    } else {
        error!("--- RAFT NEEDS TO BE INITIALISED ---");
    }

    let raft_service = RaftServiceImpl::new(raft.clone());
    let raft_service_server = RaftServiceServer::new(raft_service);
    //let raft_server = Server::builder()
        //.add_service(raft_service_server.clone())
        //.serve(raft_address)
        //.await?;


    let confer_service = ConferServiceImpl::new(raft, state_machine.clone());
    let confer_service_server = ConferServiceServer::new(confer_service);
    let confer_server = Server::builder()
        .add_service(confer_service_server)
        .add_service(raft_service_server)
        .serve(server_address.parse()?);

    //println!("Confer runing at {}", server_address);
    info!("Confer runing at {}", server_address);
    //println!("Raft runing at {}", raft_address);
    //info!("Raft runing at {}", raft_address);

    confer_server.await?;

    return Ok(())
}

fn sensible_config() -> Arc<Config> {
    let mut config = Config::default();
    config.election_timeout_min = 6000;
    config.election_timeout_max = 7500;
    config.heartbeat_interval = 3000;
    config.snapshot_policy = SnapshotPolicy::LogsSinceLast(500);
    config.cluster_name = "confer_cluster".to_string();
    Arc::new(config)
}
