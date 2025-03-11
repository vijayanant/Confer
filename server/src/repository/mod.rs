pub mod hashmap;
pub use hashmap::HashMapConferRepository;

use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use crate::proto::confer::v1::ConfigPath;
use crate::error::ConferError;

#[async_trait]
pub trait ConferRepository: Serialize + Deserialize<'static> + Send + Sync {
    async fn get(&self, path: &ConfigPath) -> Result<Vec<u8>, ConferError>;
    async fn set(&self, path: &ConfigPath, value: Vec<u8>) -> Result<(), ConferError>;
    async fn remove(&self, path: &ConfigPath) -> Result<(), ConferError>;
    async fn list(&self, path: &ConfigPath) -> Result<Vec<String>, ConferError>;
}

#[cfg(test)]
pub mod hashmap_tests;

