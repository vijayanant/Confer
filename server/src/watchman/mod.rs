use tokio::sync::broadcast;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::proto::confer::v1::WatchUpdate;

/// `WatchMan` manages watchers for configuration changes and leader changes.
pub struct WatchMan {
    /// Stores watchers for configuration paths.  The key is the configuration path,
    /// and the value is a `broadcast::Sender` which is used to send updates to
    /// all clients watching that path.
    watchers: RwLock<HashMap<String, broadcast::Sender<WatchUpdate>>>,
    /// Stores watchers for leader change events.  Each sender is used to notify
    /// clients when a leader change occurs.
    leader_watchers: RwLock<Vec<broadcast::Sender<WatchUpdate>>>,
    /// The buffer size for the broadcast channels used for both configuration
    /// watchers and leader watchers.  This determines how many messages can be
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
        WatchMan {
            watchers: RwLock::new(HashMap::new()),
            leader_watchers: RwLock::new(Vec::new()),
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
    pub async fn watch(&self, path: String) -> Result<broadcast::Receiver<WatchUpdate>, tonic::Status> {
        let mut w = self.watchers.write().await;
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
        let mut w = self.watchers.write().await;
        w.remove(path);
    }

    /// Notifies all watchers for the given path of a configuration change.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the configuration that changed.
    /// * `message` - The message to send to the watchers.
    pub async fn notify(&self, path: &str, message: WatchUpdate) {
        if let Some(sender) = self.watchers.read().await.get(path) {
            let _ = sender.send(message);
        }
    }

    /// Returns a vector of all `broadcast::Sender`s for configuration watchers.
    /// This is used when the server loses leadership to notify all watchers.
    //pub async fn all_senders(&self) -> Vec<broadcast::Sender<WatchUpdate>> {
        //let r = self.watchers.read().await;
        //r.values().cloned().collect()
    //}

    /// Clears all configuration watchers.  This is called when the server
    /// loses leadership.
    pub async fn clear(&self) {
        let mut w = self.watchers.write().await;
        w.clear();
    }

    /// Starts watching for leader change events.  This creates a new
    /// `broadcast::Sender` for leader change events and subscribes to it.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver` which can be used to receive leader change notifications.
    pub async fn watch_leader(&self) -> Result<broadcast::Receiver<WatchUpdate>, tonic::Status> {
        let mut lw = self.leader_watchers.write().await;
        let (tx, rx) = broadcast::channel(self.buffer_size);
        lw.push(tx);
        Ok(rx)
    }

    /// Notifies all leader watchers of a leader change.
    pub async fn notify_leader_change(&self, message: WatchUpdate) {
        let lw = self.leader_watchers.read().await;
        for sender in lw.iter() {
            let _ = sender.send(message.clone());
        }
    }

    /// Clears all leader watchers.  This is called when the server
    /// loses leadership.
    pub async fn clear_leader_watchers(&self) {
        let mut lw = self.leader_watchers.write().await;
        lw.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time;
    use tokio::sync::broadcast::error::RecvError;
    use crate::proto::confer::v1::watch_update::Kind;
    use crate::proto::confer::v1::LeaderInfo;

    #[tokio::test]
    async fn test_watch_and_notify() {
        let watchman = WatchMan::new(10);
        let path = "test_path".to_string();
        let mut rx = watchman.watch(path.clone()).await.unwrap();

        // Notify the watcher
        let expected_data = vec![1, 2, 3];
        let message = WatchUpdate {
            kind: Some(Kind::UpdatedValue(expected_data.clone()))
        };
        watchman.notify(path.as_str(), message).await;

        // Receive the notification
        let received = rx.recv().await.unwrap();
        match received {
            WatchUpdate{kind: Some(Kind::UpdatedValue(data))} => {
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
        let message = WatchUpdate {
            kind: Some(Kind::UpdatedValue(vec![1,2,3]))
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
    async fn test_leader_watch_and_notify() {
        let watchman = WatchMan::new(16);
        let mut rx = watchman.watch_leader().await.unwrap();

        // Notify about leader change
        let message = WatchUpdate {
            kind : Some(Kind::LeaderChanged(LeaderInfo {address: "new_address".to_string()}))
        };
        watchman.notify_leader_change(message).await;

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
        let message_1 = WatchUpdate { kind: Some(Kind::UpdatedValue(vec![1])) };
        let message_2 = WatchUpdate { kind: Some(Kind::UpdatedValue(vec![2])) };

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
    async fn test_clear_leader_watchers() {
        let watchman = WatchMan::new(16);
        let mut rx = watchman.watch_leader().await.unwrap();

        // Clear leader watchers
        watchman.clear_leader_watchers().await;

        // Notify after clear should not send message
        let message = WatchUpdate {
            kind : Some(Kind::LeaderChanged(LeaderInfo{address:"new_address".to_string()}))
        };
        watchman.notify_leader_change(message).await;

        let timeout = time::Duration::from_millis(1_000);
        let result = tokio::time::timeout(timeout, rx.recv()).await;
        match result {
            Ok(Err(RecvError::Closed)) => {},
            _ => panic!("Expected Ok(Err(Closed)), got {:?}", result),
        }
    }
}

