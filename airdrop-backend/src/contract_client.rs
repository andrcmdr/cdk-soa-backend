use anyhow::Result;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::{ProviderBuilder, RootProvider},
    signers::local::PrivateKeySigner,
    transports::http::{Client, Http},
    contract::ContractInstance,
    json_abi::JsonAbi,
};
use alloy_provider::Provider;
use crate::error::{AppError, AppResult};

pub struct ContractClient {
    provider: RootProvider<Http<Client>>,
    contract_address: Address,
    abi: JsonAbi,
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
        abi: JsonAbi,
    ) -> AppResult<Self> {
        let signer: PrivateKeySigner = private_key.parse()
            .map_err(|e| AppError::Blockchain(format!("Invalid private key: {}", e)))?;
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse()
                .map_err(|e| AppError::Blockchain(format!("Invalid RPC URL: {}", e)))?);

        Ok(Self {
            provider,
            contract_address,
            abi,
        })
    }

    pub async fn is_root_hash_exists(&self, root_hash: B256) -> AppResult<bool> {
        let contract = ContractInstance::new(self.contract_address, &self.provider, &self.abi);

        let call = contract.function("isRootHashExists", &[root_hash.into()])
            .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

        let result = call.call().await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let exists: bool = result[0].as_bool()
            .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

        Ok(exists)
    }

    pub async fn submit_trie_update(&self, round_id: u32, root_hash: B256, trie_data: Vec<u8>) -> AppResult<B256> {
        tracing::info!(
            "Submitting trie update for round {} with root hash: 0x{}",
            round_id,
            hex::encode(root_hash)
        );

        let contract = ContractInstance::new(self.contract_address, &self.provider, &self.abi);

        let call = contract.function(
            "updateTrieRoot",
            &[
                U256::from(round_id).into(),
                root_hash.into(),
                trie_data.into(),
            ]
        ).map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

        let receipt = call.send().await
            .map_err(|e| AppError::Blockchain(format!("Transaction failed: {}", e)))?
            .get_receipt().await
            .map_err(|e| AppError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        tracing::info!("Transaction confirmed: 0x{}", hex::encode(receipt.transaction_hash));

        Ok(receipt.transaction_hash)
    }

    pub async fn get_trie_root(&self, round_id: u32) -> AppResult<B256> {
        let contract = ContractInstance::new(self.contract_address, &self.provider, &self.abi);

        let call = contract.function("getTrieRoot", &[U256::from(round_id).into()])
            .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

        let result = call.call().await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let root_hash: B256 = result[0].as_fixed_bytes()
            .map(|bytes| B256::from_slice(bytes))
            .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

        Ok(root_hash)
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256,
        proof: Vec<Vec<u8>>
    ) -> AppResult<bool> {
        let contract = ContractInstance::new(self.contract_address, &self.provider, &self.abi);

        let call = contract.function(
            "verifyEligibility",
            &[
                U256::from(round_id).into(),
                address.into(),
                amount.into(),
                proof.into(),
            ]
        ).map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

        let result = call.call().await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        let is_eligible: bool = result[0].as_bool()
            .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

        Ok(is_eligible)
    }
}
