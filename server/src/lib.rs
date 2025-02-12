mod proto;
mod service;
mod core;

#[cfg(test)]
mod tests;

use service::ConfigService;
use std::error::Error;
use tonic::transport::Server;

use crate::proto::confer::confer_server::ConferServer;
use crate::core::datastore::HashMapDataStore;

pub async fn start_server(addr: &str) -> Result<(), Box<dyn Error>> {
    let data_store = HashMapDataStore::new();
    let service = ConfigService::new(data_store);

    println!("Server listening on {}", addr);

    Server::builder()
        .add_service(ConferServer::new(service))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
