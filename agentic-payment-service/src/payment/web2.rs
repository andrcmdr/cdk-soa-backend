use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::Web2Config;
use super::{PaymentGateway, GatewayPaymentRequest, GatewayPaymentResponse, GatewayStatus};

/// Web2 Payment Gateway
/// 
/// Handles traditional payment processing through providers like Stripe, PayPal
/// Supports: credit cards, bank transfers, digital wallets
#[derive(Clone)]
pub struct Web2Gateway {
    config: Web2Config,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct StripePaymentIntent {
    amount: i64,  // in cents
    currency: String,
    payment_method_types: Vec<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct StripePaymentResponse {
    id: String,
    status: String,
    amount: i64,
    client_secret: Option<String>,
}

impl Web2Gateway {
    pub fn new(config: Web2Config) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }

    async fn create_payment_intent(&self, req: StripePaymentIntent) -> Result<StripePaymentResponse> {
        let url = match self.config.provider.as_str() {
            "stripe" => "https://api.stripe.com/v1/payment_intents",
            "paypal" => "https://api.paypal.com/v2/payments",
            _ => anyhow::bail!("Unsupported provider: {}", self.config.provider),
        };

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .form(&req)
            .send()
            .await
            .context("Failed to create payment intent")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Payment intent creation failed with status {}: {}", status, error_text);
        }

        response
            .json::<StripePaymentResponse>()
            .await
            .context("Failed to parse payment response")
    }

    async fn retrieve_payment_intent(&self, payment_id: &str) -> Result<StripePaymentResponse> {
        let url = match self.config.provider.as_str() {
            "stripe" => format!("https://api.stripe.com/v1/payment_intents/{}", payment_id),
            "paypal" => format!("https://api.paypal.com/v2/payments/{}", payment_id),
            _ => anyhow::bail!("Unsupported provider: {}", self.config.provider),
        };

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .context("Failed to retrieve payment intent")?;

        response
            .json::<StripePaymentResponse>()
            .await
            .context("Failed to parse payment response")
    }

    fn map_status(status: &str) -> GatewayStatus {
        match status.to_lowercase().as_str() {
            "requires_payment_method" | "requires_confirmation" => GatewayStatus::Initiated,
            "processing" | "requires_action" => GatewayStatus::Pending,
            "succeeded" => GatewayStatus::Confirmed,
            "canceled" | "failed" => GatewayStatus::Failed,
            _ => GatewayStatus::Initiated,
        }
    }

    fn to_cents(amount: f64) -> i64 {
        (amount * 100.0) as i64
    }
}

#[async_trait]
impl PaymentGateway for Web2Gateway {
    async fn execute_payment(&self, request: GatewayPaymentRequest) -> Result<GatewayPaymentResponse> {
        tracing::info!("Executing Web2 payment via {}: {} {}",
            self.config.provider, request.amount, request.currency);

        let payment_intent = StripePaymentIntent {
            amount: Self::to_cents(request.amount),
            currency: request.currency.to_lowercase(),
            payment_method_types: vec!["card".to_string()],
            metadata: request.metadata.clone(),
        };

        let result = self.create_payment_intent(payment_intent).await?;
        
        let confirmation_url = result.client_secret.as_ref().map(|secret| {
            format!("https://checkout.stripe.com/pay/{}", secret)
        });

        Ok(GatewayPaymentResponse {
            transaction_hash: result.id,
            status: Self::map_status(&result.status),
            confirmation_url,
            estimated_completion: Some("Instant".to_string()),
            fees: Some(request.amount * 0.029 + 0.30), // Stripe's typical fee
        })
    }

    async fn verify_transaction(&self, tx_hash: &str) -> Result<GatewayStatus> {
        let payment = self.retrieve_payment_intent(tx_hash).await?;
        Ok(Self::map_status(&payment.status))
    }

    async fn estimate_fees(&self, amount: f64, _currency: &str) -> Result<f64> {
        // Typical Stripe fee structure: 2.9% + $0.30
        Ok(amount * 0.029 + 0.30)
    }

    fn gateway_name(&self) -> &str {
        &self.config.provider
    }
}