use anyhow::Result;
use tracing::info;
use tracing_subscriber;

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
    service.process_csv_and_update_trie("data/round_1.csv", 1).await?;
    service.submit_trie_update(1).await?;

    info!("Service completed successfully");
    Ok(())
}
