use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Database(DatabaseError),

    #[error("CSV processing error: {0}")]
    CsvProcessing(#[from] csv::Error),

    #[error("Blockchain error: {0}")]
    Blockchain(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error(transparent)]
    Nats(NatsError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Postgres database error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("App internal database error: {0}")]
    App(#[from] anyhow::Error),
    #[error("Database error: {0}")]
    Msg(String),
}

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("NATS library error: {0}")]
    Nats(#[from] async_nats::Error),
    #[error("App internal NATS error: {0}")]
    App(#[from] anyhow::Error),
    #[error("NATS error: {0}")]
    Msg(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
