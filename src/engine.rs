use crate::{
    client::ClientId,
    storage::Storage,
    transaction::{Event, TxId},
};
use thiserror::Error;

pub struct Engine;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Transaction ID {0} already used")]
    TxIdAlreadyUsed(TxId),
    #[error("Transaction ID {0} not known")]
    TxIdUnknown(TxId),
    #[error("Client ID {0} not known")]
    ClientIdUnknown(ClientId),
    #[error("Client ID {0} account is locked")]
    ClientIsLocked(ClientId),
    #[error("Client lacks the funds for this operation")]
    NotEnoughFunds,
    #[error("Operation would overflow the client's balance")]
    BalanceOverflow,
    #[error("Transaction {0} is not disputed")]
    TxNotDisputed(TxId),
    #[error("Transaction {0} is already disputed")]
    TxAlreadyDisputed(TxId),
    #[error("Transaction {0} was already charged back")]
    TxAlreadyChargedBack(TxId),
    #[error("Client {client} not allowed to dispute transaction {tx_id}")]
    DisputeByWrongClient { client: ClientId, tx_id: TxId },
}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }
    pub fn apply_event(ev: Event, storage: &mut Storage) -> Result<(), EngineError> {
        Ok(())
    }
}
