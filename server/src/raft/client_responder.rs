use openraft::raft::{responder::Responder, ClientWriteResult};
use tokio::sync::oneshot;

use crate::raft::{config::TypeConfig, operation};

pub struct ConferClientResponder {
    sender: oneshot::Sender<ClientWriteResult<TypeConfig>>,
}

impl Responder<TypeConfig> for ConferClientResponder {
    type Receiver = oneshot::Receiver<ClientWriteResult<TypeConfig>>;

    fn from_app_data(
        app_data: operation::Operation,
    ) -> (operation::Operation, Self, Self::Receiver) {
        let (sender, receiver) = oneshot::channel();
        let responder = ConferClientResponder { sender };
        (app_data, responder, receiver)
    }

    fn send(self, result: ClientWriteResult<TypeConfig>) {
        let _ = self.sender.send(result);
    }
}
