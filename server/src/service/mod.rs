use tonic::{Request, Response, Status};
use crate::store::KVStore;
use crate::proto::confer:: {SetRequest, SetResponse, GetRequest, GetResponse, DelRequest, DelResponse};
use crate::proto::confer::confer_server::Confer;

#[derive(Default)]
pub struct ConfigService {
    store : KVStore,
}

impl ConfigService {
    pub fn new() -> Self {
        println!("ConfigService -> new()");
        ConfigService {
           store : KVStore::new(),
        }
    }
}

#[tonic::async_trait]
impl Confer for ConfigService {
    async fn set(&self, request:Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let req = request.into_inner();
        self.store.set(&req.path, req.value);
        Ok(Response::new(SetResponse{success: true}))
    }

    async fn get(&self, request:Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let path = request.into_inner().path;
        let val = self.store.get(&path);

        Ok(Response::new(GetResponse {
            value: val.unwrap_or_default(),
        }))
    }

    async fn del(&self, request:Request<DelRequest>) -> Result<Response<DelResponse>, Status> {
        let path = request.into_inner().path;
        self.store.delete(&path);

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

//#[cfg(test)]
//mod tests {
    //use super::*;


    //#[test]
    //fn test_create_and_get() {
        //let config = ConfigService::new();
        //config.create("config/database/host", "localhost".to_string());
        //config.create("config/database/port", "5432".to_string());

        //assert_eq!(onfig.get("config/database/host"), Some("localhost".to_string()));
        //assert_eq!(onfig.get("config/database/port"), Some("5432".to_string()));
    //}

    //#[test]
    //fn test_delete() {
        //let config = ConfigService::new();
        //config.create("config/database/host", "localhost".to_string());
        //assert!(onfig.delete("config/database/host"));
        //assert_eq!(onfig.get("config/database/host"), None);
    //}

    //#[test]
    //fn test_list_children() {
        //let config = ConfigService::new();
        //config.create("config/services/user/session", "active".to_string());
        //config.create("config/services/user/preferences/theme", "dark".to_string());
        //config.create("config/services/admin/session", "inactive".to_string());

        //let mut children = config.list_children("config/services/user");
        //children.sort();
        //assert_eq!(children, vec!["preferences/theme".to_string(), "session".to_string()]);

        //let admin_children = config.list_children("config/services/admin");
        //assert_eq!(admin_children, vec!["session".to_string()]);
    //}

    //#[test]
    //fn test_non_existent_key() {
        //let config = ConfigService::new();
        //assert_eq!(onfig.get("config/nonexistent"), None);
        //assert!(!onfig.delete("config/nonexistent"));
    //}

    //#[test]
    //fn test_list_children_empty() {
        //let config = ConfigService::new();
        //let children = config.list_children("config/services");
        //assert!(children.is_empty());
    //}
//}

