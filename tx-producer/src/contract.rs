//! Universal contract interaction using JSON ABI

use alloy_contract::{ContractInstance, Interface};
use alloy_dyn_abi::DynSolValue;
use alloy_json_abi::{JsonAbi, Function, Event};
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::error::{TxProducerError, Result};
use crate::provider::{ProviderManager, TxProvider};

/// Contract configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// Contract address
    pub address: Address,
    /// Path to ABI file (JSON)
    pub abi_path: String,
}

/// Universal contract client
pub struct ContractClient {
    /// Contract address
    address: Address,
    /// Contract ABI
    abi: JsonAbi,
    /// Contract instance
    instance: ContractInstance<TxProvider>,
    /// Provider manager
    provider_manager: Arc<ProviderManager>,
}

impl ContractClient {
    /// Create a new contract client
    pub async fn new(
        config: ContractConfig,
        provider_manager: Arc<ProviderManager>,
    ) -> Result<Self> {
        // Load ABI from file
        let abi = Self::load_abi(&config.abi_path).await?;

        // Create contract interface
        let interface = Interface::new(abi.clone());

        // Create contract instance
        let instance = ContractInstance::new(
            config.address,
            provider_manager.provider().as_ref().clone(),
            interface,
        );

        Ok(Self {
            address: config.address,
            abi,
            instance,
            provider_manager,
        })
    }

    /// Load ABI from JSON file
    async fn load_abi(path: &str) -> Result<JsonAbi> {
        let abi_content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| TxProducerError::AbiLoad(format!("Failed to read ABI file {}: {}", path, e)))?;

        let abi: JsonAbi = serde_json::from_str(&abi_content)
            .map_err(|e| TxProducerError::AbiLoad(format!("Failed to parse ABI: {}", e)))?;

        Ok(abi)
    }

    /// Get contract address
    pub fn address(&self) -> Address {
        self.address
    }

    /// Get contract ABI
    pub fn abi(&self) -> &JsonAbi {
        &self.abi
    }

    /// Call a read-only function
    pub async fn call_function(
        &self,
        function_name: &str,
        args: &[DynSolValue],
    ) -> Result<Vec<DynSolValue>> {
        let call = self.instance
            .function(function_name, args)
            .map_err(|e| TxProducerError::ContractCall(format!("Failed to create function call: {}", e)))?;

        let result = call
            .call()
            .await
            .map_err(|e| TxProducerError::ContractCall(format!("Function call failed: {}", e)))?;

        Ok(result)
    }

    /// Send a transaction (state-changing function)
    pub async fn send_transaction(
        &self,
        function_name: &str,
        args: &[DynSolValue],
    ) -> Result<B256> {
        let call = self.instance
            .function(function_name, args)
            .map_err(|e| TxProducerError::ContractCall(format!("Failed to create transaction: {}", e)))?;

        let pending_tx = call
            .send()
            .await
            .map_err(|e| TxProducerError::Transaction(format!("Transaction failed: {}", e)))?;

        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| TxProducerError::Transaction(format!("Failed to get receipt: {}", e)))?;

        Ok(receipt.transaction_hash)
    }

    /// Get function by name
    pub fn get_function(&self, name: &str) -> Result<&Function> {
        self.abi
            .function(name)
            .ok_or_else(|| TxProducerError::ContractCall(format!("Function '{}' not found in ABI", name)))
    }

    /// Get event by name
    pub fn get_event(&self, name: &str) -> Result<&Event> {
        self.abi
            .event(name)
            .ok_or_else(|| TxProducerError::ContractCall(format!("Event '{}' not found in ABI", name)))
    }

    /// List all available functions
    pub fn list_functions(&self) -> Vec<String> {
        self.abi.functions().map(|f| f.name.clone()).collect()
    }

    /// List all available events
    pub fn list_events(&self) -> Vec<String> {
        self.abi.events().map(|e| e.name.clone()).collect()
    }

    /// Encode function call data
    pub fn encode_function_data(
        &self,
        function_name: &str,
        args: &[DynSolValue],
    ) -> Result<Bytes> {
        let function = self.get_function(function_name)?;
        let encoded = function
            .abi_encode_input(args)
            .map_err(|e| TxProducerError::Encoding(format!("Failed to encode function data: {}", e)))?;

        Ok(Bytes::from(encoded))
    }

    /// Decode function result
    pub fn decode_function_result(
        &self,
        function_name: &str,
        data: &[u8],
    ) -> Result<Vec<DynSolValue>> {
        let function = self.get_function(function_name)?;
        let decoded = function
            .abi_decode_output(data, false)
            .map_err(|e| TxProducerError::Decoding(format!("Failed to decode function result: {}", e)))?;

        Ok(decoded)
    }
}

/// Helper functions for common value conversions
pub mod value_helpers {
    use super::*;

    /// Convert DynSolValue to U256
    pub fn as_uint(value: &DynSolValue) -> Result<U256> {
        value
            .as_uint()
            .map(|(v, _)| v.into())
            .ok_or_else(|| TxProducerError::Decoding("Expected uint value".to_string()))
    }

    /// Convert DynSolValue to bool
    pub fn as_bool(value: &DynSolValue) -> Result<bool> {
        value
            .as_bool()
            .ok_or_else(|| TxProducerError::Decoding("Expected bool value".to_string()))
    }

    /// Convert DynSolValue to Address
    pub fn as_address(value: &DynSolValue) -> Result<Address> {
        value
            .as_address()
            .ok_or_else(|| TxProducerError::Decoding("Expected address value".to_string()))
    }

    /// Convert DynSolValue to B256
    pub fn as_fixed_bytes(value: &DynSolValue) -> Result<B256> {
        let (bytes, len) = value
            .as_fixed_bytes()
            .ok_or_else(|| TxProducerError::Decoding("Expected fixed bytes value".to_string()))?;

        if len != 32 {
            return Err(TxProducerError::Decoding(format!("Expected 32 bytes, got {}", len)));
        }

        Ok(B256::from_slice(bytes))
    }

    /// Convert DynSolValue to String
    pub fn as_string(value: &DynSolValue) -> Result<String> {
        value
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| TxProducerError::Decoding("Expected string value".to_string()))
    }

    /// Convert DynSolValue to tuple
    pub fn as_tuple(value: &DynSolValue) -> Result<&[DynSolValue]> {
        value
            .as_tuple()
            .ok_or_else(|| TxProducerError::Decoding("Expected tuple value".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_abi_invalid_path() {
        let result = ContractClient::load_abi("nonexistent.json").await;
        assert!(result.is_err());
    }
}
