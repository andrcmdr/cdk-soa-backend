use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::X402Config;
use super::{PaymentProtocol, PaymentRequest, PaymentResponse, PaymentStatus};

/// X402 Protocol Implementation
/// 
/// X402 is a payment protocol for agent-to-agent transactions
/// Features: atomic transfers, smart routing, multi-currency support
#[derive(Clone)]
pub struct X402Protocol {
    config: X402Config,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct X402PaymentPayload {
    version: String,
    amount: f64,
    currency: String,
    sender_id: String,
    recipient_id: String,
    memo: Option<String>,
    callback_url: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct X402PaymentResult {
    transaction_id: String,
    status: String,
    timestamp: String,
    message: String,
}

impl X402Protocol {
    pub fn new(config: X402Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { config, client })
    }

    async fn send_request(&self, payload: X402PaymentPayload) -> Result<X402PaymentResult> {
        let url = format!("{}/v1/payment", self.config.endpoint);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("X-Protocol-Version", &self.config.version)
            .json(&payload)
            .send()
            .await
            .context("Failed to send X402 request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("X402 request failed with status {}: {}", status, error_text);
        }

        response
            .json::<X402PaymentResult>()
            .await
            .context("Failed to parse X402 response")
    }

    fn map_status(status: &str) -> PaymentStatus {
        match status.to_lowercase().as_str() {
            "pending" => PaymentStatus::Pending,
            "processing" => PaymentStatus::Processing,
            "completed" | "success" => PaymentStatus::Completed,
            "failed" | "error" => PaymentStatus::Failed,
            "cancelled" => PaymentStatus::Cancelled,
            _ => PaymentStatus::Pending,
        }
    }
}

#[async_trait]
impl PaymentProtocol for X402Protocol {
    async fn process_payment(&self, request: PaymentRequest) -> Result<PaymentResponse> {
        tracing::info!("Processing X402 payment: {}", request.id);

        let payload = X402PaymentPayload {
            version: self.config.version.clone(),
            amount: request.amount,
            currency: request.currency.clone(),
            sender_id: request.sender.clone(),
            recipient_id: request.recipient.clone(),
            memo: request.memo.clone(),
            callback_url: None,
            metadata: request.metadata.clone(),
        };

        let mut retries = 0;
        let result = loop {
            match self.send_request(payload.clone()).await {
                Ok(result) => break result,
                Err(e) => {
                    retries += 1;
                    if retries >= self.config.max_retries {
                        return Err(e);
                    }
                    tracing::warn!("X402 request failed, retry {}/{}: {}", 
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
                "protocol": "x402",
                "version": self.config.version,
                "timestamp": result.timestamp,
            }),
        })
    }

    async fn check_status(&self, transaction_id: &str) -> Result<PaymentStatus> {
        let url = format!("{}/v1/payment/{}", self.config.endpoint, transaction_id);
        
        let response = self.client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .send()
            .await
            .context("Failed to check X402 status")?;

        let result: X402PaymentResult = response.json().await?;
        Ok(Self::map_status(&result.status))
    }

    async fn cancel_payment(&self, transaction_id: &str) -> Result<bool> {
        let url = format!("{}/v1/payment/{}/cancel", self.config.endpoint, transaction_id);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .send()
            .await
            .context("Failed to cancel X402 payment")?;

        Ok(response.status().is_success())
    }

    fn protocol_name(&self) -> &str {
        "x402"
    }

    fn protocol_version(&self) -> &str {
        &self.config.version
    }
}