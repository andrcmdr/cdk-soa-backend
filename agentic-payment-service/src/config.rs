use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub protocols: ProtocolsConfig,
    pub agent: AgentConfig,
    pub payment_gateways: PaymentGatewaysConfig,
    pub middleware: MiddlewareConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolsConfig {
    pub enabled: Vec<String>,
    pub x402: X402Config,
    pub ap2: AP2Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Config {
    pub enabled: bool,
    pub version: String,
    pub endpoint: String,
    pub api_key: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AP2Config {
    pub enabled: bool,
    pub version: String,
    pub endpoint: String,
    pub api_key: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model_path: String,
    pub model_type: String,
    pub context_size: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
    pub inference: InferenceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub threads: usize,
    pub batch_size: usize,
    pub gpu_layers: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentGatewaysConfig {
    pub web3: Web3Config,
    pub web2: Web2Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3Config {
    pub enabled: bool,
    pub blockchain: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web2Config {
    pub enabled: bool,
    pub provider: String,
    pub api_key: String,
    pub webhook_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub rate_limiting: RateLimitConfig,
    pub authentication: AuthConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_payment_amount: f64,
    pub require_confirmation: bool,
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
