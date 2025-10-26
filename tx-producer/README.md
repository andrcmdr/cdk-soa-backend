# Transaction Producer Library

Universal Ethereum Transaction Producer Library

A flexible Rust library for interacting with any Ethereum smart contract using JSON ABI files, built on top of Alloy 1.0.38.

## Features

- 🔧 **Universal Contract Interaction** - Works with any EVM-compatible contract
- 📄 **JSON ABI Support** - No need for `sol!()` macros or inline definitions
- ✍️ **Transaction Building** - Easy-to-use builder pattern for transactions
- 🚀 **Batch Transaction Support** - Execute multiple transactions efficiently
- 🔐 **Signing Support** - Built-in transaction signing with private keys
- 🌐 **Provider Management** - Flexible RPC provider configuration
- 🔍 **Read & Write Operations** - Support for both view and state-changing functions
- ⚡ **Parallel Execution** - Execute transactions in parallel with rate limiting
- 🔄 **Error Handling** - Comprehensive error handling and retry logic
- 📦 **Library Ready** - Can be used as Rust crate or compiled to `.so`

## Transaction Producer Library - Design Overview

1. **Uses only JSON ABI files** - removes dependency on `sol!()` macro
2. **Generic contract interaction** - works with any EVM-compatible contract
3. **Transaction production & signing** - handles transaction creation and signing
4. **Provider abstraction** - flexible provider configuration
5. **Library-ready** - can be included as a crate or compiled to `.so`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tx-producer = "0.1"
```

Or use as a path dependency:

```toml
[dependencies]
tx-producer = { path = "./tx-producer" }
```

## Quick Start

```rust
use tx_producer::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configure provider
    let provider_config = ProviderConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        timeout_seconds: 30,
    };

    // 2. Create provider with signer
    let provider_manager = ProviderManager::new(provider_config)?
        .with_signer("0x...")?;

    // 3. Configure contract
    let contract_config = ContractConfig {
        address: "0x...".parse().unwrap(),
        abi_path: "abi/MyContract.json".to_string(),
    };

    // 4. Create contract client
    let contract = ContractClient::new(
        contract_config,
        Arc::new(provider_manager),
    ).await?;

    // 5. Call a read function
    let result = contract.call_function("balanceOf", &[addr.into()]).await?;
    let balance = value_helpers::as_uint(&result[0])?;

    // 6. Send a transaction
    let tx_hash = contract.send_transaction(
        "transfer",
        &[recipient.into(), amount.into()],
    ).await?;

    Ok(())
}
```

## Batch Transactions

### Simple Batch Execution

```rust
use tx_producer::prelude::*;

// Execute multiple transactions in parallel
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
    .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 5 })
    .execute()
    .await?;

println!("Batch completed: {} successful, {} failed",
         batch_result.successful, batch_result.failed);
```

### Batch Execution Strategies

```rust
// 1. Sequential execution (one after another)
let batch_result = BatchTransactionBuilder::new(&contract)
    .add_transactions(transactions)
    .strategy(BatchExecutionStrategy::Sequential)
    .execute()
    .await?;

// 2. Parallel execution (all at once)
let batch_result = BatchTransactionBuilder::new(&contract)
    .add_transactions(transactions)
    .strategy(BatchExecutionStrategy::Parallel)
    .execute()
    .await?;

// 3. Rate-limited parallel execution (recommended)
let batch_result = BatchTransactionBuilder::new(&contract)
    .add_transactions(transactions)
    .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 10 })
    .execute()
    .await?;
```

### Custom Batch Transactions

```rust
let custom_batch = vec![
    BatchTransaction {
        id: "tx1".to_string(),
        contract_address: None,
        function_name: "updateData".to_string(),
        args: vec![
            serde_json::json!("1"),
            serde_json::json!("0x1234..."),
            serde_json::json!("0xabcd..."),
        ],
        gas_limit: Some(200000),
        gas_price: None,
        value: None,
    },
    // ... more transactions
];

let result = BatchTransactionBuilder::new(&contract)
    .add_transactions(custom_batch)
    .continue_on_error(true) // Continue even if some fail
    .execute()
    .await?;
```

### Batch Read Calls

```rust
// Execute multiple read calls in parallel
let results = BatchCallBuilder::new(&contract)
    .add_call("call1".to_string(), "getValue".to_string(), vec![])
    .add_call("call2".to_string(), "getVersion".to_string(), vec![])
    .add_call("call3".to_string(), "getRoundCount".to_string(), vec![])
    .execute_parallel()
    .await?;

for (id, result) in results.iter() {
    println!("Call {}: {:?}", id, result);
}
```

### Handling Batch Results

```rust
let batch_result = BatchTransactionBuilder::new(&contract)
    .add_transactions(transactions)
    .execute()
    .await?;

// Check if all succeeded
if batch_result.all_succeeded() {
    println!("All transactions successful!");
} else {
    println!("Some transactions failed");

    // Get failed transaction IDs
    let failed_ids = batch_result.failed_ids();
    println!("Failed: {:?}", failed_ids);

    // Get successful transaction hashes
    let successful_hashes = batch_result.successful_hashes();
    println!("Successful hashes: {:?}", successful_hashes);
}

// Detailed results
for tx_result in &batch_result.results {
    if tx_result.success {
        println!("✓ {} - Hash: {:?}", tx_result.id, tx_result.tx_hash);
    } else {
        println!("✗ {} - Error: {:?}", tx_result.id, tx_result.error);
    }
}
```

### Retry Logic for Failed Transactions

```rust
let max_retries = 3;
let mut transactions = /* your transactions */;
let mut retry_count = 0;

while retry_count < max_retries {
    let result = BatchTransactionBuilder::new(&contract)
        .add_transactions(transactions.clone())
        .execute()
        .await?;

    if result.all_succeeded() {
        break;
    }

    // Keep only failed transactions for retry
    let failed_ids = result.failed_ids();
    transactions.retain(|tx| failed_ids.contains(&tx.id));

    retry_count += 1;
}
```

## Usage Examples

### Reading Contract State

```rust
// Get contract version
let result = contract.call_function("getVersion", &[]).await?;
let version = value_helpers::as_string(&result[0])?;
println!("Version: {}", version);

// Get balance
let result = contract.call_function(
    "balanceOf",
    &[address.into()],
).await?;
let balance = value_helpers::as_uint(&result[0])?;
```

### Sending Transactions

```rust
// Simple transaction
let tx_hash = contract.send_transaction(
    "setValue",
    &[U256::from(42).into()],
).await?;

// Complex transaction with multiple parameters
let tx_hash = contract.send_transaction(
    "updateData",
    &[
        round_id.into(),
        root_hash.into(),
        data.into(),
    ],
).await?;
```

### Using Transaction Builder

```rust
use tx_producer::transaction::TransactionBuilder;

let tx_hash = TransactionBuilder::new(&contract, "transfer".to_string())
    .arg(serde_json::json!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"))
    .arg(serde_json::json!("1000000000000000000"))
    .gas_limit(100000)
    .send()
    .await?;
```

### Encoding Transaction Data

```rust
// Encode without sending
let encoded_data = contract.encode_function_data(
    "transfer",
    &[recipient.into(), amount.into()],
)?;

println!("Encoded data: 0x{}", hex::encode(encoded_data));
```

## Advanced Features

### Custom Value Conversions

```rust
use tx_producer::value_helpers::*;

// Extract values from contract responses
let uint_value = as_uint(&result[0])?;
let bool_value = as_bool(&result[1])?;
let address_value = as_address(&result[2])?;
let string_value = as_string(&result[3])?;
let bytes_value = as_fixed_bytes(&result[4])?;

// Handle tuples
let tuple = as_tuple(&result[0])?;
for value in tuple {
    // Process each value
}
```

### Contract Introspection

```rust
// List all available functions
for func_name in contract.list_functions() {
    println!("Function: {}", func_name);
}

// List all events
for event_name in contract.list_events() {
    println!("Event: {}", event_name);
}

// Get function details
let function = contract.get_function("transfer")?;
println!("Function signature: {}", function.signature());
```

## Real-World Examples

Check the `examples/` directory for complete working examples:

- `basic_usage.rs` - Basic contract interaction
- `batch_transactions.rs` - Various batch execution patterns
- `batch_with_error_handling.rs` - Advanced error handling and retry logic
- `airdrop_batch.rs` - Complete airdrop processing example

Run an example:

```bash
cargo run --example batch_transactions
cargo run --example airdrop_batch
```

## Building as Shared Library

To build as a `.so` library:

```bash
cargo build --release --lib
```

The library will be available at `target/release/libtx_producer.so` (or `.dylib` on macOS, `.dll` on Windows).

## Performance Considerations

### Batch Size

- **Small batches (1-10 txs)**: Use `Sequential` or `Parallel`
- **Medium batches (10-50 txs)**: Use `ParallelRateLimited` with max_concurrent: 5-10
- **Large batches (50+ txs)**: Use `ParallelRateLimited` with max_concurrent: 10-20

### Rate Limiting

Always use rate limiting for production to avoid:
- RPC rate limits
- Network congestion
- Nonce conflicts

```rust
.strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 10 })
```

### Gas Optimization

```rust
// Set appropriate gas limits
BatchTransaction {
    gas_limit: Some(200000), // Adjust based on function complexity
    // ...
}
```

## ABI File Format

The library expects standard Ethereum ABI JSON format:

```json
[
  {
    "type": "function",
    "name": "balanceOf",
    "inputs": [
      {
        "name": "account",
        "type": "address"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "uint256"
      }
    ],
    "stateMutability": "view"
  }
]
```

## Error Handling

All operations return `Result<T, TxProducerError>`:

```rust
match contract.call_function("getValue", &[]).await {
    Ok(result) => {
        println!("Success: {:?}", result);
    }
    Err(TxProducerError::ContractCall(msg)) => {
        eprintln!("Contract call failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Testing

Run tests:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

## License

Licensed under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
