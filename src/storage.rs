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
        self.clients.insert(client, Client::new(amount));
    }
    pub fn get_clients(&self) -> &HashMap<ClientId, Client> {
        &self.clients
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
