mod abi_loader;
mod db;
mod event_processor;

use crate::config::load_contracts_from_file;

use ethers::{
    providers::{Provider, StreamExt, Ws},
    types::{Address, Log},
};
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

    let configs = load_contracts_from_file("contracts.toml")?;

    let mut abi_map = HashMap::new();
    let mut address_map = HashMap::new();

    for c in configs {
        abi_map.insert(c.address, abi_loader::load_abi(c.abi_path)?);
        address_map.insert(c.address, c.address);
    }

    let mut sub = provider.subscribe_logs(&Default::default()).await?;
    println!("Listening for events...");

    while let Some(log) = sub.next().await {
        if let Some(contract) = address_map.get(&log.address) {
            if let Some(abi) = abi_map.get(contract) {
                event_processor::process_event(*contract, &log, abi, &pool).await;
            }
        }
    }

    Ok(())
}
