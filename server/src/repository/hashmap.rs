use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tracing::{debug, instrument};
use serde::{Serialize, Deserialize};

use crate::proto::confer::v1::ConfigPath;
use crate::error::ConferError;
use crate::repository::ConferRepository;



#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct HashMapConferRepository {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl HashMapConferRepository {
    pub fn new() -> Self {
        Self::default()
    }

    fn validate_path(path: &ConfigPath) -> Result<(), ConferError> {
        if path.path.is_empty() {
            return Err(ConferError::InvalidPath { path: path.path.clone() });
        }
        Ok(())
    }
}

#[async_trait]
impl ConferRepository for HashMapConferRepository {
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


    async fn get_serialized_data(&self) -> Result<Vec<u8>, ConferError> {
        let data = self.data.lock().unwrap();
        serde_json::to_vec(&*data).map_err(|e| ConferError::SerializationError{message: e.to_string()})
    }

    async fn replace_data(&mut self, serialized_data: Vec<u8>) -> Result<(), ConferError> {
        let map: HashMap<String, Vec<u8>> = serde_json::from_slice(&serialized_data)
            .map_err(|e| ConferError::DeserializationError{message: e.to_string()})?;
        let mut data = self.data.lock().unwrap();
        *data = map;
        Ok(())
    }
}
