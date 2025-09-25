mod api;
mod config;
mod db;
mod types;
mod validators;
mod miner;
mod transaction;
mod batch;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error, warn};
use alloy::primitives::{Address, U256};
use std::str::FromStr;

use crate::config::Config;
use crate::db::Database;
use crate::miner::APIMiner;
use crate::transaction::ContractClient;
use crate::batch::{get_batch_usage_report, get_batch_revenue_report};
use crate::api::create_router;

async fn initialize_blockchain_client(config: &Config) -> Result<ContractClient> {
    let rpc_url = config.blockchain_rpc_url()?;
    let private_key = config.blockchain_private_key()?;
    let contract_address_str = config.blockchain_contract_address()?;
    let chain_id = config.blockchain_chain_id()?;
    
    info!("Initializing blockchain client...");
    let contract_address = Address::from_str(&contract_address_str)
        .map_err(|e| anyhow::anyhow!("Invalid contract address: {}", e))?;
    
    ContractClient::new(rpc_url, private_key, contract_address, chain_id).await
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first
    let config = Config::load()?;
    
    // Validate configuration
    config.validate_mining_config()?;
    
    // Initialize logging with configured level
    let log_level = config.service.log_level.parse::<tracing::Level>().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("Configuration loaded successfully");
    
    info!("Starting Oracle Service on {}:{}", config.service.host, config.service.port);

    // Connect to database
    let db = Arc::new(Database::new(&config.db_url()).await?);
    db.test_connection().await?;
    
    // Get database stats
    let revenue_count = db.get_revenue_reports_count().await?;
    let usage_count = db.get_usage_reports_count().await?;
    info!("Database stats - Revenue reports: {}, Usage reports: {}", revenue_count, usage_count);
    
    // Initialize API miner if configured
    let api_miner = match (config.mining_api_url(), config.mining_api_key()) {
        (Ok(api_url), Ok(api_key)) => {
            info!("Initializing API miner with URL: {}", api_url);
            Some(APIMiner::new(
                db.clone(),
                api_key,
                api_url,
            ))
        }
        (Err(e), _) | (_, Err(e)) => {
            warn!("API miner not configured - mining will be disabled: {}", e);
            None
        }
    };
    
    // Initialize blockchain client if configured
    let contract_client = match initialize_blockchain_client(&config).await {
        Ok(client) => {
            info!("Blockchain client initialized with wallet: {:?}", client.wallet_address());
            Some(client)
        }
        Err(e) => {
            warn!("Blockchain client not configured - batching will be disabled: {}", e);
            None
        }
    };
    
    // Create API router
    let router = create_router(db.clone());
    
    info!("Oracle Service initialized successfully");
    
    // Start all tasks concurrently
    let mining_handle = if api_miner.is_some() {
        let db = db.clone();
        let config = config.clone();
        Some(tokio::spawn(async move {
            start_mining_task(db, config).await;
        }))
    } else {
        None
    };
    
    let batching_handle = if contract_client.is_some() {
        let db = db.clone();
        let config = config.clone();
        Some(tokio::spawn(async move {
            start_batching_task(db, config).await;
        }))
    } else {
        None
    };
    
    let api_handle = {
        let addr = format!("{}:{}", config.service.host, config.service.port)
            .parse::<std::net::SocketAddr>()
            .expect("Failed to parse socket address");
        tokio::spawn(async move {
            start_api_server(router, addr).await;
        })
    };
    
    info!("All components started successfully");
    
    // Wait for any task to complete (they should run indefinitely)
    tokio::select! {
        result = api_handle => {
            error!("API server task ended: {:?}", result);
        }
        result = mining_handle.unwrap_or_else(|| tokio::spawn(async {})) => {
            error!("Mining task ended: {:?}", result);
        }
        result = batching_handle.unwrap_or_else(|| tokio::spawn(async {})) => {
            error!("Batching task ended: {:?}", result);
        }
    }
    
    Ok(())
}

/// Start the mining task that periodically fetches data from external APIs
async fn start_mining_task(db: Arc<Database>, config: Config) {
    let mining_interval = Duration::from_secs(config.mining.mining_interval_seconds);
    let mut interval = interval(mining_interval);
    
    info!("Starting mining task with interval: {:?}", mining_interval);
    
    loop {
        interval.tick().await;
        
        info!("Starting mining cycle...");
        
        // Determine next time range to mine
        let (start_time, end_time) = match determine_next_mining_range(db.clone(), &config).await {
            Ok(Some(range)) => range,
            Ok(None) => {
                info!("Mining is caught up with real-time, skipping this cycle");
                continue;
            }
            Err(e) => {
                error!("Failed to determine next mining range: {}", e);
                continue;
            }
        };
        
        info!("Mining time range: {} to {} (with {}s delay applied)", start_time, end_time, config.mining.mining_delay_seconds);
        
        match mine_data_with_tracking(db.clone(), &config, start_time, end_time).await {
            Ok(records_found) => {
                info!("Mining cycle completed successfully, found {} records", records_found);
            }
            Err(e) => {
                error!("Mining cycle failed: {}", e);
            }
        }
    }
}

/// Start the batching task that periodically batches and submits data to blockchain
async fn start_batching_task(db: Arc<Database>, config: Config) {
    let batch_interval = Duration::from_secs(config.contract.batch_interval_seconds);
    let mut interval = interval(batch_interval);
    
    info!("Starting batching task with interval: {:?}", batch_interval);
    
    loop {
        interval.tick().await;
        
        info!("Starting batching cycle...");
        
        // Process usage reports
        match process_usage_reports(db.clone(), &config).await {
            Ok(()) => {
                info!("Usage reports processing completed");
            }
            Err(e) => {
                error!("Usage reports processing failed: {}", e);
            }
        }
        
        // Process revenue reports
        match process_revenue_reports(db.clone(), &config).await {
            Ok(()) => {
                info!("Revenue reports processing completed");
            }
            Err(e) => {
                error!("Revenue reports processing failed: {}", e);
            }
        }
    }
}

/// Start the API server
async fn start_api_server(router: axum::Router, addr: std::net::SocketAddr) {
    info!("Starting API server on {}", addr);
    
    if let Err(e) = tokio::net::TcpListener::bind(addr).await {
        error!("Failed to bind to address {}: {}", addr, e);
        return;
    }
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    if let Err(e) = axum::serve(listener, router).await {
        error!("API server failed: {}", e);
    }
}

/// Mine data from external API with state tracking
async fn mine_data_with_tracking(db: Arc<Database>, config: &Config, start_at: i64, end_at: i64) -> Result<i32> {
    let (api_url, api_key) = match (config.mining_api_url(), config.mining_api_key()) {
        (Ok(url), Ok(key)) => (url, key),
        (Err(e), _) | (_, Err(e)) => return Err(e),
    };
    
    let api_miner = APIMiner::new(db.clone(), api_key, api_url);
    
    info!("Mining data from {} to {}", start_at, end_at);
    
    let backend_data = api_miner.fetch_data(start_at, end_at).await?;
    info!("Fetched {} data items from API", backend_data.len());
    
    let mut records_inserted = 0;
    for data in backend_data {
        match crate::validators::validate_backend_data(&data) {
            Ok(valid) => {
                if valid {
                    db.insert_backend_data(&data).await?;
                    records_inserted += 1;
                } else {
                    warn!("Invalid data rejected: {:?}", data);
                }
            }
            Err(e) => {
                error!("Failed to validate backend data: {}", e);
            }
        }
    }
    
    // Record that this time range has been successfully mined
    // Note: start_at and end_at are the actual times we mined (with delay already applied)
    db.record_mining_completed(start_at, end_at, records_inserted).await?;
    
    Ok(records_inserted)
}

/// Determine the next time range to mine based on current state
/// Returns None if we're fully caught up and should skip mining
async fn determine_next_mining_range(db: Arc<Database>, config: &Config) -> Result<Option<(i64, i64)>> {
    let now = chrono::Utc::now().timestamp();
    let interval = config.mining.mining_interval_seconds as i64;
    let delay = config.mining.mining_delay_seconds as i64;
    
    // Get the last successfully mined timestamp
    match db.get_last_mined_timestamp().await? {
        Some(last_mined_end) => {
            // Continue from where we left off
            // last_mined_end is the actual end time we mined (with delay already applied)
            let start_time = last_mined_end;
            // Instead of using min here, an if comparison is used to make the code more readable
            let end_time = start_time + interval;
            // Ensure we don't mine data that's too recent (apply delay constraint)
            let max_allowed_end = now - delay;
            if end_time > max_allowed_end {
                // We've caught up to real-time, need to wait or mine a smaller range
                let adjusted_end_time = max_allowed_end;
                if adjusted_end_time <= start_time {
                    // No data to mine yet, we're fully caught up - skip this cycle
                    return Ok(None);
                }
                Ok(Some((start_time, adjusted_end_time)))
            } else {
                Ok(Some((start_time, end_time)))
            }
        }
        None => {
            // First time mining - start from configured lookback period to catch historical data
            let lookback_seconds = config.mining.bootstrap_lookback_seconds
                .unwrap_or((interval * 12) as u64) as i64; // Default to 12 intervals back
            
            let start_time = now - lookback_seconds;
            let end_time = start_time + interval;
            
            info!("Bootstrapping: Starting mining from {} seconds ago ({})", 
                lookback_seconds, chrono::DateTime::from_timestamp(start_time, 0)
                .unwrap_or_default().format("%Y-%m-%d %H:%M:%S UTC"));
            
            // Apply delay to both start and end times
            Ok(Some((start_time - delay, end_time - delay)))
        }
    }
}


/// Process usage reports and submit to blockchain
async fn process_usage_reports(db: Arc<Database>, config: &Config) -> Result<()> {
    let (rpc_url, private_key, contract_address, chain_id) = match (
        config.blockchain_rpc_url(),
        config.blockchain_private_key(),
        config.blockchain_contract_address(),
        config.blockchain_chain_id(),
    ) {
        (Ok(url), Ok(key), Ok(addr), Ok(id)) => (url, key, addr, id),
        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => return Err(e),
    };
    
    let contract_address = Address::from_str(&contract_address)?;
    let contract_client = ContractClient::new(
        rpc_url,
        private_key,
        contract_address,
        chain_id,
    ).await?;
    
    let (batch, ids) = get_batch_usage_report(&*db, config.contract.batch_size).await?;
    
    if batch.artifact_address.is_empty() {
        info!("No usage reports to process");
        return Ok(());
    }
    
    info!("Processing {} usage reports", batch.artifact_address.len());
    
    // Convert addresses
    let artifacts: Result<Vec<Address>, _> = batch.artifact_address
        .iter()
        .map(|addr| Address::from_str(addr))
        .collect();
    let artifacts = artifacts?;
    
    // Convert to U256
    let usages: Result<Vec<U256>, _> = batch.usage.into_iter()
        .map(|s| U256::from_str(&s))
        .collect();
    let usages = usages?;
    let timestamps: Vec<U256> = batch.timestamp.into_iter().map(U256::from).collect();
    
    // Submit to blockchain
    let tx_hash = contract_client.batch_report_artifact_usage(artifacts, usages, timestamps).await?;
    info!("Usage reports submitted to blockchain with tx hash: {:?}", tx_hash);
    
    // Mark reports as submitted in database
    let id_count = ids.len();
    db.mark_usage_reports_submitted(ids).await?;
    info!("Marked {} usage reports as submitted", id_count);
    
    Ok(())
}

/// Process revenue reports and submit to blockchain
async fn process_revenue_reports(db: Arc<Database>, config: &Config) -> Result<()> {
    let (rpc_url, private_key, contract_address, chain_id) = match (
        config.blockchain_rpc_url(),
        config.blockchain_private_key(),
        config.blockchain_contract_address(),
        config.blockchain_chain_id(),
    ) {
        (Ok(url), Ok(key), Ok(addr), Ok(id)) => (url, key, addr, id),
        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => return Err(e),
    };
    
    let contract_address = Address::from_str(&contract_address)?;
    let contract_client = ContractClient::new(
        rpc_url,
        private_key,
        contract_address,
        chain_id,
    ).await?;
    
    let (batch, ids) = get_batch_revenue_report(&*db, config.contract.batch_size).await?;
    
    if batch.artifact_address.is_empty() {
        info!("No revenue reports to process");
        return Ok(());
    }
    
    info!("Processing {} revenue reports", batch.artifact_address.len());
    
    // Convert addresses
    let artifacts: Result<Vec<Address>, _> = batch.artifact_address
        .iter()
        .map(|addr| Address::from_str(addr))
        .collect();
    let artifacts = artifacts?;
    
    // Convert to U256
    let revenues: Result<Vec<U256>, _> = batch.revenue.into_iter()
        .map(|s| U256::from_str(&s))
        .collect();
    let revenues = revenues?;
    let timestamps: Vec<U256> = batch.timestamp.into_iter().map(U256::from).collect();
    
    // Submit to blockchain
    let tx_hash = contract_client.batch_report_artifact_revenue(artifacts, revenues, timestamps).await?;
    info!("Revenue reports submitted to blockchain with tx hash: {:?}", tx_hash);
    
    // Mark reports as submitted in database
    let id_count = ids.len();
    db.mark_revenue_reports_submitted(ids).await?;
    info!("Marked {} revenue reports as submitted", id_count);
    
    Ok(())
}
