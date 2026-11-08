use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::{client::ClientId, engine::EngineError};

pub type TxId = u32;

#[derive(Deserialize)]
pub enum EventType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdraw")]
    Withdraw,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    ChargeBack,
}

#[derive(Deserialize)]
pub struct RawEvent {
    pub tx: TxId,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub client: ClientId,
    pub amount: Option<Decimal>,
}

#[derive(Error, Debug)]
pub enum TxError {
    #[error("Zero amount in transaction")]
    ZeroAmount,
    #[error("Negative amount in transaction")]
    NegativeAmount,
    #[error("Amount is out of bounds")]
    AmountOutOfBounds,
    #[error("Amount in transaction should be prices up to the 4th decimal and no more")]
    TooPrecise,
    #[error("Transaction should include an amount")]
    MissingAmount,
}

pub enum Event {
    Deposit {
        client: ClientId,
        tx: TxId,
        amount: Decimal,
    },
    Withdrawal {
        client: ClientId,
        tx: TxId,
        amount: Decimal,
    },
    Dispute {
        client: ClientId,
        tx: TxId,
    },
    Resolve {
        client: ClientId,
        tx: TxId,
    },
    ChargeBack {
        client: ClientId,
        tx: TxId,
    },
}

impl TryFrom<RawEvent> for Event {
    type Error = EngineError;

    fn try_from(value: RawEvent) -> Result<Self, TxError> {
        match value.event_type {
            EventType::Deposit => {
                let amount = value.amount.ok_or(TxError::MissingAmount)?;
                if amount.round_dp(4) != amount {
                    return Err(TxError::TooPrecise);
                }
                Ok(Self::Deposit {
                    client: value.client,
                    tx: value.tx,
                    amount,
                })
            }
        }
    }
}

pub enum MoneyTx {
    Deposit(Decimal),
    Withdrawal(Decimal),
}

pub enum Dispute {
    Open,
    ChargedBack,
}

pub struct Tx {
    client: ClientId,
    kind: MoneyTx,
    dispute: Option<Dispute>,
}
