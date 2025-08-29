mod csv_loader;
mod trie_builder;
mod contract;
mod types;

use alloy_primitives::{Address, hex};
use std::env;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let csv_path = env::var("CSV_PATH")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let rpc_url = env::var("RPC_URL")?;
    let contract_address = Address::from_str(&env::var("CONTRACT_ADDRESS")?)?;

    let entries = csv_loader::load_airdrop_csv(&csv_path)?;
    let trie_result = trie_builder::build_trie(&entries);

    println!("Root Hash: 0x{}", hex::encode(trie_result.root_hash));

    contract::send_trie_update(
        &rpc_url,
        &private_key,
        contract_address,
        trie_result.root_hash,
        trie_result.trie_nodes,
    ).await?;

    Ok(())
}
