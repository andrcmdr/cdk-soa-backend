use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use alloy_json_abi::JsonAbi;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub blockchain: BlockchainConfig,
    pub aws: AwsConfig,
    pub wallet: WalletConfig,
    pub nats: NatsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub max_upload_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub contract_address: String,
    pub chain_id: u64,
    pub contract_interface: ContractInterfaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInterfaceConfig {
    #[serde(rename = "type")]
    pub interface_type: ContractInterfaceType,
    pub abi_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractInterfaceType {
    JsonAbi,
    InlineSol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    pub region: String,
    pub kms_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub encrypted_private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub object_store: ObjectStoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStoreConfig {
    pub bucket_name: String,
    pub max_object_size: u64,
}

impl Config {
    pub async fn load_from_file(path: &str) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub async fn save_to_file(&self, path: &str) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn load_contract_abi(&self) -> Result<Option<JsonAbi>> {
        match self.blockchain.contract_interface.interface_type {
            ContractInterfaceType::JsonAbi => {
                if let Some(abi_path) = &self.blockchain.contract_interface.abi_path {
                    let abi_content = tokio::fs::read_to_string(abi_path).await?;
                    let abi: JsonAbi = serde_json::from_str(&abi_content)?;
                    Ok(Some(abi))
                } else {
                    Err(anyhow::anyhow!("ABI path is required for json_abi interface type"))
                }
            }
            ContractInterfaceType::InlineSol => {
                // Return None for inline sol usage
                Ok(None)
            }
        }
    }

    pub fn needs_key_generation(&self) -> bool {
        self.wallet.encrypted_private_key.is_empty()
    }

    pub fn set_encrypted_private_key(&mut self, encrypted_key: String) {
        self.wallet.encrypted_private_key = encrypted_key;
    }

    pub fn uses_inline_sol(&self) -> bool {
        matches!(self.blockchain.contract_interface.interface_type, ContractInterfaceType::InlineSol)
    }
}
