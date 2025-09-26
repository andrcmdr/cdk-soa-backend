use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub service: ServiceConfig,
    pub mining: MiningConfig,
    pub contract: ContractConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}


#[derive(Debug, Deserialize, Clone)]
pub struct MiningConfig {
    pub mining_interval_seconds: u64,
    pub mining_delay_seconds: u64,
    /// How far back to start mining on first run (in seconds from now)
    /// If not set, defaults to mining_interval_seconds * 12 (12 intervals back)
    pub bootstrap_lookback_seconds: Option<u64>,
    /// Number of items to fetch per page when paginating API requests
    pub page_size: u32,
    /// Maximum number of pages to fetch to prevent infinite loops
    pub max_pages: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractConfig {
    pub batch_size: i32,
    pub batch_interval_seconds: u64,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Load .env file first
        dotenv::dotenv().ok();     
        let config_builder = config::Config::builder()
            // Start with default config
            .add_source(config::File::with_name("config.toml"))
            // Add environment-specific config
            .add_source(config::File::with_name("config").required(false))
            // Add environment variables with prefix "ORACLE_"
            .add_source(config::Environment::with_prefix("ORACLE").separator("_"))
            .build()?;

        let config: Config = config_builder.try_deserialize()?;
        Ok(config)
    }

    pub fn db_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    /// Load blockchain RPC URL from environment variable
    pub fn blockchain_rpc_url(&self) -> Result<String> {
        std::env::var("RPC_URL")
            .map_err(|_| anyhow::anyhow!("RPC_URL environment variable not set"))
    }

    /// Load blockchain private key from environment variable
    pub fn blockchain_private_key(&self) -> Result<String> {
        std::env::var("PRIVATE_KEY")
            .map_err(|_| anyhow::anyhow!("PRIVATE_KEY environment variable not set"))
    }

    /// Load blockchain contract address from environment variable
    pub fn blockchain_contract_address(&self) -> Result<String> {
        std::env::var("CONTRACT_ADDRESS")
            .map_err(|_| anyhow::anyhow!("CONTRACT_ADDRESS environment variable not set"))
    }

    /// Load mining API URL from environment variable
    pub fn mining_api_url(&self) -> Result<String> {
        std::env::var("API_URL")
            .map_err(|_| anyhow::anyhow!("API_URL environment variable not set"))
    }

    /// Load mining API key from environment variable
    pub fn mining_api_key(&self) -> Result<String> {
        std::env::var("API_KEY")
            .map_err(|_| anyhow::anyhow!("API_KEY environment variable not set"))
    }

    /// Load blockchain chain ID from environment variable
    pub fn blockchain_chain_id(&self) -> Result<u64> {
        std::env::var("CHAIN_ID")
            .map_err(|_| anyhow::anyhow!("CHAIN_ID environment variable not set"))
            .and_then(|id| id.parse::<u64>().map_err(|e| anyhow::anyhow!("Invalid CHAIN_ID: {}", e)))
    }

    /// Validate mining configuration to prevent invalid time ranges
    pub fn validate_mining_config(&self) -> Result<()> {
        let interval = self.mining.mining_interval_seconds as i64;
        let delay = self.mining.mining_delay_seconds as i64;
        
        if delay >= interval {
            return Err(anyhow::anyhow!(
                "Invalid mining configuration: mining_delay_seconds ({}) must be less than mining_interval_seconds ({}) \
                to ensure valid time ranges", delay, interval
            ));
        }
        
        Ok(())
    }
}
