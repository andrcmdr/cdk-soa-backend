use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::Json,
    routing::{get, post},
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

    let config = Config::load_from_file("config.yaml").await?;
    let service = Arc::new(AirdropService::new(config.clone()).await?);

    let app = create_app(service).await;

    let listener = tokio::net::TcpListener::bind(&config.server.bind_address).await?;
    info!("Server running on {}", config.server.bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(service: Arc<AirdropService>) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/upload-csv", post(handlers::upload_csv))
        .route("/api/v1/update-trie/:round_id", post(handlers::update_trie))
        .route("/api/v1/submit-trie/:round_id", post(handlers::submit_trie))
        .route("/api/v1/verify-eligibility", post(handlers::verify_eligibility))
        .route("/api/v1/get-eligibility/:round_id/:address", get(handlers::get_eligibility))
        .route("/api/v1/trie-info/:round_id", get(handlers::get_trie_info))
        .with_state(service)
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB for CSV uploads
                .layer(CorsLayer::permissive())
        )
}
