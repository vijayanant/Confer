# Setting Up A Cluster
This section provides a step-by-step guide on using the Confer CLI to establish
a basic two-node cluster.

## Start the first server (Node 1):

```
./server --id 1 --server 127.0.0.1:10001
```  

This command launches the Confer server, configuring it as Node 1 with the
address _127.0.0.1:10001_.

## Start the second server (Node 2):

```
./server --id 2 --server 127.0.0.1:10002
```

Similarly, this starts the second Confer server instance, configured as Node 2
with the address _127.0.0.1:10002_.

## Initialize the cluster using the CLI:
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

## Verify the cluster: 
After the `init` command is executed, the Confer servers will begin the process
of forming a Raft cluster. This involves leader election and log replication.
Currently, the Confer CLI does not have a command to directly query the cluster
status. You will need to use other methods to verify that the cluster has been
successfully established. 

- Examine the log output of both Node 1 and Node 2. Look for messages
  indicating successful leader election, node joining, and log replication.

- Custom gRPC calls: There is no test client written, you will have to write
  your own client to set and get values and confirm.


## Dynamically add a learner node (Node 3):

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
