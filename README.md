# Confer: A Distributed Configuration Manager (Learning Project)

Confer is a personal learning project focused on exploring the principles of distributed systems, specifically the Raft consensus algorithm. This project aims to provide a hands-on experience with Rust, gRPC, and the challenges of building a distributed system. Inspired by ZooKeeper, Confer serves as a platform for my experimentation and deeper understanding of these technologies.

## What's the Goal?

The primary goal of Confer is to help me learn Rust and Raft. Think of it as my digital sandbox where I can experiment with: 

* Rust: Sharpening my Rust skills and exploring its features.
* gRPC: Learning how to build efficient communication channels between services (using `tokio`).
* Raft: Demystifying the Raft consensus algorithm and implementing a raft based consensus in Rust (using `async-raft`).
* Distributed Systems:  Getting hands-on experience with Rust libraries available for consensus, replication, and fault tolerance.

## What's Happening Now?

Confer is still in its early stages (think "baby steps").  Currently, I'm working on:

*   A basic, thread-safe key-value store (the foundation!).
*   A gRPC service interface to interact with the store.
*   Wrestling with `async-raft` to bring distributed consensus to life.

## What's on the Horizon?

There are lots and lots of things that can added here. But let me not get ahead of myself. The plan (subject to change, as learning journeys often do) includes:

*   Fully utilise Raft for a truly distributed setup.
*   Adding persistence to survive restarts.
*   Implementing service discovery so nodes can find each other.
*   And, of course, lots and lots of testing!
 
## Building and Running (Work in Progress)

This project is very much a work in progress.  Building and running instructions are likely to change as I learn and experiment.  The best place for now is to peek at the source code for the latest state of affairs.

### Prerequisites

*   Rust and Cargo

### Building
#### 1. Clone the repository:

   ```bash
   git clone https://github.com/vijayanant/confer.git
   ```
#### 2. Build the project 
   
   ```bash
   cd confer/server
   cargo build
   ```
   
#### 3. Running (Currently Single Node)

Currently, Confer can be run as a single node for testing the gRPC interface.  Distributed functionality using Raft is still under development.

1. Start the server:

    ```bash
    cargo run
    ```
2. You cannot yet use the `grpcurl` as reflection APIs are not available. You will have to write a gRPC client to interact with the service. Sample client is avaialbe in the project.

### Contributing
As this is primarily a personal learning project, I'm not actively seeking contributions at this time. However, if you find something interesting or have suggestions, feel free to open an issue.

### License
MIT License

### Disclaimer
This project is a work in progress and is subject to change or discontinuation at any time.  It's provided as-is, without any warranty. Use at your own risk.

