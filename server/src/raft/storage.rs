use openraft_memstore::MemStore;

// TODO: We are using MemStore for now. We will come back and implement a better
//       store later

// Type aliases for clarity
pub type ConferStore = MemStore; //<TypeConfig, ConferAdaptor<HashMapConferRepository>>;
// TODO: Consider making the repository type configurable for future flexibility.
//       This will allow us to easily switch to different repository implementations
//       (e.g., RocksDBConferRepository) without modifying this file directly.


pub async fn create_confer_store() -> ConferStore {
    ConferStore::new()
}
