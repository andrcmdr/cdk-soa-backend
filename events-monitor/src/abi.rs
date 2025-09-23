use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use alloy_json_abi::JsonAbi;
use alloy_primitives::Address;
use crate::event_decoder::EventDecoder;
use crate::config::FlattenedContract;

#[derive(Clone)]
pub struct ContractAbi {
    pub name: String,
    pub address: Address,
    pub abi: JsonAbi,
    pub parent_contract_name: Option<String>,
    pub parent_contract_address: Option<Address>,
    pub is_implementation: bool,
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
            abi: json_abi,
            parent_contract_name: None,
            parent_contract_address: None,
            is_implementation: false,
        })
    }

    pub fn from_flattened(flattened: &FlattenedContract) -> anyhow::Result<Self> {
        let address = Address::from_str(&flattened.address)?;
        let parent_address = if let Some(parent_addr) = &flattened.parent_contract_address {
            Some(Address::from_str(parent_addr)?)
        } else {
            None
        };

        let path = PathBuf::from(&flattened.abi_path);
        let json_abi_vec= fs::read(path)?;

        // Preprocess the JSON to add missing anonymous fields
        let preprocessed_json = EventDecoder::preprocess_abi_json_from_vec(&json_abi_vec)?;
        // Safely deserialize with JsonAbi
        let json_abi: JsonAbi = serde_json::from_slice(&preprocessed_json)?;

        Ok(Self {
            name: flattened.name.clone(),
            address,
            abi: json_abi,
            parent_contract_name: flattened.parent_contract_name.clone(),
            parent_contract_address: parent_address,
            is_implementation: flattened.is_implementation,
        })
    }
}
