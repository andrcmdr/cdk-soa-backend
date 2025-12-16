# AP2 and x402 Protocols — Technical Summary and Integration Guide

---

## Overview
This document summarizes the **AP2** (Agent Payments Protocol) and **x402** (HTTP 402 Payment Protocol), focusing on their design intent, interoperability, and practical Rust-based integration for agent systems.

---

## 1. AP2 — Agent Payments Protocol

### Description
**AP2** defines a cryptographically verifiable framework for **agentic payments and authorizations**. It enables users to delegate spending authority to agents via digitally signed *mandates* that specify spending rules, merchants, and limits.

### Core Concepts
| Concept | Description |
|----------|--------------|
| **Mandate** | A signed authorization defining what an agent can spend, where, and how much. |
| **Verifiable Credentials (VCs)** | Optional DID/VC artifacts that anchor mandates in decentralized identity frameworks. |
| **Agent-to-Agent (A2A) Messaging** | Used for direct transmission of mandates and payment requests. |
| **Auditability** | All mandate and payment events are cryptographically auditable for compliance. |

### Data Flow
1. **User issues mandate**: The user signs a JSON mandate defining allowed merchants, spending limits, and validity period.
2. **Agent executes transaction**: The agent presents this signed mandate with the transaction request.
3. **Merchant verifies**: The recipient validates the signature and scope of the mandate.
4. **Audit trail**: Mandates and payment receipts are recorded for transparency.

### Integration Points
- **Mandate generation** (local signing, HSM/KMS storage)
- **Mandate presentation** (HTTP header or body)
- **Verification** (signature + nonce validation)

### Rust Implementation Notes
- JSON serialization: `serde_json`
- Cryptography: `ed25519-dalek`, `ring`
- Verifiable credentials: `ssi`, `didkit`
- Secure key storage: `aws-sdk-kms`

### Example — Mandate Signing (Rust)
```rust
use ed25519_dalek::{Keypair, Signer};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Mandate {
    user_id: String,
    allowed_merchants: Vec<String>,
    max_amount_usd: f64,
    expires_at: String,
    nonce: String,
}

fn sign_mandate(mandate: &Mandate, keypair: &Keypair) -> String {
    let bytes = serde_json::to_vec(mandate).unwrap();
    let sig = keypair.sign(&bytes);
    base64::encode(sig.to_bytes())
}
```

---

## 2. x402 — HTTP 402 Payment Protocol

### Description
**x402** revives the unused HTTP `402 Payment Required` status code as a **universal payment negotiation mechanism**. It allows web services to accept **on-chain or off-chain micropayments** directly through HTTP interactions.

### Core Concepts
| Concept | Description |
|----------|--------------|
| **Payment Challenge (402 Response)** | Server responds with `402` and structured payment details. |
| **Payment Proof (Request Header)** | Client retries with proof of payment (e.g., tx hash). |
| **On-chain Settlement** | Payments occur over EVM-compatible networks, often via stablecoins. |
| **Receipts & Verification** | Proof-of-payment verification by the merchant’s backend. |

### Data Flow
1. **Client requests resource** → server replies `402 Payment Required` with a payment challenge.
2. **Client pays** the requested amount on-chain and obtains transaction proof.
3. **Client retries** the request with `X-402-Payment` header containing proof.
4. **Server verifies** and grants access to the resource.

### Integration Points
- **HTTP client modification** to detect and handle `402` responses.
- **Payment subsystem** that executes on-chain transactions and provides proof.
- **Retry logic** that attaches proof headers.

### Rust Implementation Notes
- HTTP: `reqwest`, `hyper`
- Blockchain interaction: `alloy` (preferred), `ethers-rs`
- JSON encoding: `serde_json`

### Example — x402 Client Flow (Rust + Alloy)
```rust
use reqwest::Client;
use alloy::providers::{ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use alloy::rpc::types::TransactionRequest;

async fn fetch_with_x402(url: &str) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let resp = client.get(url).send().await?;

    if resp.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        // Parse challenge
        let challenge: serde_json::Value = resp.json().await?;

        // Pay on-chain via Alloy
        let signer: PrivateKeySigner = "0x...".parse().unwrap();
        let provider = ProviderBuilder::new().wallet(signer).connect("http://localhost:8545").await.unwrap();
        let tx = TransactionRequest::default().with_to("0xRecipient").with_value(1_000_000_000_000_000u64.into());
        let pending = provider.send_transaction(tx).await.unwrap();

        // Retry with proof header
        let proof = format!("0x{:x}", pending.tx_hash());
        let resp2 = client.get(url).header("X-402-Payment", proof).send().await?;
        return Ok(resp2.text().await?);
    }

    Ok(resp.text().await?)
}
```

---

## 3. Combined Flow: AP2 + x402
1. **Agent creates AP2 mandate** (signed user authorization).
2. **Agent requests x402-protected resource**.
3. **Server responds with HTTP 402 challenge**.
4. **Agent verifies mandate covers payment**.
5. **Agent executes on-chain payment using Alloy**.
6. **Agent retries request** with both:
   - `X-402-Payment`: proof (tx hash)
   - `X-AP2-Mandate`: JSON mandate payload
7. **Server validates both**, processes payment, and serves content.

---

## 4. Security & Best Practices
- **Key custody**: Store signing keys in KMS/HSM; agents must never hold long-term keys unprotected.
- **Nonce/timestamp enforcement** to prevent replay attacks.
- **Mandate scope**: Always validate merchant ID and spending limits.
- **Auditable storage**: Record mandate ID, tx hash, timestamps for compliance.

---

## 5. Useful References
- **x402 official GitHub**: [coinbase/x402](https://github.com/coinbase/x402)
- **x402 official web-site**: [X402](https://www.x402.org)
- **x402 developer docs**:

  [Coinbase Base developer portal](https://docs.base.org/base-app/agents/x402-agents)

  [Coinbase developer portal](https://docs.cdp.coinbase.com/x402/docs/welcome)
 
- **X402 library, Rust implementation**: [x402-rs](https://github.com/x402-rs/x402-rs)
- **AP2 overview**: [Google A2A / Agentic Commerce documentation](https://a2aprotocol.ai/ap2-protocol)
- **Alloy crate documentation**: [Alloy :: Docs.rs](https://docs.rs/alloy)

---

*This document provides a unified conceptual and practical reference for developing and integrating AP2 authorization and x402 micropayments using Rust and Alloy 1.0.*

## Ap2 X402 Alloy Rust Example

The project demonstrates:
- AP2-style mandate creation + ed25519 signing (mandate JSON),
- an x402 HTTP client flow that detects `402 Payment Required`, pays on-chain using **Alloy v1.0**, then retries with `X-402-Payment` and the AP2 mandate.

```toml
# Cargo.toml
[package]
name = "ap2_x402_alloy_example"
version = "0.1.0"
edition = "2021"

[dependencies]
# Alloy v1 (full feature)
alloy = { version = "1", features = ["full"] }
# HTTP client and async runtime
reqwest = { version = "0.11", features = ["json", "gzip", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# ed25519 for AP2 mandate signing
ed25519-dalek = { version = "1.0", features = ["std"] }
# base64 helper
base64 = "0.21"
# tokio runtime
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
// src/main.rs
use std::error::Error;

mod ap2_mandate;
mod x402_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("AP2 + x402 Alloy example starting...");

    // 1) Create and sign an AP2 mandate (ed25519)
    let mandate = ap2_mandate::example_create_and_sign().await?;
    println!("Mandate created with id: {}", mandate.id);

    // 2) Perform an x402-protected fetch; when faced with 402 the client will use Alloy to pay
    // NOTE: This example assumes you have a local RPC node (anvil, ganache) at http://127.0.0.1:8545
    // and that the test account has funds.

    let url = "http://localhost:7777/protected-resource"; // replace with real x402 provider
    let result = x402_client::fetch_with_x402(url, &mandate).await;

    match result {
        Ok(body) => println!("Resource fetched: {}", body),
        Err(e) => eprintln!("Failed to fetch resource: {}", e),
    }

    Ok(())
}


// src/ap2_mandate.rs
use ed25519_dalek::{Keypair, Signature, Signer, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use serde::{Serialize, Deserialize};
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mandate {
    pub id: String,
    pub user_id: String,
    pub allowed_merchants: Vec<String>,
    pub max_amount_usd: f64,
    pub expires_at: String, // ISO8601
    pub nonce: String,
    pub signature_b64: Option<String>,
}

impl Mandate {
    pub fn new(user_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            allowed_merchants: vec![],
            max_amount_usd: 100.0,
            expires_at: "2099-01-01T00:00:00Z".to_string(),
            nonce: Uuid::new_v4().to_string(),
            signature_b64: None,
        }
    }
}

/// Example: create and sign a mandate with an ed25519 keypair
pub async fn example_create_and_sign() -> Result<Mandate, Box<dyn std::error::Error>> {
    // Create a sample mandate
    let mut mandate = Mandate::new("user:alice@example.com");
    mandate.allowed_merchants.push("merchant:example_api".to_string());
    mandate.max_amount_usd = 30.0;

    // Create a new ed25519 keypair (in real use store this securely in KMS/HSM)
    let mut csprng = OsRng{};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    // Sign the mandate (canonical JSON serialization)
    let payload = serde_json::to_vec(&mandate)?;
    let sig: Signature = keypair.sign(&payload);
    let sig_b64 = general_purpose::STANDARD.encode(sig.to_bytes());
    mandate.signature_b64 = Some(sig_b64);

    // For debug: also persist public key bytes if verification is needed
    let pubkey_b64 = general_purpose::STANDARD.encode(keypair.public.to_bytes());
    println!("Mandate signed. Public key (b64): {}", pubkey_b64);

    Ok(mandate)
}


// src/x402_client.rs
use crate::ap2_mandate::Mandate;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use alloy::providers::{ProviderBuilder, Provider};
use alloy::primitives::{address, Unit, U256};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct X402Challenge {
    // fields follow typical x402 shape; servers may use headers instead
    pub amount: String,
    pub token: String,
    pub invoice_endpoint: Option<String>,
}

/// Fetch a resource protected by x402. If server returns 402, pay using Alloy and retry with proof header.
pub async fn fetch_with_x402(url: &str, mandate: &Mandate) -> Result<String, Box<dyn Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let resp = client.get(url).send().await?;
    if resp.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        // Try to parse challenge body (some implementations use headers)
        let challenge: X402Challenge = match resp.json().await {
            Ok(c) => c,
            Err(_) => {
                return Err("Payment required but could not parse challenge body".into());
            }
        };

        println!("Received x402 challenge: amount={} token={} invoice_endpoint={:?}",
            challenge.amount, challenge.token, challenge.invoice_endpoint);

        // Verify that AP2 mandate covers this merchant/amount. Simple check:
        if !mandate.allowed_merchants.iter().any(|m| m.contains("merchant")) {
            return Err("Mandate does not allow this merchant".into());
        }

        // Pay the challenge on-chain using Alloy.
        // This example performs a simple native value transfer from a private key signer to a payment receiver.
        // In real x402 flows we may need to call an invoice contract or transfer ERC-20 tokens.

        // --- Alloy setup ---
        // Use a local test private key. DO NOT use in production. Store keys in AWS KMS / HSM.
        let pk_hex = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"; // anvil test
        let signer: PrivateKeySigner = pk_hex.parse()?;

        // Connect ProviderBuilder with wallet to local node
        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect("http://127.0.0.1:8545")
            .await?;

        // Build a TransactionRequest to the invoice endpoint or to a recipient.
        // For demonstration send a small wei amount to a known address.
        let recipient = address!("0x1000000000000000000000000000000000000001");
        // parse amount string to U256; in this demo we'll use 1_000_000_000_000_000u128 (0.001 ETH)
        let value = Unit::ETHER.wei().saturating_mul(U256::from(0u64)); // zero by default

        let tx = TransactionRequest::default()
            .with_to(recipient)
            .with_value(U256::from(1_000_000_000_000_000u128));

        println!("Sending on-chain payment via Alloy...");
        // Send the transaction and await a tx hash
        let pending = provider.send_transaction(tx).await?;
        let tx_hash = pending.tx_hash();
        println!("Payment sent. tx_hash: {:#x}", tx_hash);

        // Wait for confirmation (optional)
        let confirmed = pending.with_required_confirmations(1).watch().await?;
        println!("Payment confirmed: {:#x}", confirmed);

        // Create a simple proof: transaction hash hex
        let proof = format!("0x{:x}", tx_hash);

        // Retry the request with X-402-Payment header and AP2 mandate attached
        let retry_resp = client
            .get(url)
            .header("X-402-Payment", proof)
            .header("X-AP2-Mandate", serde_json::to_string(mandate)?)
            .send()
            .await?;

        if retry_resp.status().is_success() {
            let body = retry_resp.text().await?;
            return Ok(body);
        } else {
            return Err(format!("Retry failed, status: {}", retry_resp.status()).into());
        }

    }

    // Not a payment flow — return body
    let body = resp.text().await?;
    Ok(body)
}

// EOF
```
