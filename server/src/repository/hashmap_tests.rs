use super::*;
use crate::proto::confer::v1::ConfigPath;

fn setup_repository() -> HashMapConferRepository {
    HashMapConferRepository::new()
}

#[tokio::test]
async fn test_set_get() {
    let repository = setup_repository();
    let path = ConfigPath {
        path: "test/key".to_string(),
    };
    let value = b"test_value".to_vec();

    repository.set(&path, value.clone()).await.unwrap();
    let retrieved_value = repository.get(&path).await.unwrap();

    assert_eq!(retrieved_value, value);
}

#[tokio::test]
async fn test_get_not_found() {
    let repository = setup_repository();
    let path = ConfigPath {
        path: "nonexistent/key".to_string(),
    };

    let result = repository.get(&path).await;

    assert!(matches!(result, Err(ConferError::NotFound { .. })));
}

#[tokio::test]
async fn test_remove() {
    let repository = setup_repository();
    let path = ConfigPath {
        path: "test/key".to_string(),
    };
    let value = b"test_value".to_vec();

    repository.set(&path, value).await.unwrap();
    repository.remove(&path).await.unwrap();
    let result = repository.get(&path).await;

    assert!(matches!(result, Err(ConferError::NotFound { .. })));
}

#[tokio::test]
async fn test_list() {
    let repository = setup_repository();
    let path1 = ConfigPath {
        path: "test/key1".to_string(),
    };
    let path2 = ConfigPath {
        path: "test/key2".to_string(),
    };
    let value = b"test_value".to_vec();

    repository.set(&path1, value.clone()).await.unwrap();
    repository.set(&path2, value).await.unwrap();

    let mut list = repository
        .list(&ConfigPath {
            path: "test".to_string(),
        })
        .await
        .unwrap();
    list.sort();

    assert_eq!(list, vec!["test/key1".to_string(), "test/key2".to_string()]);
}

#[tokio::test]
async fn test_set_get_raw_bytes() {
    let repository = setup_repository();
    let path = ConfigPath {
        path: "test/raw_bytes".to_string(),
    };
    let raw_bytes = vec![1, 2, 3];

    repository.set(&path, raw_bytes.clone()).await.unwrap();
    let retrieved_bytes = repository.get(&path).await.unwrap();

    assert_eq!(retrieved_bytes, raw_bytes);
}
