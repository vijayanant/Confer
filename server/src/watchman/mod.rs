use tokio::sync::broadcast;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::proto::confer::v1::{ConfigUpdate, ClusterUpdate};

/// `WatchMan` manages watchers for configuration changes and leader changes.
pub struct WatchMan {
    /// Stores watchers for configuration paths.  The key is the configuration path,
    /// and the value is a `broadcast::Sender` which is used to send updates to
    /// all clients watching that path.
    config_watchers: RwLock<HashMap<String, broadcast::Sender<ConfigUpdate>>>,
    /// Stores watchers for cluster change events.
    cluster_watchers: RwLock<broadcast::Sender<ClusterUpdate>>,
    /// The buffer size for the broadcast channels used for both configuration
    /// watchers and cluster watchers.  This determines how many messages can be
    /// buffered if clients are slow to receive them.
    buffer_size: usize,
}

impl WatchMan {
    /// Creates a new `WatchMan` with the specified buffer size.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - The size of the buffer for the broadcast channels.
    pub fn new(buffer_size: usize) -> Self {
        let (cluster_tx, _) = broadcast::channel(buffer_size);
        WatchMan {
            config_watchers: RwLock::new(HashMap::new()),
            cluster_watchers: RwLock::new(cluster_tx),
            buffer_size,
        }
    }

    /// Starts watching the configuration at the given path.  This creates a new
    /// `broadcast::Sender` for the path if one doesn't already exist, and
    /// subscribes to it.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration to watch.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver` which can be used to receive updates for the path.
    pub async fn watch(&self, path: String) -> Result<broadcast::Receiver<ConfigUpdate>, tonic::Status> {
        let mut w = self.config_watchers.write().await;
        let sender = w.entry(path.clone()).or_insert_with(|| broadcast::channel(self.buffer_size).0);
        Ok(sender.subscribe())
    }

    /// Stops watching the configuration at the given path.  This removes the
    /// `broadcast::Sender` for the path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration to unwatch.
    pub async fn unwatch(&self, path: &str) {
        let mut w = self.config_watchers.write().await;
        w.remove(path);
    }

    /// Notifies all watchers for the given path of a configuration change.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration that changed.
    /// * `message` - The message to send to the watchers.
    pub async fn notify(&self, path: &str, message: ConfigUpdate) {
        if let Some(sender) = self.config_watchers.read().await.get(path) {
            let _ = sender.send(message);
        }
    }

    /// Starts watching for cluster change events.  This creates a new
    /// `broadcast::Sender` for cluster change events and subscribes to it.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver` which can be used to receive cluster change notifications.
    pub async fn watch_cluster(&self) -> Result<broadcast::Receiver<ClusterUpdate>, tonic::Status> {
        let sender = self.cluster_watchers.read().await;
        Ok(sender.subscribe())
    }

    /// Notifies all cluster watchers of a cluster change.
    pub async fn notify_cluster_change(&self, message: ClusterUpdate) {
        let sender = self.cluster_watchers.read().await;
        let _ = sender.send(message);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time;
    use tokio::sync::broadcast::error::RecvError;
    use crate::proto::confer::v1::watch_update::UpdateType;
    use crate::proto::confer::v1::Node;

    #[tokio::test]
    async fn test_watch_and_notify() {
        let watchman = WatchMan::new(10);
        let path = "test_path".to_string();
        let mut rx = watchman.watch(path.clone()).await.unwrap();

        // Notify the watcher
        let expected_data = vec![1, 2, 3];
        let message = ConfigUpdate {
            update_type: Some(UpdateType::ConfigUpdated(expected_data.clone()))
        };
        watchman.notify(path.as_str(), message).await;

        // Receive the notification
        let received = rx.recv().await.unwrap();
        match received {
            ConfigUpdate{update_type: Some(UpdateType::ConfigUpdated(data))} => {
                assert_eq!(data, expected_data);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[tokio::test]
    async fn test_unwatch() {
        let watchman = WatchMan::new(16);
        let path = "test_path".to_string();
        let mut rx = watchman.watch(path.clone()).await.unwrap();

        // Unwatch the path
        watchman.unwatch(path.as_str()).await;

        // Notify after unwatch should not send message
        let message = ConfigUpdate {
            update_type: Some(UpdateType::ConfigUpdated(vec![1,2,3]))
        };
        watchman.notify(path.as_str(), message).await;

        // Check that we don't receive a message within a short time
        let timeout = time::Duration::from_millis(1_000);
        let result = tokio::time::timeout(timeout, rx.recv()).await;
        match result {
            Ok(Err(RecvError::Closed)) => {},
            _ => panic!("Expected Ok(Err(Closed)), got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_cluster_watch_and_notify() {
        let watchman = WatchMan::new(16);
        let mut rx = watchman.watch_cluster().await.unwrap();

        // Notify about cluster change
        let message = ConfigUpdate {
            update_type : Some(UpdateType::LeaderChanged(Node {node_id: 1, addr: "new_address".to_string()}))
        };
        watchman.notify_cluster_change(message).await;

        // Receive the notification
        rx.recv().await.unwrap();
    }

    #[tokio::test]
    async fn test_clear_watchers() {
        let watchman = WatchMan::new(16);
        let path1 = "test_path_1".to_string();
        let path2 = "test_path_2".to_string();
        let mut _rx1 = watchman.watch(path1.clone()).await.unwrap();
        let mut _rx2 = watchman.watch(path2.clone()).await.unwrap();

        // Clear all watchers
        watchman.clear().await;

        // Notify after clear should not send messages
        let message_1 = ConfigUpdate { update_type: Some(UpdateType::ConfigUpdated(vec![1])) };
        let message_2 = ConfigUpdate { update_type: Some(UpdateType::ConfigUpdated(vec![2])) };

        watchman.notify(path1.as_str(), message_1).await;
        watchman.notify(path2.as_str(), message_2).await;

        let timeout = time::Duration::from_millis(1_000);
        let rx1_result = tokio::time::timeout(timeout, tokio::spawn(async move { _rx1.recv().await })).await.unwrap();
        let rx2_result = tokio::time::timeout(timeout, tokio::spawn(async move { _rx2.recv().await })).await.unwrap();

        match rx1_result {
            Ok(Err(RecvError::Closed)) => {},
            _ => panic!("Expected Ok(Err(Closed)), got {:?}", rx1_result),
        }
        match rx2_result {
            Ok(Err(RecvError::Closed)) => {},
            _ => panic!("Expected Ok(Err(Closed)), got {:?}", rx2_result),
        }

    }

    #[tokio::test]
    async fn test_clear_cluster_watchers() {
        let watchman = WatchMan::new(16);
        let mut rx = watchman.watch_cluster().await.unwrap();

        // Clear cluster watchers
        watchman.clear_cluster_watchers().await;

        // Notify after clear should not send message
        let message = ClusterUpdate {
            update_type : Some(UpdateType::LeaderChanged(Node {node_id: 1, addr:"new_address".to_string()}))
        };
        watchman.notify_cluster_change(message).await;

        let timeout = time::Duration::from_millis(1_000);
        let result = tokio::time::timeout(timeout, rx.recv()).await;
        match result {
            Ok(Err(RecvError::Closed)) => {},
            _ => panic!("Expected Ok(Err(Closed)), got {:?}", result),
        }
    }
}

