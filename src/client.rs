use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

pub type ClientId = u16;

#[derive(Serialize, Default)]
pub struct Client {
    available: Decimal,
    held: Decimal,
    total: Decimal,
    is_locked: bool,
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

impl Client {
    pub fn new(amount: Decimal) -> Self {
        Self {
            available: amount,
            held: Decimal::ZERO,
            total: amount,
            is_locked: false,
        }
    }
    fn lock(&mut self) {
        self.is_locked = true;
    }
    fn set_total(&mut self) -> Result<(), ClientError> {
        self.total = self
            .available
            .checked_add(self.held)
            .ok_or(AccountingError::TotalOverflow)?;
        Ok(())
    }
    pub fn credit(&mut self, amount: Decimal) -> Result<(), ClientError> {
        if self.is_locked {
            return Err(ClientError::ClientIsLocked);
        }
        self.available = self
            .available
            .checked_add(amount)
            .ok_or(AccountingError::AvailableOverflow)?;
        self.set_total()?;
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
        self.set_total()?;
        Ok(())
    }
    pub fn put_to_held(&mut self, amount: Decimal) -> Result<(), ClientError> {
        self.held = self
            .held
            .checked_add(amount)
            .ok_or(AccountingError::HeldOverflow)?;
        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(AccountingError::AvailableUnderflow)?;
        self.set_total()?;
        Ok(())
    }
    pub fn apply_chargeback(&mut self, amount: Decimal) -> Result<(), ClientError> {
        self.held = self
            .held
            .checked_sub(amount)
            .ok_or(AccountingError::HeldUnderflow)?;
        self.set_total()?;
        self.lock();
        Ok(())
    }
    pub fn resolve_held(&mut self, amount: Decimal) -> Result<(), ClientError> {
        self.available = self
            .available
            .checked_add(amount)
            .ok_or(AccountingError::AvailableOverflow)?;
        self.held = self
            .held
            .checked_sub(amount)
            .ok_or(AccountingError::HeldUnderflow)?;
        Ok(())
    }
}
