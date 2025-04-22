# Confer: A Platform for Distributed Coordination

*Confer* is a platform for building distributed applications. At its core, it provides strong consistency, fault tolerance, and coordination primitives built on the Raft consensus algorithm.

Originally conceived as a distributed configuration manager, Confer has grown into a general-purpose foundation for building reliable systems that need to replicate and agree on state across a cluster.

> **Why the name?**
> The word *"confer"* means to consult, deliberate, or come to an agreement—just like nodes in a distributed system reaching consensus. It's a fitting name for a platform built around coordination and agreement.

This is an ongoing learning project, evolving organically as new ideas and use cases emerge.

---

## What's Available Now?

- Raft-based consensus and log replication
- Cluster initialization and dynamic membership via CLI
- Hierarchical key-value store with path-like namespaces (e.g. `/app/config/timeout`)
- Watch/subscribe API:
    - `WatchConfig`: Clients can receive real-time updates on key changes.
    - `WatchCluster`: Clients can subscribe to and receive updates about the cluster's voting members (additions and removals) and the current leader.
- Rust client library (gRPC-based) enabling interaction with the Confer cluster, including subscribing to configuration and cluster updates.

---

## Beyond Configuration: The Vision for Confer

Confer began as an experiment in building a distributed configuration manager. But as the foundational elements took shape—consensus, coordination, state replication—it became clear that the same core could support a wide variety of distributed systems use cases.

### Core Capabilities

- **Distributed Key-Value Store**: Store data across a cluster with strong consistency.
- **Strong Consistency**: Writes are replicated safely to all nodes using Raft.
- **Fault Tolerance**: Tolerates node failures while maintaining availability.
- **Watch/Subscribe Mechanism**: Clients can subscribe to key changes in real time.

### Potential Applications

These capabilities unlock a wide range of distributed coordination patterns:

- **Service Discovery**: Store and watch for service endpoint registrations.
- **Distributed Locking**: Ensure mutual exclusion for shared resources.
- **Leader Election**: Raft's native leader election can coordinate responsibilities.
- **Metadata Management**: Store and replicate operational metadata reliably.
- **Real-Time State Distribution**: Push updates to dashboards or monitoring tools.
- **General Coordination Primitives**: Build custom protocols on top of consistent state.

### A Note on Comparisons

You might wonder how Confer compares to established tools like ZooKeeper or etcd. The truth is: it doesn't—at least not yet. This project wasn't built to compete, but to learn.

Confer is an experiment. It's about exploring distributed consensus, understanding coordination, and building something from scratch. That said, it's interesting to reflect on how similar foundations can support real-world use cases, and what we might learn by building them ourselves.

If this ever becomes practical or production-ready, that’ll be a fun conversation to have. For now, it’s about curiosity, clarity, and learning in public.

---

## What's Next?

- Persistence (logs and snapshots)
- Metadata and versioning support
- Cluster health and status APIs
- Service discovery APIs
- More client libraries (Go, JS?)
- Testing, testing, testing

---

## Building and Running

Build instructions, CLI usage, and cluster setup examples have been moved to the [docs/](./docs) directory. See:

- [docs/build.md](./docs/build.md) — Build prerequisites and steps
- [docs/cluster.md](./docs/cluster.md) — Cluster setup and usage examples
- [docs/client.md](./docs/client.md) — Using the Rust client SDK

---

## Contributing

This project is a learning journey, but contributions are welcome! If you have ideas, suggestions, or find bugs, feel free to open an issue or a pull request.

---

## License

MIT License

