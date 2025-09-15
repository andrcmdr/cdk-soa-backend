use alloy_provider::WalletProvider;
use anyhow::Result;
use tracing::info;
use alloy::{
    network::{EthereumWallet}, 
    primitives::{Address, U256}, 
    providers::{Identity, ProviderBuilder, RootProvider}, 
    signers::{local::PrivateKeySigner},
    sol
};
// use alloy_network::Ethereum;
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller};


type RPCProvider = FillProvider<JoinFill<JoinFill< JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, ChainIdFiller>, WalletFiller<EthereumWallet>>, RootProvider>;



// Define the contract interface using sol! macro
sol! {
    /// @title ArtifactManager
    /// @notice Manages artifacts in the system, including their registration, deprecation, and associated data
    #[sol(rpc)]
    contract ArtifactManager {
        // Events
        event ArtifactBatchRevenueReported(
            address[] indexed artifacts,
            uint256[] revenues,
            uint256[] timestamps
        );
        
        event ArtifactBatchUsageReported(
            address[] indexed artifacts,
            uint256[] usages,
            uint256[] timestamps
        );

        // Functions needed for this service
        function batchReportArtifactRevenue(
            address[] calldata _artifacts,
            uint256[] calldata revenues,
            uint256[] calldata timestamps
        ) external;

        function batchReportArtifactUsage(
            address[] calldata _artifacts,
            uint256[] calldata usages,
            uint256[] calldata timestamps
        ) external;

        function isArtifactActive(address _artifact) external view returns (bool);

        function getArtifactCount() external view returns (uint256);

    }
}


/// Blockchain client for interacting with the ArtifactManager contract using Alloy
pub struct ContractClient {
    provider: RPCProvider,
    contract_address: Address,
}

impl ContractClient {
    pub async fn new(
        http_rpc_url: String,
        private_key: String,
        contract_address: Address,
        chain_id: u64,
    ) -> Result<Self> {
        // Create provider

        // Create wallet from private key
        let signer: PrivateKeySigner = private_key.parse()
            .map_err(|e| anyhow::anyhow!(format!("Invalid private key: {}", e)))?;
        let wallet = EthereumWallet::from(signer);


        let provider = ProviderBuilder::new().with_chain_id(chain_id).wallet(wallet).connect_http(http_rpc_url.parse()?);
        
        Ok(Self {
            provider,
            contract_address,
        })
    }

    /// Submit batch revenue reports to the blockchain
    /// This calls the batchReportArtifactRevenue function on the ArtifactManager contract
    pub async fn batch_report_artifact_revenue(
        &self,
        artifacts: Vec<Address>,
        revenues: Vec<U256>,
        timestamps: Vec<U256>,
    ) -> Result<alloy::primitives::TxHash> {
        info!(
            "ContractClient: Submitting batch revenue report for {} artifacts",
            artifacts.len()
        );

        // Validate input arrays have same length
        if artifacts.len() != revenues.len() || artifacts.len() != timestamps.len() {
            return Err(anyhow::anyhow!("Array length mismatch"));
        }

        // Create contract instance and call the function
        let contract = ArtifactManager::new(self.contract_address, &self.provider);
        let call = contract.batchReportArtifactRevenue(artifacts, revenues, timestamps);
        let pending_tx = call.send().await?;
        let tx_hash = *pending_tx.tx_hash();
        
        info!("ContractClient: Batch revenue report submitted with tx hash: {:?}", tx_hash);
        Ok(tx_hash)
    }

    /// Submit batch usage reports to the blockchain
    pub async fn batch_report_artifact_usage(
        &self,
        artifacts: Vec<Address>,
        usages: Vec<U256>,
        timestamps: Vec<U256>,
    ) -> Result<alloy::primitives::TxHash> {
        info!(
            "ContractClient: Submitting batch usage report for {} artifacts",
            artifacts.len()
        );

        // Validate input arrays have same length
        if artifacts.len() != usages.len() || artifacts.len() != timestamps.len() {
            return Err(anyhow::anyhow!("Array length mismatch"));
        }

        // Create contract instance and call the function
        let contract = ArtifactManager::new(self.contract_address, &self.provider);
        let call = contract.batchReportArtifactUsage(artifacts, usages, timestamps);
        let pending_tx = call.send().await?;
        let tx_hash = *pending_tx.tx_hash();
        
        info!("ContractClient: Batch usage report submitted with tx hash: {:?}", tx_hash);
        Ok(tx_hash)
    }

    /// Get the contract address
    pub fn contract_address(&self) -> Address {
        self.contract_address
    }

    /// Get the wallet address
    pub fn wallet_address(&self) -> Address {
        self.provider.wallet().default_signer().address()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use alloy::primitives::{Address, U256};
    use crate::transaction::{ContractClient, ArtifactManager};
    use tracing::info;
    use std::str::FromStr;
    use alloy::primitives::hex;
    use alloy::contract::Error as ContractError;

    #[tokio::test]
    async fn test_function_call() {
        // empty test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init();
            
        // Load environment variables from .env file
        dotenv::dotenv().ok();
        
        // read the private key from the .env file
        let private_key = std::env::var("PRIVATE_KEY").unwrap();
        let rpc_url = std::env::var("RPC_URL").unwrap();
        let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
        let chain_id = std::env::var("CHAIN_ID").unwrap();

        let contract_addr = Address::from_str(&contract_address).unwrap();
        let chain_id_num = chain_id.parse::<u64>().unwrap();
        let client = ContractClient::new(rpc_url, private_key, contract_addr, chain_id_num).await.unwrap();

        // Debug: Print wallet address to check permissions
        let wallet_addr = client.wallet_address();
        info!("Wallet address: {:?}", wallet_addr);
        info!("Wallet address (checksum): {:?}", wallet_addr.to_checksum(Some(chain_id_num)));
        info!("Contract address: {:?}", client.contract_address());
        
        // Try a simple view function first to test connection
        let contract = ArtifactManager::new(contract_addr, &client.provider);
        match contract.getArtifactCount().call().await {
            Ok(count) => {
                info!("Artifact count: {:?}", count);
                
            },
            Err(e) => {
                info!("Failed to call getArtifactCount: {:?}", e);
                return; // Exit early if we can't even call view functions
            }
        }
    }

    #[tokio::test]
    async fn test_revenue_submission() {
        // empty test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init();

            dotenv::dotenv().ok();
        
            // read the private key from the .env file
            let private_key = std::env::var("PRIVATE_KEY").unwrap();
            let rpc_url = std::env::var("RPC_URL").unwrap();
            let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
            let chain_id = std::env::var("CHAIN_ID").unwrap();
        
            let contract_addr = Address::from_str(&contract_address).unwrap();
            let chain_id_num = chain_id.parse::<u64>().unwrap();
            let client = ContractClient::new(rpc_url, private_key, contract_addr, chain_id_num).await.unwrap();

            let artifact_address = std::env::var("ARTIFACT_ADDRESS").unwrap();
            let artifact_address = Address::from_str(&artifact_address).unwrap();
            info!("Artifact address: {:?}", artifact_address);

            let contract = ArtifactManager::new(contract_addr, &client.provider);
            let call = contract.isArtifactActive(artifact_address);

            match call.call().await {
                Ok(_) => {
                    info!("Artifact is active");
                }
                Err(e) => {
                    if let Some(data) = e.as_revert_data() {
                        info!("Revert data: 0x{}", alloy::primitives::hex::encode(data));
                    } else {
                        info!("Other error: {e:?}");
                    }
                    return;
                }
            }

            let artifacts = vec![artifact_address];
            let revenues = vec![U256::from(100)];
            let timestamp = Utc::now().timestamp() - 60;
            let timestamps = vec![U256::from(timestamp)];
            match client.batch_report_artifact_revenue(artifacts, revenues, timestamps).await {
                Ok(tx_hash) => {
                    info!("ContractClient: Batch revenue report submitted with tx hash: {:?}", tx_hash);
                    assert!(!tx_hash.is_zero());
                },
                Err(e) => {
                    info!("Failed to submit revenue report: {:?}", e);
                }
            }


            
    }
}