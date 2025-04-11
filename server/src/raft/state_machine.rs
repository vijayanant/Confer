use std::io::Cursor;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

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
use tracing::{debug, error, info};

use crate::raft::config::{Node, NodeId, TypeConfig};
use crate::raft::operation::{Operation, OperationResponse};
use crate::repository::ConferRepository;

/// Represents a stored snapshot of the state machine.
#[derive(Debug)]
pub struct StoredSnapshot {
    /// Metadata about the snapshot.
    pub meta: SnapshotMeta<NodeId, Node>,
    /// The serialized data of the snapshot.
    pub data: Vec<u8>,
}

/// The state machine for the Raft consensus algorithm.
///
/// This state machine is responsible for applying committed log entries and managing the state of the
/// application.  It uses a `ConferRepository` to persist data.
#[derive(Debug, Default)]
pub struct StateMachine<T: ConferRepository> {
    /// The repository used to store the application's data.
    pub repository: Arc<RwLock<T>>,
    /// The current membership configuration of the Raft cluster.
    pub membership: RwLock<StoredMembership<NodeId, Node>>,
    /// The log ID of the last applied log entry.
    pub last_applied_log: RwLock<Option<LogId<NodeId>>>,
    /// The most recent snapshot of the state machine.
    pub snapshot: RwLock<Option<StoredSnapshot>>,
    /// A counter used to generate unique snapshot IDs.
    snapshot_idx: AtomicU64,
}

impl<T: ConferRepository> StateMachine<T> {
    /// Creates a new `StateMachine` with the given repository.
    pub fn new(repository: T) -> Self {
        StateMachine {
            repository: Arc::new(RwLock::new(repository)),
            membership: RwLock::new(StoredMembership::default()),
            last_applied_log: RwLock::new(None),
            snapshot: RwLock::new(None),
            snapshot_idx: AtomicU64::new(0),
        }
    }
}

/// Implementation of `RaftSnapshotBuilder` for `StateMachine`.
///
/// This implementation provides the functionality to create snapshots of the state machine's data.
impl<T: ConferRepository> RaftSnapshotBuilder<TypeConfig> for Arc<StateMachine<T>> {
    /// Builds a snapshot of the state machine.
    ///
    /// This method retrieves the serialized data from the repository, constructs a `SnapshotMeta`,
    /// stores the snapshot, and returns a `Snapshot` struct.
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let repository = self.repository.read().await;
        let data = repository.get_serialized_data().await.map_err(|e| {
            error!("Failed to get serialized data: {}", e);
            StorageIOError::read_state_machine(&e)
        })?;
        drop(repository); // Explicitly drop the read lock

        let last_applied_log = *self.last_applied_log.read().await;
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
            last_membership,
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

/// Implementation of `RaftStateMachine` for `StateMachine`.
///
/// This implementation provides the core functionality of the state machine, including applying log
/// entries and managing snapshots.
impl<T: ConferRepository> RaftStateMachine<TypeConfig> for Arc<StateMachine<T>> {
    type SnapshotBuilder = Self;

    /// Returns the last applied log ID and the current membership configuration.
    #[tracing::instrument(level = "trace", skip(self))]
    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, Node>), StorageError<NodeId>> {
        let last_applied_log = *self.last_applied_log.read().await;
        let last_membership = self.membership.read().await.clone();
        Ok((last_applied_log, last_membership))
    }

    /// Applies a series of log entries to the state machine.
    ///
    /// This method iterates over the given entries, updates the state machine's data, and returns a
    /// vector of `OperationResponse` values.
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
            debug!(%entry.log_id, "replicate to sm");

            *last_applied_log = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Blank => res.push(OperationResponse::Success),
                EntryPayload::Normal(ref req) => match req {
                    Operation::Set { path, value } => {
                        repository
                            .set(path, value.value.clone())
                            .await
                            .map_err(|e| {
                                error!("Failed to set data: {}", e);
                                StorageIOError::<NodeId>::apply(entry.log_id, &e)
                            })?;
                        res.push(OperationResponse::Success);
                    }
                    Operation::Remove { path } => {
                        repository.remove(path).await.map_err(|e| {
                            error!("Failed to remove data: {}", e);
                            StorageIOError::<NodeId>::apply(entry.log_id, &e)
                        })?;
                        res.push(OperationResponse::Success);
                    }
                },
                EntryPayload::Membership(ref mem) => {
                    *membership = StoredMembership::new(*last_applied_log, mem.clone());
                    res.push(OperationResponse::Success);
                }
            };
        }
        Ok(res)
    }

    /// Begins the process of receiving a snapshot.
    #[tracing::instrument(level = "trace", skip(self))]
    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>, StorageError<NodeId>>
    {
        Ok(Box::new(Cursor::new(Vec::new()))) //changed to empty vector
    }

    /// Installs a snapshot of the state machine.
    ///
    /// This method updates the repository's data, last applied log ID, and membership configuration
    /// with the data from the provided snapshot.
    #[tracing::instrument(level = "trace", skip(self, snapshot))]
    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        info!(
            { snapshot_size = snapshot.get_ref().len() },
            "decoding snapshot for installation"
        );

        let new_snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        let mut repository = self.repository.write().await;
        repository
            .replace_data(new_snapshot.data.clone())
            .await
            .map_err(|e| {
                error!("Failed to replace repository data: {}", e);
                StorageIOError::<NodeId>::write_state_machine(&e)
            })?;
        drop(repository); // Explicitly drop

        let mut last_applied_log = self.last_applied_log.write().await;
        *last_applied_log = meta.last_log_id;

        let mut membership = self.membership.write().await;
        *membership = meta.last_membership.clone();

        let mut current_snapshot = self.snapshot.write().await;
        *current_snapshot = Some(new_snapshot);

        Ok(())
    }

    /// Retrieves the current snapshot of the state machine.
    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
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

    /// Retrieves the snapshot builder.
    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
