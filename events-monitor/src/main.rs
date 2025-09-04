mod config;
mod db;
mod nats;
mod abi;
mod subscriptions;
mod event_decoder;
mod types;

use crate::subscriptions::EventProcessor;

use std::path::Path;
use tracing_subscriber::{EnvFilter, fmt};
use tracing::{ info, debug, error, trace, warn };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    let cfg_path = std::env::args().nth(1).unwrap_or_else(|| "./config.yaml".to_string());
    let cfg = config::AppCfg::load(&cfg_path)?;

    let db_schema_path = std::env::args().nth(2).ok_or_else(|| &cfg.postgres.schema).unwrap_or_else(|_path| "./init.sql".to_string());
    let db_schema = std::fs::read_to_string(Path::new(&db_schema_path))?;

    // deps
    let pg = db::connect_pg(&cfg.postgres.dsn, db_schema.as_str()).await?;
    let nats = if cfg.nats.nats_enabled.is_some_and(|enabled| enabled > 0) {
        let nats = nats::connect(&cfg.nats.url, &cfg.nats.object_store_bucket).await?;
        Some(nats)
    } else { None };

    let event_processor = EventProcessor::new(&cfg, pg, nats).await?;
    event_processor.run().await?;

    Ok(())
}
