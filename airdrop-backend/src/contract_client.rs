use anyhow::Result;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::{ProviderBuilder, RootProvider},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
    contract::ContractInstance,
};
use alloy_provider::Provider;
use crate::error::{AppError, AppResult};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    AirdropContract,
    r#"[
        {
            "inputs": [
                {"internalType": "uint256", "name": "roundId", "type": "uint256"},
                {"internalType": "bytes32", "name": "rootHash", "type": "bytes32"},
                {"internalType": "bytes", "name": "trieData", "type": "bytes"}
            ],
            "name": "updateTrieRoot",
            "outputs": [],
            "stateMutability": "nonpayable",
            "type": "function"
        },
        {
            "inputs": [{"internalType": "bytes32", "name": "rootHash", "type": "bytes32"}],
            "name": "isRootHashExists",
            "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [{"internalType": "uint256", "name": "roundId", "type": "uint256"}],
            "name": "getTrieRoot",
            "outputs": [{"internalType": "bytes32", "name": "", "type": "bytes32"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [
                {"internalType": "uint256", "name": "roundId", "type": "uint256"},
                {"internalType": "address", "name": "user", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"},
                {"internalType": "bytes[]", "name": "proof", "type": "bytes[]"}
            ],
            "name": "verifyEligibility",
            "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
            "stateMutability": "view",
            "type": "function"
        }
    ]"#
);

pub struct ContractClient {
    provider: RootProvider<Http<Client>>,
    contract: ContractInstance<Http<Client>, RootProvider<Http<Client>>, AirdropContract::AirdropContractInstance>,
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
    ) -> AppResult<Self> {
        let signer: PrivateKeySigner = private_key.parse()
            .map_err(|e| AppError::Blockchain(format!("Invalid private key: {}", e)))?;
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse()
                .map_err(|e| AppError::Blockchain(format!("Invalid RPC URL: {}", e)))?);

        let contract = AirdropContract::new(contract_address, &provider);

        Ok(Self {
            provider,
            contract,
        })
    }

    pub async fn is_root_hash_exists(&self, root_hash: B256) -> AppResult<bool> {
        let result = self.contract
            .isRootHashExists(root_hash)
            .call()
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        Ok(result._0)
    }

    pub async fn submit_trie_update(&self, round_id: u32, root_hash: B256, trie_data: Vec<u8>) -> AppResult<B256> {
        tracing::info!(
            "Submitting trie update for round {} with root hash: 0x{}",
            round_id,
            hex::encode(root_hash)
        );

        let receipt = self.contract
            .updateTrieRoot(U256::from(round_id), root_hash, trie_data.into())
            .send()
            .await
            .map_err(|e| AppError::Blockchain(format!("Transaction failed: {}", e)))?
            .get_receipt()
            .await
            .map_err(|e| AppError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        tracing::info!("Transaction confirmed: 0x{}", hex::encode(receipt.transaction_hash));

        Ok(receipt.transaction_hash)
    }

    pub async fn get_trie_root(&self, round_id: u32) -> AppResult<B256> {
        let result = self.contract
            .getTrieRoot(U256::from(round_id))
            .call()
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        Ok(result._0)
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256,
        proof: Vec<Vec<u8>>
    ) -> AppResult<bool> {
        let proof_bytes: Vec<_> = proof.into_iter().map(|p| p.into()).collect();

        let result = self.contract
            .verifyEligibility(U256::from(round_id), address, amount, proof_bytes)
            .call()
            .await
            .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

        Ok(result._0)
    }
}
