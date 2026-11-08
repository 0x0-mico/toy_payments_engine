use crate::{
    client::ClientError,
    event::Event,
    storage::{Storage, StorageError},
    transaction::{TransactionError, Tx},
};
use thiserror::Error;

pub struct Engine;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Transaction operation error: {0}")]
    Transaction(#[from] TransactionError),
    #[error("Client operation error: {0}")]
    Client(#[from] ClientError),
    #[error("Client operation error: {0}")]
    Storage(#[from] StorageError),
}

impl Engine {
    pub fn apply_event(ev: Event, storage: &mut Storage) -> Result<(), EngineError> {
        match ev {
            Event::Deposit {
                client_id,
                tx_id,
                amount,
            } => {
                storage.check_used_tx_id(&tx_id)?;
                if let Ok(client) = storage.get_client_mut(&client_id) {
                    client.credit(amount)?;
                } else {
                    storage.init_client(client_id, amount);
                }
                storage.add_tx(tx_id, Tx::new_deposit_tx(client_id, amount));
            }
            Event::Withdraw {
                client_id,
                tx_id,
                amount,
            } => {
                storage.check_used_tx_id(&tx_id)?;
                match storage.get_client_mut(&client_id) {
                    Ok(client) => {
                        client.debit(amount)?;
                        storage.add_tx(tx_id, Tx::new_withdraw_tx(client_id, amount));
                    }
                    Err(e) => {
                        // special case - still create a new client account as per specification
                        storage.init_client(client_id, amount);
                        return Err(e.into());
                    }
                }
            }

            Event::Dispute { client_id, tx_id } => {
                let amount = storage.get_tx_mut(&tx_id)?.open_dispute(&client_id)?;
                if let Err(e) = storage.get_client_mut(&client_id)?.put_to_held(amount) {
                    storage.get_tx_mut(&tx_id)?.reset_dispute(None);
                    return Err(e.into());
                }
            }
            Event::Resolve { client_id, tx_id } => {
                let tx = storage.get_tx_mut(&tx_id)?;
                let (amount, original_dispute) = (tx.amount(), tx.dispute().clone());
                tx.resolve_dispute(&client_id)?;
                if let Err(e) = storage.get_client_mut(&client_id)?.resolve_held(amount) {
                    // revert dispute resolution in case accounting doesnt add up
                    storage.get_tx_mut(&tx_id)?.reset_dispute(original_dispute);
                    return Err(e.into());
                }
            }
            Event::ChargeBack { client_id, tx_id } => {
                let tx = storage.get_tx_mut(&tx_id)?;
                let (amount, original_dispute) = (tx.amount(), tx.dispute().clone());
                tx.chargeback_on_dispute(&client_id)?;
                if let Err(e) = storage.get_client_mut(&client_id)?.apply_chargeback(amount) {
                    // revert chargeback resolution in case accounting doesnt add up
                    storage.get_tx_mut(&tx_id)?.reset_dispute(original_dispute);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}
