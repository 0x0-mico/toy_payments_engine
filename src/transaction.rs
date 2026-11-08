use rust_decimal::Decimal;
use thiserror::Error;

use crate::client::ClientId;

pub type TxId = u32;

pub enum TxKind {
    Deposit(Decimal),
    Withdraw(Decimal),
}

pub enum Dispute {
    Open,
    ChargedBack,
}

pub struct Tx {
    client: ClientId,
    kind: TxKind,
    dispute: Option<Dispute>,
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Transaction is not disputed")]
    TxNotDisputed,
    #[error("Transaction is already disputed")]
    TxAlreadyDisputed,
    #[error("Transaction was already charged back")]
    TxAlreadyChargedBack,
    #[error("Client {0} in the event is not the same as on the referenced transaction")]
    WrongClient(ClientId),
}

impl Tx {
    pub fn new_deposit_tx(client: ClientId, amount: Decimal) -> Self {
        Self {
            client,
            kind: TxKind::Deposit(amount),
            dispute: None,
        }
    }
    pub fn new_withdraw_tx(client: ClientId, amount: Decimal) -> Self {
        Self {
            client,
            kind: TxKind::Withdraw(amount),
            dispute: None,
        }
    }
    pub fn dispute(&mut self, client_id: &ClientId) -> Result<Decimal, TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        if let Some(dispute) = &self.dispute {
            match dispute {
                Dispute::Open => return Err(TransactionError::TxAlreadyDisputed),
                Dispute::ChargedBack => return Err(TransactionError::TxAlreadyChargedBack),
            }
        } else {
            self.dispute = Some(Dispute::Open);
        }
        Ok(self.amount())
    }
    pub fn resolve(&mut self, client_id: &ClientId) -> Result<Decimal, TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        if let Some(dispute) = &self.dispute {
            match dispute {
                Dispute::Open => {
                    self.dispute = None;
                }
                Dispute::ChargedBack => return Err(TransactionError::TxAlreadyChargedBack),
            }
        } else {
            return Err(TransactionError::TxNotDisputed);
        }
        Ok(self.amount())
    }
    pub fn chargeback(&mut self, client_id: &ClientId) -> Result<Decimal, TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        if let Some(dispute) = &self.dispute {
            match dispute {
                Dispute::Open => {
                    self.dispute = Some(Dispute::ChargedBack);
                }
                Dispute::ChargedBack => return Err(TransactionError::TxAlreadyChargedBack),
            }
        } else {
            return Err(TransactionError::TxNotDisputed);
        }
        Ok(self.amount())
    }

    pub fn amount(&self) -> Decimal {
        match self.kind {
            TxKind::Deposit(amount) | TxKind::Withdraw(amount) => amount,
        }
    }
    fn deny_if_wrong_client(&self, client_id: &ClientId) -> Result<(), TransactionError> {
        if client_id.ne(&self.client) {
            return Err(TransactionError::WrongClient(*client_id));
        }
        Ok(())
    }
}
