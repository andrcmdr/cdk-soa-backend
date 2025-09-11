use anyhow::Result;
use tracing::info;
use alloy::{
    primitives::{Address, U256},
    providers::{ProviderBuilder, RootProvider, fillers::{FillProvider, JoinFill, GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller}, Identity},
    signers::{k256::ecdsa::SigningKey, local::PrivateKeySigner, Signer},
    sol,
    hex,
};
use alloy_network::Ethereum;

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
    }
}

// Type alias to avoid complex type
type Provider = FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider<Ethereum>>;

/// Blockchain client for interacting with the ArtifactManager contract using Alloy
pub struct BlockchainClient {
    provider: Provider,
    wallet: PrivateKeySigner,
    contract_address: Address,
}

impl BlockchainClient {
    pub async fn new(
        rpc_url: String,
        private_key: String,
        contract_address: Address,
        chain_id: u64,
    ) -> Result<Self> {
        // Create provider
        let provider = ProviderBuilder::new().connect(&rpc_url).await?;

        // Create wallet from private key
        let private_key_bytes = hex::decode(private_key.trim_start_matches("0x"))?;
        let signing_key = SigningKey::from_slice(&private_key_bytes)?;
        let wallet = PrivateKeySigner::from(signing_key).with_chain_id(Some(chain_id));
        
        
        Ok(Self {
            provider,
            wallet,
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
            "BlockchainClient: Submitting batch revenue report for {} artifacts",
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
        
        info!("BlockchainClient: Batch revenue report submitted with tx hash: {:?}", tx_hash);
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
            "BlockchainClient: Submitting batch usage report for {} artifacts",
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
        
        info!("BlockchainClient: Batch usage report submitted with tx hash: {:?}", tx_hash);
        Ok(tx_hash)
    }

    /// Get the contract address
    pub fn contract_address(&self) -> Address {
        self.contract_address
    }

    /// Get the wallet address
    pub fn wallet_address(&self) -> Address {
        self.wallet.address()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_blockchain_client_creation() {
        // Test implementation would go here
    }
}