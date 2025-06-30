use ethers::types::Address;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct RawContract {
    pub address: String,
    pub abi_path: String,
}

#[derive(Debug)]
pub struct ContractConfig {
    pub address: Address,
    pub abi_path: String,
}

#[derive(Debug, Deserialize)]
struct ContractsToml {
    contracts: Vec<RawContract>,
}

pub fn load_contracts_from_file(path: &str) -> eyre::Result<Vec<ContractConfig>> {
    let content = fs::read_to_string(Path::new(path))?;
    let raw: ContractsToml = toml::from_str(&content)?;

    let mut configs = Vec::new();
    for entry in raw.contracts {
        let address = entry.address.parse::<Address>().map_err(|e| {
            eyre::eyre!("Invalid Ethereum address '{}': {}", entry.address, e)
        })?;

        if !Path::new(&entry.abi_path).exists() {
            return Err(eyre::eyre!("ABI file not found: {}", entry.abi_path));
        }

        configs.push(ContractConfig {
            address,
            abi_path: entry.abi_path,
        });
    }

    Ok(configs)
}
