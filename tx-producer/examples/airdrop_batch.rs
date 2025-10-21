//! Airdrop batch processing example

use tx_producer::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Setup
    let provider_config = ProviderConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        timeout_seconds: 60,
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

    println!("=== Airdrop Batch Processing Example ===\n");

    // Simulate airdrop data (address -> amount)
    let airdrop_recipients: HashMap<String, String> = [
        ("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0", "1000000000000000000"),
        ("0x8ba1f109551bD432803012645Hac136c5a2B1A00", "2000000000000000000"),
        ("0x9876543210987654321098765432109876543210", "3000000000000000000"),
        ("0xabcdef1234567890abcdef1234567890abcdef12", "4000000000000000000"),
        ("0x1111111111111111111111111111111111111111", "5000000000000000000"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    println!("Processing {} airdrop recipients\n", airdrop_recipients.len());

    // Step 1: Batch verify eligibility (read operations)
    println!("Step 1: Batch Verify Eligibility");
    let mut verify_calls = BatchCallBuilder::new(&contract);

    for (i, (address, amount)) in airdrop_recipients.iter().enumerate() {
        verify_calls = verify_calls.add_call(
            format!("verify_{}", i),
            "verifyEligibility".to_string(),
            vec![
                serde_json::json!("1"), // round_id
                serde_json::json!(address),
                serde_json::json!(amount),
                serde_json::json!([]), // proof
            ],
        );
    }

    let verify_results = verify_calls.execute_parallel().await?;
    println!("  Verified {} addresses", verify_results.len());

    // Step 2: Update trie roots in batch
    println!("\nStep 2: Update Trie Roots");
    let update_transactions = vec![
        BatchTransaction {
            id: "update_round_1".to_string(),
            contract_address: None,
            function_name: "updateTrieRoot".to_string(),
            args: vec![
                serde_json::json!("1"),
                serde_json::json!("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
                serde_json::json!("0xabcdef123456"),
            ],
            gas_limit: Some(500000),
            gas_price: None,
            value: None,
        },
        BatchTransaction {
            id: "update_round_2".to_string(),
            contract_address: None,
            function_name: "updateTrieRoot".to_string(),
            args: vec![
                serde_json::json!("2"),
                serde_json::json!("0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"),
                serde_json::json!("0x654321fedcba"),
            ],
            gas_limit: Some(500000),
            gas_price: None,
            value: None,
        },
    ];

    let update_result = BatchTransactionBuilder::new(&contract)
        .add_transactions(update_transactions)
        .strategy(BatchExecutionStrategy::Sequential)
        .continue_on_error(false)
        .execute()
        .await?;

    println!("  Update Results:");
    println!("    Successful: {}", update_result.successful);
    println!("    Failed: {}", update_result.failed);

    if !update_result.all_succeeded() {
        println!("    ✗ Some updates failed, aborting airdrop");
        return Ok(());
    }

    // Step 3: Batch check round status (read operations)
    println!("\nStep 3: Batch Check Round Status");
    let status_results = BatchCallBuilder::new(&contract)
        .add_call("round_1_active".to_string(), "isRoundActive".to_string(), vec![
            serde_json::json!("1"),
        ])
        .add_call("round_2_active".to_string(), "isRoundActive".to_string(), vec![
            serde_json::json!("2"),
        ])
        .add_call("round_count".to_string(), "getRoundCount".to_string(), vec![])
        .execute_parallel()
        .await?;

    println!("  Status checks completed: {} results", status_results.len());

    // Step 4: Process airdrop distribution
    println!("\nStep 4: Process Airdrop Distribution");
    let mut distribution_txs = Vec::new();

    for (i, (address, amount)) in airdrop_recipients.iter().enumerate() {
        distribution_txs.push(BatchTransaction {
            id: format!("airdrop_{}", i),
            contract_address: None,
            function_name: "processAirdrop".to_string(),
            args: vec![
                serde_json::json!("1"), // round_id
                serde_json::json!(address),
                serde_json::json!(amount),
            ],
            gas_limit: Some(200000),
            gas_price: None,
            value: None,
        });
    }

    let distribution_result = BatchTransactionBuilder::new(&contract)
        .add_transactions(distribution_txs)
        .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 5 })
        .continue_on_error(true)
        .execute()
        .await?;

    println!("  Distribution Results:");
    println!("    Total recipients: {}", distribution_result.total);
    println!("    Successful: {}", distribution_result.successful);
    println!("    Failed: {}", distribution_result.failed);
    println!("    Total gas used: {}", distribution_result.total_gas_used);

    // Summary
    println!("\n=== Airdrop Summary ===");
    println!("Distribution success rate: {:.1}%",
        (distribution_result.successful as f64 / distribution_result.total as f64) * 100.0
    );

    if !distribution_result.all_succeeded() {
        println!("\nFailed distributions:");
        for id in distribution_result.failed_ids() {
            println!("  - {}", id);
        }
    } else {
        println!("\n✓ All airdrops distributed successfully!");
    }

    println!("\nSuccessful transaction hashes:");
    for (i, hash) in distribution_result.successful_hashes().iter().enumerate() {
        println!("  {}: 0x{}", i + 1, hex::encode(hash));
    }

    Ok(())
}
