mod proto;

use crate::proto::confer::confer_service_client::ConferServiceClient;
use crate::proto::confer:: {
    SetConfigRequest,
    ConfigValue,
    ConfigPath,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ConferServiceClient::connect("http://127.0.0.1:6789").await?;

    let p1 = String::from("config/user/theme");
    //let v1 = String::from("dark");
    let v1 = vec![1,2,3,4];

    let request = tonic::Request::new(SetConfigRequest {
        path : Some(ConfigPath { path: p1}),
        //value: Some(ConfigValue { value: v1.into()}),
        value: Some(ConfigValue { value: v1}),
    });

    let response = client.set(request).await?;
    println!("Got: '{:?}' from service", response.into_inner());

    Ok(())
}
