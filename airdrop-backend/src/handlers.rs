use axum::{
    extract::{Path, State, Multipart},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use alloy_primitives::{Address, U256};

use crate::service::AirdropService;
use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize)]
pub struct VerifyEligibilityRequest {
    pub round_id: u32,
    pub address: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyEligibilityResponse {
    pub is_eligible: bool,
    pub round_id: u32,
    pub address: String,
    pub amount: String,
}

#[derive(Serialize)]
pub struct TrieInfoResponse {
    pub round_id: u32,
    pub root_hash: String,
    pub entry_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn upload_csv(
    State(service): State<Arc<AirdropService>>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    let mut round_id: Option<u32> = None;
    let mut csv_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::InvalidInput(format!("Multipart error: {}", e)))? {

        let name = field.name().unwrap_or_default();

        match name {
            "round_id" => {
                let text = field.text().await
                    .map_err(|e| AppError::InvalidInput(format!("Invalid round_id: {}", e)))?;
                round_id = Some(text.parse()
                    .map_err(|e| AppError::InvalidInput(format!("Invalid round_id format: {}", e)))?);
            }
            "csv_file" => {
                csv_data = Some(field.bytes().await
                    .map_err(|e| AppError::InvalidInput(format!("Failed to read CSV file: {}", e)))?
                    .to_vec());
            }
            _ => {
                // Skip unknown fields
            }
        }
    }

    let round_id = round_id.ok_or_else(|| AppError::InvalidInput("round_id is required".to_string()))?;
    let csv_data = csv_data.ok_or_else(|| AppError::InvalidInput("csv_file is required".to_string()))?;

    service.process_csv_data(&csv_data, round_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("CSV data processed for round {}", round_id),
        "round_id": round_id
    })))
}

pub async fn update_trie(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    // This endpoint can be used to manually trigger trie updates
    // The trie should already be updated when CSV is processed
    let trie_info = service.get_trie_info(round_id).await?;

    match trie_info {
        Some(info) => Ok(Json(json!({
            "success": true,
            "message": format!("Trie for round {} is up to date", round_id),
            "round_id": round_id,
            "root_hash": info.root_hash,
            "entry_count": info.metadata.entry_count
        }))),
        None => Err(AppError::NotFound(format!("No trie data found for round {}", round_id)))
    }
}

pub async fn submit_trie(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    let tx_hash = service.submit_trie_update(round_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Trie update submitted for round {}", round_id),
        "round_id": round_id,
        "transaction_hash": format!("0x{}", hex::encode(tx_hash))
    })))
}

pub async fn verify_eligibility(
    State(service): State<Arc<AirdropService>>,
    Json(payload): Json<VerifyEligibilityRequest>,
) -> AppResult<Json<VerifyEligibilityResponse>> {
    let address: Address = payload.address.parse()
        .map_err(|e| AppError::InvalidInput(format!("Invalid address: {}", e)))?;

    let amount: U256 = payload.amount.parse()
        .map_err(|e| AppError::InvalidInput(format!("Invalid amount: {}", e)))?;

    let is_eligible = service.verify_eligibility(payload.round_id, address, amount).await?;

    Ok(Json(VerifyEligibilityResponse {
        is_eligible,
        round_id: payload.round_id,
        address: payload.address,
        amount: payload.amount,
    }))
}

pub async fn get_eligibility(
    Path((round_id, address_str)): Path<(u32, String)>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    let address: Address = address_str.parse()
        .map_err(|e| AppError::InvalidInput(format!("Invalid address: {}", e)))?;

    match service.get_eligibility(round_id, address).await? {
        Some(amount) => Ok(Json(json!({
            "eligible": true,
            "round_id": round_id,
            "address": address_str,
            "amount": amount.to_string()
        }))),
        None => Ok(Json(json!({
            "eligible": false,
            "round_id": round_id,
            "address": address_str,
            "amount": "0"
        })))
    }
}

pub async fn get_trie_info(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<TrieInfoResponse>> {
    match service.get_trie_info(round_id).await? {
        Some(info) => Ok(Json(TrieInfoResponse {
            round_id,
            root_hash: info.root_hash,
            entry_count: info.metadata.entry_count,
            created_at: info.metadata.created_at.to_rfc3339(),
            updated_at: info.metadata.updated_at.to_rfc3339(),
        })),
        None => Err(AppError::NotFound(format!("No trie info found for round {}", round_id)))
    }
}
