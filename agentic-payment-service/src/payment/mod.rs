use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod web3;
pub mod web2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPaymentRequest {
    pub amount: f64,
    pub currency: String,
    pub from: String,
    pub to: String,
    pub memo: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPaymentResponse {
    pub transaction_hash: String,
    pub status: GatewayStatus,
    pub confirmation_url: Option<String>,
    pub estimated_completion: Option<String>,
    pub fees: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GatewayStatus {
    Initiated,
    Pending,
    Confirmed,
    Failed,
}

#[async_trait]
pub trait PaymentGateway: Send + Sync {
    async fn execute_payment(&self, request: GatewayPaymentRequest) -> Result<GatewayPaymentResponse>;
    async fn verify_transaction(&self, tx_hash: &str) -> Result<GatewayStatus>;
    async fn estimate_fees(&self, amount: f64, currency: &str) -> Result<f64>;
    fn gateway_name(&self) -> &str;
}

pub struct PaymentGatewayManager {
    gateways: HashMap<String, Box<dyn PaymentGateway>>,
}

impl PaymentGatewayManager {
    pub fn new() -> Self {
        Self {
            gateways: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, gateway: Box<dyn PaymentGateway>) {
        self.gateways.insert(name.to_string(), gateway);
    }

    pub fn get(&self, name: &str) -> Result<&Box<dyn PaymentGateway>> {
        self.gateways
            .get(name)
            .ok_or_else(|| anyhow!("Gateway '{}' not found", name))
    }

    pub async fn execute_payment(
        &self,
        gateway_name: &str,
        request: GatewayPaymentRequest,
    ) -> Result<GatewayPaymentResponse> {
        let gateway = self.get(gateway_name)?;
        gateway.execute_payment(request).await
    }

    pub fn list_gateways(&self) -> Vec<String> {
        self.gateways.keys().cloned().collect()
    }
}

impl Default for PaymentGatewayManager {
    fn default() -> Self {
        Self::new()
    }
}
