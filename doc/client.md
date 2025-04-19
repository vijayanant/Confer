# Confer Client Library/SDK

## Rust

The Confer Rust client library allows Rust applications to interact with a Confer server.

### Usage

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
Refer to the example in the repo for using the watch API. 
