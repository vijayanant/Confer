mod proto;
mod service;
mod repository;
mod raft;
mod error;

use tonic::transport::Server;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::proto::confer::v1::confer_service_server::ConferServiceServer;
use crate::service::ConferServiceImpl;

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

    //let addr = "[::1]:67891".parse()?;
    //let state_machine = HashMapStateMachine::new();
    //let confer_service = ConferServiceImpl::new(Box::new(state_machine));

    //Server::builder()
        //.add_service(ConferServiceServer::new(confer_service))
        //.serve(addr)
        //.await?;
    //info!("Server listening on: {}", addr);


    Ok(())
}
