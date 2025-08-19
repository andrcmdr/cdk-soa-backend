use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ContractConfig {
    pub address: String,
    pub abi_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(flatten)]
    pub contracts: HashMap<String, ContractConfig>,
    pub ws_provider: String,
    pub db_url: String,
    pub nats_url: String,
    pub nats_bucket: String,
    pub indexing: IndexingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexingConfig {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
