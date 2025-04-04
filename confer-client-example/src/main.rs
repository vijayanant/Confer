use confer_client::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Connect to the Confer server.
    let server_address_1 = "http://127.0.0.1:10001".to_string();
    let server_address_2 = "http://127.0.0.1:10002".to_string();
    let mut client_1 = ConferClient::connect(server_address_1).await?;
    let mut client_2 = ConferClient::connect(server_address_2).await?;

    // Set a configuration value.
    let key = "my-key".to_string();
    let value = "my-value".as_bytes().to_vec();
    println!("\n\nSERVER 1\nSetting key: {}, value: {:?}", key, value);
    client_1.set(key.clone(), value).await?;

    // Get the configuration value.
    println!("\n\nSERVER 2\nGetting key: {}", key);
    let retrieved_value = client_2.get(key.clone()).await?;
    println!("Retrieved value: {:?}", retrieved_value);

    // Convert the value back to a string (if it was originally a string).
    let value_string = String::from_utf8(retrieved_value)?;
    println!("Retrieved value as string: {}", value_string);

    // Delete the configuration value.
    println!("\n\nServer 2\nDeleting key: {}", key);
    client_2.delete(key.clone()).await?;
    println!("Server 2\nDeleted key: {}", key);

    // Attempt to get the deleted value (should return an error).
    println!("\n\nSERVER 1\nGetting deleted key: {}", key);
    let result = client_1.get(key.clone()).await;
    match result {
        Ok(_) => println!("--------- Error: Value was not deleted!--------------"),
        Err(_) => println!("Received Error. Expected Behaviour."),
    }
    Ok(())
}

