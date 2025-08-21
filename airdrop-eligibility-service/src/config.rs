use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub rpc_url: String,
    pub contract_address: String,
    pub private_key: String,
    pub chain_id: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/airdrop".to_string()),
            rpc_url: std::env::var("RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8545".to_string()),
            contract_address: std::env::var("CONTRACT_ADDRESS")
                .expect("CONTRACT_ADDRESS must be set"),
            private_key: std::env::var("PRIVATE_KEY")
                .expect("PRIVATE_KEY must be set"),
            chain_id: std::env::var("CHAIN_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()?,
        })
    }
}
