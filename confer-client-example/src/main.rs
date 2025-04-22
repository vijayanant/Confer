use confer_client::*;
//use confer_client::error::ClientError;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use confer_client::confer::v1::config_update::UpdateType as ConfigUpdateKind;
use confer_client::confer::v1::cluster_update::UpdateType as ClusterUpdateKind;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_address = "http://127.0.0.1:10001".to_string();
    let config_key = "my-watched-config".to_string();
    let tls_config = None;

    // --- Create a single ConferClient instance ---
    println!("Creating a single ConferClient instance...");
    let client = Arc::new(ConferClient::new(server_address.clone(), tls_config.clone()).await?);
    let client_clone = Arc::clone(&client);

    // --- Watch Config Updates ---
    println!("\n--- Watching Config Updates ---");
    let config_key_clone = config_key.clone();
    tokio::spawn(async move {
        let result = client_clone.watch_config_with_callback(config_key_clone, move |update_result| {
            match update_result {
                Ok(update) => {
                    println!("CONFIG WATCHER: Received update for path: {:?}", update.path);
                    match update.update_type {
                        Some(ConfigUpdateKind::UpdatedValue(value)) => {
                            println!("  Kind: UpdatedValue, Value: {:?}", value);
                        }
                        Some(ConfigUpdateKind::Removed(_)) => {
                            println!("  Kind: Removed");
                        }
                        None => println!("  Kind: None"),
                    }
                }
                Err(e) => eprintln!("CONFIG WATCHER Error: {}", e),
            }
        }).await;
        if let Err(e) = result {
            eprintln!("Error setting up config watcher: {}", e);
        }
        println!("CONFIG WATCHER STARTED.");
    });

    // Simulate config changes using the original client
    println!("\nSimulating config changes...");
    client.set(config_key.clone(), "initial-value".as_bytes().to_vec()).await?;
    sleep(Duration::from_secs(2)).await;
    client.set(config_key.clone(), "updated-value-1".as_bytes().to_vec()).await?;
    sleep(Duration::from_secs(2)).await;
    client.remove(config_key.clone()).await?;
    sleep(Duration::from_secs(2)).await;

    // --- Watch Cluster Updates ---
    println!("\n--- Watching Cluster Updates ---");
    tokio::spawn(async move {
        let result = client.watch_cluster(move |update_result| {
            match update_result {
                Ok(update) => {
                    println!("CLUSTER WATCHER: Received update:");
                    match update.update_type {
                        Some(ClusterUpdateKind::LeaderChanged(node)) => {
                            println!("  Kind: LeaderChanged, Leader: {:?}", node);
                        }
                        Some(ClusterUpdateKind::MemberAdded(node)) => {
                            println!("  Kind: MemberAdded, Member: {:?}", node);
                        }
                        Some(ClusterUpdateKind::MemberRemoved(member_id)) => {
                            println!("  Kind: MemberRemoved, ID: {}", member_id);
                        }
                        None => println!("  Kind: None"),
                    }
                }
                Err(e) => eprintln!("CLUSTER WATCHER Error: {}", e),
            }
        });
        if let Err(e) = result.await {
            eprintln!("Error setting up cluster watcher: {}", e);
        }
        println!("CLUSTER WATCHER STARTED.");
    });

    // Simulate (potential) cluster changes - this will depend on your server implementation
    sleep(Duration::from_secs(15)).await;

    println!("\nExiting.");
    Ok(())
}
