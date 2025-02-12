use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::ConfigPath;
use super::DataStoreError;

#[derive(Default)]
pub struct HashMapDataStore {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl HashMapDataStore {
    pub fn new() -> Self {
        HashMapDataStore {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get(&self, path: &ConfigPath) -> Result<Vec<u8>, DataStoreError> {
        let data = self.data.lock().unwrap();
        match data.get(&path.0) {
            Some(value) => Ok(value.clone()),
            None => Err(DataStoreError::NotFound {
                path: path.0.clone(),
            }),
        }
    }

    pub async fn set(&self, path: &ConfigPath, value: Vec<u8>) -> Result<(), DataStoreError> {
        if path.0.is_empty() {
            return Err(DataStoreError::InvalidPath {
                path: path.0.clone(),
            });
        }

        let mut data = self.data.lock().unwrap();
        data.insert(path.0.clone(), value);
        Ok(())
    }

    pub async fn remove(&self, path: &ConfigPath) -> Result<(), DataStoreError> {
        let mut data = self.data.lock().unwrap();
        match data.remove(&path.0) {
            Some(_) => Ok(()),
            None => Err(DataStoreError::NotFound {
                path: path.0.clone(),
            }),
        }
    }

    pub async fn list(&self, path: &ConfigPath) -> Result<Vec<String>, DataStoreError> {
        let data = self.data.lock().unwrap();
        let result = data
            .keys()
            .filter(|k| k.starts_with(&path.0))
            .cloned()
            .collect();
        Ok(result)
    }
}
