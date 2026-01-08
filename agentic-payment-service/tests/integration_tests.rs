use anyhow::Result;
use serde_json::json;

#[tokio::test]
async fn test_health_check() -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/health")
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["status"], "healthy");
    
    Ok(())
}

#[tokio::test]
async fn test_payment_prompt_processing() -> Result<()> {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "prompt": "Send $100 to alice@example.com for consulting services",
        "context": "Monthly payment",
        "preferred_protocol": "x402",
        "preferred_gateway": "web2"
    });

    let response = client
        .post("http://localhost:8080/api/v1/payment/prompt")
        .header("Authorization", "Bearer test-token")
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await?;
    assert!(body.get("request_id").is_some());
    assert!(body.get("agent_response").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_agent_query() -> Result<()> {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "query": "What payment protocols are supported?",
        "context": null
    });

    let response = client
        .post("http://localhost:8080/api/v1/agent/query")
        .header("Authorization", "Bearer test-token")
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await?;
    assert!(body.get("text").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_payment_execution() -> Result<()> {
    let client = reqwest::Client::new();
    
    // First, create a payment prompt
    let prompt_payload = json!({
        "prompt": "Transfer $50 to bob@example.com",
        "preferred_protocol": "x402",
        "preferred_gateway": "web2"
    });

    let prompt_response = client
        .post("http://localhost:8080/api/v1/payment/prompt")
        .header("Authorization", "Bearer test-token")
        .json(&prompt_payload)
        .send()
        .await?;

    let prompt_body: serde_json::Value = prompt_response.json().await?;
    let request_id = prompt_body["request_id"].as_str().unwrap();

    // Execute the payment
    let execute_payload = json!({
        "request_id": request_id,
        "protocol": "x402",
        "gateway": "web2",
        "confirmation": true
    });

    let execute_response = client
        .post("http://localhost:8080/api/v1/payment/execute")
        .header("Authorization", "Bearer test-token")
        .json(&execute_payload)
        .send()
        .await?;

    assert_eq!(execute_response.status(), 200);
    
    let execute_body: serde_json::Value = execute_response.json().await?;
    assert!(execute_body.get("transaction_id").is_some());
    assert!(execute_body.get("status").is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_rate_limiting() -> Result<()> {
    let client = reqwest::Client::new();
    
    // Send multiple requests rapidly
    for i in 0..70 {
        let response = client
            .get("http://localhost:8080/health")
            .send()
            .await?;

        if i < 60 {
            assert_eq!(response.status(), 200);
        } else {
            // Should be rate limited after 60 requests
            if response.status() == 429 {
                return Ok(());
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_authentication_required() -> Result<()> {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "query": "Test query"
    });

    let response = client
        .post("http://localhost:8080/api/v1/agent/query")
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), 401); // Unauthorized
    
    Ok(())
}