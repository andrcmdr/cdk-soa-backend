use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub contract_name: String,
    pub contract_address: String,
    pub block_number: String,
    pub block_hash: String,
    pub block_timestamp: String,
    pub block_time: String,
    pub transaction_hash: String,
    pub transaction_index: String,
    pub log_index: String,
    pub event_name: String,
    pub event_signature: String,
    pub event_data: Value,
}
