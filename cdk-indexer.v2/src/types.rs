use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct EventPayload {
    pub contract_name: String,
    pub contract_address: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub event_name: String,
    pub params: HashMap<String, String>,
}
