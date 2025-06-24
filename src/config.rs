use ethers::types::Address;
use std::collections::HashMap;

pub struct ContractConfig {
    pub address: Address,
    pub abi_path: &'static str,
}

pub fn load_contracts() -> Vec<ContractConfig> {
    vec![
        ContractConfig {
            address: "0xabc123abc123abc123abc123abc123abc123abc1".parse().unwrap(),
            abi_path: "./abis/SimpleEvent.json",
        },
    ]
}
