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

    // Historical blocks processing
    pub historical_blocks_processing: Option<u8>,
    pub blocks_sync_protocol: Option<String>, // "http" or "ws"
    pub blocks_chunk_size: Option<u64>, // Number of blocks to fetch in each chunk. Defaults to 100 if not specified.
    pub full_blocks_historical: Option<bool>, // true for full blocks, false for headers only

    // New blocks subscription
    pub new_blocks_subscription: Option<u8>,
    pub new_blocks_subscription_protocol: Option<String>, // "ws" or "http"
    pub http_polling_interval_secs: Option<u64>, // Polling interval in seconds for HTTP RPC
    pub full_blocks_subscription: Option<bool>, // true for full blocks, false for headers only

    // Transaction filtering
    pub filter_senders: Option<Vec<String>>,
    pub filter_receivers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PgCfg {
    pub dsn: String,
    pub schema: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AwsRdsCfg {
    pub enabled: Option<u8>,
    pub endpoint: String,
    pub port: Option<u16>,
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub region: Option<String>,
    pub ssl_mode: Option<String>,
    pub connection_timeout: Option<u64>,
    pub max_connections: Option<u32>,
    pub schema: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NatsCfg {
    pub nats_enabled: Option<u8>,
    pub url: String,
    pub object_store_bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppCfg {
    pub name: Option<String>, // Optional name field for task identification
    pub chain: ChainCfg,
    pub indexing: IndexingCfg,
    pub postgres: PgCfg,
    pub aws_rds: Option<AwsRdsCfg>,
    pub nats: NatsCfg,
}

impl AppCfg {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let config: Self = serde_yaml::from_str(&std::fs::read_to_string(path)?)?;
        Ok(config)
    }

    pub fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            format!("monitor-{}", chrono::Utc::now().timestamp())
        })
    }

    pub fn is_aws_rds_enabled(&self) -> bool {
        self.aws_rds
            .as_ref()
            .map(|rds| rds.enabled.unwrap_or(0) > 0)
            .unwrap_or(false)
    }
}
