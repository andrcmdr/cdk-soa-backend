use std::collections::HashMap;
use anyhow::Result;
use alloy_primitives::{Address, B256, U256};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalEligibilityData {
    pub eligibility: HashMap<String, String>, // address -> amount as strings
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTrieData {
    pub round_id: u32,
    pub root_hash: String,
    pub trie_data: String, // Can be hex, base64, or JSON
    pub format: String, // "hex", "base64", or "json"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTrieInfo {
    pub round_id: u32,
    pub root_hash: B256,
    pub trie_data: Vec<u8>,
    pub merkle_proofs: Option<HashMap<String, Vec<String>>>, // address -> proof
}

pub struct ExternalBackendClient {
    client: Client,
}

impl ExternalBackendClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_eligibility_data(&self, url: &str) -> AppResult<HashMap<Address, U256>> {
        tracing::info!("Fetching eligibility data from: {}", url);

        let response = self.client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let external_data: ExternalEligibilityData = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JSON parsing failed: {}", e)))?;

        // Convert string addresses and amounts to proper types
        let mut eligibility_data = HashMap::new();
        for (address_str, amount_str) in external_data.eligibility {
            let address: Address = address_str.parse()
                .map_err(|e| AppError::InvalidInput(format!("Invalid address '{}': {}", address_str, e)))?;

            let amount: U256 = amount_str.parse()
                .map_err(|e| AppError::InvalidInput(format!("Invalid amount '{}': {}", amount_str, e)))?;

            eligibility_data.insert(address, amount);
        }

        tracing::info!("Fetched {} eligibility records", eligibility_data.len());
        Ok(eligibility_data)
    }

    pub async fn fetch_trie_data(&self, url: &str) -> AppResult<ExternalTrieInfo> {
        tracing::info!("Fetching trie data from: {}", url);

        let response = self.client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let external_data: ExternalTrieData = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JSON parsing failed: {}", e)))?;

        // Parse root hash
        let root_hash = if external_data.root_hash.starts_with("0x") {
            B256::from_slice(&hex::decode(&external_data.root_hash[2..])
                .map_err(|e| AppError::InvalidInput(format!("Invalid root hash hex: {}", e)))?)
        } else {
            B256::from_slice(&hex::decode(&external_data.root_hash)
                .map_err(|e| AppError::InvalidInput(format!("Invalid root hash hex: {}", e)))?)
        };

        // Parse trie data based on format
        let trie_data = match external_data.format.as_str() {
            "hex" => {
                if external_data.trie_data.starts_with("0x") {
                    hex::decode(&external_data.trie_data[2..])
                        .map_err(|e| AppError::InvalidInput(format!("Invalid hex data: {}", e)))?
                } else {
                    hex::decode(&external_data.trie_data)
                        .map_err(|e| AppError::InvalidInput(format!("Invalid hex data: {}", e)))?
                }
            }
            "base64" => {
                base64::decode(&external_data.trie_data)
                    .map_err(|e| AppError::InvalidInput(format!("Invalid base64 data: {}", e)))?
            }
            "json" => {
                // Assume JSON contains binary data as array of bytes
                let json_data: Vec<u8> = serde_json::from_str(&external_data.trie_data)
                    .map_err(|e| AppError::InvalidInput(format!("Invalid JSON data: {}", e)))?;
                json_data
            }
            _ => return Err(AppError::InvalidInput(format!("Unsupported format: {}", external_data.format)))
        };

        Ok(ExternalTrieInfo {
            round_id: external_data.round_id,
            root_hash,
            trie_data,
            merkle_proofs: None, // Could be extended to include proofs
        })
    }

    pub async fn post_eligibility_data(&self, url: &str, eligibility_data: &HashMap<Address, U256>) -> AppResult<()> {
        tracing::info!("Posting eligibility data to: {}", url);

        // Convert to string format for JSON
        let mut string_data = HashMap::new();
        for (address, amount) in eligibility_data {
            string_data.insert(format!("0x{}", hex::encode(address)), amount.to_string());
        }

        let payload = ExternalEligibilityData {
            eligibility: string_data,
        };

        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        tracing::info!("Successfully posted eligibility data");
        Ok(())
    }
}
