use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod database;
mod merkle_trie;
mod csv_processor;
mod contract_client;
mod service;
mod handlers;
mod encryption;
mod nats_storage;
mod error;
mod external_client;

use crate::config::Config;
use crate::service::AirdropService;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    info!("Starting Airdrop Backend Service");

    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    let config = Config::load_from_file(&config_path).await?;

    let service = Arc::new(AirdropService::new(config.clone(), config_path).await?);

    let app = create_app(service).await;

    let listener = tokio::net::TcpListener::bind(&config.server.bind_address).await?;
    info!("Server running on {}", config.server.bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(service: Arc<AirdropService>) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        // CSV endpoints
        .route("/api/v1/upload-csv", post(handlers::upload_csv))
        .route("/api/v1/download-csv/:round_id", get(handlers::download_csv))
        // JSON eligibility endpoints
        .route("/api/v1/upload-json-eligibility/:round_id", post(handlers::upload_json_eligibility))
        .route("/api/v1/download-json-eligibility/:round_id", get(handlers::download_json_eligibility))
        // Trie data endpoints
        .route("/api/v1/download-trie-data/:round_id", get(handlers::download_trie_data))
        .route("/api/v1/upload-compare-trie/:round_id", post(handlers::upload_and_compare_trie_data))
        // External data endpoints
        .route("/api/v1/fetch-external-data/:round_id", post(handlers::fetch_external_data_and_update))
        .route("/api/v1/compare-external-trie/:round_id", post(handlers::fetch_and_compare_external_trie))
        // Original endpoints
        .route("/api/v1/update-trie/:round_id", post(handlers::update_trie))
        .route("/api/v1/submit-trie/:round_id", post(handlers::submit_trie))
        .route("/api/v1/verify-eligibility", post(handlers::verify_eligibility))
        .route("/api/v1/get-eligibility/:round_id/:address", get(handlers::get_eligibility))
        .route("/api/v1/trie-info/:round_id", get(handlers::get_trie_info))
        .route("/api/v1/rounds/statistics", get(handlers::get_round_statistics))
        .route("/api/v1/processing-logs", get(handlers::get_processing_logs))
        .route("/api/v1/processing-logs/:round_id", get(handlers::get_round_processing_logs))
        .route("/api/v1/rounds/:round_id", delete(handlers::delete_round))
        // Contract endpoints
        .route("/api/v1/contract/info", get(handlers::get_contract_info))
        .route("/api/v1/rounds/:round_id/active", get(handlers::check_round_active))
        .route("/api/v1/rounds/:round_id/metadata", get(handlers::get_round_metadata))
        .route("/api/v1/rounds/:round_id/validate-consistency", get(handlers::validate_consistency))
        .with_state(service)
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB for CSV uploads
                .layer(CorsLayer::permissive())
        )
}
