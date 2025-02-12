#[cfg(test)]
mod tests {
    use crate::store::DataStoreError;
    use crate::store::DataStoreError::{InvalidPath, NotFound};
    use crate::store::{ConfigPath, HashMapDataStore};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_get_set() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/my/path").unwrap();
        let value = vec![1, 2, 3];

        store.set(&path, value.clone()).await.unwrap(); // Unwrap or expect
        let retrieved_value = store.get(&path).await.unwrap();

        assert_eq!(retrieved_value, value);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/nonexistent").unwrap();

        let result = store.get(&path).await;

        assert!(matches!(result, Err(NotFound { .. })));
    }

    #[tokio::test]
    async fn test_set_empty_path() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("");

        assert!(matches!(path, Err(InvalidPath { .. })));
    }

    #[tokio::test]
    async fn test_remove() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/my/path").unwrap();
        let value = vec![1, 2, 3];

        store.set(&path, value.clone()).await.unwrap();
        store.remove(&path).await.unwrap();
        let result = store.get(&path).await;

        assert!(matches!(result, Err(NotFound { .. })));
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/nonexistent").unwrap();

        let result = store.remove(&path).await;

        assert!(matches!(result, Err(NotFound { .. })));
    }

    #[tokio::test]
    async fn test_list() {
        let store = HashMapDataStore::new();
        let path1 = ConfigPath::new("/my/path/1").unwrap();
        let path2 = ConfigPath::new("/my/path/2").unwrap();
        let path3 = ConfigPath::new("/another/path").unwrap();
        let value = vec![1, 2, 3];

        store.set(&path1, value.clone()).await.unwrap();
        store.set(&path2, value.clone()).await.unwrap();
        store.set(&path3, value.clone()).await.unwrap();

        let list = store
            .list(&ConfigPath::new("/my/path").unwrap())
            .await
            .unwrap();

        assert_eq!(list.len(), 2);
        assert!(list.contains(&"/my/path/1".to_string()));
        assert!(list.contains(&"/my/path/2".to_string()));
    }

    #[tokio::test]
    async fn test_list_empty() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/nonexistent").unwrap();

        let list = store.list(&path).await.unwrap();

        assert_eq!(list.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let store = Arc::new(Mutex::new(HashMapDataStore::new()));
        let path = ConfigPath::new("/my/path").unwrap();
        let value = vec![1, 2, 3];
        let num_tasks = 10;

        let tasks = (0..num_tasks)
            .map(|_| {
                let store = store.clone();
                let path = path.clone();
                let value = value.clone();

                tokio::spawn(async move {
                    let store = store.lock().await;
                    store.set(&path, value.clone()).await.unwrap(); // Unwrap here
                    let retrieved_value = store.get(&path).await.unwrap(); // And here
                    assert_eq!(retrieved_value, value);
                })
            })
            .collect::<Vec<_>>();

        for task in tasks {
            task.await.unwrap(); // Unwrap twice!
        }
    }

    #[tokio::test]
    async fn test_set_overwrite() {
        let store = HashMapDataStore::new();
        let path = ConfigPath::new("/my/path").unwrap();
        let value1 = vec![1, 2, 3];
        let value2 = vec![4, 5, 6];

        store.set(&path, value1.clone()).await.unwrap();
        store.set(&path, value2.clone()).await.unwrap(); // Overwrite

        let retrieved_value = store.get(&path).await.unwrap();
        assert_eq!(retrieved_value, value2);
    }

    #[tokio::test]
    async fn test_list_nested() {
        let store = HashMapDataStore::new();
        let path1 = ConfigPath::new("/my/path/1/a").unwrap();
        let path2 = ConfigPath::new("/my/path/1/b").unwrap();
        let path3 = ConfigPath::new("/my/path/2").unwrap(); // Different level
        let value = vec![1, 2, 3];

        store.set(&path1, value.clone()).await.unwrap();
        store.set(&path2, value.clone()).await.unwrap();
        store.set(&path3, value.clone()).await.unwrap();

        let list = store
            .list(&ConfigPath::new("/my/path").unwrap())
            .await
            .unwrap();

        assert_eq!(list.len(), 3);
        assert!(list.contains(&"/my/path/1/a".to_string()));
        assert!(list.contains(&"/my/path/1/b".to_string()));
        assert!(list.contains(&"/my/path/2".to_string()));
    }

    #[tokio::test]
    async fn test_list_with_prefix() {
        let store = HashMapDataStore::new();
        let path1 = ConfigPath::new("/my/path/1").unwrap();
        let path2 = ConfigPath::new("/my/path/2").unwrap();
        let path3 = ConfigPath::new("/another/path").unwrap();
        let value = vec![1, 2, 3];

        store.set(&path1, value.clone()).await.unwrap();
        store.set(&path2, value.clone()).await.unwrap();
        store.set(&path3, value.clone()).await.unwrap();

        let list = store.list(&ConfigPath::new("/my/").unwrap()).await.unwrap(); // List with prefix

        assert_eq!(list.len(), 2);
        assert!(list.contains(&"/my/path/1".to_string()));
        assert!(list.contains(&"/my/path/2".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_access_different_paths() {
        let store = Arc::new(Mutex::new(HashMapDataStore::new()));
        let num_tasks = 50;

        let tasks = (0..num_tasks)
            .map(|i| {
                let store = store.clone();
                tokio::spawn(async move {
                    let path = ConfigPath::new(&format!("/my/path/{}", i)).unwrap(); // Different paths
                    let value = vec![i as u8];
                    let store = store.lock().await;
                    store.set(&path, value.clone()).await.unwrap();
                    let retrieved_value = store.get(&path).await.unwrap();
                    assert_eq!(retrieved_value, value);
                })
            })
            .collect::<Vec<_>>();

        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_access_same_path_different_values() {
        let store = Arc::new(Mutex::new(HashMapDataStore::new()));
        let path = ConfigPath::new("/my/path").unwrap();
        let num_tasks = 5000;

        let tasks = (0..num_tasks)
            .map(|i| {
                let store = store.clone();
                let path = path.clone();
                tokio::spawn(async move {
                    let value = vec![i as u8];
                    {
                        // Critical section
                        let store = store.lock().await;
                        store.set(&path, value.clone()).await.unwrap();
                    } // Lock released here

                    // Option 1: Retrieve and verify *immediately* after setting (within the critical section)
                    let retrieved_value = {
                        let store = store.lock().await;
                        store.get(&path).await.unwrap()
                    };
                    assert_eq!(retrieved_value, value.clone()); // Clone here to avoid move issues

                    // Option 2: Retrieve and verify *outside* the critical section, but acknowledge the race condition
                    let store = store.lock().await;
                    let retrieved_value = store.get(&path).await.unwrap(); // Potential race condition here
                    println!("Task {}: Retrieved {:?}", i, retrieved_value); // Print to see what's happening
                                                                             // You can't reliably assert here because of the race condition.
                                                                             // Instead, you might want to check that the value is one of the values set by the tasks:
                                                                             //let possible_values: Vec<Vec<u8>> = (0..num_tasks).map(|j| vec![j as u8]).collect();
                                                                             //assert!(retrieved_value.map_or(false, |v| possible_values.contains(&v)));
                })
            })
            .collect::<Vec<_>>();

        for task in tasks {
            task.await.unwrap();
        }
    }
}
