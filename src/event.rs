use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::{client::ClientId, transaction::TxId};

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
pub enum EventError {
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
        client_id: ClientId,
        tx_id: TxId,
        amount: Decimal,
    },
    Withdraw {
        client_id: ClientId,
        tx_id: TxId,
        amount: Decimal,
    },
    Dispute {
        client_id: ClientId,
        tx_id: TxId,
    },
    Resolve {
        client_id: ClientId,
        tx_id: TxId,
    },
    ChargeBack {
        client_id: ClientId,
        tx_id: TxId,
    },
}

impl TryFrom<RawEvent> for Event {
    type Error = EventError;
    fn try_from(value: RawEvent) -> Result<Self, EventError> {
        match value.event_type {
            EventType::Deposit => {
                let amount = parse_amount(value.amount)?;
                Ok(Self::Deposit {
                    client_id: value.client,
                    tx_id: value.tx,
                    amount,
                })
            }
            EventType::Withdraw => {
                let amount = parse_amount(value.amount)?;
                Ok(Self::Withdraw {
                    client_id: value.client,
                    tx_id: value.tx,
                    amount,
                })
            }
            EventType::Dispute => Ok(Self::Dispute {
                client_id: value.client,
                tx_id: value.tx,
            }),
            EventType::Resolve => Ok(Self::Resolve {
                client_id: value.client,
                tx_id: value.tx,
            }),
            EventType::ChargeBack => Ok(Self::ChargeBack {
                client_id: value.client,
                tx_id: value.tx,
            }),
        }
    }
}

fn parse_amount(amount: Option<Decimal>) -> Result<Decimal, EventError> {
    let amount = amount.ok_or(EventError::MissingAmount)?;
    if amount.is_sign_negative() {
        return Err(EventError::NegativeAmount);
    }
    if amount.round_dp(4) != amount {
        // TODO: not good yet. i would rather not round 0.00005 to 0.0001
        return Err(EventError::TooPrecise);
    }
    if amount.eq(&Decimal::ZERO) {
        return Err(EventError::ZeroAmount);
    }
    Ok(amount)
}
