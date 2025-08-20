use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ChainCfg {
    pub http_rpc_url: String,
    pub ws_rpc_url: String,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexingCfg {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PgCfg {
    pub dsn: String,
    pub schema: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NatsCfg {
    pub url: String,
    pub object_store_bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractCfg {
    pub name: String,
    pub address: String,
    pub abi_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppCfg {
    pub chain: ChainCfg,
    pub indexing: IndexingCfg,
    pub postgres: PgCfg,
    pub nats: NatsCfg,
    pub contracts: Vec<ContractCfg>,
}

impl AppCfg {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        Ok(serde_yaml::from_str(&std::fs::read_to_string(path)?)?)
    }
}
