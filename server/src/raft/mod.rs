pub mod proto;
pub mod operation;
pub mod config;
pub mod storage;
pub mod network;
pub mod client_responder;

//use std::sync::Arc;

//use openraft::{Config, SnapshotPolicy};

//use crate::raft::network::NetworkFactory;

//async fn setup_raft() {
    //let config = sensible_config();
    //let network = Arc::new(NetworkFactory::new());

    //let raft = Raft::new(1, config, network, log_store, state_machine).await.unwrap();
//}


//fn sensible_config() -> Arc<Config> {
    //let mut config = Config::default();
    //config.election_timeout_min = 1000;
    //config.election_timeout_max = 1500;
    //config.heartbeat_interval = 500;
    //config.snapshot_policy = SnapshotPolicy::LogsSinceLast(500);
    //config.cluster_name = "confer_cluster".to_string();
    //Arc::new(config)
//}
