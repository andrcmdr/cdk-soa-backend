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
}

impl ContractAbi {
    pub fn load(name: &str, address_hex: &str, abi_path: &str) -> anyhow::Result<Self> {
        let address = Address::from_str(address_hex)?;

        let path = PathBuf::from(abi_path);
        let json_abi_vec= fs::read(path)?;

        // Preprocess the JSON to add missing anonymous fields
        let preprocessed_json = EventDecoder::preprocess_abi_json_from_vec(&json_abi_vec)?;
        // Safely deserialize with JsonAbi
        let json_abi: JsonAbi = serde_json::from_slice(&preprocessed_json)?;

        Ok(Self {
            name: name.to_string(),
            address,
            abi: json_abi
        })
    }
}
