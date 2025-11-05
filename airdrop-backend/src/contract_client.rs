use anyhow::Result;
use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

// Import the new tx-producer library
use tx_producer::prelude::*;

use crate::config::Config;
use crate::error::{AppError, AppResult};

// Re-export for compatibility
pub use tx_producer::{ProviderManager as TxProviderManager, ContractClient as TxContractClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundMetadata {
    pub round_id: U256,
    pub root_hash: B256,
    pub total_eligible: U256,
    pub total_amount: U256,
    pub start_time: U256,
    pub end_time: U256,
    pub is_active: bool,
    pub metadata_uri: String,
}

/// Wrapper around the universal contract client
pub struct ContractClient {
    inner: TxContractClient,
    contract_address: Address,
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
        config: &Config,
    ) -> AppResult<Self> {
        // Configure provider
        let provider_config = ProviderConfig {
            rpc_url: rpc_url.to_string(),
            chain_id: config.blockchain.chain_id,
            timeout_seconds: 30,
        };

        // Create provider manager with signer
        let provider_manager = ProviderManager::new(provider_config)
            .map_err(|e| AppError::Blockchain(format!("Failed to create provider: {}", e)))?
            .with_signer(private_key)
            .map_err(|e| AppError::Blockchain(format!("Failed to add signer: {}", e)))?;

        // Get ABI path
        let abi_path = config.blockchain.contract_interface.abi_path
            .as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("ABI path is required")))?
            .clone();

        // Configure contract
        let contract_config = ContractConfig {
            address: contract_address,
            abi_path,
        };

        // Create contract client
        let inner = TxContractClient::new(
            contract_config,
            Arc::new(provider_manager),
        )
        .await
        .map_err(|e| AppError::Blockchain(format!("Failed to create contract client: {}", e)))?;

        info!("Contract client initialized for address: {}", contract_address);

        Ok(Self {
            inner,
            contract_address,
        })
    }

    pub async fn is_root_hash_exists(&self, root_hash: B256) -> AppResult<bool> {
        let result = self.inner
            .call_function("isRootHashExists", &[root_hash.into()])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let exists = value_helpers::as_bool(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(exists)
    }

    pub async fn submit_trie_update(&self, round_id: u32, root_hash: B256, trie_data: Vec<u8>) -> AppResult<B256> {
        tracing::info!(
            "Submitting trie update for round {} with root hash: 0x{}",
            round_id,
            hex::encode(root_hash)
        );

        let tx_hash = self.inner
            .send_transaction(
                "updateTrieRoot",
                &[
                    U256::from(round_id).into(),
                    root_hash.into(),
                    trie_data.into(),
                ]
            )
            .await
            .map_err(|e| AppError::Blockchain(format!("Transaction failed: {}", e)))?;

        tracing::info!("Transaction confirmed: 0x{}", hex::encode(tx_hash));
        Ok(tx_hash)
    }

    pub async fn get_trie_root(&self, round_id: u32) -> AppResult<B256> {
        let result = self.inner
            .call_function("getTrieRoot", &[U256::from(round_id).into()])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let root_hash = value_helpers::as_fixed_bytes(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(root_hash)
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256,
        proof: Vec<Vec<u8>>
    ) -> AppResult<bool> {
        let proof_values: Vec<DynSolValue> = proof.into_iter()
            .map(|p| p.into())
            .collect();

        let result = self.inner
            .call_function(
                "verifyEligibility",
                &[
                    U256::from(round_id).into(),
                    address.into(),
                    amount.into(),
                    proof_values.into(),
                ]
            )
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let is_eligible = value_helpers::as_bool(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(is_eligible)
    }

    pub async fn get_contract_version(&self) -> AppResult<String> {
        let result = self.inner
            .call_function("getContractVersion", &[])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let version = value_helpers::as_string(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(version)
    }

    pub async fn get_round_count(&self) -> AppResult<U256> {
        let result = self.inner
            .call_function("getRoundCount", &[])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let count = value_helpers::as_uint(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(count)
    }

    pub async fn is_round_active(&self, round_id: u32) -> AppResult<bool> {
        let result = self.inner
            .call_function("isRoundActive", &[U256::from(round_id).into()])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let is_active = value_helpers::as_bool(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(is_active)
    }

    pub async fn get_round_metadata(&self, round_id: u32) -> AppResult<RoundMetadata> {
        let result = self.inner
            .call_function("getRoundMetadata", &[U256::from(round_id).into()])
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let tuple = value_helpers::as_tuple(&result[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        if tuple.len() != 8 {
            return Err(AppError::Blockchain("Invalid tuple length".to_string()));
        }

        let round_id = value_helpers::as_uint(&tuple[0])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let root_hash = value_helpers::as_fixed_bytes(&tuple[1])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let total_eligible = value_helpers::as_uint(&tuple[2])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let total_amount = value_helpers::as_uint(&tuple[3])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let start_time = value_helpers::as_uint(&tuple[4])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let end_time = value_helpers::as_uint(&tuple[5])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let is_active = value_helpers::as_bool(&tuple[6])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;
        let metadata_uri = value_helpers::as_string(&tuple[7])
            .map_err(|e| AppError::Blockchain(format!("Invalid response format: {}", e)))?;

        Ok(RoundMetadata {
            round_id,
            root_hash,
            total_eligible,
            total_amount,
            start_time,
            end_time,
            is_active,
            metadata_uri,
        })
    }

    pub fn get_contract_address(&self) -> Address {
        self.contract_address
    }

    pub fn get_interface_type(&self) -> &str {
        "universal_abi"
    }
}
