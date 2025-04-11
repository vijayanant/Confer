mod error;
mod proto;
mod raft;
mod repository;
mod service;

use clap::Parser;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use std::sync::Arc;

use std::net::SocketAddr;
use tonic::transport::Server;

use openraft::Config;
use openraft::Raft;
use openraft::SnapshotPolicy;

use crate::proto::confer::v1::confer_service_server::ConferServiceServer;
use crate::raft::log_storage::ConferLogStore;
use crate::raft::network::NetworkFactory;
use crate::raft::proto::raft_service_server::RaftServiceServer;
use crate::raft::raft_server::RaftServiceImpl;
use crate::raft::state_machine::StateMachine;
use crate::repository::HashMapConferRepository;
use crate::service::ConferServiceImpl;

/// Initializes tracing for the application.
///
/// This function sets up the tracing subscriber to log events at specified levels.
/// It uses environment variables `APP_LOG` and `LIB_LOG` to configure the filter.
/// If the environment variables are not set, it defaults to logging at the `info`
/// level for the application and `info` level for openraft.  Any errors during
/// tracing initialization are logged to standard error.
fn init_tracing() {
    let app_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("APP_LOG").unwrap_or_else(|_| "confer-server=info".into()),
    );
    //let lib_filter = tracing_subscriber::EnvFilter::new(
    //    std::env::var("LIB_LOG").unwrap_or_else(|_| "openraft=info".into()));

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

/// Arguments for the Confer server CLI.
///
/// This struct defines the command-line arguments that can be passed to the
/// `confer-server` executable.  It uses `clap` to handle argument parsing.
#[derive(Parser, Debug)]
#[clap(
    name = "Raft Node",
    version = "0.1.0",
    author = "Your Name",
    about = "Runs a node for a distributed Confer system."
)]
struct Args {
    /// Unique ID for this node (e.g., '1').
    #[clap(
        short = 'i',
        long = "id",
        help = "Unique ID for this node (e.g., '1')."
    )]
    node_id: String,

    /// Address for the App to listen on (e.g., '127.0.0.1:45671').
    #[clap(
        short = 's',
        long = "server",
        help = "Address for the App to listen on (e.g., '127.0.0.1:45671')."
    )]
    server_address: String,
}

/// Main function for the Confer server.
///
/// This function initializes the Raft node, starts the gRPC server, and listens
/// for client requests.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>`:  A result indicating success or
///    an error.  Errors are boxed to allow for different error types.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let args = Args::parse();
    debug!("Parsed arguments: {:?}", args);
    let node_id = args.node_id.parse()?;
    let server_address = args.server_address;

    let config = sensible_config();
    let repository = HashMapConferRepository::new();
    let state_machine = Arc::new(StateMachine::new(repository));
    let log_store = Arc::new(ConferLogStore::new());
    let network = Arc::new(NetworkFactory::new());

    //create a raft instance
    let raft = Raft::new(
        node_id,
        config.clone(),
        network.clone(),
        log_store.clone(),
        state_machine.clone(),
    )
    .await
    .unwrap();

    info!("Raft Node {} created!", node_id);

    let raft_service = RaftServiceImpl::new(raft.clone());
    let raft_service_server = RaftServiceServer::new(raft_service);

    let confer_service = ConferServiceImpl::new(raft, state_machine.clone());
    let confer_service_server = ConferServiceServer::new(confer_service);
    let confer_server = Server::builder()
        .add_service(raft_service_server)
        .add_service(confer_service_server);

    let addr: SocketAddr = server_address.parse()?;
    info!("Confer Server listening on {}", addr.clone().to_string());
    confer_server.serve(addr).await?;

    Ok(())
}

/// Returns a sensible default configuration for the Raft node.
///
/// This function creates a `Config` object with reasonable default values for
/// election timeouts, heartbeat interval, snapshot policy, and cluster name.
///
/// # Returns
///
/// * `Arc<Config>`:  An `Arc` (atomically reference counted) pointer to the
///    configuration object.  This allows the configuration to be shared
///    safely between different parts of the application.
fn sensible_config() -> Arc<Config> {
    let config = Config {
        election_timeout_min: 6000,
        election_timeout_max: 7500,
        heartbeat_interval: 3000,
        snapshot_policy: SnapshotPolicy::LogsSinceLast(500),
        cluster_name: "confer_cluster".to_string(),
        ..Default::default()
    };
    Arc::new(config)
}
