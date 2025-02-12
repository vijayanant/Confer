#[cfg(test)]
mod tests {
    use crate::proto::confer::confer_server::Confer;
    use crate::proto::confer::{
        DelRequest, GetRequest, SetRequest,
    }; // Import your generated proto code (e.g., mod proto;)
    use crate::service::ConfigService;
     // Update with your error type
    use crate::store::HashMapDataStore; // Update with your actual path
    
    
    use tonic::Request; // Import your service implementation

    #[tokio::test]
    async fn test_get_value() {
        let store = HashMapDataStore::new();
        let service = ConfigService::new(store);
        let path = "/my/path";
        let value = "my_value".to_string();

        let set_request = Request::new(SetRequest {
            path: path.to_string(),
            value: value.clone(),
        });

        service.set(set_request).await.unwrap();

        let get_request = Request::new(GetRequest {
            path: path.to_string(),
        });
        let response = service.get(get_request).await.unwrap();

        assert_eq!(response.into_inner().value, value);
    }

    #[tokio::test]
    async fn test_get_value_not_found() {
        let store = HashMapDataStore::new();
        let service = ConfigService::new(store);
        let path = "/nonexistent";

        let get_request = Request::new(GetRequest {
            path: path.to_string(),
        });
        let response = service.get(get_request).await;

        assert!(response.is_err());
        let status = response.unwrap_err();

        assert_eq!(status.message(), "Path not found: /nonexistent");
        assert_eq!(status.code(), tonic::Code::Internal);
    }

    #[tokio::test]
    async fn test_set_value() {
        let store = HashMapDataStore::new();
        let service = ConfigService::new(store);
        let path = "/my/path";
        let value = "my_value".to_string();

        let set_request = Request::new(SetRequest {
            path: path.to_string(),
            value,
        });
        let response = service.set(set_request).await.unwrap();

        assert!(response.into_inner().success);
    }

    #[tokio::test]
    async fn test_del_value() {
        let store = HashMapDataStore::new();
        let service = ConfigService::new(store);
        let path = "/my/path";
        let value = "my_value".to_string();

        let set_request = Request::new(SetRequest {
            path: path.to_string(),
            value,
        });
        service.set(set_request).await.unwrap();

        let del_request = Request::new(DelRequest {
            path: path.to_string(),
        });
        let response = service.del(del_request).await.unwrap();

        assert!(response.into_inner().success);

        let get_request = Request::new(GetRequest {
            path: path.to_string(),
        });
        let get_response = service.get(get_request).await;

        assert!(get_response.is_err());
        let status = get_response.unwrap_err();
        assert_eq!(status.message(), "Path not found: /my/path");
        assert_eq!(status.code(), tonic::Code::Internal);
    }

    //#[tokio::test]
    //async fn test_concurrent_requests() {
    //let store = Arc::new(Mutex::new(HashMapDataStore::new()));
    //let service = ConfigService::new(store.clone()); // Share the same store

    //let num_tasks = 10;
    //let path = "/my/path";
    //let value_prefix = "value_".to_string();

    //let tasks = (0..num_tasks).map(|i| {
    //let service = service.clone(); // Clone the service (which contains the Arc)
    //tokio::spawn(async move {
    //let set_request = Request::new(SetRequest {
    //path: path.to_string(),
    //value: format!("{}{}", value_prefix, i),
    //});
    //service.set(set_request).await.unwrap();

    //let get_request = Request::new(GetRequest { path: path.to_string() });
    //let get_response = service.get(get_request).await.unwrap();

    //Because of concurrency, we can't assert on a specific value,
    //but we can check that the retrieved value *starts* with the prefix.
    //assert!(get_response.into_inner().value.starts_with(&value_prefix));
    //})
    //}).collect::<Vec<_>>();

    //for task in tasks {
    //task.await.unwrap();
    //}
    //}

    // ... Add more tests for error handling, invalid requests, etc.
}
