# Transaction Producer Library

Universal Ethereum Transaction Producer Library

A flexible Rust library for interacting with any Ethereum smart contract using JSON ABI files, built on top of Alloy 1.0.38.

## Features

- 🔧 **Universal Contract Interaction** - Works with any EVM-compatible contract
- 📄 **JSON ABI Support** - No need for `sol!()` macros or inline definitions
- ✍️ **Transaction Building** - Easy-to-use builder pattern for transactions
- 🔐 **Signing Support** - Built-in transaction signing with private keys
- 🌐 **Provider Management** - Flexible RPC provider configuration
- 🔍 **Read & Write Operations** - Support for both view and state-changing functions
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

## Building as Shared Library

To build as a `.so` library:

```bash
cargo build --release --lib
```

The library will be available at `target/release/libtx_producer.so` (or `.dylib` on macOS, `.dll` on Windows).

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

## License

Licensed under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Summary

This repository contains a comprehensive **universal transaction producer library** (`tx-producer`) that:

### Key Features:
1. **✅ JSON ABI Only** - No `sol!()` macros, pure JSON ABI files
2. **✅ Universal Contract Support** - Works with any EVM-compatible contract
3. **✅ Transaction Production** - Build, sign, and send transactions
4. **✅ Provider Management** - Flexible RPC configuration
5. **✅ Library Format** - Can be used as Rust crate or compiled to `.so`
6. **✅ Alloy 1.0.38** - Built on latest Alloy framework
7. **✅ Type-safe** - Strong typing with proper error handling

### Structure:
```
tx-producer/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          # Main entry point
│   ├── provider.rs     # Provider management
│   ├── contract.rs     # Contract interaction
│   ├── transaction.rs  # Transaction building
│   └── error.rs        # Error types
└── examples/
    └── basic_usage.rs  # Usage examples
```
