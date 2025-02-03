use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct KVStore {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl KVStore {
    fn new() -> Self {
        KVStore {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn set(&self, path: &str, value: String) {
        let mut data = self.data.lock().unwrap();
        data.insert(path.to_string(), value);
    }

    fn get(&self, path: &str) -> Option<String> {
        let data = self.data.lock().unwrap();
        data.get(path).cloned()
    }

    fn delete(&self, path: &str) -> bool {
        let mut data = self.data.lock().unwrap();
        data.remove(path).is_some()
    }

    //List all keys that starts with a given prefix
    fn list(&self, prefix: &str) -> Vec<String> {
        let data = self.data.lock().unwrap();
        data.keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect()
    }
}

#[derive(Default)]
pub struct Confer {
    store : KVStore,
}

impl Confer {
    pub fn new() -> Self {
        Confer {
           store : KVStore::new(),
        }
    }

    pub fn create(&self, path: &str, value: String) {
        self.store.set(path, value)
    }

    pub fn get(&self, path: &str) -> Option<String> {
        self.store.get(path)
    }

    pub fn delete(&self, path: &str) -> bool {
        self.store.delete(path)
    }

    //List all children of a given path (prefix)
    pub fn list_children(&self, path: &str) -> Vec<String> {
        let prefix = format!("{}/", path);
        self.store.list(&prefix)
            .iter()
            .map(|key| key.strip_prefix(&prefix).unwrap_or(key).to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_create_and_get() {
        let confer = Confer::new();
        confer.create("config/database/host", "localhost".to_string());
        confer.create("config/database/port", "5432".to_string());

        assert_eq!(confer.get("config/database/host"), Some("localhost".to_string()));
        assert_eq!(confer.get("config/database/port"), Some("5432".to_string()));
    }

    #[test]
    fn test_delete() {
        let confer = Confer::new();
        confer.create("config/database/host", "localhost".to_string());
        assert!(confer.delete("config/database/host"));
        assert_eq!(confer.get("config/database/host"), None);
    }

    #[test]
    fn test_list_children() {
        let confer = Confer::new();
        confer.create("config/services/user/session", "active".to_string());
        confer.create("config/services/user/preferences/theme", "dark".to_string());
        confer.create("config/services/admin/session", "inactive".to_string());

        let mut children = confer.list_children("config/services/user");
        children.sort();
        assert_eq!(children, vec!["preferences/theme".to_string(), "session".to_string()]);

        let admin_children = confer.list_children("config/services/admin");
        assert_eq!(admin_children, vec!["session".to_string()]);
    }

    #[test]
    fn test_non_existent_key() {
        let confer = Confer::new();
        assert_eq!(confer.get("config/nonexistent"), None);
        assert!(!confer.delete("config/nonexistent"));
    }

    #[test]
    fn test_list_children_empty() {
        let confer = Confer::new();
        let children = confer.list_children("config/services");
        assert!(children.is_empty());
    }
}

