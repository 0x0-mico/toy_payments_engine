use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::client::ClientId;

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
    Withdraw {
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
    type Error = TxError;
    fn try_from(value: RawEvent) -> Result<Self, TxError> {
        match value.event_type {
            EventType::Deposit => {
                let amount = parse_amount(value.amount)?;
                Ok(Self::Deposit {
                    client: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            EventType::Withdraw => {
                let amount = parse_amount(value.amount)?;
                Ok(Self::Withdraw {
                    client: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            EventType::Dispute => Ok(Self::Dispute {
                client: value.client,
                tx: value.tx,
            }),
            EventType::Resolve => Ok(Self::Resolve {
                client: value.client,
                tx: value.tx,
            }),
            EventType::ChargeBack => Ok(Self::ChargeBack {
                client: value.client,
                tx: value.tx,
            }),
        }
    }
}

fn parse_amount(amount: Option<Decimal>) -> Result<Decimal, TxError> {
    let amount = amount.ok_or(TxError::MissingAmount)?;
    if amount.is_sign_negative() {
        return Err(TxError::NegativeAmount);
    }
    if amount.round_dp(4) != amount {
        return Err(TxError::TooPrecise);
    }
    if amount.eq(&Decimal::ZERO) {
        return Err(TxError::ZeroAmount);
    }
    Ok(amount)
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
