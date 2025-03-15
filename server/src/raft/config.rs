use std::fmt;

use openraft::{ Entry, TokioRuntime, RaftTypeConfig, };
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use openraft::BasicNode;
use crate::raft::{
    operation:: {Operation, OperationResponse},
    client_responder::ConferClientResponder};

use openraft_memstore::MemStore;
use crate::raft::state_machine::ConferRepositoryAdaptor;
use crate::repository::HashMapConferRepository;

#[derive(
    Clone, Debug,
    Serialize, Deserialize,
    PartialEq, Eq, Default, PartialOrd, Ord,)]
pub struct NodeInfo {
    pub address: String,
    pub port: u16,
    pub custom_data: String, // Or a more structured type
}


#[derive(
    Copy, Clone, Debug,
    Serialize, Deserialize,
    PartialEq, Eq, Default, PartialOrd, Ord,
    Hash)]
pub struct TypeConfig {}

impl RaftTypeConfig for TypeConfig {
    type D            = Operation;
    type R            = OperationResponse;
    type NodeId       = u64;
    type Node         = BasicNode;
    type Entry        = Entry<TypeConfig>;
    type SnapshotData = ConferRepositoryAdaptor<HashMapConferRepository>;
    type AsyncRuntime = TokioRuntime;
    type Responder    = ConferClientResponder;
}



// TypeConfig struct is used as a generic parameter in the InstallSnapshotError
// type. It  has  implement the std::fmt::Display trait, which is required for
// formatting it in certain contexts.

impl fmt::Display for TypeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Implement how you want TypeConfig to be displayed
        write!(f, "TypeConfig {{ /* your fields */ }}")
    }
}
