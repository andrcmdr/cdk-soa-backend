use anyhow::Result;
use tracing::info;
use tracing_subscriber;
use alloy_primitives::{Address, U256};
use std::str::FromStr;

mod config;
mod database;
mod merkle_trie;
mod csv_processor;
mod contract_client;
mod service;

use crate::service::AirdropService;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Airdrop Backend Service");

    let config = config::Config::from_env()?;
    let mut service = AirdropService::new(config).await?;

    // Example usage

    // Process multiple rounds
    service.process_csv_and_update_trie("data/round_1.csv", 1).await?;
    service.process_csv_and_update_trie("data/round_2.csv", 2).await?;

    // Submit to blockchain
    service.submit_trie_update(1).await?;
    service.submit_trie_update(2).await?;

    // Verify eligibility
    let user_address = Address::from_str("0x1234567890123456789012345678901234567890")?;
    let amount = U256::from(1000);
    let is_eligible = service.verify_eligibility(1, user_address, amount).await?;

    info!("Service completed successfully");
    Ok(())
}
