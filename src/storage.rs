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
    pub fn init_client(&mut self, client: ClientId, amount: rust_decimal::prelude::Decimal) {
        self.clients.insert(client, Client::new(amount));
    }
    pub fn get_clients(&self) -> &HashMap<ClientId, Client> {
        &self.clients
    }
    pub fn get_client_mut(&mut self, client_id: &ClientId) -> Option<&mut Client> {
        self.clients.get_mut(client_id)
    }
    pub fn add_tx(&mut self, tx_id: TxId, tx: Tx) {
        self.transactions.insert(tx_id, tx);
    }
    pub fn get_tx_mut(&mut self, tx_id: TxId) -> Option<&mut Tx> {
        self.transactions.get_mut(&tx_id)
    }
    pub fn is_unknown_tx(&self, tx_id: &TxId) -> bool {
        !self.transactions.contains_key(tx_id)
    }
}
