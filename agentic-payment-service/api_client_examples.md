# API Client Examples

## Python Client

```python
import requests
import json

class PaymentServiceClient:
    def __init__(self, base_url, api_token):
        self.base_url = base_url
        self.headers = {
            "Authorization": f"Bearer {api_token}",
            "Content-Type": "application/json"
        }
    
    def health_check(self):
        response = requests.get(f"{self.base_url}/health")
        return response.json()
    
    def process_payment_prompt(self, prompt, context=None, protocol=None, gateway=None):
        payload = {
            "prompt": prompt,
            "context": context,
            "preferred_protocol": protocol,
            "preferred_gateway": gateway
        }
        response = requests.post(
            f"{self.base_url}/api/v1/payment/prompt",
            headers=self.headers,
            json=payload
        )
        return response.json()
    
    def execute_payment(self, request_id, protocol, gateway, confirmation=True):
        payload = {
            "request_id": request_id,
            "protocol": protocol,
            "gateway": gateway,
            "confirmation": confirmation
        }
        response = requests.post(
            f"{self.base_url}/api/v1/payment/execute",
            headers=self.headers,
            json=payload
        )
        return response.json()
    
    def check_payment_status(self, transaction_id):
        response = requests.get(
            f"{self.base_url}/api/v1/payment/status/{transaction_id}",
            headers=self.headers
        )
        return response.json()
    
    def agent_query(self, query, context=None):
        payload = {
            "query": query,
            "context": context
        }
        response = requests.post(
            f"{self.base_url}/api/v1/agent/query",
            headers=self.headers,
            json=payload
        )
        return response.json()

# Usage example
client = PaymentServiceClient("http://localhost:8080", "your-api-token")

# Process a payment prompt
result = client.process_payment_prompt(
    prompt="Send $150 to alice@example.com for design work",
    protocol="x402",
    gateway="web2"
)

print(f"Request ID: {result['request_id']}")
print(f"Suggested Protocol: {result['suggested_protocol']}")
print(f"Estimated Fees: ${result['estimated_fees']}")

# Execute the payment
if result.get('agent_response', {}).get('action'):
    execution = client.execute_payment(
        request_id=result['request_id'],
        protocol=result['suggested_protocol'],
        gateway='web2',
        confirmation=True
    )
    print(f"Transaction ID: {execution['transaction_id']}")
    print(f"Status: {execution['status']}")
```

## JavaScript/Node.js Client

```javascript
const axios = require('axios');

class PaymentServiceClient {
    constructor(baseUrl, apiToken) {
        this.baseUrl = baseUrl;
        this.client = axios.create({
            baseURL: baseUrl,
            headers: {
                'Authorization': `Bearer ${apiToken}`,
                'Content-Type': 'application/json'
            }
        });
    }

    async healthCheck() {
        const response = await axios.get(`${this.baseUrl}/health`);
        return response.data;
    }

    async processPaymentPrompt(prompt, context = null, protocol = null, gateway = null) {
        const payload = {
            prompt,
            context,
            preferred_protocol: protocol,
            preferred_gateway: gateway
        };
        const response = await this.client.post('/api/v1/payment/prompt', payload);
        return response.data;
    }

    async executePayment(requestId, protocol, gateway, confirmation = true) {
        const payload = {
            request_id: requestId,
            protocol,
            gateway,
            confirmation
        };
        const response = await this.client.post('/api/v1/payment/execute', payload);
        return response.data;
    }

    async checkPaymentStatus(transactionId) {
        const response = await this.client.get(`/api/v1/payment/status/${transactionId}`);
        return response.data;
    }

    async agentQuery(query, context = null) {
        const payload = { query, context };
        const response = await this.client.post('/api/v1/agent/query', payload);
        return response.data;
    }
}

// Usage example
(async () => {
    const client = new PaymentServiceClient('http://localhost:8080', 'your-api-token');

    try {
        // Health check
        const health = await client.healthCheck();
        console.log('Service Status:', health.status);

        // Process payment
        const result = await client.processPaymentPrompt(
            'Transfer $200 to bob@example.com for consulting',
            'Monthly payment',
            'x402',
            'web2'
        );

        console.log('Request ID:', result.request_id);
        console.log('Agent Response:', result.agent_response);

        // Execute payment
        const execution = await client.executePayment(
            result.request_id,
            result.suggested_protocol,
            'web2'
        );

        console.log('Transaction ID:', execution.transaction_id);
        console.log('Status:', execution.status);

        // Check status
        const status = await client.checkPaymentStatus(execution.transaction_id);
        console.log('Payment Status:', status);

    } catch (error) {
        console.error('Error:', error.response?.data || error.message);
    }
})();
```

## cURL Examples

### Health Check
```bash
curl http://localhost:8080/health
```

### Process Payment Prompt
```bash
curl -X POST http://localhost:8080/api/v1/payment/prompt \
  -H "Authorization: Bearer your-api-token" \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Send $100 to alice@example.com",
    "context": "Test payment",
    "preferred_protocol": "x402",
    "preferred_gateway": "web2"
  }'
```

### Execute Payment
```bash
curl -X POST http://localhost:8080/api/v1/payment/execute \
  -H "Authorization: Bearer your-api-token" \
  -H "Content-Type: application/json" \
  -d '{
    "request_id": "uuid-from-prompt-response",
    "protocol": "x402",
    "gateway": "web2",
    "confirmation": true
  }'
```

### Check Payment Status
```bash
curl http://localhost:8080/api/v1/payment/status/transaction-id \
  -H "Authorization: Bearer your-api-token"
```

### Agent Query
```bash
curl -X POST http://localhost:8080/api/v1/agent/query \
  -H "Authorization: Bearer your-api-token" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What payment methods are available?",
    "context": null
  }'
```

## Go Client

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

type PaymentServiceClient struct {
    BaseURL   string
    APIToken  string
    Client    *http.Client
}

type PaymentPromptRequest struct {
    Prompt            string  `json:"prompt"`
    Context           *string `json:"context,omitempty"`
    PreferredProtocol *string `json:"preferred_protocol,omitempty"`
    PreferredGateway  *string `json:"preferred_gateway,omitempty"`
}

type PaymentPromptResponse struct {
    RequestID         string      `json:"request_id"`
    AgentResponse     interface{} `json:"agent_response"`
    SuggestedProtocol *string     `json:"suggested_protocol"`
    EstimatedFees     *float64    `json:"estimated_fees"`
}

func NewPaymentServiceClient(baseURL, apiToken string) *PaymentServiceClient {
    return &PaymentServiceClient{
        BaseURL:  baseURL,
        APIToken: apiToken,
        Client:   &http.Client{},
    }
}

func (c *PaymentServiceClient) ProcessPaymentPrompt(req PaymentPromptRequest) (*PaymentPromptResponse, error) {
    body, _ := json.Marshal(req)
    
    httpReq, _ := http.NewRequest("POST", c.BaseURL+"/api/v1/payment/prompt", bytes.NewBuffer(body))
    httpReq.Header.Set("Authorization", "Bearer "+c.APIToken)
    httpReq.Header.Set("Content-Type", "application/json")
    
    resp, err := c.Client.Do(httpReq)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    respBody, _ := io.ReadAll(resp.Body)
    
    var result PaymentPromptResponse
    json.Unmarshal(respBody, &result)
    
    return &result, nil
}

func main() {
    client := NewPaymentServiceClient("http://localhost:8080", "your-api-token")
    
    protocol := "x402"
    gateway := "web2"
    
    result, err := client.ProcessPaymentPrompt(PaymentPromptRequest{
        Prompt:            "Send $75 to carol@example.com",
        PreferredProtocol: &protocol,
        PreferredGateway:  &gateway,
    })
    
    if err != nil {
        fmt.Println("Error:", err)
        return
    }
    
    fmt.Printf("Request ID: %s\n", result.RequestID)
    if result.EstimatedFees != nil {
        fmt.Printf("Estimated Fees: $%.2f\n", *result.EstimatedFees)
    }
}
```
