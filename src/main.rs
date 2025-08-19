mod config;
mod db;

use anyhow::Result;
use tracing::info;

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
    
    // Keep the service running indefinitely
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        info!("Oracle Service is running...");
    }
}
