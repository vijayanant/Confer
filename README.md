# Confer: A Distributed Configuration Manager

*Confer* is a distributed configuration manager. It uses the Raft consensus
algorithm to provide a reliable and consistent way to store and distribute
configuration data.

Confer is in its early stages of development; you are welcome to
[contribute](#contributing).

## What's Available Now?

- Cluster set-up.
- CLI tool to manage the cluster.
- API for clients to manage  configurations.
- A hierarchical (path-like) namespace for keys. (Ex. `/app/config/timeout`)

## What's on the Horizon?

There are lots and lots of things that can be added here. But let me not get
ahead of myself. Let me only add what can be done in near future. The plan
includes: 

- Adding persistence (currently logs are in-memory).
- Watch/Subscribe
- Dynamic discovery (preferably using Confer itself!)
- API for monitoring cluster status
- Metadata and versioning of snapshots?
- And, of course, lots and lots of testing!

## Building and Running (Work in Progress)

This project is very much a work in progress.  Building and running
instructions are likely to change as I learn and experiment. The best place for
now is to peek at the source code for the latest state of affairs.

### Prerequisites

- Rust and Cargo
- Protobuf compiler

### Building

#### Clone the repository:

   ```bash
   git clone https://github.com/vijayanant/confer.git
   ```
#### Build the project 
   
   ```bash
   cargo build
   ```

This should build both the server and the CLI (in the client).

## Setting Up A Cluster
This section provides a step-by-step guide on using the Confer CLI to establish
a basic two-node cluster.

### Start the first server (Node 1):

```
./target/debug/server --id 1 --server 127.0.0.1:10001
```  

This command launches the Confer server, configuring it as Node 1 with the
address _127.0.0.1:10001_.

### Start the second server (Node 2):

```
./server --id 2 --server 127.0.0.1:10002
```

Similarly, this starts the second Confer server instance, configured as Node 2
with the address _127.0.0.1:10002_.

### Initialize the cluster using the CLI:
```
./target/debug/confer-cli --address http://127.0.0.1:10001 init --nodes 1=http://127.0.0.1:10001,2=http://127.0.0.1:10002
```

This forms the Raft cluster with two nodes.  

- `--address` specifies that the CLI should send the init command to the Confer
  server running as Node 1.  This server will coordinate the cluster
initialization. 
- `init` invokes the init command. 
- `--nodes` provides the list of nodes that should be part of the initial
  cluster configuration.

The CLI command can be executed from any machine that has network connectivity
to the Confer servers. The `--address` option determines which server receives
the initialization request. 

### Verify the cluster: 
After the `init` command is executed, the Confer servers will begin the process
of forming a Raft cluster. This involves leader election and log replication.
Currently, the Confer CLI does not have a command to directly query the cluster
status. You will need to use other methods to verify that the cluster has been
successfully established. 

- Examine the log output of both Node 1 and Node 2. Look for messages
  indicating successful leader election, node joining, and log replication.

- Custom gRPC calls: There is no test client written, you will have to write
  your own client to set and get values and confirm.


### Dynamically add a learner node (Node 3):

Start the third server (Node 3):

```
./server --id 3 --server 127.0.0.1:10003
```

Use the `add-learner` command to add `Node 3` as a learner to the cluster. This
command can be sent to either `Node 1` or `Node 2``, as they are part of the
cluster.

``` 
./confer-cli --address http://127.0.0.1:10001 add-learner --node 3=http://127.0.0.1:10003
```

This command instructs `Node 1` to add `Node 3` as a learner.  Node 3 will
start replicating the log, but will not participate in voting.

To add Node 3 as a full member (voter) to the cluster, you would use the
`change-membership` command.  This command requires you to specify the desired
membership configuration for the cluster.

```
./confer-cli --address http://127.0.0.1:10001 change-membership --members 1,2,3
```

This command tells Node 1 to change the cluster membership to include nodes 1,
2, and 3 as voters.  Node 3 will participate in voting.

If Node 3 was previously a learner, this command will promote it to a voter.

## Confer Client Library/SDK

### Rust

The Confer Rust client library allows Rust applications to interact with a Confer server.

#### Usage

```rust
use confer_client::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = connect("http://[::1]:10001".to_string()).await?;
    set_value(&mut client, "my-key".to_string(), "my-value".as_bytes().to_vec()).await?;
    let value = get_value(&mut client, "my-key".to_string()).await?;
    println!("Value: {:?}", value);
    Ok(())
}
```

**TODO**

- Include a complete, runnable example.
- Show how to handle errors.
- Publish to crates.io, include the crate version.


## Contributing
Confer is an open-source project, and contributions are welcome!  If you find
something interesting or have suggestions, feel free to open an issue or better
send us a pull request.

If you'd like to contribute, please follow these guidelines:

- **Fork the repository:** Start by creating your own fork of the Confer
repository on GitHub.

- **Create a branch:** Create a new branch in your fork for the feature or bug
fix you're working on.

- **Make your changes:** Implement your changes, ensuring that they adhere to
the project's coding style and include appropriate tests.

- **Submit a pull request:** Once you're satisfied with your changes, submit a
pull request to the main Confer repository.

We appreciate your contributions!

## License
MIT License
