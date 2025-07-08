mod abi_loader;
mod config;
mod db;
mod event_processor;

use ethers::providers::{Provider, StreamExt, Ws};
use ethers::types::{Log, Filter};
use ethers::middleware::Middleware;
use ethers::prelude::SubscriptionStream;
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let app_config = config::load_app_config("config.toml")?;

    let db_url = app_config.db_url;
    let pool = db::init_db(&db_url).await;

    let ws = Ws::connect(app_config.ws_provider).await?;
    let provider = Provider::new(ws);

    let contracts_config = config::load_named_contracts("contracts.toml")?;

    let mut abi_map = HashMap::new();
    let mut address_map = HashMap::new();

    for contract in contracts_config {
        let abi = abi_loader::load_abi(&contract.abi_path)?;
        abi_map.insert(contract.address, abi);
        address_map.insert(contract.address, contract.name);
    }

    let mut log_stream: SubscriptionStream<_, Log>;
    let from_block = app_config.indexing.from_block.unwrap_or(0);
    let to_block = app_config.indexing.to_block;

    if let Some(to_block) = to_block {
        log_stream = provider.subscribe_logs(&Filter::new().select(from_block..to_block)).await?;
        println!("Listening for logs from block {} to block {}...", from_block, to_block);
    } else {
        log_stream = provider.subscribe_logs(&Filter::new().select(from_block..)).await?;
        println!("Listening for logs from block {}...", from_block);
    }

    while let Some(log) = log_stream.next().await {
        if let Some(_name) = address_map.get(&log.address) {
            if let Some(abi) = abi_map.get(&log.address) {
                event_processor::process_event(log.address, &log, abi, &pool).await;
            } else {
                println!("No ABI found for address: {:?}", log.address);
            }
        }
    }

    Ok(())
}
