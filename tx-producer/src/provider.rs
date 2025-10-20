//! Provider configuration and management

use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_provider::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
};
use alloy_network::EthereumWallet;
use alloy_signers::local::PrivateKeySigner;
use alloy_transport_http::Http;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{TxProducerError, Result};

/// Provider type with all necessary fillers
pub type TxProvider = FillProvider<
    JoinFill<
        alloy_provider::Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>
    >,
    RootProvider<Http<Client>>
>;

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// RPC endpoint URL (HTTP)
    pub rpc_url: String,
    /// Chain ID
    pub chain_id: u64,
    /// Optional timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    30
}

/// Provider builder and manager
#[derive(Clone)]
pub struct ProviderManager {
    config: ProviderConfig,
    provider: Arc<TxProvider>,
    wallet: Option<Arc<EthereumWallet>>,
}

impl ProviderManager {
    /// Create a new provider manager
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let http_url: reqwest::Url = config.rpc_url
            .parse()
            .map_err(|e| TxProducerError::Configuration(format!("Invalid RPC URL: {}", e)))?;

        let provider = ProviderBuilder::new()
            .connect_http(http_url);

        Ok(Self {
            config,
            provider: Arc::new(provider),
            wallet: None,
        })
    }

    /// Add a signer to the provider
    pub fn with_signer(mut self, private_key: &str) -> Result<Self> {
        let signer: PrivateKeySigner = private_key
            .parse()
            .map_err(|e| TxProducerError::Configuration(format!("Invalid private key: {}", e)))?;

        let wallet = EthereumWallet::from(signer);
        self.wallet = Some(Arc::new(wallet));

        Ok(self)
    }

    /// Get the provider
    pub fn provider(&self) -> Arc<TxProvider> {
        Arc::clone(&self.provider)
    }

    /// Get the wallet (if configured)
    pub fn wallet(&self) -> Option<Arc<EthereumWallet>> {
        self.wallet.as_ref().map(Arc::clone)
    }

    /// Get chain ID
    pub fn chain_id(&self) -> u64 {
        self.config.chain_id
    }

    /// Get provider configuration
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Check connection to the RPC endpoint
    pub async fn check_connection(&self) -> Result<u64> {
        let block_number = self.provider
            .get_block_number()
            .await
            .map_err(|e| TxProducerError::Provider(format!("Failed to get block number: {}", e)))?;

        Ok(block_number)
    }

    /// Get signer address (if wallet is configured)
    pub fn signer_address(&self) -> Option<Address> {
        self.wallet.as_ref().map(|w| w.default_signer().address())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            timeout_seconds: default_timeout(),
        };

        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_provider_manager_creation() {
        let config = ProviderConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            timeout_seconds: 30,
        };

        let manager = ProviderManager::new(config);
        assert!(manager.is_ok());
    }
}
