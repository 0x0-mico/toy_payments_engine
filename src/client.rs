use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

pub type ClientId = u16;

pub struct Client {
    available: Decimal,
    held: Decimal,
    is_locked: bool,
}

#[derive(Debug, Serialize)]
pub struct ClientRow {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Client lacks the funds for this operation")]
    NotEnoughFunds,
    #[error("Client account is locked")]
    ClientIsLocked,
    #[error("Client accounting error: {0}")]
    AccountingError(#[from] AccountingError),
}

#[derive(Error, Debug)]
pub enum AccountingError {
    #[error("Operation would overflow total balance")]
    TotalOverflow,
    #[error("Operation would overflow held balance")]
    HeldOverflow,
    #[error("Operation would overflow available balance")]
    AvailableOverflow,

    #[error("Operation would underflow held balance")]
    HeldUnderflow,
    #[error("Operation would underflow available balance")]
    AvailableUnderflow,
}

impl ClientRow {
    pub fn new(client: ClientId, account: &Client) -> Result<Self, AccountingError> {
        let total = account
            .available
            .checked_add(account.held)
            .ok_or(AccountingError::TotalOverflow)?;
        Ok(Self {
            client,
            available: account.available,
            held: account.held,
            total,
            locked: account.is_locked,
        })
    }
}

impl Client {
    pub fn new(amount: Decimal) -> Self {
        Self {
            available: amount,
            held: Decimal::ZERO,
            is_locked: false,
        }
    }
    fn lock(&mut self) {
        self.is_locked = true;
    }
    pub fn credit(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if self.is_locked {
            return Err(ClientError::ClientIsLocked);
        }
        let available = self
            .available
            .checked_add(amount)
            .ok_or(AccountingError::AvailableOverflow)?;
        available
            .checked_add(self.held)
            .ok_or(AccountingError::TotalOverflow)?;
        self.available = available;
        Ok(())
    }
    pub fn debit(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if self.is_locked {
            return Err(ClientError::ClientIsLocked);
        }
        if amount > self.available {
            return Err(ClientError::NotEnoughFunds);
        }
        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(AccountingError::AvailableUnderflow)?;
        Ok(())
    }
    pub fn put_to_held(&mut self, amount: Decimal) -> Result<(), ClientError> {
        let available = self
            .available
            .checked_sub(amount)
            .ok_or(AccountingError::AvailableUnderflow)?;
        let held = self
            .held
            .checked_add(amount)
            .ok_or(AccountingError::HeldOverflow)?;
        self.available = available;
        self.held = held;
        Ok(())
    }
    pub fn apply_chargeback(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if amount > self.held {
            return Err(AccountingError::HeldUnderflow.into());
        }
        let held = self
            .held
            .checked_sub(amount)
            .ok_or(AccountingError::HeldUnderflow)?;
        self.held = held;
        self.lock();
        Ok(())
    }
    pub fn resolve_held(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if amount > self.held {
            return Err(AccountingError::HeldUnderflow.into());
        }
        let available = self
            .available
            .checked_add(amount)
            .ok_or(AccountingError::AvailableOverflow)?;
        let held = self
            .held
            .checked_sub(amount)
            .ok_or(AccountingError::HeldUnderflow)?;
        self.available = available;
        self.held = held;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn snapshot(account: &Client) -> ClientRow {
        ClientRow::new(0, account).unwrap()
    }

    fn locked(amount: &str) -> Client {
        let mut account = Client::new(d(amount));
        account.put_to_held(d(amount)).unwrap();
        account.apply_chargeback(d(amount)).unwrap();
        account
    }

    #[test]
    fn new_account_holds_the_given_amount() {
        let row = snapshot(&Client::new(d("10")));
        assert_eq!(row.available, d("10"));
        assert_eq!(row.held, d("0"));
        assert_eq!(row.total, d("10"));
        assert!(!row.locked);
    }

    #[test]
    fn credit_increases_available() {
        let mut account = Client::new(d("10"));
        account.credit(d("5")).unwrap();
        assert_eq!(snapshot(&account).available, d("15"));
    }

    #[test]
    fn credit_is_refused_on_a_locked_account() {
        let mut account = locked("10");
        assert!(matches!(
            account.credit(d("1")),
            Err(ClientError::ClientIsLocked)
        ));
        assert_eq!(snapshot(&account).available, d("0"));
    }

    #[test]
    fn debit_reduces_available() {
        let mut account = Client::new(d("10"));
        account.debit(d("3")).unwrap();
        assert_eq!(snapshot(&account).available, d("7"));
    }

    #[test]
    fn debit_of_the_exact_balance_leaves_zero() {
        let mut account = Client::new(d("10"));
        account.debit(d("10")).unwrap();
        let row = snapshot(&account);
        assert_eq!((row.available, row.total), (d("0"), d("0")));
    }

    #[test]
    fn debit_beyond_the_balance_is_refused() {
        let mut account = Client::new(d("10"));
        assert!(matches!(
            account.debit(d("11")),
            Err(ClientError::NotEnoughFunds)
        ));
        assert_eq!(snapshot(&account).available, d("10"));
    }

    #[test]
    fn debit_is_refused_on_a_locked_account() {
        let mut account = locked("10");
        assert!(matches!(
            account.debit(d("1")),
            Err(ClientError::ClientIsLocked)
        ));
    }

    #[test]
    fn put_to_held_moves_funds_without_changing_total() {
        let mut account = Client::new(d("10"));
        account.put_to_held(d("4")).unwrap();
        let row = snapshot(&account);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("6"), d("4"), d("10"))
        );
    }

    #[test]
    fn put_to_held_may_leave_available_negative() {
        let mut account = Client::new(d("10"));
        account.put_to_held(d("15")).unwrap();
        let row = snapshot(&account);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("-5"), d("15"), d("10"))
        );
    }

    #[test]
    fn resolve_held_returns_funds() {
        let mut account = Client::new(d("10"));
        account.put_to_held(d("4")).unwrap();
        account.resolve_held(d("4")).unwrap();
        let row = snapshot(&account);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("10"), d("0"), d("10"))
        );
    }

    #[test]
    fn chargeback_clears_held_and_locks() {
        let mut account = Client::new(d("10"));
        account.put_to_held(d("10")).unwrap();
        account.apply_chargeback(d("10")).unwrap();
        let row = snapshot(&account);
        assert_eq!(
            (row.available, row.held, row.total),
            (d("0"), d("0"), d("0"))
        );
        assert!(row.locked);
    }

    #[test]
    fn chargeback_beyond_held_is_refused_and_leaves_state() {
        let mut account = Client::new(d("10"));
        account.put_to_held(d("4")).unwrap();
        assert!(matches!(
            account.apply_chargeback(d("10")),
            Err(ClientError::AccountingError(AccountingError::HeldUnderflow))
        ));
        let row = snapshot(&account);
        assert_eq!((row.available, row.held), (d("6"), d("4")));
        assert!(!row.locked);
    }

    #[test]
    fn credit_overflow_is_refused() {
        let mut account = Client::new(Decimal::MAX);
        assert!(matches!(
            account.credit(d("1")),
            Err(ClientError::AccountingError(
                AccountingError::AvailableOverflow
            ))
        ));
        assert_eq!(snapshot(&account).available, Decimal::MAX);
    }

    #[test]
    fn resolve_held_overflow_is_refused() {
        let mut account = Client {
            available: Decimal::MAX,
            held: d("1"),
            is_locked: false,
        };
        assert!(matches!(
            account.resolve_held(d("1")),
            Err(ClientError::AccountingError(
                AccountingError::AvailableOverflow
            ))
        ));
    }

    #[test]
    fn client_row_reports_an_overflowing_total() {
        let account = Client {
            available: Decimal::MAX,
            held: Decimal::MAX,
            is_locked: false,
        };
        assert!(matches!(
            ClientRow::new(0, &account),
            Err(AccountingError::TotalOverflow)
        ));
    }
}
