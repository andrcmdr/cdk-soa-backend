//! Basic usage example

use tx_producer::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Step 1: Configure provider
    let provider_config = ProviderConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        timeout_seconds: 30,
    };

    // Step 2: Create provider manager with private key
    let provider_manager = ProviderManager::new(provider_config)?
        .with_signer("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")?; // Example key

    // Step 3: Check connection
    let block_number = provider_manager.check_connection().await?;
    println!("Connected! Current block: {}", block_number);

    // Step 4: Configure contract
    let contract_config = ContractConfig {
        address: "0x1234567890123456789012345678901234567890".parse().unwrap(),
        abi_path: "abi/MyContract.json".to_string(),
    };

    // Step 5: Create contract client
    let contract = ContractClient::new(
        contract_config,
        Arc::new(provider_manager),
    ).await?;

    // Step 6: List available functions
    println!("Available functions:");
    for func in contract.list_functions() {
        println!("  - {}", func);
    }

    // Step 7: Call a read function
    println!("\nCalling getVersion()...");
    let result = contract.call_function("getVersion", &[]).await?;
    let version = value_helpers::as_string(&result[0])?;
    println!("Contract version: {}", version);

    // Step 8: Send a transaction
    println!("\nSending transaction...");
    let tx_hash = contract.send_transaction(
        "updateValue",
        &[DynSolValue::Uint(U256::from(42).into(), 256)],
    ).await?;
    println!("Transaction sent: 0x{}", hex::encode(tx_hash));

    Ok(())
}
