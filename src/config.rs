use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub service: ServiceConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self> {
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
}
