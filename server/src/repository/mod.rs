pub mod hashmap;
pub use hashmap::HashMapConferRepository;

use crate::error::ConferError;
use crate::proto::confer::v1::ConfigPath;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait ConferRepository: Serialize + Deserialize<'static> + Send + Sync + 'static {
    async fn get(&self, path: &ConfigPath) -> Result<Vec<u8>, ConferError>;
    async fn set(&self, path: &ConfigPath, value: Vec<u8>) -> Result<(), ConferError>;
    async fn remove(&self, path: &ConfigPath) -> Result<(), ConferError>;
    async fn list(&self, path: &ConfigPath) -> Result<Vec<String>, ConferError>;
    async fn get_serialized_data(&self) -> Result<Vec<u8>, ConferError>;
    async fn replace_data(&mut self, serialized_data: Vec<u8>) -> Result<(), ConferError>;
}

#[cfg(test)]
pub mod hashmap_tests;
