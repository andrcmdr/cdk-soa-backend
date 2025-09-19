use axum::{
    extract::{Path, State, Multipart, Query},
    response::Json,
    http::{StatusCode, header},
    body::Body,
    response::{Response, IntoResponse},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use alloy_primitives::{Address, U256, B256};

use crate::service::AirdropService;
use crate::error::{AppError, AppResult};
use crate::database::ProcessingLog;
use crate::contract_client::RoundMetadata;

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

#[derive(Serialize, Deserialize)]
pub struct EligibilityDataJson {
    pub eligibility: HashMap<String, String>, // address -> amount as strings
}

#[derive(Serialize, Deserialize)]
pub struct TrieDataRequest {
    pub round_id: u32,
    pub root_hash: String,
    pub trie_data: String,
    pub format: String, // "hex", "base64", or "json"
}

#[derive(Serialize, Deserialize)]
pub struct TrieDataResponse {
    pub round_id: u32,
    pub root_hash: String,
    pub trie_data: String,
    pub format: String,
    pub merkle_proofs: HashMap<String, Vec<String>>, // address -> proof
}

#[derive(Serialize, Deserialize)]
pub struct ExternalDataRequest {
    pub external_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct ComparisonResult {
    pub matches: bool,
    pub local_root_hash: String,
    pub external_root_hash: String,
    pub differences: Vec<String>,
}

#[derive(Serialize)]
pub struct TrieInfoResponse {
    pub round_id: u32,
    pub root_hash: String,
    pub entry_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct RoundStatistics {
    pub round_id: u32,
    pub entry_count: i32,
    pub last_updated: String,
}

#[derive(Serialize)]
pub struct ContractInfoResponse {
    pub contract_address: String,
    pub contract_version: String,
    pub round_count: String,
    pub interface_type: String,
}

#[derive(Serialize)]
pub struct RoundMetadataResponse {
    pub round_id: String,
    pub root_hash: String,
    pub total_eligible: String,
    pub total_amount: String,
    pub start_time: String,
    pub end_time: String,
    pub is_active: bool,
    pub metadata_uri: String,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub round_id: Option<u32>,
}

#[derive(Deserialize)]
pub struct FormatQuery {
    pub format: Option<String>, // "json", "hex", "base64"
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "airdrop-backend"
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
        "round_id": round_id,
        "data_size_bytes": csv_data.len()
    })))
}

pub async fn download_csv(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Response<Body>> {
    let csv_data = service.get_round_csv_data(round_id).await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv")
        .header(header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"round_{}_eligibility.csv\"", round_id))
        .body(Body::from(csv_data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create response: {}", e)))?;

    Ok(response)
}

pub async fn upload_json_eligibility(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
    Json(payload): Json<EligibilityDataJson>,
) -> AppResult<Json<serde_json::Value>> {
    // Convert string addresses and amounts to proper types
    let mut eligibility_data = HashMap::new();
    for (address_str, amount_str) in payload.eligibility {
        let address: Address = address_str.parse()
            .map_err(|e| AppError::InvalidInput(format!("Invalid address '{}': {}", address_str, e)))?;

        let amount: U256 = amount_str.parse()
            .map_err(|e| AppError::InvalidInput(format!("Invalid amount '{}': {}", amount_str, e)))?;

        eligibility_data.insert(address, amount);
    }

    service.process_json_eligibility_data(eligibility_data, round_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("JSON eligibility data processed for round {}", round_id),
        "round_id": round_id
    })))
}

pub async fn download_json_eligibility(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<EligibilityDataJson>> {
    let eligibility_data = service.get_round_eligibility_records(round_id).await?;

    // Convert to string format for JSON
    let mut string_data = HashMap::new();
    for (address, amount) in eligibility_data {
        string_data.insert(format!("0x{}", hex::encode(address)), amount.to_string());
    }

    Ok(Json(EligibilityDataJson {
        eligibility: string_data,
    }))
}

pub async fn download_trie_data(
    Path(round_id): Path<u32>,
    Query(params): Query<FormatQuery>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<TrieDataResponse>> {
    let format = params.format.unwrap_or_else(|| "json".to_string());

    let trie_state = service.get_trie_info(round_id).await?
        .ok_or_else(|| AppError::NotFound(format!("No trie data found for round {}", round_id)))?;

    // Generate merkle proofs for all addresses
    let eligibility_data = service.get_round_eligibility_records(round_id).await?;
    let mut merkle_proofs = HashMap::new();

    for address in eligibility_data.keys() {
        match service.get_merkle_proof_for_address(round_id, *address).await {
            Ok(proof) => {
                let proof_strings: Vec<String> = proof.iter()
                    .map(|p| format!("0x{}", hex::encode(p)))
                    .collect();
                merkle_proofs.insert(format!("0x{}", hex::encode(address)), proof_strings);
            }
            Err(e) => {
                tracing::warn!("Failed to generate proof for address {}: {}", address, e);
            }
        }
    }

    let trie_data_formatted = match format.as_str() {
        "hex" => format!("0x{}", hex::encode(&trie_state.trie_data)),
        "base64" => base64::encode(&trie_state.trie_data),
        "json" => serde_json::to_string(&trie_state.trie_data)
            .map_err(|e| AppError::Serialization(e))?,
        _ => return Err(AppError::InvalidInput(format!("Unsupported format: {}", format)))
    };

    Ok(Json(TrieDataResponse {
        round_id,
        root_hash: format!("0x{}", hex::encode(trie_state.root_hash)),
        trie_data: trie_data_formatted,
        format,
        merkle_proofs,
    }))
}

pub async fn upload_and_compare_trie_data(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
    Json(payload): Json<TrieDataRequest>,
) -> AppResult<Json<ComparisonResult>> {
    if payload.round_id != round_id {
        return Err(AppError::InvalidInput("Round ID mismatch".to_string()));
    }

    // Parse external root hash
    let external_root_hash = if payload.root_hash.starts_with("0x") {
        B256::from_slice(&hex::decode(&payload.root_hash[2..])
            .map_err(|e| AppError::InvalidInput(format!("Invalid root hash hex: {}", e)))?)
    } else {
        B256::from_slice(&hex::decode(&payload.root_hash)
            .map_err(|e| AppError::InvalidInput(format!("Invalid root hash hex: {}", e)))?)
    };

    // Parse external trie data based on format
    let external_trie_data = match payload.format.as_str() {
        "hex" => {
            if payload.trie_data.starts_with("0x") {
                hex::decode(&payload.trie_data[2..])
                    .map_err(|e| AppError::InvalidInput(format!("Invalid hex data: {}", e)))?
            } else {
                hex::decode(&payload.trie_data)
                    .map_err(|e| AppError::InvalidInput(format!("Invalid hex data: {}", e)))?
            }
        }
        "base64" => {
            base64::decode(&payload.trie_data)
                .map_err(|e| AppError::InvalidInput(format!("Invalid base64 data: {}", e)))?
        }
        "json" => {
            let json_data: Vec<u8> = serde_json::from_str(&payload.trie_data)
                .map_err(|e| AppError::InvalidInput(format!("Invalid JSON data: {}", e)))?;
            json_data
        }
        _ => return Err(AppError::InvalidInput(format!("Unsupported format: {}", payload.format)))
    };

    // Compare with local data
    let matches = service.compare_external_trie_data(round_id, &external_trie_data, external_root_hash).await?;

    // Get local data for comparison details
    let local_trie_state = service.get_trie_info(round_id).await?
        .ok_or_else(|| AppError::NotFound(format!("No local trie data found for round {}", round_id)))?;

    let mut differences = Vec::new();
    if local_trie_state.root_hash != external_root_hash {
        differences.push("Root hash mismatch".to_string());
    }
    if local_trie_state.trie_data != external_trie_data {
        differences.push("Trie data mismatch".to_string());
    }

    Ok(Json(ComparisonResult {
        matches,
        local_root_hash: format!("0x{}", hex::encode(local_trie_state.root_hash)),
        external_root_hash: format!("0x{}", hex::encode(external_root_hash)),
        differences,
    }))
}

pub async fn fetch_external_data_and_update(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
    Json(payload): Json<ExternalDataRequest>,
) -> AppResult<Json<serde_json::Value>> {
    service.fetch_and_update_from_external(round_id, &payload.external_url).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Successfully updated round {} with external data", round_id),
        "round_id": round_id,
        "external_url": payload.external_url
    })))
}

pub async fn fetch_and_compare_external_trie(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
    Json(payload): Json<ExternalDataRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let matches = service.fetch_and_compare_external_trie(round_id, &payload.external_url).await?;

    Ok(Json(json!({
        "success": true,
        "comparison_result": matches,
        "message": if matches { "Trie data matches" } else { "Trie data differs" },
        "round_id": round_id,
        "external_url": payload.external_url
    })))
}

// This endpoint can be used to manually trigger trie updates
// The trie should already be updated when CSV is processed
pub async fn update_trie(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    let trie_info = service.get_trie_info(round_id).await?;

    match trie_info {
        Some(info) => Ok(Json(json!({
            "success": true,
            "message": format!("Trie for round {} is up to date", round_id),
            "round_id": round_id,
            "root_hash": format!("0x{}", hex::encode(info.root_hash)),
            "entry_count": info.entry_count,
            "last_updated": info.updated_at.to_rfc3339()
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
            root_hash: format!("0x{}", hex::encode(info.root_hash)),
            entry_count: info.entry_count,
            created_at: info.created_at.to_rfc3339(),
            updated_at: info.updated_at.to_rfc3339(),
        })),
        None => Err(AppError::NotFound(format!("No trie info found for round {}", round_id)))
    }
}

pub async fn get_round_statistics(
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<Vec<RoundStatistics>>> {
    let stats = service.get_all_round_statistics().await?;

    let response: Vec<RoundStatistics> = stats
        .into_iter()
        .map(|(round_id, entry_count, last_updated)| RoundStatistics {
            round_id,
            entry_count,
            last_updated: last_updated.to_rfc3339(),
        })
        .collect();

    Ok(Json(response))
}

pub async fn get_processing_logs(
    Query(params): Query<LogsQuery>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<Vec<ProcessingLog>>> {
    let logs = service.get_processing_logs(params.round_id).await?;
    Ok(Json(logs))
}

pub async fn get_round_processing_logs(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<Vec<ProcessingLog>>> {
    let logs = service.get_processing_logs(Some(round_id)).await?;
    Ok(Json(logs))
}

pub async fn delete_round(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    service.delete_round(round_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Round {} deleted successfully", round_id),
        "round_id": round_id
    })))
}

// Contract information endpoints
pub async fn get_contract_info(
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<ContractInfoResponse>> {
    let contract_version = service.get_contract_version().await?;
    let round_count = service.get_round_count().await?;

    Ok(Json(ContractInfoResponse {
        contract_address: format!("0x{}", hex::encode(service.get_contract_address())),
        contract_version,
        round_count: round_count.to_string(),
        interface_type: service.get_contract_interface_type().to_string(),
    }))
}

pub async fn check_round_active(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    let is_active = service.is_round_active(round_id).await?;

    Ok(Json(json!({
        "round_id": round_id,
        "is_active": is_active
    })))
}

pub async fn get_round_metadata(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<RoundMetadataResponse>> {
    let metadata = service.get_round_metadata(round_id).await?;

    Ok(Json(RoundMetadataResponse {
        round_id: metadata.round_id.to_string(),
        root_hash: format!("0x{}", hex::encode(metadata.root_hash)),
        total_eligible: metadata.total_eligible.to_string(),
        total_amount: metadata.total_amount.to_string(),
        start_time: metadata.start_time.to_string(),
        end_time: metadata.end_time.to_string(),
        is_active: metadata.is_active,
        metadata_uri: metadata.metadata_uri,
    }))
}

pub async fn validate_consistency(
    Path(round_id): Path<u32>,
    State(service): State<Arc<AirdropService>>,
) -> AppResult<Json<serde_json::Value>> {
    let is_consistent = service.validate_on_chain_consistency(round_id).await?;

    Ok(Json(json!({
        "round_id": round_id,
        "is_consistent": is_consistent,
        "message": if is_consistent {
            "Local trie root matches on-chain root"
        } else {
            "Local trie root does not match on-chain root"
        }
    })))
}
