use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayload {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub block_timestamp: String,
    pub block_time: String,
    pub parent_hash: String,
    pub gas_used: String,
    pub gas_limit: String,
    pub transactions: Option<Vec<serde_json::Value>>,
}
