mod abi_loader;
mod config;
mod db;
mod event_processor;

use ethers::providers::{Provider, StreamExt, Ws};
use std::collections::HashMap;
use tokio;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")?;
    let pool = db::init_db(&db_url).await;

    let ws = Ws::connect("ws://localhost:8000/stream").await?;
    let provider = Provider::new(ws);

    let configs = config::load_named_contracts("contracts.toml")?;

    let mut abi_map = HashMap::new();
    let mut address_map = HashMap::new();

    for contract in configs {
        let abi = abi_loader::load_abi(&contract.abi_path)?;
        abi_map.insert(contract.address, abi);
        address_map.insert(contract.address, contract.name);
    }

    let mut log_stream = provider.subscribe_logs(&Default::default()).await?;
    println!("Listening for logs...");

    while let Some(log) = log_stream.next().await {
        if let Some(name) = address_map.get(&log.address) {
            if let Some(abi) = abi_map.get(&log.address) {
                event_processor::process_event(log.address, &log, abi, &pool).await;
            } else {
                println!("No ABI found for address: {:?}", log.address);
            }
        }
    }

    Ok(())
}
