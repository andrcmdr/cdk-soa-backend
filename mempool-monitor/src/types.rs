use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPayload {
    pub chain_id: String,
    pub transaction_hash: String,
    pub transaction_sender: String,
    pub transaction_receiver: Option<String>,
    pub nonce: String,
    pub value: String,
    pub gas_limit: String,
    pub gas_price: Option<String>,
    pub max_fee_per_gas: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
    pub input_data: String,
    pub transaction_type: String,
    pub timestamp: String,
}
