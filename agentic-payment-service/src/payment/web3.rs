use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::Web3Config;
use super::{PaymentGateway, GatewayPaymentRequest, GatewayPaymentResponse, GatewayStatus};

/// Web3 Payment Gateway
/// 
/// Handles blockchain transactions on Ethereum and compatible chains
/// Supports: ETH transfers, ERC-20 tokens, smart contract interactions
#[derive(Clone)]
pub struct Web3Gateway {
    config: Web3Config,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct Web3TransactionRequest {
    from: String,
    to: String,
    value: String,  // in Wei
    gas: String,
    gas_price: Option<String>,
    data: Option<String>,
    chain_id: u64,
}

#[derive(Debug, Deserialize)]
struct Web3TransactionResponse {
    tx_hash: String,
    status: String,
    block_number: Option<u64>,
    confirmations: u32,
}

impl Web3Gateway {
    pub fn new(config: Web3Config) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }

    async fn send_transaction(&self, tx: Web3TransactionRequest) -> Result<Web3TransactionResponse> {
        // Call blockchain RPC endpoint
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [tx],
            "id": 1
        });

        let response = self.client
            .post(&self.config.rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .context("Failed to send Web3 transaction")?;

        let rpc_response: serde_json::Value = response.json().await?;
        
        if let Some(error) = rpc_response.get("error") {
            anyhow::bail!("Web3 RPC error: {}", error);
        }

        let tx_hash = rpc_response["result"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid transaction hash"))?
            .to_string();

        Ok(Web3TransactionResponse {
            tx_hash,
            status: "pending".to_string(),
            block_number: None,
            confirmations: 0,
        })
    }

    async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<Web3TransactionResponse> {
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });

        let response = self.client
            .post(&self.config.rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .context("Failed to get transaction receipt")?;

        let rpc_response: serde_json::Value = response.json().await?;
        
        let result = &rpc_response["result"];
        
        if result.is_null() {
            return Ok(Web3TransactionResponse {
                tx_hash: tx_hash.to_string(),
                status: "pending".to_string(),
                block_number: None,
                confirmations: 0,
            });
        }

        let status = if result["status"].as_str() == Some("0x1") {
            "confirmed"
        } else {
            "failed"
        };

        Ok(Web3TransactionResponse {
            tx_hash: tx_hash.to_string(),
            status: status.to_string(),
            block_number: result["blockNumber"].as_u64(),
            confirmations: 1,
        })
    }

    fn wei_to_eth(wei: f64) -> String {
        format!("0x{:x}", (wei * 1e18) as u128)
    }

    fn map_status(status: &str) -> GatewayStatus {
        match status {
            "pending" => GatewayStatus::Pending,
            "confirmed" => GatewayStatus::Confirmed,
            "failed" => GatewayStatus::Failed,
            _ => GatewayStatus::Initiated,
        }
    }
}

#[async_trait]
impl PaymentGateway for Web3Gateway {
    async fn execute_payment(&self, request: GatewayPaymentRequest) -> Result<GatewayPaymentResponse> {
        tracing::info!("Executing Web3 payment: {} {} from {} to {}",
            request.amount, request.currency, request.from, request.to);

        let tx = Web3TransactionRequest {
            from: request.from.clone(),
            to: request.to.clone(),
            value: Self::wei_to_eth(request.amount),
            gas: format!("0x{:x}", self.config.gas_limit),
            gas_price: None,  // Use network default
            data: None,
            chain_id: self.config.chain_id,
        };

        let result = self.send_transaction(tx).await?;
        
        // Estimate fees (simplified)
        let fees = self.estimate_fees(request.amount, &request.currency).await?;

        Ok(GatewayPaymentResponse {
            transaction_hash: result.tx_hash,
            status: Self::map_status(&result.status),
            confirmation_url: Some(format!("https://etherscan.io/tx/{}", result.tx_hash)),
            estimated_completion: Some("2-5 minutes".to_string()),
            fees: Some(fees),
        })
    }

    async fn verify_transaction(&self, tx_hash: &str) -> Result<GatewayStatus> {
        let receipt = self.get_transaction_receipt(tx_hash).await?;
        Ok(Self::map_status(&receipt.status))
    }

    async fn estimate_fees(&self, _amount: f64, _currency: &str) -> Result<f64> {
        // Simplified fee estimation
        // In production, query gas price and calculate: gasPrice * gasLimit
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        });

        let response = self.client
            .post(&self.config.rpc_url)
            .json(&rpc_request)
            .send()
            .await?;

        let rpc_response: serde_json::Value = response.json().await?;
        let gas_price_hex = rpc_response["result"].as_str().unwrap_or("0x0");
        let gas_price = u64::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
            .unwrap_or(20_000_000_000); // 20 Gwei default

        let fee_wei = gas_price * self.config.gas_limit;
        let fee_eth = fee_wei as f64 / 1e18;
        
        Ok(fee_eth)
    }

    fn gateway_name(&self) -> &str {
        "web3"
    }
}