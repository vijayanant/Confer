mod proto;
mod service;
mod repository;
mod raft;
mod error;

use clap::Parser;
use tracing::{info, debug, error};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::Arc;

use tonic::transport::Server;
use std::net::SocketAddr;

use openraft::Raft;
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
    let app_filter = tracing_subscriber::EnvFilter::new(
            std::env::var("APP_LOG").unwrap_or_else(|_| "server=debug".into()));
    //let lib_filter = tracing_subscriber::EnvFilter::new(
            //std::env::var("LIB_LOG").unwrap_or_else(|_| "openraft=debug".into()));

    let app_layer = tracing_subscriber::fmt::layer().with_filter(app_filter);
    //let lib_layer = tracing_subscriber::fmt::layer().with_filter(lib_filter);

    if let Err(e) = tracing_subscriber::registry()
        .with(app_layer)
        //.with(lib_layer)
        //.with(tracing_subscriber::fmt::layer())
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
    #[clap(short = 'i', long = "id", help = "Unique ID for this node (e.g., '1').")]
    node_id: String,

    #[clap(short = 's', long = "server", help = "Address for the App to listen on (e.g., '127.0.0.1:45671').")]
    server_address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    init_tracing();

    let args = Args::parse();
    debug!("{:?}", args);
    let node_id        = args.node_id.parse()?;
    let server_address = args.server_address;

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

    let raft_service = RaftServiceImpl::new(raft.clone());
    let raft_service_server = RaftServiceServer::new(raft_service);

    let confer_service = ConferServiceImpl::new(raft, state_machine.clone());
    let confer_service_server = ConferServiceServer::new(confer_service);
    let confer_server = Server::builder()
        .add_service(raft_service_server)
        .add_service(confer_service_server);


    let addr: SocketAddr = server_address.parse()?;
    info!("Confer Server runing at {}", addr.clone().to_string());
    confer_server.serve(addr).await?;


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
