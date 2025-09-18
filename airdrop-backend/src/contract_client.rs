use anyhow::Result;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::{ProviderBuilder, RootProvider},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
    contract::ContractInstance,
    json_abi::JsonAbi,
    dyn_abi::DynSolValue,
};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};
use crate::config::{ContractInterfaceType, Config};
use crate::error::{AppError, AppResult};

// Inline Solidity interface using sol! macro
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IAirdropContract,
    r#"
    struct RoundMetadata {
        uint256 roundId;
        bytes32 rootHash;
        uint256 totalEligible;
        uint256 totalAmount;
        uint256 startTime;
        uint256 endTime;
        bool isActive;
        string metadataUri;
    }

    interface IAirdropContract {
        // Core trie management functions
        function updateTrieRoot(uint256 roundId, bytes32 rootHash, bytes calldata trieData) external;
        function isRootHashExists(bytes32 rootHash) external view returns (bool);
        function getTrieRoot(uint256 roundId) external view returns (bytes32);

        // Eligibility verification
        function verifyEligibility(
            uint256 roundId,
            address user,
            uint256 amount,
            bytes[] calldata proof
        ) external view returns (bool);

        // Contract metadata functions
        function getContractVersion() external view returns (string memory);
        function getRoundCount() external view returns (uint256);
        function isRoundActive(uint256 roundId) external view returns (bool);
        function getRoundMetadata(uint256 roundId) external view returns (RoundMetadata memory);

        // Events
        event TrieRootUpdated(
            uint256 indexed roundId,
            bytes32 indexed rootHash,
            uint256 totalEligible,
            uint256 totalAmount
        );

        event EligibilityVerified(
            uint256 indexed roundId,
            address indexed user,
            uint256 amount
        );

        event RoundCreated(
            uint256 indexed roundId,
            uint256 startTime,
            uint256 endTime,
            string metadataUri
        );

        event RoundStatusChanged(
            uint256 indexed roundId,
            bool isActive
        );
    }
    "#
);

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

pub enum ContractInterface {
    InlineSol(IAirdropContract::IAirdropContractInstance<Http<Client>, RootProvider<Http<Client>>>),
    JsonAbi {
        instance: ContractInstance<Http<Client>, RootProvider<Http<Client>>>,
        abi: JsonAbi,
    },
}

pub struct ContractClient {
    provider: RootProvider<Http<Client>>,
    contract_address: Address,
    interface: ContractInterface,
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
        config: &Config,
    ) -> AppResult<Self> {
        let signer: PrivateKeySigner = private_key.parse()
            .map_err(|e| AppError::Blockchain(format!("Invalid private key: {}", e)))?;
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse()
                .map_err(|e| AppError::Blockchain(format!("Invalid RPC URL: {}", e)))?);

        let interface = match config.blockchain.contract_interface.interface_type {
            ContractInterfaceType::InlineSol => {
                let contract = IAirdropContract::new(contract_address, &provider);
                ContractInterface::InlineSol(contract)
            }
            ContractInterfaceType::JsonAbi => {
                let abi = config.load_contract_abi().await
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to load ABI: {}", e)))?
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("ABI not found")))?;

                let instance = ContractInstance::new(contract_address, &provider, &abi);
                ContractInterface::JsonAbi { instance, abi }
            }
        };

        Ok(Self {
            provider,
            contract_address,
            interface,
        })
    }

    pub async fn is_root_hash_exists(&self, root_hash: B256) -> AppResult<bool> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .isRootHashExists(root_hash)
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("isRootHashExists", &[root_hash.into()])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let exists: bool = result[0].as_bool()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

                Ok(exists)
            }
        }
    }

    pub async fn submit_trie_update(&self, round_id: u32, root_hash: B256, trie_data: Vec<u8>) -> AppResult<B256> {
        tracing::info!(
            "Submitting trie update for round {} with root hash: 0x{}",
            round_id,
            hex::encode(root_hash)
        );

        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let receipt = contract
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
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function(
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
        }
    }

    pub async fn get_trie_root(&self, round_id: u32) -> AppResult<B256> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .getTrieRoot(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("getTrieRoot", &[U256::from(round_id).into()])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let root_hash: B256 = result[0].as_fixed_bytes()
                    .map(|bytes| B256::from_slice(bytes))
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

                Ok(root_hash)
            }
        }
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256,
        proof: Vec<Vec<u8>>
    ) -> AppResult<bool> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let proof_bytes: Vec<_> = proof.into_iter().map(|p| p.into()).collect();

                let result = contract
                    .verifyEligibility(U256::from(round_id), address, amount, proof_bytes)
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function(
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
    }

    pub async fn get_contract_version(&self) -> AppResult<String> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .getContractVersion()
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("getContractVersion", &[])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let version: String = result[0].as_str()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?
                    .to_string();

                Ok(version)
            }
        }
    }

    pub async fn get_round_count(&self) -> AppResult<U256> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .getRoundCount()
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("getRoundCount", &[])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let count: U256 = result[0].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?
                    .0.into();

                Ok(count)
            }
        }
    }

    pub async fn is_round_active(&self, round_id: u32) -> AppResult<bool> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .isRoundActive(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(result._0)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("isRoundActive", &[U256::from(round_id).into()])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let is_active: bool = result[0].as_bool()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;

                Ok(is_active)
            }
        }
    }

    pub async fn get_round_metadata(&self, round_id: u32) -> AppResult<RoundMetadata> {
        match &self.interface {
            ContractInterface::InlineSol(contract) => {
                let result = contract
                    .getRoundMetadata(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(RoundMetadata {
                    round_id: result._0.roundId,
                    root_hash: result._0.rootHash,
                    total_eligible: result._0.totalEligible,
                    total_amount: result._0.totalAmount,
                    start_time: result._0.startTime,
                    end_time: result._0.endTime,
                    is_active: result._0.isActive,
                    metadata_uri: result._0.metadataUri,
                })
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("getRoundMetadata", &[U256::from(round_id).into()])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                // Parse tuple response
                let tuple = result[0].as_tuple()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format: expected tuple".to_string()))?;

                if tuple.len() != 8 {
                    return Err(AppError::Blockchain("Invalid tuple length".to_string()));
                }

                let round_id = tuple[0].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid roundId format".to_string()))?
                    .0.into();

                let root_hash = tuple[1].as_fixed_bytes()
                    .map(|bytes| B256::from_slice(bytes))
                    .ok_or_else(|| AppError::Blockchain("Invalid rootHash format".to_string()))?;

                let total_eligible = tuple[2].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid totalEligible format".to_string()))?
                    .0.into();

                let total_amount = tuple[3].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid totalAmount format".to_string()))?
                    .0.into();

                let start_time = tuple[4].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid startTime format".to_string()))?
                    .0.into();

                let end_time = tuple[5].as_uint()
                    .ok_or_else(|| AppError::Blockchain("Invalid endTime format".to_string()))?
                    .0.into();

                let is_active = tuple[6].as_bool()
                    .ok_or_else(|| AppError::Blockchain("Invalid isActive format".to_string()))?;

                let metadata_uri = tuple[7].as_str()
                    .ok_or_else(|| AppError::Blockchain("Invalid metadataUri format".to_string()))?
                    .to_string();

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
        }
    }

    pub fn get_contract_address(&self) -> Address {
        self.contract_address
    }

    pub fn get_interface_type(&self) -> &str {
        match &self.interface {
            ContractInterface::InlineSol(_) => "inline_sol",
            ContractInterface::JsonAbi { .. } => "json_abi",
        }
    }
}
