use anyhow::Result;
use alloy::{
    contract::{ContractInstance, Interface as ContractAbi},
    dyn_abi::DynSolValue,
    json_abi::JsonAbi,
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::{ProviderBuilder, RootProvider, Identity},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
    transports::ws::WsConnect,
};
use alloy_provider::Provider;
use alloy_provider::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::config::{ContractInterfaceType, Config};
use crate::error::{AppError, AppResult};

type RPCProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>
    >,
    RootProvider
>;

// Inline Solidity interface using sol! macro (block form)
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IAirdropContract {
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
}

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
    InlineSol(IAirdropContract::IAirdropContractInstance<RPCProvider>),
    JsonAbi {
        instance: ContractInstance<RPCProvider>,
        abi: JsonAbi,
    },
}

pub struct ContractClient {
    provider: RPCProvider,
    contract_address: Address,
    interface: ContractInterface,
}

/// Build HTTP and WS providers using Alloy
pub async fn build_providers(
    ws_rpc_url: WsConnect,
    http_rpc_url: reqwest::Url
) -> anyhow::Result<(RPCProvider, RPCProvider)> {
    let ws_rpc_provider = ProviderBuilder::new().connect_ws(ws_rpc_url.clone()).await?;
    let http_rpc_provider = ProviderBuilder::new().connect_http(http_rpc_url.clone());
    info!("Connecting to RPC endpoints: ws: {:?}, http: {:?}", ws_rpc_url, http_rpc_url);

    Ok((ws_rpc_provider, http_rpc_provider))
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
        config: &Config,
    ) -> AppResult<Self> {
        // Keep key parsing to preserve signature; not attached to provider to keep RPCProvider type.
        let _signer: PrivateKeySigner = private_key
            .parse()
            .map_err(|e| AppError::Blockchain(format!("Invalid private key: {}", e)))?;
        let _wallet = EthereumWallet::from(_signer);

        // Use new connect_http() API
        let http_url: reqwest::Url = rpc_url
            .parse()
            .map_err(|e| AppError::Blockchain(format!("Invalid RPC URL: {}", e)))?;
        let provider: RPCProvider = ProviderBuilder::new().connect_http(http_url);

        let interface = match config.blockchain.contract_interface.interface_type {
            ContractInterfaceType::InlineSol => {
                let contract = IAirdropContract::new(contract_address, provider.clone());
                ContractInterface::InlineSol(contract)
            }
            ContractInterfaceType::JsonAbi => {
                let abi = config
                    .load_contract_abi()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to load ABI: {}", e))
                    })?
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("ABI not found")))?;

                // ContractInstance expects alloy_contract::Interface
                let iface = ContractAbi::new(abi.clone());
                let instance: ContractInstance<RPCProvider> =
                    ContractInstance::new(contract_address, provider.clone(), iface);
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
                let exists: bool = contract
                    .isRootHashExists(root_hash)
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(exists)
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
                let root: B256 = contract
                    .getTrieRoot(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(root)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                let call = instance.function("getTrieRoot", &[U256::from(round_id).into()])
                    .map_err(|e| AppError::Blockchain(format!("Failed to create contract call: {}", e)))?;

                let result = call.call().await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                let (bytes, _len) = result[0].as_fixed_bytes()
                    .ok_or_else(|| AppError::Blockchain("Invalid response format".to_string()))?;
                let root_hash = B256::from_slice(bytes);

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

                let is_ok: bool = contract
                    .verifyEligibility(U256::from(round_id), address, amount, proof_bytes)
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(is_ok)
            }
            ContractInterface::JsonAbi { instance, .. } => {
                // Convert each Vec<u8> to DynSolValue first, then collect into Vec<DynSolValue>
                let proof_values: Vec<DynSolValue> = proof.into_iter()
                    .map(|p| p.into())
                    .collect();

                let call = instance.function(
                    "verifyEligibility",
                    &[
                        U256::from(round_id).into(),
                        address.into(),
                        amount.into(),
                        proof_values.into(),
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
                let version: String = contract
                    .getContractVersion()
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(version)
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
                let count: U256 = contract
                    .getRoundCount()
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(count)
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
                let is_active: bool = contract
                    .isRoundActive(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;
                Ok(is_active)
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
                let rm: IAirdropContract::RoundMetadata = contract
                    .getRoundMetadata(U256::from(round_id))
                    .call()
                    .await
                    .map_err(|e| AppError::Blockchain(format!("Contract call failed: {}", e)))?;

                Ok(RoundMetadata {
                    round_id: rm.roundId,
                    root_hash: rm.rootHash,
                    total_eligible: rm.totalEligible,
                    total_amount: rm.totalAmount,
                    start_time: rm.startTime,
                    end_time: rm.endTime,
                    is_active: rm.isActive,
                    metadata_uri: rm.metadataUri,
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

                let (bytes, _len) = tuple[1].as_fixed_bytes()
                    .ok_or_else(|| AppError::Blockchain("Invalid rootHash format".to_string()))?;
                let root_hash = B256::from_slice(bytes);

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
