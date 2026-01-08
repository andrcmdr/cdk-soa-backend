use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    middleware as axum_middleware,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod agent;
mod protocols;
mod middleware;
mod handlers;
mod payment;
mod error;

use config::Config;
use agent::AgentRunner;
use protocols::{ProtocolManager, x402::X402Protocol, ap2::AP2Protocol};
use payment::{PaymentGatewayManager, web3::Web3Gateway, web2::Web2Gateway};

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    agent: Arc<AgentRunner>,
    protocol_manager: Arc<ProtocolManager>,
    gateway_manager: Arc<PaymentGatewayManager>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agentic_payment_service=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Agentic Payment Service");

    // Load configuration
    let config = Config::load("config.yaml")?;
    tracing::info!("Configuration loaded successfully");

    // Initialize agent runner
    let agent = AgentRunner::new(&config.agent)?;
    tracing::info!("Agent runner initialized");

    // Initialize protocol manager
    let mut protocol_manager = ProtocolManager::new();
    
    if config.protocols.x402.enabled {
        let x402 = X402Protocol::new(config.protocols.x402.clone())?;
        protocol_manager.register("x402", Box::new(x402));
        tracing::info!("X402 protocol registered");
    }
    
    if config.protocols.ap2.enabled {
        let ap2 = AP2Protocol::new(config.protocols.ap2.clone())?;
        protocol_manager.register("ap2", Box::new(ap2));
        tracing::info!("AP2 protocol registered");
    }

    // Initialize payment gateways
    let mut gateway_manager = PaymentGatewayManager::new();
    
    if config.payment_gateways.web3.enabled {
        let web3 = Web3Gateway::new(config.payment_gateways.web3.clone())?;
        gateway_manager.register("web3", Box::new(web3));
        tracing::info!("Web3 gateway registered");
    }
    
    if config.payment_gateways.web2.enabled {
        let web2 = Web2Gateway::new(config.payment_gateways.web2.clone())?;
        gateway_manager.register("web2", Box::new(web2));
        tracing::info!("Web2 gateway registered");
    }

    // Create shared state
    let state = AppState {
        config: Arc::new(config.clone()),
        agent: Arc::new(agent),
        protocol_manager: Arc::new(protocol_manager),
        gateway_manager: Arc::new(gateway_manager),
    };

    // Build application router
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/payment/prompt", post(handlers::process_payment_prompt))
        .route("/api/v1/payment/execute", post(handlers::execute_payment))
        .route("/api/v1/payment/status/:id", get(handlers::get_payment_status))
        .route("/api/v1/agent/query", post(handlers::agent_query))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::auth_middleware,
        ))
        .layer(axum_middleware::from_fn(middleware::rate_limit::rate_limit_middleware))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Server listening on {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
