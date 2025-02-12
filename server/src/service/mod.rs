mod service_test;

use std::sync::Arc;
use tokio::sync::Mutex; // async compatible Guard
use tonic::{Request, Response, Status};

use crate::store::{HashMapDataStore, ConfigPath, DataStoreError};
use crate::proto::confer:: {SetRequest, SetResponse, GetRequest, GetResponse, DelRequest, DelResponse};
use crate::proto::confer::confer_server::Confer;

#[derive(Default)]
pub struct ConfigService {
    store : Arc<Mutex<HashMapDataStore>>,
}

impl ConfigService {
    pub fn new(data_store:HashMapDataStore) -> Self {
        ConfigService {
           store : Arc::new(Mutex::new(data_store)),
        }
    }
}

#[tonic::async_trait]
impl Confer for ConfigService {
    async fn set(&self, request:Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let req = request.into_inner();
        let path = ConfigPath::new(&req.path)
            .map_err(|e| Status::internal(e.to_string()))?;
        let data = self.store.lock();
        data.await.set(&path, req.value.into_bytes()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SetResponse{success: true}))
    }

    async fn get(&self, request:Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let req = request.into_inner();
        let path = ConfigPath::new(&req.path)
            .map_err(|e| Status::internal(e.to_string()))?;
        let data = self.store.lock();
        let val = data.await.get(&path).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetResponse {
            value: String::from_utf8(val).expect("{val} is is expected to be String" ),
        }))
    }

    async fn del(&self, request:Request<DelRequest>) -> Result<Response<DelResponse>, Status> {
        let req = request.into_inner();
        let path = ConfigPath::new(&req.path)
            .map_err(|e| Status::internal(e.to_string()))?;
        let data = self.store.lock();
        data.await.remove(&path).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DelResponse{success:true}))
    }


    //List all children of a given path (prefix)
    //pub fn list_children(&self, path: &str) -> Vec<String> {
        //let prefix = format!("{}/", path);
        //self.store.list(&prefix)
            //.iter()
            //.map(|key| key.strip_prefix(&prefix).unwrap_or(key).to_string())
            //.collect()
    //}
}

