use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::{client::ClientId, transaction::TxId};

#[derive(Deserialize)]
pub enum EventType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
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
    pub amount: Option<String>,
}

#[derive(Error, Debug)]
pub enum EventError {
    #[error("Zero amount in transaction")]
    ZeroAmount,
    #[error("Negative amount in transaction")]
    NegativeAmount,
    #[error("Amount in transaction should be precise up to the 4th decimal and no more")]
    TooPrecise,
    #[error("Transaction should include an amount")]
    MissingAmount,
    #[error("Amount {0} is not a decimal number")]
    MalformedAmount(String),
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

fn parse_amount(raw: Option<String>) -> Result<Decimal, EventError> {
    let raw = raw.ok_or(EventError::MissingAmount)?;
    // Parsed from the field text, never through a float: `Decimal` keeps the exact
    // value and the written scale, and accepts the full range up to `Decimal::MAX`.
    let amount = Decimal::from_str(&raw).map_err(|_| EventError::MalformedAmount(raw))?;
    if amount.is_sign_negative() {
        return Err(EventError::NegativeAmount);
    }
    if amount.normalize().scale() > 4 {
        return Err(EventError::TooPrecise);
    }
    if amount.eq(&Decimal::ZERO) {
        return Err(EventError::ZeroAmount);
    }
    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn raw(event_type: EventType, amount: Option<&str>) -> RawEvent {
        RawEvent {
            tx: 1,
            event_type,
            client: 1,
            amount: amount.map(str::to_owned),
        }
    }

    #[test]
    fn every_event_type_parses() {
        let cases = [
            (EventType::Deposit, Some("2.5")),
            (EventType::Withdraw, Some("2.5")),
            (EventType::Dispute, None),
            (EventType::Resolve, None),
            (EventType::ChargeBack, None),
        ];
        for (event_type, amount) in cases {
            assert!(Event::try_from(raw(event_type, amount)).is_ok());
        }
    }

    #[test]
    fn deposit_keeps_the_amount() {
        let event = Event::try_from(raw(EventType::Deposit, Some("2.5"))).unwrap();
        assert!(matches!(event, Event::Deposit { amount, .. } if amount == d("2.5")));
    }

    #[test]
    fn deposit_without_an_amount_is_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Deposit, None)),
            Err(EventError::MissingAmount)
        ));
    }

    #[test]
    fn withdrawal_without_an_amount_is_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Withdraw, None)),
            Err(EventError::MissingAmount)
        ));
    }

    #[test]
    fn zero_amount_is_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Deposit, Some("0.0000"))),
            Err(EventError::ZeroAmount)
        ));
    }

    #[test]
    fn negative_amount_is_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Deposit, Some("-1"))),
            Err(EventError::NegativeAmount)
        ));
    }

    #[test]
    fn five_decimals_are_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Deposit, Some("1.00005"))),
            Err(EventError::TooPrecise)
        ));
    }

    #[test]
    fn six_decimals_of_trailing_zeros_are_accepted() {
        let event = Event::try_from(raw(EventType::Deposit, Some("1.000000"))).unwrap();
        assert!(matches!(event, Event::Deposit { amount, .. } if amount.scale() == 6));
    }

    #[test]
    fn four_decimals_are_accepted() {
        let event = Event::try_from(raw(EventType::Deposit, Some("1.2345"))).unwrap();
        assert!(matches!(event, Event::Deposit { amount, .. } if amount == d("1.2345")));
    }

    #[test]
    fn non_numeric_amount_is_refused() {
        assert!(matches!(
            Event::try_from(raw(EventType::Deposit, Some("abc"))),
            Err(EventError::MalformedAmount(_))
        ));
    }

    #[test]
    fn full_width_amount_is_accepted() {
        let event = Event::try_from(raw(
            EventType::Deposit,
            Some("79228162514264337593543950335"),
        ))
        .unwrap();
        assert!(matches!(event, Event::Deposit { amount, .. } if amount == Decimal::MAX));
    }
}
