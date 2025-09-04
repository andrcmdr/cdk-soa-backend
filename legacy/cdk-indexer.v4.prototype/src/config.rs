use alloy::primitives::Address;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct NatsCfg {
    pub url: String,
    pub subject: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresCfg {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractCfg {
    pub name: String,
    pub address: String,
    pub abi_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub rpc_http_url: String,
    pub rpc_ws_url: String,
    pub nats: Option<NatsCfg>,
    pub postgres: PostgresCfg,
    pub contracts: Vec<ContractCfg>,
    pub from_block: Option<String>,
}

impl AppConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let s = fs::read_to_string(path)?;
        let cfg: Self = serde_yaml::from_str(&s)?;
        Ok(cfg)
    }

    pub fn addresses_map(&self) -> anyhow::Result<HashMap<Address, String>> {
        let mut m = HashMap::new();
        for c in &self.contracts {
            let addr: Address = c.address.parse()?;
            m.insert(addr, c.name.clone());
        }
        Ok(m)
    }
}
