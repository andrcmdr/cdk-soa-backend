# Agent Payments Framework

Below is a concise, practical summary of `AP2` and `x402` as they’re being rolled out, plus a Rust-oriented integration blueprint we can adapt today. Because these specs and SDKs are evolving, treat this as a starter template and check the linked sources for changes first.

## What AP2 and x402 are:

- `AP2` (`Agent Payments Protocol`): A protocol introduced in the Google "Agentic" ecosystem to let agents initiate and authorize payments on behalf of users in a standardized, auditable, and consent-aware way. It complements agent-to-agent (`A2A`) messaging by adding a payments layer and policy/consent primitives.
- `x402`: A payments extension/rail that plugs into AP2 workflows to actually move funds (with a focus on low-friction, per-use, micropayment-style flows). Coinbase positions `x402` as a stablecoin facilitator (e.g., USDC), enabling agents to pay one another or monetize services. In short: `AP2` = how agents request/authorize; `x402` = how value moves.

### Useful references
- Coinbase: "Google Agentic Payments Protocol + x402" (positioning, developer onramp, examples)
  https://www.coinbase.com/developer-platform/discover/launches/google_x402
- Overview articles on the Agentic Payments Protocol (AP2) and A2A:

  https://dr-arsanjani.medium.com/the-era-of-agentic-commerce-starts-now-with-the-new-agent-payments-protocol-ap2-2197b8762dd9

  https://dr-arsanjani.medium.com/building-multi-agent-systems-on-vertex-ai-agent-engine-with-the-a2a-protocol-3d34f43191cd

### Core concepts we’ll need to implement
- Identity and trust: Agents have identities (keys, OAuth clients, or both) and operate with user consent and spending limits/policies.
- Payment intents and invoices: An agent publishes a payment request (intent). Counterparty (payer) authorizes per policy. x402 fulfills by creating/settling an invoice/transfer on a supported rail (often USDC).
- Policy and consent: Rate limits, max-per-transaction, allowlists, and explainability logs.
- Webhooks/callbacks: Delivery of payment status (pending, settled, failed, refunded) to agents.

### High-level integration flow (agent-to-agent)
1) Discovery: Consumer agent finds a provider agent and its AP2/x402 capabilities (often via capability registry or manifest).
2) Quote/intent: Consumer agent requests a price/quote, then creates a payment intent including amount, asset (e.g., USDC), and memo.
3) Authorization: Apply user/tenant policy (spending cap, vendor allowlist). If approved, sign or attach an authorization token.
4) Settlement via x402: Create and pay an x402 invoice/transfer using the selected rail (Coinbase-backed USDC rail is common).
5) Acknowledgment: Provider acknowledges payment; delivers service/resource. Both agents log the transaction for audit.
6) Reconciliation and refunds (optional): Handle partial fills, reversals, or disputes per protocol.

## Rust integration blueprint
The code samples use stable crates from Rust stack: `reqwest`, `serde`, `serde_json`, `chrono`, `sha3`/`hex` for signing helpers, and `tokio` for async. Need to replace placeholder URLs, headers, and field names with the real API when will adopt a specific AP2/x402 endpoint.

### Data models
- AP2 PaymentIntent: request to pay for a service.
- x402 Invoice: represents the payable object and final settlement status.

### Rust example: domain models and client skeleton

```rust
// Cargo.toml
// [dependencies]
// anyhow = "1.0.99"
// serde = { version = "1.0.219", features = ["derive"] }
// serde_json = "1.0.143"
// reqwest = { version = "0.12.23", features = ["json", "gzip", "rustls-tls"] }
// tokio = { version = "1.47.1", features = ["macros", "rt-multi-thread"] }
// chrono = "0.4.41"
// hex = "0.4.3"
// sha3 = "0.10.8"
```

```rust
// src/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentIdentity {
    pub agent_id: String,            // unique agent id
    pub pubkey_hex: Option<String>,  // if signing-based auth is used
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentPolicy {
    pub max_per_tx: String,          // "10.00"
    pub daily_cap: Option<String>,   // "100.00"
    pub currency: String,            // e.g., "USDC"
    pub vendor_allowlist: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentIntent {
    pub intent_id: String,
    pub created_at: DateTime<Utc>,
    pub payer: AgentIdentity,
    pub payee: AgentIdentity,
    pub amount: String,              // decimal string
    pub asset: String,               // "USDC"
    pub chain: Option<String>,       // e.g. "CDK", "ETH", "Base"
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub approved: bool,
    pub reason: Option<String>,
    pub policy_snapshot: Option<PaymentPolicy>,
    pub signature_hex: Option<String>, // signature over canonical intent
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct X402Invoice {
    pub invoice_id: String,
    pub intent_id: String,
    pub amount: String,
    pub asset: String,               // "USDC"
    pub chain: Option<String>,       // "CDK", "ETH", "Base"
    pub status: String,              // "pending" | "paid" | "failed" | "expired" | "refunded"
    pub payee_wallet: Option<String>,
    pub payer_wallet: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateIntentRequest {
    pub payee_id: String,
    pub amount: String,
    pub asset: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateInvoiceRequest {
    pub intent_id: String,
    pub amount: String,   // must match intent, or reflect final quote
    pub asset: String,
    pub chain: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookEvent<T> {
    pub event_type: String, // "invoice.paid" etc.
    pub data: T,
    pub occurred_at: DateTime<Utc>,
}
```

### Rust example: AP2 + x402 client
- AP2: create payment intents and authorize
- x402: create and pay invoice, query status

```rust
// src/client.rs
use anyhow::Result;
use reqwest::{Client, StatusCode};
use serde_json::json;

use crate::models::*;

#[derive(Clone)]
pub struct Ap2Client {
    http: Client,
    base_url: String,   // e.g. "https://ap2.example.com/v1"
    bearer: String,     // OAuth2 access token or API key
}

impl Ap2Client {
    pub fn new(base_url: impl Into<String>, bearer: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
            bearer: bearer.into(),
        }
    }

    pub async fn create_intent(&self, req: &CreateIntentRequest) -> Result<PaymentIntent> {
        let url = format!("{}/intents", self.base_url);
        let res = self.http.post(&url)
            .bearer_auth(&self.bearer)
            .json(req)
            .send()
            .await?;

        if res.status() != StatusCode::CREATED {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("AP2 create_intent failed: {}: {}", res.status(), body);
        }
        Ok(res.json().await?)
    }

    // Example authorization call (policy check + signature, depending on AP2 server)
    pub async fn authorize_intent(&self, intent_id: &str, auth: &Authorization) -> Result<Authorization> {
        let url = format!("{}/intents/{}/authorize", self.base_url, intent_id);
        let res = self.http.post(&url)
            .bearer_auth(&self.bearer)
            .json(auth)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("AP2 authorize_intent failed: {}: {}", res.status(), body);
        }
        Ok(res.json().await?)
    }
}

#[derive(Clone)]
pub struct X402Client {
    http: Client,
    base_url: String,   // e.g. "https://x402.example.com/v1"
    bearer: String,
}

impl X402Client {
    pub fn new(base_url: impl Into<String>, bearer: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
            bearer: bearer.into(),
        }
    }

    pub async fn create_invoice(&self, req: &CreateInvoiceRequest) -> Result<X402Invoice> {
        let url = format!("{}/invoices", self.base_url);
        let res = self.http.post(&url)
            .bearer_auth(&self.bearer)
            .json(req)
            .send()
            .await?;

        if res.status() != StatusCode::CREATED {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("x402 create_invoice failed: {}: {}", res.status(), body);
        }
        Ok(res.json().await?)
    }

    pub async fn get_invoice(&self, invoice_id: &str) -> Result<X402Invoice> {
        let url = format!("{}/invoices/{}", self.base_url, invoice_id);
        let res = self.http.get(&url)
            .bearer_auth(&self.bearer)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("x402 get_invoice failed: {}: {}", res.status(), body);
        }
        Ok(res.json().await?)
    }

    // Depending on rail, paying an invoice might be a separate call or an off-platform wallet action.
    pub async fn mark_paid_for_demo(&self, invoice_id: &str) -> Result<X402Invoice> {
        let url = format!("{}/invoices/{}/mark-paid", self.base_url, invoice_id);
        let res = self.http.post(&url)
            .bearer_auth(&self.bearer)
            .json(&json!({}))
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("x402 mark_paid failed: {}: {}", res.status(), body);
        }
        Ok(res.json().await?)
    }
}
```

### Putting it together: example flow
- Consumer agent requests a service and pays the provider.

```rust
// src/main.rs
mod models;
mod client;

use anyhow::Result;
use chrono::Utc;
use models::*;
use client::{Ap2Client, X402Client};

#[tokio::main]
async fn main() -> Result<()> {
    let ap2 = Ap2Client::new("https://ap2.example.com/v1", "<AP2_BEARER_TOKEN>");
    let x402 = X402Client::new("https://x402.example.com/v1", "<X402_BEARER_TOKEN>");

    // 1) Create a payment intent
    let intent = ap2.create_intent(&CreateIntentRequest {
        payee_id: "provider-agent-123".into(),
        amount: "0.50".into(),
        asset: "USDC".into(),
        description: Some("Per-crawl fee for web indexing".into()),
        metadata: None,
    }).await?;
    println!("Created intent: {}", intent.intent_id);

    // 2) Authorize under policy (this would include user consent in a real app)
    let auth = ap2.authorize_intent(
        &intent.intent_id,
        &Authorization {
            approved: true,
            reason: Some("Policy OK".into()),
            policy_snapshot: Some(PaymentPolicy {
                max_per_tx: "2.00".into(),
                daily_cap: Some("10.00".into()),
                currency: "USDC".into(),
                vendor_allowlist: Some(vec!["provider-agent-123".into()]),
            }),
            signature_hex: None, // set if AP2 service requires a signature over the intent
            expires_at: None,
        }
    ).await?;
    if !auth.approved {
        anyhow::bail!("Authorization denied");
    }

    // 3) Create an x402 invoice for settlement
    let invoice = x402.create_invoice(&CreateInvoiceRequest {
        intent_id: intent.intent_id.clone(),
        amount: intent.amount.clone(),
        asset: intent.asset.clone(),
        chain: Some("Base".into()),
    }).await?;
    println!("Invoice created: {} status={}", invoice.invoice_id, invoice.status);

    // 4) Pay invoice (Real flow might involve sending from a wallet)
    // Here we simulate with a demo endpoint or await a webhook from the rail
    let paid = x402.mark_paid_for_demo(&invoice.invoice_id).await?;
    println!("Invoice paid: {} status={}", paid.invoice_id, paid.status);

    // 5) Provider proceeds with the service; both sides log the transaction.

    Ok(())
}
```

### Webhook handling (invoice events)
- We'll receive events like `invoice.paid`. Validate the signature from the sender.

```rust
// src/webhook.rs
use std::net::SocketAddr;
use anyhow::Result;
use axum::{routing::post, Router, extract::State, Json};
use serde_json::Value;

use crate::models::{WebhookEvent, X402Invoice};

#[derive(Clone)]
pub struct WebhookState {
    pub shared_secret: String,
}

pub async fn run(addr: SocketAddr, state: WebhookState) -> Result<()> {
    let app = Router::new()
        .route("/webhooks/x402", post(x402_handler))
        .with_state(state);

    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

async fn x402_handler(
    State(state): State<WebhookState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<WebhookEvent<X402Invoice>>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    // Verify signature from headers with state.shared_secret (implementation omitted)
    // Implement order/payment state machine

    println!("Received event {} for invoice {}", payload.event_type, payload.data.invoice_id);
    Ok(Json(serde_json::json!({"ok": true})))
}
```

## Security and policy considerations
- Strong authentication: Use OAuth2 or mTLS for server-to-server. If signing is required, use stable key management (HSM/KMS) and sign a canonicalized intent payload.
- Idempotency: Use idempotency keys on create_intent and create_invoice to avoid double charges on retries.
- Webhook verification: Verify HMAC/signatures and replay-protect with timestamps and nonces.
- Policy engine: Enforce per-user limits, vendor allowlists, and risk checks before authorizing AP2 intents.
- Auditability: Persist all intents, authorizations, invoices, and status changes with timestamps.

## How to adapt this to a real provider
- For integration with Coinbase’s x402 rail, we need to follow their API/SDK for:
  - Creating invoices/charges in USDC (often on Base).
  - Settling from custodial or non-custodial wallets.
  - Webhook events and refund flows.
- In Google’s AP2 context, check the official docs for:
  - Intent schema, required fields, and auth.
  - Consent UX requirements and explainability logs.
  - A2A discovery and capability advertising for the agent.

## Testing tips
- Spin up a sandbox/staging account for x402 and use testnets.
- Mock AP2 and x402 servers locally with WireMock or a simple Axum server to validate our flows.
- Add property tests for canonicalization/signing of intents to prevent broken signatures.
