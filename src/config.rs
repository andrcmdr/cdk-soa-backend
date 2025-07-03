use ethers::types::Address;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

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
