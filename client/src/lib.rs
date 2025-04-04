mod error;

use tonic::{Request, transport::Channel};

use crate::error::ClientError;
use confer::v1::confer_service_client::ConferServiceClient;
use confer::v1::{ConfigPath, ConfigValue, SetConfigRequest};

pub mod confer {
    pub mod v1 {
        tonic::include_proto!("confer.v1");
    }
}

pub struct ConferClient {
    client: ConferServiceClient<Channel>,
}


impl ConferClient {
    // Connect to the Confer server
    pub async fn connect(address: String) -> Result<Self, ClientError> {
        let channel = Channel::from_shared(address)
            .map_err(ClientError::from)?
            .connect()
            .await
            .map_err(ClientError::from)?;
        let client = ConferServiceClient::new(channel);
        Ok(Self{ client })
    }

    // Set a configuration value
    pub async fn set(
        &mut self,
        path: String,
        value: Vec<u8>,
    ) -> Result<(), ClientError>
    {
        let request = Request::new(SetConfigRequest {
            path: Some(ConfigPath{path}),
            value: Some(ConfigValue{value}),
        });

        let _response = self.client.set(request).await?;
        Ok(())
    }

    // Get a configuration value
    pub async fn get(
        &mut self,
        path: String,
    ) -> Result<Vec<u8>, ClientError>
    {
        let request = Request::new(ConfigPath { path, });
        let response = self.client.get(request).await?;
        let r = response.into_inner();
        Ok(r.value)
    }

    // Removing a config
    pub async fn delete(
        &mut self,
        path: String,
    ) -> Result<(), ClientError> {
        let request = Request::new(ConfigPath { path });
        let _response = self.client.remove(request).await?;
        Ok(())
    }

    // Get all configurations under the given path
    pub async fn list(
        &mut self,
        path: String,
    ) -> Result<Vec<String>, ClientError> {
        let request = Request::new(ConfigPath { path });
        let response = self.client.list(request).await?;
        let r = response.into_inner();
        Ok(r.paths)
    }
}
