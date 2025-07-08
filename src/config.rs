use ethers::types::Address;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub ws_provider: String,
    pub db_url: String,
    pub indexing: IndexingConfig,
}

#[derive(Debug, Deserialize)]
pub struct IndexingConfig {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

#[derive(Debug)]
pub struct ContractConfig {
    pub name: String,
    pub address: Address,
    pub abi_path: String,
}

#[derive(Debug, Deserialize)]
struct RawContract {
    address: String,
    abi_path: String,
}

pub fn load_app_config(path: &str) -> eyre::Result<AppConfig> {
    let content = fs::read_to_string(Path::new(path))?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn load_named_contracts(path: &str) -> eyre::Result<Vec<ContractConfig>> {
    let content = fs::read_to_string(Path::new(path))?;
    let raw_map: HashMap<String, RawContract> = toml::from_str(&content)?;

    let mut configs = Vec::new();

    for (name, entry) in raw_map {
        let address = entry.address.parse::<Address>().map_err(|e| {
            eyre::eyre!("Invalid Ethereum address for {}: {}", name, e)
        })?;

        if !Path::new(&entry.abi_path).exists() {
            return Err(eyre::eyre!("ABI file not found for {}: {}", name, entry.abi_path));
        }

        configs.push(ContractConfig {
            name,
            address,
            abi_path: entry.abi_path,
        });
    }

    Ok(configs)
}
