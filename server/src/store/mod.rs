mod hashmap; //import the HashMap implementation
mod hashmap_test; // Declare the test module


pub use self::hashmap::HashMapDataStore; // re-export

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataStoreError {
    #[error("Path not found: {path}")]
    NotFound { path: String },
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },
    #[error("Unknownerror: {source}")]
    UnknownError { source: Box<dyn std::error::Error> }, // For other unexpected errors
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigPath(pub String); // Public field for easy access

impl ConfigPath {
    pub fn new(path: &str) -> Result<Self, DataStoreError> {
        if path.is_empty() {
            return Err(DataStoreError::InvalidPath { path: path.to_string() }); //
        }
        Ok(ConfigPath(path.to_string()))
    }
    // ... other path-related methods
}


    //List all keys that starts with a given prefix
    //pub fn list(&self, prefix: &str) -> Vec<String> {
        //let data = self.data.lock().unwrap();
        //data.keys()
            //.filter(|key| key.starts_with(prefix))
            //.cloned()
            //.collect()
    //}


