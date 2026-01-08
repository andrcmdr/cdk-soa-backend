use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentServiceError {
    #[error("Agent processing error: {0}")]
    AgentError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Gateway error: {0}")]
    GatewayError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Invalid request: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for PaymentServiceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            PaymentServiceError::AgentError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            PaymentServiceError::ProtocolError(msg) => {
                (StatusCode::BAD_GATEWAY, msg)
            }
            PaymentServiceError::GatewayError(msg) => {
                (StatusCode::BAD_GATEWAY, msg)
            }
            PaymentServiceError::ConfigError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            PaymentServiceError::AuthError(msg) => {
                (StatusCode::UNAUTHORIZED, msg)
            }
            PaymentServiceError::RateLimitError => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string())
            }
            PaymentServiceError::ValidationError(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            PaymentServiceError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg)
            }
            PaymentServiceError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}