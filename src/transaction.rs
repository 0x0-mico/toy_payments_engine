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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn disputed_deposit() -> Tx {
        let mut tx = Tx::new_deposit_tx(1, d("10"));
        tx.open_dispute(&1).unwrap();
        tx
    }

    #[test]
    fn deposit_keeps_its_amount() {
        assert_eq!(Tx::new_deposit_tx(1, d("2.5")).amount(), d("2.5"));
    }

    #[test]
    fn withdrawal_keeps_its_amount() {
        assert_eq!(Tx::new_withdraw_tx(1, d("2.5")).amount(), d("2.5"));
    }

    #[test]
    fn open_dispute_on_a_deposit_returns_its_amount() {
        let mut tx = Tx::new_deposit_tx(1, d("10"));
        assert_eq!(tx.open_dispute(&1).unwrap(), d("10"));
        assert!(matches!(tx.dispute(), Some(Dispute::Open)));
    }

    #[test]
    fn open_dispute_on_a_withdrawal_is_refused() {
        let mut tx = Tx::new_withdraw_tx(1, d("10"));
        assert!(matches!(
            tx.open_dispute(&1),
            Err(TransactionError::WrongTxTypeToDispute)
        ));
        assert!(tx.dispute().is_none());
    }

    #[test]
    fn open_dispute_by_a_foreign_client_is_refused() {
        let mut tx = Tx::new_deposit_tx(1, d("10"));
        assert!(matches!(
            tx.open_dispute(&2),
            Err(TransactionError::WrongClient(2))
        ));
    }

    #[test]
    fn open_dispute_twice_is_refused() {
        let mut tx = disputed_deposit();
        assert!(matches!(
            tx.open_dispute(&1),
            Err(TransactionError::TxAlreadyDisputed)
        ));
    }

    #[test]
    fn open_dispute_after_chargeback_is_refused() {
        let mut tx = disputed_deposit();
        tx.chargeback_on_dispute(&1).unwrap();
        assert!(matches!(
            tx.open_dispute(&1),
            Err(TransactionError::TxAlreadyChargedBack)
        ));
    }

    #[test]
    fn resolve_clears_an_open_dispute() {
        let mut tx = disputed_deposit();
        tx.resolve_dispute(&1).unwrap();
        assert!(tx.dispute().is_none());
    }

    #[test]
    fn resolve_without_a_dispute_is_refused() {
        let mut tx = Tx::new_deposit_tx(1, d("10"));
        assert!(matches!(
            tx.resolve_dispute(&1),
            Err(TransactionError::TxNotDisputed)
        ));
    }

    #[test]
    fn resolve_after_chargeback_is_refused() {
        let mut tx = disputed_deposit();
        tx.chargeback_on_dispute(&1).unwrap();
        assert!(matches!(
            tx.resolve_dispute(&1),
            Err(TransactionError::TxAlreadyChargedBack)
        ));
    }

    #[test]
    fn chargeback_closes_an_open_dispute() {
        let mut tx = disputed_deposit();
        tx.chargeback_on_dispute(&1).unwrap();
        assert!(matches!(tx.dispute(), Some(Dispute::ChargedBack)));
    }

    #[test]
    fn chargeback_twice_is_refused() {
        let mut tx = disputed_deposit();
        tx.chargeback_on_dispute(&1).unwrap();
        assert!(matches!(
            tx.chargeback_on_dispute(&1),
            Err(TransactionError::TxAlreadyChargedBack)
        ));
    }

    #[test]
    fn chargeback_without_a_dispute_is_refused() {
        let mut tx = Tx::new_deposit_tx(1, d("10"));
        assert!(matches!(
            tx.chargeback_on_dispute(&1),
            Err(TransactionError::TxNotDisputed)
        ));
    }

    #[test]
    fn reset_dispute_restores_a_previous_state() {
        let mut tx = disputed_deposit();
        tx.reset_dispute(None);
        assert!(tx.dispute().is_none());
        tx.reset_dispute(Some(Dispute::ChargedBack));
        assert!(matches!(tx.dispute(), Some(Dispute::ChargedBack)));
    }
}
