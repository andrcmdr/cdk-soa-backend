mod config;
mod db;
mod nats;
mod subscriptions;
mod types;
mod task_manager;
mod web_api;
mod aws_rds;

use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt};
use tracing::{info, error};

use crate::task_manager::TaskManager;
use crate::web_api::start_web_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    // Check if we should run in API mode or single task mode
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--api" {
        // Run in API mode
        let bind_address = std::env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        info!("Starting Mempool Monitor API server");

        let task_manager = Arc::new(TaskManager::new());

        // Start cleanup task
        let task_manager_cleanup = Arc::clone(&task_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                task_manager_cleanup.cleanup_finished_tasks().await;
            }
        });

        start_web_server(task_manager, &bind_address).await?;
    } else {
        // Run in single task mode (original behavior)
        info!("Starting Mempool Monitor in single task mode");

        let cfg_path = args.get(1).unwrap_or(&"./config.yaml".to_string()).clone();
        let cfg = config::AppCfg::load(&cfg_path)?;

        let db_schema_path = if let Some(path) = args.get(2) {
            path.clone()
        } else {
            if cfg.postgres.schema.is_empty() {
                "./init.sql".to_string()
            } else {
                cfg.postgres.schema.clone()
            }
        };

        let db_schema = std::fs::read_to_string(&db_schema_path)?;

        // Initialize database connections (local + AWS RDS if enabled)
        let aws_rds_config = if cfg.is_aws_rds_enabled() {
            cfg.aws_rds.as_ref()
        } else {
            None
        };

        let db_clients = db::DatabaseClients::new(
            &cfg.postgres.dsn,
            &db_schema,
            aws_rds_config
        ).await?;

        // Test database connections
        db_clients.test_connections().await?;

        let nats = if cfg.nats.nats_enabled.is_some_and(|enabled| enabled > 0) {
            let nats = nats::connect(&cfg.nats.url, &cfg.nats.object_store_bucket).await?;
            Some(nats)
        } else {
            None
        };

        let tx_processor = subscriptions::TxProcessor::new(&cfg, db_clients, nats).await?;
        tx_processor.run().await?;
    }

    Ok(())
}
