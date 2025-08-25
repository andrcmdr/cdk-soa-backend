use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use alloy_json_abi::JsonAbi;
use alloy_primitives::Address;

#[derive(Clone)]
pub struct ContractAbi {
    pub name: String,
    pub address: Address,
    pub abi: JsonAbi,
}

impl ContractAbi {
    pub fn load(name: &str, address_hex: &str, abi_path: &str) -> anyhow::Result<Self> {
        let address = Address::from_str(address_hex)?;
        let path = PathBuf::from(abi_path);
        let contents = fs::read(path)?;
        let abi: JsonAbi = serde_json::from_slice(&contents)?;

        Ok(Self { name: name.to_string(), address, abi })
    }
}
