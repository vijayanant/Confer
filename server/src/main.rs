mod proto;
mod service;
mod store;

use confer::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:6789";
    start_server(addr).await?;
    Ok(())
}
