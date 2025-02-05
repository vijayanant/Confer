use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug)]
pub struct KVStore {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl KVStore {
    pub fn new() -> Self {
        println!("KVStore -> new()");
        KVStore {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set(&self, path: &str, value: String) {
        let mut data = self.data.lock().unwrap();
        data.insert(path.to_string(), value);
    }

    pub fn get(&self, path: &str) -> Option<String> {
        let data = self.data.lock().unwrap();
        data.get(path).cloned()
    }

    pub fn delete(&self, path: &str) -> bool {
        let mut data = self.data.lock().unwrap();
        data.remove(path).is_some()
    }

    //List all keys that starts with a given prefix
    pub fn list(&self, prefix: &str) -> Vec<String> {
        let data = self.data.lock().unwrap();
        data.keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect()
    }
}


