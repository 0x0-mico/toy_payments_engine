use rust_decimal::Decimal;
use serde::Serialize;

pub type ClientId = u16;

#[derive(Serialize, Default)]
pub struct Client {
    available: Decimal,
    held: Decimal,
    total: Decimal,
    is_locked: bool,
}
