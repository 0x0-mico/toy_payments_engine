use crate::{
    client::{ClientError, ClientId},
    event::Event,
    storage::Storage,
    transaction::{TransactionError, Tx, TxId},
};
use rust_decimal::Decimal;
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
    #[error("Transaction operation error: {0}")]
    TransactionError(#[from] TransactionError),
    #[error("Client operation error: {0}")]
    ClientError(#[from] ClientError),
}

impl Engine {
    pub fn apply_event(ev: Event, storage: &mut Storage) -> Result<(), EngineError> {
        match ev {
            Event::Deposit {
                client_id,
                tx_id,
                amount,
            } => {
                reject_reused_tx_id(storage, &tx_id)?;
                if let Some(client) = storage.get_client_mut(&client_id) {
                    client.credit(amount)?;
                    storage.add_tx(tx_id, Tx::new_deposit_tx(client_id, amount));
                } else {
                    storage.init_client(client_id, amount);
                }
            }
            Event::Withdraw {
                client_id,
                tx_id,
                amount,
            } => {
                reject_reused_tx_id(storage, &tx_id)?;
                if let Some(client) = storage.get_client_mut(&client_id) {
                    client.debit(amount)?;
                    storage.add_tx(tx_id, Tx::new_withdraw_tx(client_id, amount));
                } else {
                    storage.init_client(client_id, Decimal::ZERO);
                    return Err(EngineError::ClientIdUnknown(client_id));
                }
            }
            Event::Dispute { client_id, tx_id } => {
                if let Some(tx) = storage.get_tx_mut(tx_id) {
                    let amount = tx.dispute(&client_id)?;
                    let client = storage
                        .get_client_mut(&client_id)
                        .ok_or(EngineError::ClientIdUnknown(client_id))?;
                    client.put_to_held(amount)?;
                } else {
                    return Err(EngineError::TxIdUnknown(tx_id));
                }
            }
            Event::Resolve { client_id, tx_id } => {
                if let Some(tx) = storage.get_tx_mut(tx_id) {
                    let amount = tx.resolve(&client_id)?;
                    let client = storage
                        .get_client_mut(&client_id)
                        .ok_or(EngineError::ClientIdUnknown(client_id))?;
                    client.resolve_held(amount)?;
                } else {
                    return Err(EngineError::TxIdUnknown(tx_id));
                }
            }
            Event::ChargeBack { client_id, tx_id } => {
                if let Some(tx) = storage.get_tx_mut(tx_id) {
                    let amount = tx.chargeback(&client_id)?;
                    let client = storage
                        .get_client_mut(&client_id)
                        .ok_or(EngineError::ClientIdUnknown(client_id))?;
                    client.apply_chargeback(amount)?;
                } else {
                    return Err(EngineError::TxIdUnknown(tx_id));
                }
            }
        }

        Ok(())
    }
}

fn reject_reused_tx_id(storage: &Storage, tx_id: &TxId) -> Result<(), EngineError> {
    storage
        .is_unknown_tx(tx_id)
        .ok_or(EngineError::TxIdAlreadyUsed(*tx_id))
}
