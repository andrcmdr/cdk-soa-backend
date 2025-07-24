use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ContractConfig {
    pub address: String,
    pub abi_path: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(flatten)]
    pub contracts: HashMap<String, ContractConfig>,
    pub ws_provider: String,
    pub db_url: String,
    pub nats_url: String,
    pub nats_bucket: String,
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
