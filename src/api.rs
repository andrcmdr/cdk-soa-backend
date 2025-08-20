use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::db::Database;

// Response models for the API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct SixMonthsRevenueResponse {
    pub artifact_address: String,
    pub six_month_revenue: String,
    pub calculated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TotalUsageResponse {
    pub artifact_address: String,
    pub total_usage: String,
    pub calculated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

// App state to hold database connection
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

// Create the main router with all endpoints
pub fn create_router(db: Database) -> Router {
    let state = AppState {
        db: Arc::new(db),
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/artifacts/{address}/six-month-revenue", get(get_six_months_revenue))
        .route("/api/v1/artifacts/{address}/total-usage", get(get_total_usage))
        .with_state(state)
}

// Health check endpoint
async fn health_check() -> StatusCode {
    StatusCode::OK
}

// Get six months revenue endpoint
async fn get_six_months_revenue(
    Path(address): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SixMonthsRevenueResponse>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement the actual database query logic
    
    let current_time = chrono::Utc::now().timestamp();
    
    // Placeholder response. This would instead be created from the database response.
    let response = SixMonthsRevenueResponse {
        artifact_address: address,
        six_month_revenue: "0".to_string(),
        calculated_at: current_time,
    };
    
    Ok(Json(response))
}

// Get total usage endpoint
async fn get_total_usage(
    Path(address): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TotalUsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement the actual database query logic
    
    let current_time = chrono::Utc::now().timestamp();
    
    // Placeholder response. This would instead be created from the database response.
    let response = TotalUsageResponse {
        artifact_address: address,
        total_usage: "0".to_string(),
        calculated_at: current_time,
    };
    
    Ok(Json(response))
}

// // Helper function to create error response from status code and message
// fn create_error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
//     (
//         status,
//         Json(ErrorResponse {
//             error: status.to_string(),
//             message: message.to_string(),
//         }),
//     )
// }
