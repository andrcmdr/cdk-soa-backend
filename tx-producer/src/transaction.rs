//! Transaction building and signing

use alloy_dyn_abi::DynSolValue;
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};

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
