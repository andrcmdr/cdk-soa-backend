//! Batch transaction execution example

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
        .with_signer("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")?;

    // Step 3: Configure contract
    let contract_config = ContractConfig {
        address: "0x1234567890123456789012345678901234567890".parse().unwrap(),
        abi_path: "abi/MyContract.json".to_string(),
    };

    // Step 4: Create contract client
    let contract = ContractClient::new(
        contract_config,
        Arc::new(provider_manager),
    ).await?;

    println!("=== Batch Transaction Examples ===\n");

    // Example 1: Sequential batch execution
    println!("Example 1: Sequential Batch Execution");
    let batch_result = BatchTransactionBuilder::new(&contract)
        .add("tx1".to_string(), "setValue".to_string(), vec![
            serde_json::json!("100"),
        ])
        .add("tx2".to_string(), "setValue".to_string(), vec![
            serde_json::json!("200"),
        ])
        .add("tx3".to_string(), "setValue".to_string(), vec![
            serde_json::json!("300"),
        ])
        .strategy(BatchExecutionStrategy::Sequential)
        .execute()
        .await?;

    print_batch_result("Sequential", &batch_result);

    // Example 2: Parallel batch execution
    println!("\nExample 2: Parallel Batch Execution");
    let batch_result = BatchTransactionBuilder::new(&contract)
        .add("tx1".to_string(), "updateData".to_string(), vec![
            serde_json::json!("data1"),
        ])
        .add("tx2".to_string(), "updateData".to_string(), vec![
            serde_json::json!("data2"),
        ])
        .add("tx3".to_string(), "updateData".to_string(), vec![
            serde_json::json!("data3"),
        ])
        .strategy(BatchExecutionStrategy::Parallel)
        .execute()
        .await?;

    print_batch_result("Parallel", &batch_result);

    // Example 3: Rate-limited parallel execution
    println!("\nExample 3: Rate-Limited Parallel Execution");
    let batch_result = BatchTransactionBuilder::new(&contract)
        .add("tx1".to_string(), "transfer".to_string(), vec![
            serde_json::json!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"),
            serde_json::json!("1000000000000000000"),
        ])
        .add("tx2".to_string(), "transfer".to_string(), vec![
            serde_json::json!("0x8ba1f109551bD432803012645Hac136c5a2B1A00"),
            serde_json::json!("2000000000000000000"),
        ])
        .add("tx3".to_string(), "transfer".to_string(), vec![
            serde_json::json!("0x9876543210987654321098765432109876543210"),
            serde_json::json!("3000000000000000000"),
        ])
        .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 2 })
        .execute()
        .await?;

    print_batch_result("Rate-Limited Parallel", &batch_result);

    // Example 4: Complex batch with custom transactions
    println!("\nExample 4: Complex Batch with Custom Transactions");
    let custom_batch = vec![
        BatchTransaction {
            id: "custom1".to_string(),
            contract_address: None,
            function_name: "updateTrieRoot".to_string(),
            args: vec![
                serde_json::json!("1"),
                serde_json::json!("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
                serde_json::json!("0xabcdef"),
            ],
            gas_limit: Some(200000),
            gas_price: None,
            value: None,
        },
        BatchTransaction {
            id: "custom2".to_string(),
            contract_address: None,
            function_name: "verifyEligibility".to_string(),
            args: vec![
                serde_json::json!("1"),
                serde_json::json!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"),
                serde_json::json!("1000000000000000000"),
                serde_json::json!([]),
            ],
            gas_limit: Some(100000),
            gas_price: None,
            value: None,
        },
    ];

    let batch_result = BatchTransactionBuilder::new(&contract)
        .add_transactions(custom_batch)
        .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 5 })
        .continue_on_error(true)
        .execute()
        .await?;

    print_batch_result("Complex Custom", &batch_result);

    // Example 5: Batch encoding without execution
    println!("\nExample 5: Batch Encoding (No Execution)");
    let encoded = BatchTransactionBuilder::new(&contract)
        .add("enc1".to_string(), "setValue".to_string(), vec![
            serde_json::json!("999"),
        ])
        .add("enc2".to_string(), "setValue".to_string(), vec![
            serde_json::json!("888"),
        ])
        .encode_all()?;

    for (id, data) in encoded.iter() {
        println!("  Transaction {}: 0x{}", id, hex::encode(data));
    }

    // Example 6: Batch read calls
    println!("\nExample 6: Batch Read Calls");
    let call_results = BatchCallBuilder::new(&contract)
        .add_call("call1".to_string(), "getValue".to_string(), vec![])
        .add_call("call2".to_string(), "getVersion".to_string(), vec![])
        .add_call("call3".to_string(), "getRoundCount".to_string(), vec![])
        .execute_parallel()
        .await?;

    for (id, result) in call_results.iter() {
        println!("  Call {}: {:?}", id, result);
    }

    Ok(())
}

fn print_batch_result(label: &str, result: &BatchResult) {
    println!("  {} Results:", label);
    println!("    Total: {}", result.total);
    println!("    Successful: {}", result.successful);
    println!("    Failed: {}", result.failed);
    println!("    Total Gas Used: {}", result.total_gas_used);

    if !result.all_succeeded() {
        println!("    Failed IDs: {:?}", result.failed_ids());
    }

    for tx_result in &result.results {
        if tx_result.success {
            println!("      ✓ {} - Hash: 0x{}",
                tx_result.id,
                tx_result.tx_hash.map(|h| hex::encode(h)).unwrap_or_else(|| "N/A".to_string())
            );
        } else {
            println!("      ✗ {} - Error: {}",
                tx_result.id,
                tx_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
            );
        }
    }
}
