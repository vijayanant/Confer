use std::sync::Arc;
use std::collections::BTreeMap;
use tokio::sync::RwLock;
use openraft::{LogId, RaftLogId, Vote};
use openraft::storage::{LogState, RaftLogReader, RaftLogStorage, LogFlushed};
use openraft::StorageError;
use std::ops::RangeBounds;
use std::fmt::Debug;
use openraft::OptionalSend;

use crate::raft::config::{TypeConfig, NodeId, Entry};
#[derive(Debug)]
pub struct LogStorage {
    log: RwLock<BTreeMap<u64, Entry>>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            log: RwLock::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConferLogStore {
    log_storage: Arc<LogStorage>,
    vote: Arc<RwLock<Option<Vote<NodeId>>>>,
    committed: Arc<RwLock<Option<LogId<NodeId>>>>,
    last_purged_log_id: Arc<RwLock<Option<LogId<NodeId>>>>,
}

impl ConferLogStore {
    pub fn new() -> Self {
        Self {
            log_storage: Arc::new(LogStorage::new()),
            vote: Arc::new(RwLock::new(None)),
            committed: Arc::new(RwLock::new(None)),
            last_purged_log_id: Arc::new(RwLock::new(None)),
        }
    }
}


impl RaftLogReader<TypeConfig> for Arc<ConferLogStore> {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry>, StorageError<NodeId>> {

        let log = self.log_storage.log.read().await;

        let response = log.range(range.clone()).map(|(_, val)| val.clone()).collect::<Vec<_>>();
        Ok(response)
    }
}

impl RaftLogStorage<TypeConfig> for Arc<ConferLogStore> {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let log = self.log_storage.log.read().await;
        let last = log.iter().next_back().map(|(_, ent)| ent.get_log_id().clone());

        let last_purged = self.last_purged_log_id.read().await.clone();
        let last = match last {
            None => last_purged.clone(),
            Some(x) => Some(x),
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        *self.vote.write().await = Some(vote.clone());
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(self.vote.read().await.clone())
    }

    async fn save_committed(&mut self, committed: Option<LogId<NodeId>>) -> Result<(), StorageError<NodeId>> {
        *self.committed.write().await = committed;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogId<NodeId>>, StorageError<NodeId>> {
        Ok(self.committed.read().await.clone())
    }

    async fn append<I>(&mut self, entries: I, callback: LogFlushed<TypeConfig>) -> Result<(), StorageError<NodeId>>
    where I: IntoIterator<Item = Entry> + OptionalSend {
        let mut log = self.log_storage.log.write().await;
        for entry in entries {
            log.insert(entry.get_log_id().index, entry);
        }
        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let mut log = self.log_storage.log.write().await;
        let keys = log.range(log_id.index..).map(|(k, _)| k.clone()).collect::<Vec<_>>();
        for key in keys {
            log.remove(&key);
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        *self.last_purged_log_id.write().await = Some(log_id.clone());
        let mut log = self.log_storage.log.write().await;
        let keys = log.range(..=log_id.index).map(|(k, _)| k.clone()).collect::<Vec<_>>();
        for key in keys {
            log.remove(&key);
        }
        Ok(())
    }
}
