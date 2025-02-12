# Raft Concepts Refresher

Raft is a consensus algorithm designed to manage a replicated state machine in a distributed system.  It ensures that all nodes in the cluster agree on the state of the machine, even in the presence of failures.  Here are the core concepts:

## Roles:

Each node in a Raft cluster can be in one of three states:

- **Leader**: There is only one leader at a time. The leader is responsible for receiving client requests, replicating log entries to followers, and managing the consensus process.
- **Follower**: Followers passively accept log entries from the leader and apply them to their local state machines. They also vote for candidates during leader elections.
- **Candidate**: A follower becomes a candidate when it suspects the leader has failed. Candidates campaign for leadership by requesting votes from other followers.

## Leader Election:

- **Heartbeats**: Leaders periodically send heartbeat messages to followers. If a follower doesn't receive a heartbeat from the leader within a certain timeout period, it becomes a candidate.
- **Campaigning**: Candidates start a new election term and request votes from other followers. If a candidate receives a majority of votes, it becomes the new leader.
- **Term**: A term is a period of time that begins with a leader election and ends with a new leader election. Terms are used to detect stale data and prevent conflicts.

## Log Replication:

- **Log Entries**: All changes to the state machine are recorded as log entries. Log entries are replicated from the leader to the followers.
- **Append Entries RPC**: The leader sends Append Entries RPCs to followers to replicate log entries.
- **Commit**: Once a log entry has been replicated to a majority of followers, it is considered committed. Committed entries are then applied to the state machine.
- **Consistency**: Raft guarantees that all committed log entries are applied to the state machine in the same order on all nodes.

## State Machine:

- **Replicated State**: Each node in the cluster maintains a copy of the state machine. The state machine is the core data that the Raft cluster is managing.
- **Deterministic Application**: Log entries are applied to the state machine in a deterministic manner. This ensures that all nodes have the same state.

## Safety Properties:

Raft guarantees several important safety properties:

- **Election Safety**: Only one leader can be elected in a given term.
- **Log Matching**: If two logs contain an entry with the same index and term, then the logs are identical in all entries up to the given index 
- **Leader Completeness**: If a log entry is committed in a given term, then it will be present on the logs of all future leaders.
**State Machine Safety**: If a server has applied a particular log entry to its state machine, all servers will eventually apply that log entry to their state machines.

## Handling Failures:

Raft is designed to tolerate various failures:

- **Node Crashes**: Raft can handle the crash and restart of any number of nodes (as long as a majority of nodes are still functioning).
- **Network Partitions**: Raft can handle network partitions that isolate some nodes from others.
- **Message Loss**: Raft uses timeouts and retries to handle message loss.

## Log Compaction:

- **Snapshots**: To keep the log manageable, Raft uses snapshots. A snapshot is a snapshot of the state machine at a particular point in time.
- **Truncation**: Older log entries that have already been applied to the state machine can be truncated and replaced with the snapshot.

## Key Interactions:

- **Client Requests**: Clients send requests to the leader.
- **Append Entries RPC**: Leader replicates log entries to followers.
- **Request Vote RPC**: Candidates request votes from followers during leader elections.
