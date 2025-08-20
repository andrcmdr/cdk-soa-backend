mod api;
mod config;
mod db;
mod models;
mod mock_data;
mod validators;


use anyhow::Result;
use crate::mock_data::{load_mock_revenue_reports, load_mock_usage_reports};
use crate::validators::{validate_revenue_report, validate_usage_report};
use crate::api::create_router;
use tracing::{info, debug, error};
use std::net::SocketAddr;

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

    info!("Loading mock data...");
    let revenue_reports = load_mock_revenue_reports()?;
    let usage_reports = load_mock_usage_reports()?;
    info!("Mock data loaded successfully");
    
    debug!("Revenue reports: {:?}", revenue_reports);
    debug!("Usage reports: {:?}", usage_reports);

    info!("Inserting mock data into database...");

    // validate revenue reports and insert into database
    for (i, revenue_report) in revenue_reports.iter().enumerate() {
        match validate_revenue_report(&revenue_report) {
            Ok(valid) => {
                if valid {
                    match db.insert_revenue_report(&revenue_report).await {
                        Ok(_) => info!("Inserted revenue report {}", i),
                        Err(e) => error!("Failed to insert revenue report {}: {}", i, e),
                    }
                }
            }
            Err(e) => info!("Validation of revenue report {} failed: {}", i, e),
        }
    }

    for (i, usage_report) in usage_reports.iter().enumerate() {
        match validate_usage_report(&usage_report) {
            Ok(valid) => {
                if valid {
                    match db.insert_usage_report(&usage_report).await {
                        Ok(_) => info!("Inserted usage report {}", i),
                        Err(e) => error!("Failed to insert usage report {}: {}", i, e),
                    }
                }
            }
            Err(e) => info!("Validation of revenue report {} failed: {}", i, e),
        }
    }

    info!("Mock data inserted into database successfully");

    info!("Database stats - Revenue reports: {}, Usage reports: {}", db.get_revenue_reports_count().await?, db.get_usage_reports_count().await?);

    // Create the API router
    let app = create_router(db);
    
    // Bind to the configured address
    let addr = format!("{}:{}", config.service.host, config.service.port)
        .parse::<SocketAddr>()
        .expect("Failed to parse socket address");
    
    info!("Starting HTTP server on {}", addr);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
