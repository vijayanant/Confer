use clap::{Parser, Subcommand};
use tonic::transport::Channel;
use tonic::{Request, Status};

// Include the generated gRPC client code.  Make sure
// the path is correct for your project structure.
pub mod confer {
    pub mod v1 {
        tonic::include_proto!("confer.v1");
    }
}

use confer::v1::{
    confer_service_client::ConferServiceClient,
    AddLearnerRequest, ChangeMembershipRequest, ConfigPath, InitRequest, Node,
};

#[derive(Parser)]
#[clap(version = "1.0", author = "Your Name", about = "CLI for Confer App (Cluster Management)")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
    #[clap(long, default_value = "http://[::1]:45671")]
    address: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new Raft cluster.
    Init {
        /// List of node addresses in the format "node_id=address" (e.g., "1=http://localhost:10001,2=http://localhost:10002")
        #[clap(required = true, value_parser = parse_node_address)]
        nodes: Vec<(u64, String)>,
    },
    /// Adds a learner node to the Raft cluster.
    AddLearner {
        /// Node ID and address of the learner node in the format "node_id=address" (e.g., "3=http://localhost:10003")
        #[clap(required = true, value_parser = parse_node_address)]
        node: (u64, String),
    },
    /// Changes the membership of the Raft cluster.
    ChangeMembership {
        /// List of voter node IDs (e.g., "1,2,3")
        #[clap(required = true, value_parser = parse_node_ids)]
        members: Vec<u64>,
        /// Whether to retain existing configuration
        #[clap(long)]
        retain: bool,
    },
}

// Custom parser for node addresses
fn parse_node_address(s: &str) -> Result<(u64, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        Err(format!("Invalid node address format: {}", s))
    } else {
        let node_id = parts[0]
            .parse::<u64>()
            .map_err(|e| format!("Invalid node ID: {}", e))?;
        let address = parts[1].to_string();
        Ok((node_id, address))
    }
}

// Custom parser for comma-separated node IDs
fn parse_node_ids(s: &str) -> Result<Vec<u64>, String> {
    s.split(',')
        .map(|id| {
            id.parse::<u64>()
                .map_err(|e| format!("Invalid node ID: {}", e))
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let channel = Channel::from_shared(cli.address.clone())
        .map_err(|e| format!("failed to create channel: {}", e))?
        .connect()
        .await
        .map_err(|e| format!("failed to connect to server: {}", e))?;

    let mut client = ConferServiceClient::new(channel);

    match cli.command {
        Commands::Init { nodes } => {
            let nodes_proto: Vec<Node> = nodes
                .into_iter()
                .map(|(node_id, addr)| Node { node_id, addr })
                .collect();
            let request = Request::new(InitRequest { nodes: nodes_proto });
            let response = client.init(request).await?;
            println!("Cluster initialized: {:?}", response.into_inner());
        }
        Commands::AddLearner { node } => {
            let node_proto = Node {
                node_id: node.0,
                addr: node.1,
            };
            let request = Request::new(AddLearnerRequest { node: Some(node_proto) });
            let response = client.add_learner(request).await?;
            println!("Learner added: {:?}", response.into_inner());
        }
        Commands::ChangeMembership { members, retain } => {
            let request = Request::new(ChangeMembershipRequest {
                members,
                retain,
            });
            let response = client.change_membership(request).await?;
            println!("Membership changed: {:?}", response.into_inner());
        }
    }

    Ok(())
}
