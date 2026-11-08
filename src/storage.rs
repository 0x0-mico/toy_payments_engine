use std::collections::HashMap;
use thiserror::Error;

use crate::{
    client::{Client, ClientId},
    transaction::{Tx, TxId},
};

pub struct Storage {
    clients: HashMap<ClientId, Client>,
    transactions: HashMap<TxId, Tx>,
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Transaction ID {0} already used")]
    TxIdAlreadyUsed(TxId),
    #[error("Transaction ID {0} not known")]
    TxIdUnknown(TxId),
    #[error("Client ID {0} not known")]
    ClientIdUnknown(ClientId),
}

impl Storage {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
    pub fn init_client(&mut self, client: ClientId, amount: rust_decimal::prelude::Decimal) {
        self.clients
            .entry(client)
            .or_insert_with(|| Client::new(amount));
    }
    pub fn sorted_clients(&self) -> Vec<(ClientId, &Client)> {
        let mut rows: Vec<(ClientId, &Client)> = self
            .clients
            .iter()
            .map(|(id, account)| (*id, account))
            .collect();
        rows.sort_unstable_by_key(|(id, _)| *id);
        rows
    }
    pub fn get_client_mut(&mut self, client_id: &ClientId) -> Result<&mut Client, StorageError> {
        self.clients
            .get_mut(client_id)
            .ok_or(StorageError::ClientIdUnknown(*client_id))
    }
    pub fn add_tx(&mut self, tx_id: TxId, tx: Tx) {
        self.transactions.insert(tx_id, tx);
    }
    pub fn get_tx_mut(&mut self, tx_id: &TxId) -> Result<&mut Tx, StorageError> {
        self.transactions
            .get_mut(tx_id)
            .ok_or(StorageError::TxIdUnknown(*tx_id))
    }
    pub fn check_used_tx_id(&self, tx_id: &TxId) -> Result<(), StorageError> {
        (!self.transactions.contains_key(tx_id)).ok_or(StorageError::TxIdAlreadyUsed(*tx_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientRow;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn available(storage: &Storage, client: ClientId) -> Decimal {
        let rows = storage.sorted_clients();
        let (id, account) = rows.iter().find(|(id, _)| *id == client).unwrap();
        ClientRow::new(*id, account).unwrap().available
    }

    #[test]
    fn init_client_creates_an_account() {
        let mut storage = Storage::new();
        storage.init_client(1, d("5"));
        assert_eq!(available(&storage, 1), d("5"));
    }

    #[test]
    fn init_client_keeps_an_existing_account() {
        let mut storage = Storage::new();
        storage.init_client(1, d("5"));
        storage.init_client(1, d("99"));
        assert_eq!(available(&storage, 1), d("5"));
    }

    #[test]
    fn get_client_mut_on_an_unknown_client_fails() {
        let mut storage = Storage::new();
        assert!(matches!(
            storage.get_client_mut(&1),
            Err(StorageError::ClientIdUnknown(1))
        ));
    }

    #[test]
    fn add_tx_then_get_tx() {
        let mut storage = Storage::new();
        storage.add_tx(1, Tx::new_deposit_tx(1, d("5")));
        assert_eq!(storage.get_tx_mut(&1).unwrap().amount(), d("5"));
    }

    #[test]
    fn get_tx_on_an_unknown_id_fails() {
        let mut storage = Storage::new();
        assert!(matches!(
            storage.get_tx_mut(&1),
            Err(StorageError::TxIdUnknown(1))
        ));
    }

    #[test]
    fn check_used_tx_id_reports_free_then_taken() {
        let mut storage = Storage::new();
        storage.check_used_tx_id(&1).unwrap();
        storage.add_tx(1, Tx::new_deposit_tx(1, d("5")));
        assert!(matches!(
            storage.check_used_tx_id(&1),
            Err(StorageError::TxIdAlreadyUsed(1))
        ));
    }

    #[test]
    fn sorted_clients_are_ordered_by_id() {
        let mut storage = Storage::new();
        storage.init_client(3, d("1"));
        storage.init_client(1, d("1"));
        storage.init_client(2, d("1"));
        let ids: Vec<ClientId> = storage.sorted_clients().iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
