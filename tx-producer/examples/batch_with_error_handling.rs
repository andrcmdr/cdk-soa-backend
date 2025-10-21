//! Advanced batch transaction example with error handling and retry logic

use tx_producer::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Setup
    let provider_config = ProviderConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        timeout_seconds: 30,
    };

    let provider_manager = ProviderManager::new(provider_config)?
        .with_signer("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")?;

    let contract_config = ContractConfig {
        address: "0x1234567890123456789012345678901234567890".parse().unwrap(),
        abi_path: "abi/AirdropContract.json".to_string(),
    };

    let contract = ContractClient::new(
        contract_config,
        Arc::new(provider_manager),
    ).await?;

    println!("=== Advanced Batch Processing with Error Handling ===\n");

    // Create a large batch
    let mut transactions = Vec::new();
    for i in 0..50 {
        transactions.push(BatchTransaction {
            id: format!("tx_{}", i),
            contract_address: None,
            function_name: "setValue".to_string(),
            args: vec![serde_json::json!(i * 100)],
            gas_limit: Some(100000),
            gas_price: None,
            value: None,
        });
    }

    // Execute with retry logic
    let max_retries = 3;
    let mut retry_count = 0;
    let mut final_result = None;

    while retry_count < max_retries {
        println!("Attempt {} of {}", retry_count + 1, max_retries);

        let result = BatchTransactionBuilder::new(&contract)
            .add_transactions(transactions.clone())
            .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 10 })
            .continue_on_error(true)
            .execute()
            .await?;

        println!("Batch execution completed:");
        println!("  Successful: {}", result.successful);
        println!("  Failed: {}", result.failed);

        if result.all_succeeded() {
            println!("✓ All transactions succeeded!");
            final_result = Some(result);
            break;
        } else {
            println!("✗ Some transactions failed, analyzing...");

            // Extract failed transactions for retry
            let failed_ids: Vec<String> = result.failed_ids();
            println!("  Failed transaction IDs: {:?}", failed_ids);

            // Filter to only retry failed transactions
            transactions.retain(|tx| failed_ids.contains(&tx.id));

            if transactions.is_empty() {
                println!("No transactions to retry");
                final_result = Some(result);
                break;
            }

            retry_count += 1;
            if retry_count < max_retries {
                println!("Retrying {} failed transactions in 5 seconds...\n", transactions.len());
                sleep(Duration::from_secs(5)).await;
            } else {
                println!("Max retries reached");
                final_result = Some(result);
            }
        }
    }

    // Final summary
    if let Some(result) = final_result {
        println!("\n=== Final Results ===");
        println!("Total transactions: {}", result.total);
        println!("Successful: {} ({:.1}%)",
            result.successful,
            (result.successful as f64 / result.total as f64) * 100.0
        );
        println!("Failed: {} ({:.1}%)",
            result.failed,
            (result.failed as f64 / result.total as f64) * 100.0
        );
        println!("Total gas used: {}", result.total_gas_used);

        if !result.all_succeeded() {
            println!("\nPermanently failed transactions:");
            for id in result.failed_ids() {
                println!("  - {}", id);
            }
        }
    }

    Ok(())
}
