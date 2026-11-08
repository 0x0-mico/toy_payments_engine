use crate::{
    client::{ClientError, ClientId},
    storage::{AmountError, Storage},
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
    #[error("Transaction {0} is not disputed")]
    TxNotDisputed(TxId),
    #[error("Transaction {0} is already disputed")]
    TxAlreadyDisputed(TxId),
    #[error("Transaction {0} was already charged back")]
    TxAlreadyChargedBack(TxId),
    #[error("Client {client} not allowed to dispute transaction {tx_id}")]
    DisputeByWrongClient { client: ClientId, tx_id: TxId },
    #[error("Client operation error: {0}")]
    ClientError(#[from] ClientError),
}

impl Engine {
    pub fn apply_event(ev: Event, storage: &mut Storage) -> Result<(), EngineError> {
        match ev {
            Event::Deposit { client, tx, amount } => {
                reject_reused_tx_id(storage, &tx)?;
                if let Some(client) = storage.get_client(&client) {
                    if client.is_locked() {
                        return Err(EngineError::ClientIsLocked(client));
                    }
                    storage.credit_client()?;
                } else {
                    storage.init_client(client, amount);
                }
            }
            Event::Withdraw { client, tx, amount } => {}
            Event::Dispute { client, tx } => {}
            Event::Resolve { client, tx } => {}
            Event::ChargeBack { client, tx } => {}
        }

        Ok(())
    }
}

fn reject_reused_tx_id(storage: &Storage, tx_id: &TxId) -> Result<(), EngineError> {
    storage
        .is_unknown_tx(tx_id)
        .ok_or(EngineError::TxIdAlreadyUsed(*tx_id))
}
