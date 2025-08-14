mod csv_loader;
mod trie_builder;
mod contract;
mod types;

use alloy_primitives::hex;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let csv_path = env::args().nth(1).expect("CSV path not provided");
    let entries = csv_loader::load_airdrop_csv(&csv_path)?;
    let trie_result = trie_builder::build_trie(&entries);

    println!("Root Hash: 0x{}", hex::encode(trie_result.root_hash));

    let calldata = contract::encode_contract_call(trie_result.root_hash, trie_result.trie_nodes);

    println!("Encoded Contract Call Data: 0x{}", hex::encode(calldata));

    // ToDo: send transaction using alloy-rpc here

    Ok(())
}
