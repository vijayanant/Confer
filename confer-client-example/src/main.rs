use confer_client::*;
use std::error::Error;
use tokio::time::{sleep, Duration};
use tokio::task::JoinHandle;
use confer_client::confer::v1::watch_update::Kind;
//use tonic::transport::ClientTlsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // --- Configuration ---
    let server_address_1 = "http://127.0.0.1:10001".to_string(); // Leader
    let server_address_2 = "http://127.0.0.1:10002".to_string(); // Follower
    let key = "my-watched-key".to_string();
    let initial_value = "initial-value".as_bytes().to_vec();
    let tls_config = None; // Or Some(ClientTlsConfig::new()) if you have TLS setup

    // --- Connect to Confer servers ---
    println!("Connecting to Confer servers...");
    let mut leader_client = ConferClient::new(server_address_1.clone(), tls_config.clone()).await?;
    let mut follower_client = ConferClient::new(server_address_2.clone(), tls_config.clone()).await?;

    // --- Basic Data Manipulation (set, get, delete) ---
    println!("\n\n--- Basic Data Manipulation ---");
    println!("\nSetting initial value for key: {} on leader ({}): value: {:?}", key, server_address_1, initial_value);
    leader_client.set(key.clone(), initial_value.clone()).await?;

    println!("\nGetting value for key: {} on follower ({}):", key, server_address_2);
    let retrieved_value_follower = follower_client.get(key.clone()).await?;
    println!("Retrieved value on follower: {:?}", retrieved_value_follower);

    println!("\nGetting value for key: {} on leader ({}):", key, server_address_1);
    let retrieved_value_leader = leader_client.get(key.clone()).await?;
    println!("Retrieved value on leader: {:?}", retrieved_value_leader);

    let new_value_1 = "new-value-1".as_bytes().to_vec();
    println!("\nUpdating value for key: {} on follower ({}): value: {:?}", key, server_address_2, new_value_1);
    follower_client.set(key.clone(), new_value_1.clone()).await?;
    sleep(Duration::from_secs(1)).await;

    println!("\nGetting value for key: {} on leader ({}) after update from follower:", key, server_address_1);
    let retrieved_value_leader_2 = leader_client.get(key.clone()).await?;
    println!("Retrieved value on leader after update from follower: {:?}", retrieved_value_leader_2);

    println!("\nDeleting key: {} on leader ({})", key, server_address_1);
    leader_client.remove(key.clone()).await?;

    println!("\nGetting value for key: {} on follower ({}) after deletion:", key, server_address_2);
    let result_follower = follower_client.get(key.clone()).await;
    match result_follower {
        Ok(_) => println!("--------- Error: Value was not deleted! --------------"),
        Err(_) => println!("Received Error. Expected Behaviour."),
    }
    println!("\nGetting value for key: {} on leader ({}) after deletion:", key, server_address_1);
    let result_leader = leader_client.get(key.clone()).await;
    match result_leader {
        Ok(_) => println!("--------- Error: Value was not deleted! --------------"),
        Err(_) => println!("Received Error. Expected Behaviour."),
    }

    // --- Watcher Demonstration (Leader Node) ---
    println!("\n\n--- Watcher Demonstration (Leader Node) ---");
    println!("\nSetting up watcher for key: {} on leader ({})", key, server_address_1);

    // Use a channel to communicate between the watcher and the main thread.
    let (tx, mut rx) = tokio::sync::mpsc::channel(10); // Increased capacity

    let watch_handle: JoinHandle<()> = tokio::spawn({
        let mut leader_client_clone = ConferClient::new(server_address_1.clone(), tls_config.clone()).await.unwrap(); //clone
        let key_clone = key.clone();
        async move {
            let result = leader_client_clone.watch_config_with_callback(key_clone, move |update_result| {
                let tx_clone = tx.clone(); // Clone the sender to use in the closure
                tokio::spawn(async move { //spawn
                    match update_result {
                        Ok(update) => {
                            println!("\nLEADER WATCHER: Received update:");
                            if let Some(kind) = update.kind {
                                match kind {
                                    Kind::UpdatedValue(value) => {
                                        println!("  Kind: UpdatedValue");
                                        println!("  Value: {:?}", value);
                                        if let Ok(s) = String::from_utf8(value.clone()) {
                                            println!("  Value as string: {}", s);
                                            tx_clone.send(format!("Updated: {}", s)).await.unwrap(); //send
                                        }
                                    }
                                    Kind::LeaderChanged(leader_info) => {
                                        println!("  Kind: LeaderChanged");
                                        println!("  Leader Address: {}", leader_info.address);
                                         tx_clone.send(format!("LeaderChanged: {}", leader_info.address)).await.unwrap();
                                    }
                                    Kind::Removed(_) => {
                                        println!("  Kind: Removed");
                                        tx_clone.send("Removed".to_string()).await.unwrap();
                                    }
                                }
                            } else {
                                println!("  Kind: None");
                                tx_clone.send("Kind: None".to_string()).await.unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("LEADER WATCHER: Error in watcher: {}", e);
                            tx_clone.send(format!("Error: {}", e)).await.unwrap();
                        }
                    }
                });
            }).await;
            if let Err(e) = result{
                 eprintln!("LEADER WATCHER: Error setting up the watcher: {}", e);
            }
        }
    });

    // Give the watcher some time to start.
    sleep(Duration::from_secs(1)).await;

    let new_value_2 = "new-value-2".as_bytes().to_vec();
    println!("\nSetting new value for key: {} on leader ({}): value: {:?}", key, server_address_1, new_value_2);
    leader_client.set(key.clone(), new_value_2.clone()).await?;
    sleep(Duration::from_secs(2)).await;

    if let Ok(msg) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
        println!("Main Received from watcher: {}", msg.unwrap());
    }

    let new_value_3 = "new-value-3".as_bytes().to_vec();
    println!("\nSetting another new value for key: {} on leader ({}): value: {:?}", key, server_address_1, new_value_3);
    leader_client.set(key.clone(), new_value_3.clone()).await?;
    sleep(Duration::from_secs(2)).await;
     if let Ok(msg) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
        println!("Main Received from watcher: {}", msg.unwrap());
    }

    // --- Delete the configuration from the leader ---
    println!("\n\nLEADER: Deleting key: {}", key);
    leader_client.remove(key.clone()).await?;
    sleep(Duration::from_secs(2)).await;
     if let Ok(msg) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
        println!("Main Received from watcher: {}", msg.unwrap());
    }

     // --- Cancel watchers and cleanup ---
    println!("\n\n--- Cancelling watchers and exiting ---");
    leader_client.cancel_watchers().await;
    follower_client.cancel_watchers().await;
    // Optionally await the handles
    watch_handle.await.unwrap();

    Ok(())
}
