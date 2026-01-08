# Agentic Payment Service

A sophisticated Rust-based payment service that integrates AI agents with multiple payment protocols (X402 and AP2) and gateways (Web3 and Web2).

## Features

- **Multi-Protocol Support**: X402 and AP2 payment protocols
- **AI-Powered Agent**: Custom LLM/SLM integration for intelligent payment processing
- **Dual Gateway Support**: Web3 (blockchain) and Web2 (traditional) payment gateways
- **Configurable**: YAML-based configuration for all components
- **Middleware**: Authentication, rate limiting, and logging
- **Async/Await**: Built on Tokio for high-performance async operations

## Architecture

```
┌────────────────────────────────────────────────────┐
│                  API Layer (Axum)                  │
├────────────────────────────────────────────────────┤
│                   Middleware                       │
│         (Auth, Rate Limit, Logging)                │
├────────────────────────────────────────────────────┤
│              Agent Runner (LLM)                    │
│         (Payment Intent Parser)                    │
├──────────────────┬─────────────────────────────────┤
│  Protocol Layer  │      Gateway Layer              │
│  ┌──────────┐    │   ┌──────────┐  ┌──────────┐    │
│  │   X402   │    │   │   Web3   │  │   Web2   │    │
│  └──────────┘    │   └──────────┘  └──────────┘    │
│  ┌──────────┐    │                                 │
│  │   AP2    │    │                                 │
│  └──────────┘    │                                 │
└──────────────────┴─────────────────────────────────┘
```

## Prerequisites

- Rust >1.91 (2024 edition)
- Custom fine-tuned LLM model (GGUF format)
- API keys for payment protocols and gateways

## Installation

1. Clone the repository
```bash
git clone https://github.com/andrcmdr/cdk-dev-stack
cd cdk-dev-stack/agentic-payment-service/
```

2. Build the project
```bash
cargo build --release
```

3. Configure the service
```bash
cp config.yaml.example config.yaml
# Edit config.yaml with your settings
```

## Configuration

Edit `config.yaml` to configure:

### Protocols
- **X402**: Endpoint, API key, timeout, retries
- **AP2**: Endpoint, API key, timeout, retries

### Agent
- **model_path**: Path to your GGUF model file
- **model_type**: Model architecture (llama, gpt2, etc.)
- **context_size**: Context window size
- **temperature**: Sampling temperature
- **inference**: Thread count, batch size, GPU layers

### Payment Gateways
- **Web3**: RPC URL, chain ID, gas limit
- **Web2**: Provider (Stripe/PayPal), API keys

### Middleware
- **rate_limiting**: Enable/disable, requests per minute
- **authentication**: Enable/disable, JWT secret
- **logging**: Level and format

## Usage

### Start the Service

```bash
cargo run --release
```

The service will start on `http://0.0.0.0:8080` by default.

### API Endpoints

#### 1. Health Check
```bash
GET /health
```

#### 2. Process Payment Prompt
```bash
POST /api/v1/payment/prompt
Content-Type: application/json

{
  "prompt": "Send $100 to alice@example.com",
  "context": "Monthly subscription payment",
  "preferred_protocol": "x402",
  "preferred_gateway": "web2"
}
```

Response:
```json
{
  "request_id": "uuid",
  "agent_response": {
    "text": "...",
    "protocol": "x402",
    "action": {
      "action_type": "transfer",
      "amount": 100.0,
      "currency": "USD",
      "recipient": "alice@example.com",
      "memo": "Monthly subscription"
    },
    "confidence": 0.95
  },
  "suggested_protocol": "x402",
  "estimated_fees": 3.20
}
```

#### 3. Execute Payment
```bash
POST /api/v1/payment/execute
Content-Type: application/json
Authorization: Bearer <token>

{
  "request_id": "uuid-from-prompt",
  "protocol": "x402",
  "gateway": "web2",
  "confirmation": true
}
```

#### 4. Check Payment Status
```bash
GET /api/v1/payment/status/:transaction_id
Authorization: Bearer <token>
```

#### 5. Agent Query
```bash
POST /api/v1/agent/query
Content-Type: application/json
Authorization: Bearer <token>

{
  "query": "What payment methods are available?",
  "context": "User inquiry"
}
```

## LLM Model Integration

### Using Your Custom Model

Place your fine-tuned GGUF model in the `models/` directory:

```bash
mkdir -p models
cp /path/to/your/payment-agent.gguf models/
```

Update `config.yaml`:
```yaml
agent:
  model_path: "./models/payment-agent.gguf"
  model_type: "llama"
```

### Model Training Recommendations

Your custom LLM should be trained to:

1. **Parse payment intents** from natural language
2. **Extract key information**: amount, currency, recipient, method
3. **Output structured JSON** with payment actions
4. **Handle edge cases**: ambiguous amounts, multiple recipients, etc.

Example training data format:
```
Input: "Transfer $50 to Bob for lunch"
Output: {
  "protocol": "x402",
  "action": {
    "action_type": "transfer",
    "amount": 50.0,
    "currency": "USD",
    "recipient": "Bob",
    "memo": "lunch"
  }
}
```

## Development

### Running Tests
```bash
cargo test
```

### Running with Debug Logging
```bash
RUST_LOG=debug cargo run
```

### Development Mode (Mock Model)

If no model file is found, the service runs in mock mode with a simulated agent for testing.

## Protocol Details

### X402 Protocol
- **Purpose**: Fast, atomic agent-to-agent payments
- **Features**: Smart routing, multi-currency, low latency
- **Use Cases**: Microtransactions, API payments, inter-agent transfers

### AP2 Protocol
- **Purpose**: Advanced multi-party settlements
- **Features**: Escrow, conditional payments, cross-chain
- **Use Cases**: Complex workflows, smart contracts, group payments

## Gateway Details

### Web3 Gateway
- **Blockchain**: Ethereum-compatible chains
- **Supported**: ETH, ERC-20 tokens
- **Features**: Gas estimation, transaction tracking

### Web2 Gateway
- **Providers**: Stripe, PayPal (configurable)
- **Supported**: Credit cards, bank transfers
- **Features**: Instant processing, webhook support

## Security

- **Authentication**: JWT-based (configurable)
- **Rate Limiting**: Per-IP request throttling
- **CORS**: Configurable allowed origins
- **Input Validation**: All inputs sanitized
- **Max Payment Limits**: Configurable safety limits

## Production Deployment

### Docker
```dockerfile
FROM rust:1.91 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:testing-slim
COPY --from=builder /app/target/release/agentic-payment-service /usr/local/bin/
COPY config.yaml /etc/agentic-payment/
CMD ["agentic-payment-service"]
```

### Environment Variables
```bash
export X402_API_KEY="your-x402-key"
export AP2_API_KEY="your-ap2-key"
export WEB3_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/your-key"
export WEB2_API_KEY="your-stripe-key"
export JWT_SECRET="your-secret"
```

## Monitoring

The service logs structured JSON for easy parsing:
```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "info",
  "message": "Processing payment",
  "protocol": "x402",
  "amount": 100.0
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

