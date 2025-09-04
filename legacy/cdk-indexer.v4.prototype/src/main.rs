mod abi;
mod config;
mod db;
mod messaging;
mod subscriptions;

use abi::{AbiIndex, ContractAbi};
use alloy::primitives::{Address, B256};
use config::AppConfig;
use db::Db;
use messaging::Nats;
use subscriptions::{build_providers, Subscriptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    // Config
    let cfg = AppConfig::load("./config.yaml")?;

    // Providers
    let (http, ws) = build_providers(&cfg.rpc_http_url, &cfg.rpc_ws_url).await?;

    // DB
    let db = Db::connect(&cfg.postgres.url).await?;
    db.init_schema(include_str!("../schema.sql")).await?;

    // NATS (optional)
    let nats = if let Some(nc) = &cfg.nats {
        Some(Nats::connect(&nc.url, nc.subject.clone()).await?)
    } else { None };

    // Load ABIs and build index
    let addr_to_name = cfg.addresses_map()?;
    let mut contracts = Vec::new();
    for c in &cfg.contracts {
        let address: Address = c.address.parse()?;
        let abi = ContractAbi::load(c.name.clone(), address, &c.abi_path)?;
        contracts.push(abi);
    }
    let abi_index = AbiIndex::new(contracts);

    // Subscription manager
    let subs = Subscriptions::new(http, ws, db, nats, abi_index);

    // Addresses list to subscribe to
    let addresses: Vec<Address> = addr_to_name.keys().copied().collect();

    // From block (optional), parse hex -> B256 if given as block hash (you can extend for block number)
    let from_block: Option<B256> = None;
    let _ = subs.run(addresses, from_block).await?;

    Ok(())
}
