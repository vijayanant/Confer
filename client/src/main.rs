mod proto;

use crate::proto::confer::{SetRequest, SetResponse, GetRequest, GetResponse};
use crate::proto::confer::confer_client::ConferClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ConferClient::connect("http://127.0.0.1:6789").await?;

    let p1 = String::from("config/user/theme");
    let v1 = String::from("dark");

    let request = tonic::Request::new(SetRequest {
        path: p1,
        value: v1,
    });

    let response = client.set(request).await?;
    println!("Got: '{:?}' from service", response.into_inner());

    Ok(())
}
