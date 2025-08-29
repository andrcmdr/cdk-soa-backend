use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use alloy_json_abi::JsonAbi;
use alloy_primitives::Address;
use serde_json::Value;
use crate::event_decoder::EventDecoder;

#[derive(Clone)]
pub struct ContractAbi {
    pub name: String,
    pub address: Address,
    pub abi: JsonAbi,
    pub json_abi: Value,
}

impl ContractAbi {
    pub fn load(name: &str, address_hex: &str, abi_path: &str) -> anyhow::Result<Self> {
        let address = Address::from_str(address_hex)?;
        let path = PathBuf::from(abi_path);
        let json_abi_vec= fs::read(path)?;
        let json_abi: Value = serde_json::from_slice(&json_abi_vec)?;
        let abi: JsonAbi = serde_json::from_slice(&json_abi_vec)?;

        Ok(Self { name: name.to_string(), address, abi, json_abi })
    }
}
