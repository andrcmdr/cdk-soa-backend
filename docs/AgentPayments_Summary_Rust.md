## AP2 (Agent Payments Protocol)

**AP2** is an open standard protocol developed by Google that enables AI agents to initiate and complete secure, auditable purchases on behalf of users [[1]](https://www.lightspark.com/news/insights/agent-payments-protocol). Key features include:

- **Open Standard**: Payment-agnostic protocol for agent-driven commerce [[2]](https://www.cmswire.com/digital-experience/google-pushes-standards-for-agentic-ai-commerce-with-ap2/)
- **Security**: Built-in transparency and governance features with cryptographic primitives like ECDSA [[3]](https://kenhuangus.substack.com/p/secure-use-of-the-google-agent-payments)
- **Role-based Architecture**: Separates roles and anchors consent in signed mandates [[4]](https://developer.paypal.com/community/blog/PayPal-Agent-Payments-Protocol/)
- **Mandate System**: Uses Intent Mandates and Cart Mandates for secure transaction authorization [[5]](https://dev.to/vishalmysore/what-is-google-ap2-protocol-step-by-step-guide-with-examples-2lcp)

### AP2 Core Components:
1. **Intent Mandate**: User authorization for spending within limits
2. **Cart Mandate**: Specific items and pricing approval
3. **Payment Mandate**: Final payment execution with verification

## x402 Protocol

**x402** is a protocol that activates the HTTP 402 "Payment Required" status code to enable instant, blockchain-based payments for web resources and APIs [[6]](https://blog.thirdweb.com/what-is-x402-protocol-the-http-based-payment-standard-for-onchain-commerce/). Key characteristics:

- **HTTP-based**: Built around HTTP 402 status code for seamless integration [[7]](https://docs.base.org/base-app/agents/x402-agents)
- **Chain Agnostic**: Standard for payments on top of HTTP [[8]](https://github.com/coinbase/x402)
- **Zero Fees**: No protocol fees for customers or merchants [[9]](https://www.x402.org/)
- **Instant Payments**: Enables real-time micropayments without registration or complex authentication [[10]](https://docs.cdp.coinbase.com/x402/docs/welcome)

### x402 Technical Specifications:
- Uses standard HTTP headers for payment communication
- Supports USDC payments on Base network
- Compatible with existing web infrastructure
- Enables programmatic payments for AI agents [[11]](https://vercel.com/blog/introducing-x402-mcp-open-protocol-payments-for-mcp-tools)

## Rust Integration

For Rust development, there are several options available:

### x402 Rust Implementation

The **x402-rs** crate provides a Rust implementation for the x402 protocol [[12]](https://github.com/x402-rs/x402-rs):

```rust
// Add to Cargo.toml
[dependencies]
x402-rs = "0.3.1"
```

Here's a basic example of integrating x402 in Rust:

```rust
use x402_rs::{PaymentRequest, PaymentResponse};
use serde_json::json;
use reqwest;

// Example middleware for protected routes
async fn handle_payment_required_endpoint(
    payment_amount: &str,
    resource: &str,
    pay_to: &str,
    asset: &str,
    network: &str,
) -> Result<PaymentResponse, Box<dyn std::error::Error>> {
    
    // Create payment request
    let payment_data = json!({
        "maxAmountRequired": payment_amount,
        "resource": resource,
        "description": "Access to premium API endpoint",
        "payTo": pay_to,
        "asset": asset,
        "network": network
    });
    
    // Return 402 Payment Required with payment details
    let response = PaymentResponse {
        status: 402,
        headers: vec![
            ("X-Payment-Required".to_string(), "true".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body: payment_data.to_string(),
    };
    
    Ok(response)
}

// Example client implementation for making payments
async fn make_payment_request(
    endpoint: &str,
    payment_signature: &str,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    
    let client = reqwest::Client::new();
    
    let response = client
        .get(endpoint)
        .header("X-Payment", payment_signature)
        .header("Authorization", "Bearer your-auth-token")
        .send()
        .await?;
    
    if response.status() == 402 {
        // Handle payment required
        let payment_info: serde_json::Value = response.json().await?;
        println!("Payment required: {}", payment_info);
        // Process payment logic here
    }
    
    Ok(response)
}
```

### Agent Integration Example

Here's how we might integrate an AI agent with x402 payments:

```rust
use tokio;
use serde::{Deserialize, Serialize};
use reqwest;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
struct AgentPaymentConfig {
    wallet_address: String,
    max_payment_amount: f64,
    supported_assets: Vec<String>,
}

struct PaymentAgent {
    config: AgentPaymentConfig,
    client: reqwest::Client,
}

impl PaymentAgent {
    pub fn new(config: AgentPaymentConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn execute_paid_request(
        &self,
        endpoint: &str,
        payment_amount: f64,
    ) -> Result<String> {
        // First attempt without payment
        let response = self.client.get(endpoint).send().await?;
        
        if response.status() == 402 {
            // Payment required - process payment
            let payment_info: serde_json::Value = response.json().await?;
            
            if payment_amount <= self.config.max_payment_amount {
                // Generate payment signature (simplified)
                let payment_signature = self.generate_payment_signature(&payment_info)?;
                
                // Retry with payment
                let paid_response = self.client
                    .get(endpoint)
                    .header("X-Payment", payment_signature)
                    .send()
                    .await?;
                
                return Ok(paid_response.text().await?);
            } else {
                return Err(anyhow::anyhow!("Payment amount exceeds maximum allowed"));
            }
        }
        
        Ok(response.text().await?)
    }
    
    fn generate_payment_signature(
        &self,
        payment_info: &serde_json::Value,
    ) -> Result<String> {
        // Implement payment signature generation
        // This would involve blockchain transaction signing
        // Using set of Alloy libraries, like alloy-primitives, alloy-provider, etc., from dependencies
        
        // Simplified example - in reality this would be a proper crypto signature
        Ok("payment_signature_here".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AgentPaymentConfig {
        wallet_address: "0x1234...".to_string(),
        max_payment_amount: 1.0,
        supported_assets: vec!["USDC".to_string()],
    };
    
    let agent = PaymentAgent::new(config);
    
    // Example usage
    let result = agent.execute_paid_request(
        "https://api.example.com/premium-data",
        0.10
    ).await?;
    
    println!("Received data: {}", result);
    
    Ok(())
}
```

## Integration with MCP (Model Context Protocol)

Both protocols work well with MCP servers. Cloudflare's Agents SDK now supports x402 payments [[13]](https://blog.cloudflare.com/x402/), allowing agents to pay for MCP tools automatically.

## Key Benefits for Rust Development

1. **Type Safety**: Rust's type system ensures secure payment handling
2. **Performance**: Efficient processing of payment requests
3. **Ecosystem**: Integration with existing Rust web frameworks like Axum or Warp
4. **Blockchain**: Natural fit with Rust's blockchain ecosystem (alloy, etc.)

## Conclusion

Both AP2 and x402 represent the future of autonomous commerce, enabling AI agents to make secure, auditable payments without human intervention. The `x402-rs` crate provides a solid foundation for Rust developers to build payment-enabled applications and agents.
