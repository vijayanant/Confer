use super::*;
use crate::proto::confer::v1::ConfigPath;

fn setup_state_machine() -> HashMapStateMachine {
    HashMapStateMachine::new()
}

#[tokio::test]
async fn test_set_get() {
    let state_machine = setup_state_machine();
    let path = ConfigPath { path: "test/key".to_string() };
    let value = b"test_value".to_vec();

    state_machine.set(&path, value.clone()).await.unwrap();
    let retrieved_value = state_machine.get(&path).await.unwrap();

    assert_eq!(retrieved_value, value);
}

#[tokio::test]
async fn test_get_not_found() {
    let state_machine = setup_state_machine();
    let path = ConfigPath { path: "nonexistent/key".to_string() };

    let result = state_machine.get(&path).await;

    assert!(matches!(result, Err(ConferError::NotFound { .. })));
}

#[tokio::test]
async fn test_remove() {
    let state_machine = setup_state_machine();
    let path = ConfigPath { path: "test/key".to_string() };
    let value = b"test_value".to_vec();

    state_machine.set(&path, value).await.unwrap();
    state_machine.remove(&path).await.unwrap();
    let result = state_machine.get(&path).await;

    assert!(matches!(result, Err(ConferError::NotFound { .. })));
}

#[tokio::test]
async fn test_list() {
    let state_machine = setup_state_machine();
    let path1 = ConfigPath { path: "test/key1".to_string() };
    let path2 = ConfigPath { path: "test/key2".to_string() };
    let value = b"test_value".to_vec();

    state_machine.set(&path1, value.clone()).await.unwrap();
    state_machine.set(&path2, value).await.unwrap();

    let mut list = state_machine.list(&ConfigPath { path: "test".to_string() }).await.unwrap();
    list.sort();

    assert_eq!(list, vec!["test/key1".to_string(), "test/key2".to_string()]);
}

#[tokio::test]
async fn test_set_get_raw_bytes() {
    let state_machine = setup_state_machine();
    let path = ConfigPath { path: "test/raw_bytes".to_string() };
    let raw_bytes = vec![1, 2, 3];

    state_machine.set(&path, raw_bytes.clone()).await.unwrap();
    let retrieved_bytes = state_machine.get(&path).await.unwrap();

    assert_eq!(retrieved_bytes, raw_bytes);
}
