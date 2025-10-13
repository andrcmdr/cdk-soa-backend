use std::sync::Arc;
use axum::{
    extract::{Path, State, Multipart},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{info, error};

use crate::config::AppCfg;
use crate::task_manager::{TaskManager, TaskInfo};

#[derive(Clone)]
pub struct AppState {
    task_manager: Arc<TaskManager>,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: Option<String>,
    pub config_yaml: String,
    pub db_schema: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTaskResponse {
    pub task_id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn create_web_api(task_manager: Arc<TaskManager>) -> Router {
    let app_state = AppState { task_manager };

    Router::new()
        .route("/api/tasks", post(create_task_handler))
        .route("/api/tasks", get(list_tasks_handler))
        .route("/api/tasks/:task_id", get(get_task_handler))
        .route("/api/tasks/:task_id/stop", post(stop_task_handler))
        .route("/api/tasks/:task_id", delete(delete_task_handler))
        .route("/api/health", get(health_check_handler))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        )
}

async fn create_task_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CreateTaskResponse>, (StatusCode, Json<ApiError>)> {
    let mut config_yaml: Option<String> = None;
    let mut task_name: Option<String> = None;
    let mut db_schema: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: format!("Failed to parse multipart data: {}", e) })))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        let field_data = field
            .text()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: format!("Failed to read field data: {}", e) })))?;

        match field_name.as_str() {
            "config_yaml" => config_yaml = Some(field_data),
            "name" => task_name = Some(field_data),
            "db_schema" => db_schema = Some(field_data),
            _ => {}
        }
    }

    let config_yaml = config_yaml
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Missing config_yaml field".to_string() })))?;

    // Parse the YAML configuration
    let config: AppCfg = serde_yaml::from_str(&config_yaml)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: format!("Invalid YAML configuration: {}", e) })))?;

    // Extract task name from config if not provided
    let name = task_name.unwrap_or_else(|| {
        // Try to derive name from config or use a default
        format!("task-{}", chrono::Utc::now().timestamp())
    });

    // Use provided schema or read default
    let schema = if let Some(schema_content) = db_schema {
        schema_content
    } else {
        let schema_path = if config.postgres.schema.is_empty() {
            "./init.sql".to_string()
        } else {
            config.postgres.schema.clone()
        };

        std::fs::read_to_string(&schema_path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError {
                error: format!("Failed to read database schema from {}: {}", schema_path, e)
            })))?
    };

    // Create the task
    match state.task_manager.create_task(name.clone(), config, schema).await {
        Ok(task_id) => {
            info!("Created task {} with ID {}", name, task_id);
            Ok(Json(CreateTaskResponse {
                task_id,
                message: format!("Task '{}' created successfully", name),
            }))
        }
        Err(e) => {
            error!("Failed to create task {}: {:?}", name, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError {
                error: format!("Failed to create task: {}", e)
            })))
        }
    }
}

async fn list_tasks_handler(
    State(state): State<AppState>,
) -> Json<Vec<TaskInfo>> {
    // Clean up finished tasks before listing
    state.task_manager.cleanup_finished_tasks().await;

    let tasks = state.task_manager.list_tasks().await;
    Json(tasks)
}

async fn get_task_handler(
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TaskInfo>, (StatusCode, Json<ApiError>)> {
    match state.task_manager.get_task(&task_id).await {
        Some(task) => Ok(Json(task)),
        None => Err((StatusCode::NOT_FOUND, Json(ApiError {
            error: format!("Task not found: {}", task_id)
        })))
    }
}

async fn stop_task_handler(
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    match state.task_manager.stop_task(&task_id).await {
        Ok(_) => {
            info!("Stopping task: {}", task_id);
            Ok(Json(json!({
                "message": format!("Task {} stop signal sent", task_id)
            })))
        }
        Err(e) => {
            error!("Failed to stop task {}: {:?}", task_id, e);
            Err((StatusCode::NOT_FOUND, Json(ApiError {
                error: e.to_string()
            })))
        }
    }
}

async fn delete_task_handler(
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    // First try to stop the task
    let _ = state.task_manager.stop_task(&task_id).await;

    // Clean up finished tasks (which will remove stopped tasks)
    state.task_manager.cleanup_finished_tasks().await;

    Ok(Json(json!({
        "message": format!("Task {} deletion requested", task_id)
    })))
}

async fn health_check_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn start_web_server(
    task_manager: Arc<TaskManager>,
    bind_address: &str,
) -> anyhow::Result<()> {
    let app = create_web_api(task_manager).await;

    info!("Starting web server on {}", bind_address);

    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", bind_address, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Web server error: {}", e))?;

    Ok(())
}
