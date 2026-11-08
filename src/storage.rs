use std::collections::HashMap;

use crate::{
    client::{Client, ClientId},
    transaction::{Tx, TxId},
};

pub struct Storage {
    clients: HashMap<ClientId, Client>,
    transactions: HashMap<TxId, Tx>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
    pub fn add_account(&mut self, client: ClientId) {
        self.clients.insert(client, Client::default());
    }
    pub fn get_accounts(&self) -> &HashMap<ClientId, Client> {
        &self.clients
    }
    pub fn add_tx(&mut self, tx_id: TxId, tx: Tx) {
        self.transactions.insert(tx_id, tx);
    }
    pub fn get_tx(&self, tx_id: TxId) -> Option<&Tx> {
        self.transactions.get(&tx_id)
    }
}
