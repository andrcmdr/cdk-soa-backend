mod api;
mod config;
mod db;
mod models;
mod mock_data;
mod validators;
mod miner;


use anyhow::Result;
use crate::miner::APIMiner;
use crate::api::create_router;
use tracing::{info, error};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = config::Config::load()?;
    info!("Configuration loaded successfully");
    
    // Initialize logging with configured level
    let log_level = config.service.log_level.parse::<tracing::Level>().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("Starting Oracle Service on {}:{}", config.service.host, config.service.port);

    // Connect to database using configuration
    let db = db::Database::new(&config.db_url()).await?;
    
    // Test connection
    db.test_connection().await?;
    
    // Get some basic stats
    let revenue_count = db.get_revenue_reports_count().await?;
    let usage_count = db.get_usage_reports_count().await?;
    
    info!("Database stats - Revenue reports: {}, Usage reports: {}", revenue_count, usage_count);
    
    info!("Oracle Service started successfully!");

    // Create shared database handle
    let db_handle = Arc::new(db);
    
    // Create the API router
    let app = create_router(db_handle.clone());
    
    // Bind to the configured address
    let addr = format!("{}:{}", config.service.host, config.service.port)
        .parse::<SocketAddr>()
        .expect("Failed to parse socket address");
    
    info!("Starting HTTP server on {}", addr);
    
    // Start miners concurrently with the API server
    let db_for_api_miner = db_handle.clone();
    
    // Spawn API miner task
    let api_miner_handle = tokio::spawn(async move {
        info!("Starting API miner...");
        let api_miner = APIMiner::new(db_for_api_miner, 60);
        if let Err(e) = api_miner.start().await {
            error!("API miner failed: {}", e);
        }
    });
    
    // Start the HTTP server
    let server_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });
    
    // Wait for any of the tasks to complete (they should run indefinitely)
    tokio::select! {
        result = api_miner_handle => {
            error!("API miner task ended: {:?}", result);
        }
        result = server_handle => {
            error!("HTTP server task ended: {:?}", result);
        }
    }



    Ok(())
}
