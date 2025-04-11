use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, instrument};

use crate::error::ConferError;
use crate::proto::confer::v1::ConfigPath;
use crate::repository::ConferRepository;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
/// HashMapConferRepository stores configuration data in memory using a HashMap.
///
///   - The keys are Strings representing configuration paths.
///   - The values are `Vec<u8>` representing the configuration data (raw bytes).
///   - data is wrapped in an Arc<Mutex<...>> for thread-safe access:
///     - Arc allows shared ownership across multiple threads.
///     - Mutex provides mutual exclusion, ensuring only one thread can access the HashMap at a time, preventing data corruption.
pub struct HashMapConferRepository {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl HashMapConferRepository {
    /// Creates a new HashMapConferRepository.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the given configuration path.
    ///
    /// Returns an error if the path is empty.
    fn validate_path(path: &ConfigPath) -> Result<(), ConferError> {
        if path.path.is_empty() {
            return Err(ConferError::InvalidPath {
                path: path.path.clone(),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl ConferRepository for HashMapConferRepository {
    /// Retrieves a configuration value from the repository.
    ///
    /// # Arguments
    ///
    /// * `path`: The path of the configuration value to retrieve.
    ///
    /// # Returns
    ///
    /// The configuration value as a byte vector, or an error if the path is invalid or the value is not found.
    #[instrument(skip(self))]
    async fn get(&self, path: &ConfigPath) -> Result<Vec<u8>, ConferError> {
        debug!("Getting value for path: {}", path.path);
        Self::validate_path(path)?;

        let data = self.data.lock().unwrap();
        match data.get(&path.path) {
            Some(value) => {
                debug!("Value found for path: {}", path.path);
                Ok(value.clone())
            }
            None => {
                debug!("Value not found for path: {}", path.path);
                Err(ConferError::NotFound {
                    path: path.path.clone(),
                })
            }
        }
    }

    /// Sets a configuration value in the repository.
    ///
    /// # Arguments
    ///
    /// * `path`: The path of the configuration value to set.
    /// * `value`: The configuration value as a byte vector.
    ///
    /// # Returns
    ///
    /// An empty result on success, or an error if the path is invalid.
    #[instrument(skip(self))]
    async fn set(&self, path: &ConfigPath, value: Vec<u8>) -> Result<(), ConferError> {
        debug!("Setting value for path: {}", path.path);
        Self::validate_path(path)?;
        if path.path.is_empty() {
            debug!("Invalid path: {}", path.path);
            return Err(ConferError::InvalidPath {
                path: path.path.clone(),
            });
        }

        let mut data = self.data.lock().unwrap();
        data.insert(path.path.clone(), value);
        debug!("Value set for path: {}", path.path);
        Ok(())
    }

    /// Removes a configuration value from the repository.
    ///
    /// # Arguments
    ///
    /// * `path`: The path of the configuration value to remove.
    ///
    /// # Returns
    ///
    /// An empty result on success, or an error if the path is invalid or the value is not found.
    #[instrument(skip(self))]
    async fn remove(&self, path: &ConfigPath) -> Result<(), ConferError> {
        debug!("Removing value for path: {}", path.path);
        Self::validate_path(path)?;
        let mut data = self.data.lock().unwrap();
        match data.remove(&path.path) {
            Some(_) => {
                debug!("Value removed for path: {}", path.path);
                Ok(())
            }
            None => {
                debug!("Value not found for removal: {}", path.path);
                Err(ConferError::NotFound {
                    path: path.path.clone(),
                })
            }
        }
    }

    /// Lists configuration paths with a given prefix.
    ///
    /// # Arguments
    ///
    /// * `path`: The prefix to list paths under.
    ///
    /// # Returns
    ///
    /// A vector of configuration paths, or an error if the path is invalid.
    #[instrument(skip(self))]
    async fn list(&self, path: &ConfigPath) -> Result<Vec<String>, ConferError> {
        debug!("Listing paths with prefix: {}", path.path);
        Self::validate_path(path)?;
        let data = self.data.lock().unwrap();
        let result: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(&path.path))
            .cloned()
            .collect();
        debug!("Found {} paths with prefix: {}", result.len(), path.path);
        Ok(result)
    }

    /// Retrieves all data from the repository as a serialized byte vector.
    /// This is used to take a snapshot.
    ///
    /// # Returns
    ///
    /// A result containing the serialized data or an error.
    async fn get_serialized_data(&self) -> Result<Vec<u8>, ConferError> {
        let data = self.data.lock().unwrap();
        serde_json::to_vec(&*data).map_err(|e| ConferError::SerializationError {
            message: e.to_string(),
        })
    }

    /// Replaces all data in the repository with the provided serialized data.
    /// Thisis used to load data from a snapshot.
    ///
    /// # Arguments
    ///
    /// * `serialized_data`: The serialized data to replace the repository's contents.
    ///
    /// # Returns
    ///
    /// An empty result on success or an error.
    async fn replace_data(&mut self, serialized_data: Vec<u8>) -> Result<(), ConferError> {
        debug!("Replacing data with serialized_data.");
        let map: HashMap<String, Vec<u8>> =
            serde_json::from_slice(&serialized_data).map_err(|e| {
                ConferError::DeserializationError {
                    message: e.to_string(),
                }
            })?;
        let mut data = self.data.lock().unwrap();
        *data = map;
        Ok(())
    }
}
