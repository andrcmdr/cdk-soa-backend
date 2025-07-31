use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct EventPayload {
    pub contract_name: String,
    pub contract_address: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub event_name: String,
    pub event_data: Value,
}
