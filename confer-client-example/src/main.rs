use confer_client::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Connect to the Confer server.
    let server_address = "http://127.0.0.1:10001".to_string();
    let mut client = connect(server_address).await?;

    // Set a configuration value.
    let key = "my-key".to_string();
    let value = "my-value".as_bytes().to_vec();
    println!("Setting key: {}, value: {:?}", key, value);
    set_value(&mut client, key.clone(), value).await?;

    // Get the configuration value.
    println!("Getting key: {}", key);
    let retrieved_value = get_value(&mut client, key.clone()).await?;
    println!("Retrieved value: {:?}", retrieved_value);

    // Convert the value back to a string (if it was originally a string).
    let value_string = String::from_utf8(retrieved_value)?;
    println!("Retrieved value as string: {}", value_string);

    // Delete the configuration value.
    println!("Deleting key: {}", key);
    delete_value(&mut client, key.clone()).await?;

    // Attempt to get the deleted value (should return an error).
    println!("Getting deleted key: {}", key);
    let result = get_value(&mut client, key.clone()).await;
    match result {
        Ok(_) => println!("Error: Value was not deleted!"),
        Err(e) => println!("Expected error: {:?}", e),
    }
    Ok(())
}

