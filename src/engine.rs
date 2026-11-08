use crate::{
    client::ClientError,
    event::Event,
    storage::{Storage, StorageError},
    transaction::{TransactionError, Tx},
};
use rust_decimal::Decimal;
use thiserror::Error;

pub struct Engine;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Transaction operation error: {0}")]
    Transaction(#[from] TransactionError),
    #[error("Client operation error: {0}")]
    Client(#[from] ClientError),
    #[error("Storage error: {0}")]
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
                        storage.init_client(client_id, Decimal::ZERO);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{ClientId, ClientRow};
    use crate::transaction::TxId;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn deposit(client_id: ClientId, tx_id: TxId, amount: &str) -> Event {
        Event::Deposit {
            client_id,
            tx_id,
            amount: d(amount),
        }
    }

    fn withdraw(client_id: ClientId, tx_id: TxId, amount: &str) -> Event {
        Event::Withdraw {
            client_id,
            tx_id,
            amount: d(amount),
        }
    }

    fn dispute(client_id: ClientId, tx_id: TxId) -> Event {
        Event::Dispute { client_id, tx_id }
    }

    fn resolve(client_id: ClientId, tx_id: TxId) -> Event {
        Event::Resolve { client_id, tx_id }
    }

    fn chargeback(client_id: ClientId, tx_id: TxId) -> Event {
        Event::ChargeBack { client_id, tx_id }
    }

    fn snapshot(storage: &Storage, client_id: ClientId) -> ClientRow {
        let rows = storage.sorted_clients();
        let (id, account) = rows
            .iter()
            .find(|(id, _)| *id == client_id)
            .expect("client exists");
        ClientRow::new(*id, account).unwrap()
    }

    fn locked_storage() -> Storage {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(dispute(1, 1), &mut storage).unwrap();
        Engine::apply_event(chargeback(1, 1), &mut storage).unwrap();
        storage
    }

    #[test]
    fn deposit_creates_the_client() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        let row = snapshot(&storage, 1);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("10"), d("0"), d("10"))
        );
        assert!(!row.locked);
    }

    #[test]
    fn deposit_credits_an_existing_client() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(deposit(1, 2, "5"), &mut storage).unwrap();
        assert_eq!(snapshot(&storage, 1).available, d("15"));
    }

    #[test]
    fn deposit_on_a_locked_client_is_refused() {
        let mut storage = locked_storage();
        let err = Engine::apply_event(deposit(1, 2, "1"), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Client(ClientError::ClientIsLocked)
        ));
        let row = snapshot(&storage, 1);
        assert_eq!((row.available, row.total), (d("0"), d("0")));
        assert!(row.locked);
    }

    #[test]
    fn withdrawal_reduces_available() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(withdraw(1, 2, "3"), &mut storage).unwrap();
        assert_eq!(snapshot(&storage, 1).available, d("7"));
    }

    #[test]
    fn withdrawal_beyond_available_is_refused() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        let err = Engine::apply_event(withdraw(1, 2, "11"), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Client(ClientError::NotEnoughFunds)
        ));
        assert_eq!(snapshot(&storage, 1).available, d("10"));
    }

    #[test]
    fn withdrawal_from_an_unknown_client_creates_a_zeroed_account() {
        let mut storage = Storage::new();
        let err = Engine::apply_event(withdraw(7, 1, "3"), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Storage(StorageError::ClientIdUnknown(7))
        ));
        let row = snapshot(&storage, 7);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("0"), d("0"), d("0"))
        );
    }

    #[test]
    fn dispute_moves_funds_to_held() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(dispute(1, 1), &mut storage).unwrap();
        let row = snapshot(&storage, 1);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("0"), d("10"), d("10"))
        );
    }

    #[test]
    fn resolve_returns_funds_to_available() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(dispute(1, 1), &mut storage).unwrap();
        Engine::apply_event(resolve(1, 1), &mut storage).unwrap();
        let row = snapshot(&storage, 1);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("10"), d("0"), d("10"))
        );
    }

    #[test]
    fn chargeback_clears_held_and_locks() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(dispute(1, 1), &mut storage).unwrap();
        Engine::apply_event(chargeback(1, 1), &mut storage).unwrap();
        let row = snapshot(&storage, 1);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("0"), d("0"), d("0"))
        );
        assert!(row.locked);
    }

    #[test]
    fn dispute_by_a_foreign_client_is_refused() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        let err = Engine::apply_event(dispute(2, 1), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Transaction(TransactionError::WrongClient(2))
        ));
        assert_eq!(snapshot(&storage, 1).available, d("10"));
    }

    #[test]
    fn dispute_of_a_withdrawal_is_refused() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        Engine::apply_event(withdraw(1, 2, "5"), &mut storage).unwrap();
        let err = Engine::apply_event(dispute(1, 2), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Transaction(TransactionError::WrongTxTypeToDispute)
        ));
    }

    #[test]
    fn deposit_with_a_used_tx_id_is_refused() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        let err = Engine::apply_event(deposit(1, 1, "5"), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Storage(StorageError::TxIdAlreadyUsed(1))
        ));
        assert_eq!(snapshot(&storage, 1).available, d("10"));
    }

    #[test]
    fn dispute_of_an_unknown_tx_is_refused() {
        let mut storage = Storage::new();
        Engine::apply_event(deposit(1, 1, "10"), &mut storage).unwrap();
        let err = Engine::apply_event(dispute(1, 99), &mut storage).unwrap_err();
        assert!(matches!(
            err,
            EngineError::Storage(StorageError::TxIdUnknown(99))
        ));
    }
}
