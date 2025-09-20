use alloy_primitives::Address;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AirdropEntry {
    pub address: Address,
    pub amount: u64,
    pub round: u32,
}
