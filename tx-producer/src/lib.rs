//! Universal Ethereum Transaction Producer Library
//!
//! A flexible library for interacting with any Ethereum smart contract using JSON ABI files,
//! built on top of Alloy 1.0.38.
//!
//! # Features
//!
//! - Universal contract interaction using JSON ABI
//! - Transaction building and signing
//! - **Batch transaction support** - Execute multiple transactions efficiently
//! - Provider management
//! - Read and write operations
//! - Event handling
//!
//! # Example
//!
//! ```rust,no_run
//! use tx_producer::*;
//! use alloy_primitives::U256;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Configure provider
//!     let provider_config = ProviderConfig {
//!         rpc_url: "http://localhost:8545".to_string(),
//!         chain_id: 1,
//!         timeout_seconds: 30,
//!     };
//!
//!     // Create provider manager with signer
//!     let provider_manager = ProviderManager::new(provider_config)?
//!         .with_signer("0x...")?;
//!
//!     // Configure contract
//!     let contract_config = ContractConfig {
//!         address: "0x...".parse().unwrap(),
//!         abi_path: "path/to/contract.json".to_string(),
//!     };
//!
//!     // Create contract client
//!     let contract = ContractClient::new(
//!         contract_config,
//!         std::sync::Arc::new(provider_manager),
//!     ).await?;
//!
//!     // Call a function
//!     let result = contract.call_function(
//!         "balanceOf",
//!         &["0x...".parse().unwrap()],
//!     ).await?;
//!
//!     // Execute batch transactions
//!     let batch_result = BatchTransactionBuilder::new(&contract)
//!         .add("tx1".to_string(), "transfer".to_string(), vec![
//!             serde_json::json!("0x..."),
//!             serde_json::json!("1000000000000000000"),
//!         ])
//!         .add("tx2".to_string(), "transfer".to_string(), vec![
//!             serde_json::json!("0x..."),
//!             serde_json::json!("2000000000000000000"),
//!         ])
//!         .strategy(BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 5 })
//!         .execute()
//!         .await?;
//!
//!     println!("Batch completed: {} successful, {} failed",
//!              batch_result.successful, batch_result.failed);
//!
//!     Ok(())
//! }
//! ```

pub mod contract;
pub mod error;
pub mod provider;
pub mod transaction;

// Re-export commonly used types
pub use contract::{ContractClient, ContractConfig, value_helpers};
pub use error::{Result, TxProducerError};
pub use provider::{ProviderConfig, ProviderManager, TxProvider};
pub use transaction::{
    CallBuilder, TransactionBuilder, TransactionParams,
    BatchTransaction, BatchTransactionBuilder, BatchTransactionResult, BatchResult,
    BatchCallBuilder, BatchExecutionStrategy,
};

// Re-export Alloy types for convenience
pub use alloy_dyn_abi::DynSolValue;
pub use alloy_json_abi::JsonAbi;
pub use alloy_primitives::{Address, B256, Bytes, U256};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::contract::{ContractClient, ContractConfig, value_helpers};
    pub use crate::error::{Result, TxProducerError};
    pub use crate::provider::{ProviderConfig, ProviderManager};
    pub use crate::transaction::{
        CallBuilder, TransactionBuilder,
        BatchTransaction, BatchTransactionBuilder, BatchResult,
        BatchCallBuilder, BatchExecutionStrategy,
    };
    pub use alloy_dyn_abi::DynSolValue;
    pub use alloy_primitives::{Address, B256, U256};
}
