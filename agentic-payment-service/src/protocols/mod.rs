use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod x402;
pub mod ap2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub id: String,
    pub amount: f64,
    pub currency: String,
    pub sender: String,
    pub recipient: String,
    pub memo: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub transaction_id: String,
    pub status: PaymentStatus,
    pub message: String,
    pub protocol_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[async_trait]
pub trait PaymentProtocol: Send + Sync {
    async fn process_payment(&self, request: PaymentRequest) -> Result<PaymentResponse>;
    async fn check_status(&self, transaction_id: &str) -> Result<PaymentStatus>;
    async fn cancel_payment(&self, transaction_id: &str) -> Result<bool>;
    fn protocol_name(&self) -> &str;
    fn protocol_version(&self) -> &str;
}

pub struct ProtocolManager {
    protocols: HashMap<String, Box<dyn PaymentProtocol>>,
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            protocols: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, protocol: Box<dyn PaymentProtocol>) {
        self.protocols.insert(name.to_string(), protocol);
    }

    pub fn get(&self, name: &str) -> Result<&Box<dyn PaymentProtocol>> {
        self.protocols
            .get(name)
            .ok_or_else(|| anyhow!("Protocol '{}' not found", name))
    }

    pub async fn process_payment(
        &self,
        protocol_name: &str,
        request: PaymentRequest,
    ) -> Result<PaymentResponse> {
        let protocol = self.get(protocol_name)?;
        protocol.process_payment(request).await
    }

    pub fn list_protocols(&self) -> Vec<String> {
        self.protocols.keys().cloned().collect()
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}
