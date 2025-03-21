use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use openraft::storage::RaftStateMachine;
use openraft::storage::Snapshot;
use openraft::Entry;
use openraft::EntryPayload;
use openraft::LogId;
use openraft::RaftSnapshotBuilder;
use openraft::SnapshotMeta;
use openraft::StorageError;
use openraft::StorageIOError;
use openraft::StoredMembership;

use tokio::sync::RwLock;

use crate::raft::config::{TypeConfig, Node, NodeId};
use crate::raft::operation::{Operation, OperationResponse};
use crate::repository::ConferRepository;

#[derive(Debug)]
pub struct StoredSnapshot {
    pub meta: SnapshotMeta<NodeId, Node>,
    pub data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct StateMachine<T: ConferRepository> {
    pub repository: RwLock<T>,
    pub membership: RwLock<StoredMembership<NodeId, Node>>,
    pub last_applied_log: RwLock<Option<LogId<NodeId>>>,
    pub snapshot: RwLock<Option<StoredSnapshot>>,
    snapshot_idx: AtomicU64,
}

impl<T: ConferRepository> StateMachine<T> {
    pub fn new(repository: T) -> Self {
        StateMachine {
            repository: RwLock::new(repository),
            membership: RwLock::new(StoredMembership::default()),
            last_applied_log: RwLock::new(None),
            snapshot: RwLock::new(None),
            snapshot_idx: AtomicU64::new(0),
        }
    }
}

impl<T: ConferRepository> RaftSnapshotBuilder<TypeConfig> for Arc<StateMachine<T>> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let repository = self.repository.read().await;
        let data = repository
            .get_serialized_data()
            .await
            .map_err(|e| StorageIOError::read_state_machine(&e))?;
        drop(repository);

        let last_applied_log = self.last_applied_log.read().await.clone();
        let last_membership = self.membership.read().await.clone();

        let mut current_snapshot = self.snapshot.write().await;

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{}", last.leader_id, last.index, snapshot_idx)
        } else {
            format!("--{}", snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership: last_membership,
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        *current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

impl<T: ConferRepository> RaftStateMachine<TypeConfig> for Arc<StateMachine<T>> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, Node>), StorageError<NodeId>> {
        let last_applied_log = self.last_applied_log.read().await.clone();
        let last_membership = self.membership.read().await.clone();
        Ok((last_applied_log, last_membership))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply<I>(&mut self, entries: I) -> Result<Vec<OperationResponse>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + Send,
    {
        let mut res = Vec::new();

        let mut membership = self.membership.write().await;
        let mut last_applied_log = self.last_applied_log.write().await;
        let repository = self.repository.write().await;

        for entry in entries {
            tracing::debug!(%entry.log_id, "replicate to sm");

            *last_applied_log = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Blank => res.push(OperationResponse::Success),
                EntryPayload::Normal(ref req) => {
                    match req {
                        Operation::Set { path, value } => {
                            repository.set(&path, value.value.clone()).await.map_err(|e| StorageIOError::<NodeId>::apply(entry.log_id, &e))?;
                            res.push(OperationResponse::Success);
                        }
                        Operation::Remove { path } => {
                            repository.remove(&path).await.map_err(|e| StorageIOError::<NodeId>::apply(entry.log_id, &e))?;
                            res.push(OperationResponse::Success);
                        }
                    }
                }
                EntryPayload::Membership(ref mem) => {
                    *membership = StoredMembership::new(*last_applied_log, mem.clone());
                    res.push(OperationResponse::Success);
                }
            };
        }
        Ok(res)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    #[tracing::instrument(level = "trace", skip(self, snapshot))]
    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        tracing::info!(
            { snapshot_size = snapshot.get_ref().len() },
            "decoding snapshot for installation"
        );

        let new_snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        //let updated_repository_data = serde_json::from_slice(&new_snapshot.data)
            //.map_err(|e| StorageIOError::read_snapshot(Some(new_snapshot.meta.signature()), &e))?;

        let mut repository = self.repository.write().await;
        repository.replace_data(new_snapshot.data.clone()).await.map_err(|e| StorageIOError::<NodeId>::write_state_machine(&e))?;
        drop(repository);

        let mut last_applied_log = self.last_applied_log.write().await;
        *last_applied_log = meta.last_log_id;

        let mut membership = self.membership.write().await;
        *membership = meta.last_membership.clone();

        let mut current_snapshot = self.snapshot.write().await;
        *current_snapshot = Some(new_snapshot);

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
        match &*self.snapshot.read().await {
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta.clone(),
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
