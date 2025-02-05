mod proto;
mod service;
mod store;

use std::error::{Error};
use tonic::transport::Server;
use service::ConfigService;

use crate::proto::confer::confer_server::ConferServer;

pub async fn start_server(addr: &str) -> Result<(), Box<dyn Error>> {

    let service = ConfigService::new();

    println!("Server listening on {}", addr);

    Server::builder()
        .add_service(ConferServer::new(service))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
