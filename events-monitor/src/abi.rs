use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use alloy_json_abi::JsonAbi;
use alloy_primitives::Address;
use tracing::error;
use crate::event_decoder::EventDecoder;
use crate::config::ContractWithImplementation;

#[derive(Clone)]
pub struct ContractAbi {
    pub name: String,
    pub address: Address,
    pub abi: JsonAbi,
    pub implementation_name: Option<String>,
    pub implementation_address: Option<Address>,
    pub parent_contract_name: Option<String>,
    pub parent_contract_address: Option<Address>,
}

impl ContractAbi {
    pub fn load(name: &str, address_hex: &str, abi_path: &str) -> anyhow::Result<Self> {
        let address = Address::from_str(address_hex)?;

        let path = PathBuf::from(abi_path);
        let json_abi_vec = fs::read(path.clone()).unwrap_or_else(
            |e| {
                error!("Contract name: {:?}; Contract address: {:?}; ABI file: {:?}; Read error: {:?}", name, address, path, e);
                eprintln!("Contract information: {:?}; Contract address: {:?}; ABI file: {:?}; Read error: {:?}", name, address, path, e);
                vec![]
            }
        );

        // Preprocess the JSON to add missing anonymous fields
        let preprocessed_json = EventDecoder::preprocess_abi_json_from_vec(&json_abi_vec)?;
        // Safely deserialize with JsonAbi
        let json_abi: JsonAbi = serde_json::from_slice(&preprocessed_json)?;

        Ok(Self {
            name: name.to_string(),
            address,
            abi: json_abi,
            implementation_name: None,
            implementation_address: None,
            parent_contract_name: None,
            parent_contract_address: None,
        })
    }

    pub fn from_contract_with_implementation(contract_info: &ContractWithImplementation) -> anyhow::Result<Self> {
        let address = Address::from_str(&contract_info.address)?;
        let parent_address = contract_info.parent_contract_address
            .as_ref()
            .map(|addr| Address::from_str(addr))
            .transpose()?;

        let path = PathBuf::from(&contract_info.abi_path);
        let json_abi_vec = fs::read(path.clone()).unwrap_or_else(
            |e| {
                error!("Contract information: {:?}; ABI file: {:?}; Read error: {:?}", contract_info, path, e);
                eprintln!("Contract information: {:?}; ABI file: {:?}; Read error: {:?}", contract_info, path, e);
                vec![]
            }
        );

        // Preprocess the JSON to add missing anonymous fields
        let preprocessed_json = EventDecoder::preprocess_abi_json_from_vec(&json_abi_vec)?;
        // Safely deserialize with JsonAbi
        let json_abi: JsonAbi = serde_json::from_slice(&preprocessed_json)?;

        Ok(Self {
            name: contract_info.name.clone(),
            address,
            abi: json_abi,
            implementation_name: Some(contract_info.name.clone()),
            implementation_address: Some(address),
            parent_contract_name: contract_info.parent_contract_name.clone(),
            parent_contract_address: parent_address,
        })
    }

    /// Check if this contract represents an implementation
    pub fn is_implementation(&self) -> bool {
        self.parent_contract_name.is_some()
    }

    /// Get the effective contract name (parent if this is an implementation, self otherwise)
    pub fn get_effective_contract_name(&self) -> &str {
        self.parent_contract_name.as_ref().unwrap_or(&self.name)
    }

    /// Get the effective contract address (parent if this is an implementation, self otherwise)
    pub fn get_effective_contract_address(&self) -> Address {
        self.parent_contract_address.unwrap_or(self.address)
    }
}
