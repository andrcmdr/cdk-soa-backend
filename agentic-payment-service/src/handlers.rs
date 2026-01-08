use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    agent::{AgentRequest, AgentResponse},
    protocols::PaymentRequest,
    payment::GatewayPaymentRequest,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PaymentPromptRequest {
    pub prompt: String,
    pub context: Option<String>,
    pub preferred_protocol: Option<String>,
    pub preferred_gateway: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentPromptResponse {
    pub request_id: String,
    pub agent_response: AgentResponse,
    pub suggested_protocol: Option<String>,
    pub estimated_fees: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutePaymentRequest {
    pub request_id: String,
    pub protocol: String,
    pub gateway: String,
    pub confirmation: bool,
}

#[derive(Debug, Serialize)]
pub struct ExecutePaymentResponse {
    pub transaction_id: String,
    pub status: String,
    pub message: String,
    pub details: serde_json::Value,
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "agentic-payment-service",
        "version": "0.1.0"
    }))
}

pub async fn process_payment_prompt(
    State(state): State<AppState>,
    Json(payload): Json<PaymentPromptRequest>,
) -> Result<Json<PaymentPromptResponse>, (StatusCode, String)> {
    tracing::info!("Processing payment prompt: {}", payload.prompt);

    let agent_request = AgentRequest {
        prompt: payload.prompt.clone(),
        context: payload.context.clone(),
        max_tokens: None,
    };

    let agent_response = state
        .agent
        .process(agent_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let suggested_protocol = payload
        .preferred_protocol
        .clone()
        .or_else(|| agent_response.protocol.clone())
        .or_else(|| Some("x402".to_string()));

    // Estimate fees based on action
    let estimated_fees = if let Some(action) = &agent_response.action {
        let protocol = suggested_protocol.as_ref().unwrap();
        let gateway = payload.preferred_gateway.as_deref().unwrap_or("web2");
        
        if let Ok(gw) = state.gateway_manager.get(gateway) {
            gw.estimate_fees(action.amount, &action.currency)
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    let request_id = Uuid::new_v4().to_string();

    Ok(Json(PaymentPromptResponse {
        request_id,
        agent_response,
        suggested_protocol,
        estimated_fees,
    }))
}

pub async fn execute_payment(
    State(state): State<AppState>,
    Json(payload): Json<ExecutePaymentRequest>,
) -> Result<Json<ExecutePaymentResponse>, (StatusCode, String)> {
    tracing::info!("Executing payment with protocol: {}, gateway: {}", 
        payload.protocol, payload.gateway);

    if !payload.confirmation {
        return Err((
            StatusCode::BAD_REQUEST,
            "Payment confirmation required".to_string(),
        ));
    }

    // Create mock payment request for demonstration
    let payment_request = PaymentRequest {
        id: payload.request_id.clone(),
        amount: 100.0,
        currency: "USD".to_string(),
        sender: "agent_001".to_string(),
        recipient: "agent_002".to_string(),
        memo: Some("Payment via agentic service".to_string()),
        metadata: serde_json::json!({}),
    };

    // Process through protocol
    let protocol_response = state
        .protocol_manager
        .process_payment(&payload.protocol, payment_request.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Execute through gateway
    let gateway_request = GatewayPaymentRequest {
        amount: payment_request.amount,
        currency: payment_request.currency.clone(),
        from: payment_request.sender.clone(),
        to: payment_request.recipient.clone(),
        memo: payment_request.memo.clone(),
        metadata: payment_request.metadata.clone(),
    };

    let gateway_response = state
        .gateway_manager
        .execute_payment(&payload.gateway, gateway_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ExecutePaymentResponse {
        transaction_id: protocol_response.transaction_id.clone(),
        status: format!("{:?}", protocol_response.status),
        message: protocol_response.message.clone(),
        details: serde_json::json!({
            "protocol_response": protocol_response,
            "gateway_response": gateway_response,
        }),
    }))
}

pub async fn get_payment_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("Checking payment status: {}", id);

    // In a real implementation, you'd track payments in a database
    // For now, return a mock response
    let protocol = state
        .protocol_manager
        .get("x402")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = protocol
        .check_status(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "transaction_id": id,
        "status": format!("{:?}", status),
        "last_updated": chrono::Utc::now().to_rfc3339(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct AgentQueryRequest {
    pub query: String,
    pub context: Option<String>,
}

pub async fn agent_query(
    State(state): State<AppState>,
    Json(payload): Json<AgentQueryRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, String)> {
    tracing::info!("Processing agent query");

    let agent_request = AgentRequest {
        prompt: payload.query,
        context: payload.context,
        max_tokens: None,
    };

    let response = state
        .agent
        .process(agent_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(response))
}