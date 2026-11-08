use rust_decimal::Decimal;
use thiserror::Error;

use crate::client::ClientId;

pub type TxId = u32;

#[derive(Clone)]
pub enum TxKind {
    Deposit(Decimal),
    Withdraw(Decimal),
}

#[derive(Clone)]
pub enum Dispute {
    Open,
    ChargedBack,
}

#[derive(Clone)]
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
    #[error("Wrong TX type to dispute. Only deposits can be disputed")]
    WrongTxTypeToDispute,
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
    pub fn dispute(&self) -> &Option<Dispute> {
        &self.dispute
    }
    pub fn reset_dispute(&mut self, dispute: Option<Dispute>) {
        self.dispute = dispute
    }
    pub fn open_dispute(&mut self, client_id: &ClientId) -> Result<Decimal, TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        self.deny_if_wrong_tx_type()?;
        if let Some(dispute) = &self.dispute {
            match dispute {
                Dispute::Open => return Err(TransactionError::TxAlreadyDisputed),
                Dispute::ChargedBack => return Err(TransactionError::TxAlreadyChargedBack),
            }
        }
        self.dispute = Some(Dispute::Open);
        Ok(self.amount())
    }
    pub fn resolve_dispute(&mut self, client_id: &ClientId) -> Result<(), TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        match &self.dispute {
            Some(dispute) => match dispute {
                Dispute::Open => {
                    self.dispute = None;
                    Ok(())
                }
                Dispute::ChargedBack => Err(TransactionError::TxAlreadyChargedBack),
            },
            _ => Err(TransactionError::TxNotDisputed),
        }
    }
    pub fn chargeback_on_dispute(&mut self, client_id: &ClientId) -> Result<(), TransactionError> {
        self.deny_if_wrong_client(client_id)?;
        match &self.dispute {
            Some(dispute) => match dispute {
                Dispute::Open => {
                    self.dispute = Some(Dispute::ChargedBack);
                    Ok(())
                }
                Dispute::ChargedBack => Err(TransactionError::TxAlreadyChargedBack),
            },
            _ => Err(TransactionError::TxNotDisputed),
        }
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
    pub fn deny_if_wrong_tx_type(&self) -> Result<(), TransactionError> {
        matches!(self.kind, TxKind::Deposit(_)).ok_or(TransactionError::WrongTxTypeToDispute)
    }
}
