use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::AP2Config;
use super::{PaymentProtocol, PaymentRequest, PaymentResponse, PaymentStatus};

/// AP2 (Agent Payment Protocol v2) Implementation
/// 
/// AP2 is an advanced payment protocol with:
/// - Multi-party settlements
/// - Conditional payments (escrow)
/// - Cross-chain support
#[derive(Clone)]
pub struct AP2Protocol {
    config: AP2Config,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct AP2TransactionRequest {
    protocol_version: String,
    transaction_type: String,  // "direct", "escrow", "batch"
    parties: Vec<AP2Party>,
    amount: f64,
    currency: String,
    conditions: Option<Vec<String>>,
    expiry: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct AP2Party {
    party_id: String,
    role: String,  // "sender", "recipient", "intermediary"
    amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AP2TransactionResponse {
    transaction_id: String,
    status: String,
    parties_confirmed: Vec<String>,
    created_at: String,
    message: String,
    settlement_details: Option<serde_json::Value>,
}

impl AP2Protocol {
    pub fn new(config: AP2Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { config, client })
    }

    async fn send_transaction(&self, req: AP2TransactionRequest) -> Result<AP2TransactionResponse> {
        let url = format!("{}/api/v2/transaction", self.config.endpoint);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("AP2-Version", &self.config.version)
            .json(&req)
            .send()
            .await
            .context("Failed to send AP2 transaction")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("AP2 transaction failed with status {}: {}", status, error_text);
        }

        response
            .json::<AP2TransactionResponse>()
            .await
            .context("Failed to parse AP2 response")
    }

    fn map_status(status: &str) -> PaymentStatus {
        match status.to_lowercase().as_str() {
            "initiated" | "pending_confirmation" => PaymentStatus::Pending,
            "processing" | "settling" => PaymentStatus::Processing,
            "settled" | "completed" => PaymentStatus::Completed,
            "failed" | "rejected" => PaymentStatus::Failed,
            "cancelled" | "expired" => PaymentStatus::Cancelled,
            _ => PaymentStatus::Pending,
        }
    }
}

#[async_trait]
impl PaymentProtocol for AP2Protocol {
    async fn process_payment(&self, request: PaymentRequest) -> Result<PaymentResponse> {
        tracing::info!("Processing AP2 payment: {}", request.id);

        let parties = vec![
            AP2Party {
                party_id: request.sender.clone(),
                role: "sender".to_string(),
                amount: Some(request.amount),
            },
            AP2Party {
                party_id: request.recipient.clone(),
                role: "recipient".to_string(),
                amount: Some(request.amount),
            },
        ];

        let transaction_req = AP2TransactionRequest {
            protocol_version: self.config.version.clone(),
            transaction_type: "direct".to_string(),
            parties,
            amount: request.amount,
            currency: request.currency.clone(),
            conditions: None,
            expiry: None,
            metadata: request.metadata.clone(),
        };

        let mut retries = 0;
        let result = loop {
            match self.send_transaction(transaction_req.clone()).await {
                Ok(result) => break result,
                Err(e) => {
                    retries += 1;
                    if retries >= self.config.max_retries {
                        return Err(e);
                    }
                    tracing::warn!("AP2 transaction failed, retry {}/{}: {}", 
                        retries, self.config.max_retries, e);
                    tokio::time::sleep(Duration::from_secs(2u64.pow(retries))).await;
                }
            }
        };

        let status = Self::map_status(&result.status);
        
        Ok(PaymentResponse {
            transaction_id: result.transaction_id,
            status,
            message: result.message,
            protocol_data: serde_json::json!({
                "protocol": "ap2",
                "version": self.config.version,
                "parties_confirmed": result.parties_confirmed,
                "created_at": result.created_at,
                "settlement_details": result.settlement_details,
            }),
        })
    }

    async fn check_status(&self, transaction_id: &str) -> Result<PaymentStatus> {
        let url = format!("{}/api/v2/transaction/{}", self.config.endpoint, transaction_id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .context("Failed to check AP2 status")?;

        let result: AP2TransactionResponse = response.json().await?;
        Ok(Self::map_status(&result.status))
    }

    async fn cancel_payment(&self, transaction_id: &str) -> Result<bool> {
        let url = format!("{}/api/v2/transaction/{}/cancel", 
            self.config.endpoint, transaction_id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .context("Failed to cancel AP2 payment")?;

        Ok(response.status().is_success())
    }

    fn protocol_name(&self) -> &str {
        "ap2"
    }

    fn protocol_version(&self) -> &str {
        &self.config.version
    }
}