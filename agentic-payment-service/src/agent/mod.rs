use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::sync::Mutex;

use crate::config::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub prompt: String,
    pub context: Option<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub text: String,
    pub protocol: Option<String>,
    pub action: Option<PaymentAction>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAction {
    pub action_type: String,  // "transfer", "request", "approve"
    pub amount: f64,
    pub currency: String,
    pub recipient: String,
    pub memo: Option<String>,
    pub protocol_params: serde_json::Value,
}

pub struct AgentRunner {
    config: AgentConfig,
    model: Mutex<Option<Box<dyn ModelInference + Send>>>,
}

impl AgentRunner {
    pub fn new(config: &AgentConfig) -> Result<Self> {
        let model_path = Path::new(&config.model_path);
        
        if !model_path.exists() {
            tracing::warn!(
                "Model file not found at {}. Agent will run in mock mode.",
                config.model_path
            );
        }

        Ok(Self {
            config: config.clone(),
            model: Mutex::new(None),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        let mut model = self.model.lock().await;
        
        if model.is_some() {
            return Ok(());
        }

        let model_path = Path::new(&self.config.model_path);
        
        if !model_path.exists() {
            *model = Some(Box::new(MockModel::new()));
            tracing::info!("Using mock model for development");
            return Ok(());
        }

        // Initialize actual model inference
        let inference = LlamaModel::load(
            &self.config.model_path,
            self.config.context_size,
            self.config.inference.threads,
            self.config.inference.gpu_layers,
        )?;
        
        *model = Some(Box::new(inference));
        tracing::info!("Model loaded successfully from {}", self.config.model_path);
        
        Ok(())
    }

    pub async fn process(&self, request: AgentRequest) -> Result<AgentResponse> {
        self.initialize().await?;
        
        let model = self.model.lock().await;
        let model = model.as_ref().context("Model not initialized")?;

        let prompt = self.build_prompt(&request);
        
        let response = model.generate(&prompt, self.config.max_tokens).await?;
        
        self.parse_response(response)
    }

    fn build_prompt(&self, request: &AgentRequest) -> String {
        let system_prompt = r#"You are a payment processing agent. Analyze user requests and generate structured payment actions.

When processing a payment request:
1. Extract: amount, currency, recipient, payment method
2. Determine the appropriate protocol (X402 or AP2)
3. Generate action in JSON format

Response format:
{
  "protocol": "x402" or "ap2",
  "action": {
    "action_type": "transfer|request|approve",
    "amount": numeric,
    "currency": "USD|EUR|ETH|etc",
    "recipient": "address or identifier",
    "memo": "optional description"
  }
}"#;

        let context = request.context.as_deref().unwrap_or("");
        
        format!(
            "{}\n\nContext: {}\n\nUser Request: {}\n\nResponse:",
            system_prompt, context, request.prompt
        )
    }

    fn parse_response(&self, text: String) -> Result<AgentResponse> {
        // Try to extract JSON from response
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                let json_str = &text[start..=end];
                
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let protocol = parsed["protocol"].as_str().map(String::from);
                    
                    let action = if let Some(action_obj) = parsed.get("action") {
                        Some(PaymentAction {
                            action_type: action_obj["action_type"]
                                .as_str()
                                .unwrap_or("transfer")
                                .to_string(),
                            amount: action_obj["amount"].as_f64().unwrap_or(0.0),
                            currency: action_obj["currency"]
                                .as_str()
                                .unwrap_or("USD")
                                .to_string(),
                            recipient: action_obj["recipient"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            memo: action_obj["memo"].as_str().map(String::from),
                            protocol_params: parsed.clone(),
                        })
                    } else {
                        None
                    };

                    return Ok(AgentResponse {
                        text: text.clone(),
                        protocol,
                        action,
                        confidence: 0.85,
                    });
                }
            }
        }

        // Fallback if no JSON found
        Ok(AgentResponse {
            text,
            protocol: None,
            action: None,
            confidence: 0.3,
        })
    }
}

// Trait for model inference abstraction
#[async_trait::async_trait]
pub trait ModelInference {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;
}

// Mock model for development
struct MockModel;

impl MockModel {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ModelInference for MockModel {
    async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        // Simple mock response for testing
        tracing::debug!("Mock model processing prompt: {}", prompt);
        
        let response = r#"{
  "protocol": "x402",
  "action": {
    "action_type": "transfer",
    "amount": 100.0,
    "currency": "USD",
    "recipient": "user@example.com",
    "memo": "Payment processed by mock agent"
  }
}"#;
        
        Ok(response.to_string())
    }
}

// Llama model implementation
struct LlamaModel {
    _path: String,
    // Add actual llm crate types here when model is available
}

impl LlamaModel {
    fn load(
        path: &str,
        _context_size: usize,
        _threads: usize,
        _gpu_layers: i32,
    ) -> Result<Self> {
        // Placeholder for actual model loading
        // Use llm crate: llm::load(path, tokenizer, params)
        Ok(Self {
            _path: path.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl ModelInference for LlamaModel {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        // Placeholder for actual inference
        // Use llm crate inference session
        tracing::debug!("Generating with prompt length: {}, max_tokens: {}", 
            prompt.len(), max_tokens);
        
        // Implement actual inference here using llm crate
        Ok("Model response placeholder".to_string())
    }
}
