//! Transaction building and signing

use alloy_dyn_abi::DynSolValue;
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, error};

use crate::contract::ContractClient;
use crate::error::{TxProducerError, Result};

/// Transaction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionParams {
    /// Function name to call
    pub function_name: String,
    /// Function arguments
    pub args: Vec<serde_json::Value>,
    /// Optional gas limit
    pub gas_limit: Option<u64>,
    /// Optional gas price
    pub gas_price: Option<U256>,
    /// Optional value to send (in Wei)
    pub value: Option<U256>,
}

/// Transaction builder
pub struct TransactionBuilder<'a> {
    contract: &'a ContractClient,
    params: TransactionParams,
}

impl<'a> TransactionBuilder<'a> {
    /// Create a new transaction builder
    pub fn new(contract: &'a ContractClient, function_name: String) -> Self {
        Self {
            contract,
            params: TransactionParams {
                function_name,
                args: Vec::new(),
                gas_limit: None,
                gas_price: None,
                value: None,
            },
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: serde_json::Value) -> Self {
        self.params.args.push(arg);
        self
    }

    /// Add multiple arguments
    pub fn args(mut self, args: Vec<serde_json::Value>) -> Self {
        self.params.args.extend(args);
        self
    }

    /// Set gas limit
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.params.gas_limit = Some(gas_limit);
        self
    }

    /// Set gas price
    pub fn gas_price(mut self, gas_price: U256) -> Self {
        self.params.gas_price = Some(gas_price);
        self
    }

    /// Set value to send
    pub fn value(mut self, value: U256) -> Self {
        self.params.value = Some(value);
        self
    }

    /// Build and send the transaction
    pub async fn send(self) -> Result<B256> {
        // Convert JSON values to DynSolValue
        let args = self.json_to_dyn_sol_values(&self.params.args)?;

        // Send transaction
        self.contract.send_transaction(&self.params.function_name, &args).await
    }

    /// Encode transaction data without sending
    pub fn encode(self) -> Result<Bytes> {
        // Convert JSON values to DynSolValue
        let args = self.json_to_dyn_sol_values(&self.params.args)?;

        // Encode function data
        self.contract.encode_function_data(&self.params.function_name, &args)
    }

    /// Convert JSON values to DynSolValue
    fn json_to_dyn_sol_values(&self, values: &[serde_json::Value]) -> Result<Vec<DynSolValue>> {
        values
            .iter()
            .map(|v| self.json_to_dyn_sol_value(v))
            .collect()
    }

    /// Convert a single JSON value to DynSolValue
    fn json_to_dyn_sol_value(&self, value: &serde_json::Value) -> Result<DynSolValue> {
        match value {
            serde_json::Value::Bool(b) => Ok(DynSolValue::Bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Ok(DynSolValue::Uint(U256::from(u).into(), 256))
                } else if let Some(i) = n.as_i64() {
                    Ok(DynSolValue::Int(U256::from(i as u64).into(), 256))
                } else {
                    Err(TxProducerError::Encoding("Invalid number format".to_string()))
                }
            }
            serde_json::Value::String(s) => {
                // Try to parse as address
                if s.starts_with("0x") && s.len() == 42 {
                    let addr: Address = s.parse()
                        .map_err(|e| TxProducerError::Encoding(format!("Invalid address: {}", e)))?;
                    Ok(DynSolValue::Address(addr))
                } else if s.starts_with("0x") {
                    // Assume it's bytes
                    let bytes = hex::decode(&s[2..])
                        .map_err(|e| TxProducerError::Encoding(format!("Invalid hex: {}", e)))?;
                    Ok(DynSolValue::Bytes(bytes))
                } else {
                    // String value
                    Ok(DynSolValue::String(s.clone()))
                }
            }
            serde_json::Value::Array(arr) => {
                let values = self.json_to_dyn_sol_values(arr)?;
                Ok(DynSolValue::Array(values))
            }
            _ => Err(TxProducerError::Encoding("Unsupported JSON value type".to_string())),
        }
    }
}

/// Call builder for read-only operations
pub struct CallBuilder<'a> {
    contract: &'a ContractClient,
    function_name: String,
    args: Vec<serde_json::Value>,
}

impl<'a> CallBuilder<'a> {
    /// Create a new call builder
    pub fn new(contract: &'a ContractClient, function_name: String) -> Self {
        Self {
            contract,
            function_name,
            args: Vec::new(),
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: serde_json::Value) -> Self {
        self.args.push(arg);
        self
    }

    /// Add multiple arguments
    pub fn args(mut self, args: Vec<serde_json::Value>) -> Self {
        self.args.extend(args);
        self
    }

    /// Execute the call
    pub async fn call(self) -> Result<Vec<DynSolValue>> {
        // Convert JSON values to DynSolValue
        let tx_builder = TransactionBuilder::new(self.contract, self.function_name);
        let args = tx_builder.json_to_dyn_sol_values(&self.args)?;

        // Call function
        self.contract.call_function(&tx_builder.params.function_name, &args).await
    }
}

/// Batch transaction item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransaction {
    /// Unique identifier for this transaction
    pub id: String,
    /// Contract address (if different from batch default)
    pub contract_address: Option<Address>,
    /// Function name
    pub function_name: String,
    /// Function arguments
    pub args: Vec<serde_json::Value>,
    /// Optional gas limit
    pub gas_limit: Option<u64>,
    /// Optional gas price
    pub gas_price: Option<U256>,
    /// Optional value to send
    pub value: Option<U256>,
}

/// Result of a single transaction in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransactionResult {
    /// Transaction ID
    pub id: String,
    /// Success status
    pub success: bool,
    /// Transaction hash (if successful)
    pub tx_hash: Option<B256>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Gas used
    pub gas_used: Option<u64>,
}

/// Batch transaction execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Total number of transactions
    pub total: usize,
    /// Number of successful transactions
    pub successful: usize,
    /// Number of failed transactions
    pub failed: usize,
    /// Individual transaction results
    pub results: Vec<BatchTransactionResult>,
    /// Total gas used
    pub total_gas_used: u64,
}

impl BatchResult {
    /// Check if all transactions succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    /// Get successful transaction hashes
    pub fn successful_hashes(&self) -> Vec<B256> {
        self.results
            .iter()
            .filter_map(|r| r.tx_hash)
            .collect()
    }

    /// Get failed transaction IDs
    pub fn failed_ids(&self) -> Vec<String> {
        self.results
            .iter()
            .filter(|r| !r.success)
            .map(|r| r.id.clone())
            .collect()
    }
}

/// Batch execution strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BatchExecutionStrategy {
    /// Execute all transactions in parallel
    Parallel,
    /// Execute transactions sequentially
    Sequential,
    /// Execute in parallel with rate limiting
    ParallelRateLimited { max_concurrent: usize },
}

impl Default for BatchExecutionStrategy {
    fn default() -> Self {
        BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 10 }
    }
}

/// Batch transaction builder
pub struct BatchTransactionBuilder<'a> {
    contract: &'a ContractClient,
    transactions: Vec<BatchTransaction>,
    strategy: BatchExecutionStrategy,
    continue_on_error: bool,
}

impl<'a> BatchTransactionBuilder<'a> {
    /// Create a new batch transaction builder
    pub fn new(contract: &'a ContractClient) -> Self {
        Self {
            contract,
            transactions: Vec::new(),
            strategy: BatchExecutionStrategy::default(),
            continue_on_error: true,
        }
    }

    /// Add a transaction to the batch
    pub fn add_transaction(mut self, tx: BatchTransaction) -> Self {
        self.transactions.push(tx);
        self
    }

    /// Add multiple transactions
    pub fn add_transactions(mut self, txs: Vec<BatchTransaction>) -> Self {
        self.transactions.extend(txs);
        self
    }

    /// Add a simple transaction by function name and args
    pub fn add(mut self, id: String, function_name: String, args: Vec<serde_json::Value>) -> Self {
        self.transactions.push(BatchTransaction {
            id,
            contract_address: None,
            function_name,
            args,
            gas_limit: None,
            gas_price: None,
            value: None,
        });
        self
    }

    /// Set execution strategy
    pub fn strategy(mut self, strategy: BatchExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set whether to continue on error
    pub fn continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }

    /// Execute the batch
    pub async fn execute(self) -> Result<BatchResult> {
        info!("Executing batch of {} transactions with strategy: {:?}",
              self.transactions.len(), self.strategy);

        let results = match self.strategy {
            BatchExecutionStrategy::Sequential => {
                self.execute_sequential().await?
            }
            BatchExecutionStrategy::Parallel => {
                self.execute_parallel(None).await?
            }
            BatchExecutionStrategy::ParallelRateLimited { max_concurrent } => {
                self.execute_parallel(Some(max_concurrent)).await?
            }
        };

        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;
        let total_gas_used = results.iter()
            .filter_map(|r| r.gas_used)
            .sum();

        let batch_result = BatchResult {
            total: results.len(),
            successful,
            failed,
            results,
            total_gas_used,
        };

        info!("Batch execution completed: {} successful, {} failed, {} total gas used",
              batch_result.successful, batch_result.failed, batch_result.total_gas_used);

        Ok(batch_result)
    }

    /// Execute transactions sequentially
    async fn execute_sequential(&self) -> Result<Vec<BatchTransactionResult>> {
        let mut results = Vec::new();

        for tx in &self.transactions {
            info!("Executing transaction {}: {}", tx.id, tx.function_name);

            match self.execute_single_transaction(tx).await {
                Ok(result) => {
                    results.push(result.clone());
                    if !result.success && !self.continue_on_error {
                        error!("Transaction {} failed, stopping batch execution", tx.id);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to execute transaction {}: {}", tx.id, e);
                    results.push(BatchTransactionResult {
                        id: tx.id.clone(),
                        success: false,
                        tx_hash: None,
                        error: Some(e.to_string()),
                        gas_used: None,
                    });
                    if !self.continue_on_error {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute transactions in parallel
    async fn execute_parallel(&self, max_concurrent: Option<usize>) -> Result<Vec<BatchTransactionResult>> {
        let semaphore = max_concurrent.map(|n| Arc::new(Semaphore::new(n)));

        let futures: Vec<_> = self.transactions
            .iter()
            .map(|tx| {
                let tx = tx.clone();
                let semaphore = semaphore.clone();

                async move {
                    // Acquire semaphore permit if rate limiting is enabled
                    let _permit = if let Some(sem) = semaphore {
                        Some(sem.acquire().await.unwrap())
                    } else {
                        None
                    };

                    info!("Executing transaction {}: {}", tx.id, tx.function_name);
                    self.execute_single_transaction(&tx).await
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        Ok(results
            .into_iter()
            .map(|r| match r {
                Ok(result) => result,
                Err(e) => {
                    error!("Transaction execution error: {}", e);
                    BatchTransactionResult {
                        id: "unknown".to_string(),
                        success: false,
                        tx_hash: None,
                        error: Some(e.to_string()),
                        gas_used: None,
                    }
                }
            })
            .collect())
    }

    /// Execute a single transaction
    async fn execute_single_transaction(&self, tx: &BatchTransaction) -> Result<BatchTransactionResult> {
        // Convert JSON args to DynSolValue
        let builder = TransactionBuilder::new(self.contract, tx.function_name.clone());
        let args = builder.json_to_dyn_sol_values(&tx.args)?;

        // Execute transaction
        match self.contract.send_transaction(&tx.function_name, &args).await {
            Ok(tx_hash) => {
                info!("Transaction {} succeeded: 0x{}", tx.id, hex::encode(tx_hash));

                // TODO: Get actual gas used from receipt
                Ok(BatchTransactionResult {
                    id: tx.id.clone(),
                    success: true,
                    tx_hash: Some(tx_hash),
                    error: None,
                    gas_used: None, // Could be fetched from receipt
                })
            }
            Err(e) => {
                warn!("Transaction {} failed: {}", tx.id, e);
                Ok(BatchTransactionResult {
                    id: tx.id.clone(),
                    success: false,
                    tx_hash: None,
                    error: Some(e.to_string()),
                    gas_used: None,
                })
            }
        }
    }

    /// Encode all transactions without executing
    pub fn encode_all(&self) -> Result<HashMap<String, Bytes>> {
        let mut encoded = HashMap::new();

        for tx in &self.transactions {
            let builder = TransactionBuilder::new(self.contract, tx.function_name.clone());
            let args = builder.json_to_dyn_sol_values(&tx.args)?;
            let data = self.contract.encode_function_data(&tx.function_name, &args)?;
            encoded.insert(tx.id.clone(), data);
        }

        Ok(encoded)
    }
}

/// Batch call builder for read-only operations
pub struct BatchCallBuilder<'a> {
    contract: &'a ContractClient,
    calls: Vec<(String, String, Vec<serde_json::Value>)>, // (id, function_name, args)
}

impl<'a> BatchCallBuilder<'a> {
    /// Create a new batch call builder
    pub fn new(contract: &'a ContractClient) -> Self {
        Self {
            contract,
            calls: Vec::new(),
        }
    }

    /// Add a call to the batch
    pub fn add_call(mut self, id: String, function_name: String, args: Vec<serde_json::Value>) -> Self {
        self.calls.push((id, function_name, args));
        self
    }

    /// Execute all calls in parallel
    pub async fn execute_parallel(self) -> Result<HashMap<String, Vec<DynSolValue>>> {
        let futures: Vec<_> = self.calls
            .iter()
            .map(|(id, function_name, args)| {
                let id = id.clone();
                let function_name = function_name.clone();
                let args = args.clone();

                async move {
                    let builder = TransactionBuilder::new(self.contract, function_name.clone());
                    let dyn_args = builder.json_to_dyn_sol_values(&args)?;
                    let result = self.contract.call_function(&function_name, &dyn_args).await?;
                    Ok::<_, TxProducerError>((id, result))
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut call_results = HashMap::new();
        for result in results {
            match result {
                Ok((id, values)) => {
                    call_results.insert(id, values);
                }
                Err(e) => {
                    warn!("Batch call failed: {}", e);
                }
            }
        }

        Ok(call_results)
    }

    /// Execute all calls sequentially
    pub async fn execute_sequential(self) -> Result<HashMap<String, Vec<DynSolValue>>> {
        let mut results = HashMap::new();

        for (id, function_name, args) in self.calls {
            let builder = TransactionBuilder::new(self.contract, function_name.clone());
            let dyn_args = builder.json_to_dyn_sol_values(&args)?;

            match self.contract.call_function(&function_name, &dyn_args).await {
                Ok(result) => {
                    results.insert(id, result);
                }
                Err(e) => {
                    warn!("Call {} failed: {}", id, e);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result_all_succeeded() {
        let result = BatchResult {
            total: 3,
            successful: 3,
            failed: 0,
            results: vec![
                BatchTransactionResult {
                    id: "1".to_string(),
                    success: true,
                    tx_hash: Some(B256::default()),
                    error: None,
                    gas_used: Some(21000),
                },
                BatchTransactionResult {
                    id: "2".to_string(),
                    success: true,
                    tx_hash: Some(B256::default()),
                    error: None,
                    gas_used: Some(21000),
                },
                BatchTransactionResult {
                    id: "3".to_string(),
                    success: true,
                    tx_hash: Some(B256::default()),
                    error: None,
                    gas_used: Some(21000),
                },
            ],
            total_gas_used: 63000,
        };

        assert!(result.all_succeeded());
        assert_eq!(result.successful_hashes().len(), 3);
        assert_eq!(result.failed_ids().len(), 0);
    }

    #[test]
    fn test_batch_result_with_failures() {
        let result = BatchResult {
            total: 3,
            successful: 2,
            failed: 1,
            results: vec![
                BatchTransactionResult {
                    id: "1".to_string(),
                    success: true,
                    tx_hash: Some(B256::default()),
                    error: None,
                    gas_used: Some(21000),
                },
                BatchTransactionResult {
                    id: "2".to_string(),
                    success: false,
                    tx_hash: None,
                    error: Some("Gas limit exceeded".to_string()),
                    gas_used: None,
                },
                BatchTransactionResult {
                    id: "3".to_string(),
                    success: true,
                    tx_hash: Some(B256::default()),
                    error: None,
                    gas_used: Some(21000),
                },
            ],
            total_gas_used: 42000,
        };

        assert!(!result.all_succeeded());
        assert_eq!(result.successful_hashes().len(), 2);
        assert_eq!(result.failed_ids(), vec!["2"]);
    }
}
