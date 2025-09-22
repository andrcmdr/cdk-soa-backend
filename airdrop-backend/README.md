# Airdrop Backend Service

A comprehensive backend service for managing airdrop eligibility data using Merkle Patricia Tries with PostgreSQL storage, NATS object storage, AWS KMS encryption, and Ethereum blockchain integration.

## Features

- **Merkle Patricia Trie**: Ethereum-compatible trie implementation for eligibility data
- **Multi-round Support**: Separate tries per round with versioning
- **PostgreSQL Storage**: Primary database backend with audit trails
- **NATS Object Storage**: Secondary storage for data availability and backup
- **AWS KMS Encryption**: Secure private key management with envelope encryption
- **Smart Contract Integration**: Ethereum contract interaction using Alloy 1.0.25
- **Dual ABI Support**: Both JSON ABI files and inline Solidity interfaces
- **External Data Integration**: Fetch and compare data with other backends
- **Multi-format Support**: JSON, CSV, hex, and base64 data formats
- **RESTful API**: Comprehensive HTTP API with proper error handling

## Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 13+
- NATS Server with JetStream
- AWS KMS access
- Ethereum RPC endpoint

### Installation

1. Clone the repository:
```bash
git clone https://github.com/your-org/airdrop-backend.git
cd airdrop-backend
```

2. Install dependencies:
```bash
cargo build --release
```

3. Setup environment:
```bash
cp config.yaml.example config.yaml
# Edit config.yaml with your settings
```

4. Start services with Docker Compose:
```bash
docker-compose up -d postgres nats
```

5. Run the application:
```bash
cargo run
```

The service will be available at `http://localhost:3000`

## Configuration

The service uses a YAML configuration file (`config.yaml`):

```yaml
server:
  bind_address: "0.0.0.0:3000"
  max_upload_size: 52428800 # 50MB

database:
  url: "postgresql://username:password@localhost/airdrop_db"
  max_connections: 10

blockchain:
  rpc_url: "https://rpc.polygon-cdk-chain.example"
  contract_address: "0x1234567890123456789012345678901234567890"
  chain_id: 1
  contract_interface:
    type: "json_abi"  # or "inline_sol"
    abi_path: "abi/AirdropContract.json"

aws:
  region: "us-east-1"
  kms_key_id: "arn:aws:kms:us-east-1:123456789012:key/..."

wallet:
  encrypted_private_key: ""  # Auto-generated on first run

nats:
  url: "nats://localhost:4222"
  object_store:
    bucket_name: "airdrop-data"
    max_object_size: 104857600 # 100MB
```

## API Documentation

### Base URL
```
http://localhost:3000/api/v1
```

### Authentication
Currently, the API doesn't require authentication. In production, implement proper authentication mechanisms.

---

## Health Check

### Get Service Health
```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "service": "airdrop-backend"
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/health
```

---

## CSV Data Management

### Upload CSV File
```http
POST /api/v1/upload-csv
Content-Type: multipart/form-data
```

**Parameters:**
- `round_id` (form field): Round ID (integer)
- `csv_file` (file): CSV file with columns: `address`, `amount`

**CSV Format:**
```csv
address,amount
0x742C4d97C86bCF0176776C16e073b8c6f9Db4021,1000000000000000000
0x8ba1f109551bD432803012645Hac136c5a2B1A,500000000000000000
```

**Response:**
```json
{
  "success": true,
  "message": "CSV data processed for round 1",
  "round_id": 1,
  "data_size_bytes": 1024
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/upload-csv \
  -F "round_id=1" \
  -F "csv_file=@eligibility_round_1.csv"
```

### Download CSV File
```http
GET /api/v1/download-csv/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:** CSV file download with proper headers

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/download-csv/1 \
  -o round_1_eligibility.csv
```

---

## JSON Eligibility Data

### Upload JSON Eligibility Data
```http
POST /api/v1/upload-json-eligibility/{round_id}
Content-Type: application/json
```

**Parameters:**
- `round_id` (path): Round ID

**Request Body:**
```json
{
  "eligibility": {
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": "1000000000000000000",
    "0x8ba1f109551bD432803012645Hac136c5a2B1A": "500000000000000000"
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "JSON eligibility data processed for round 1",
  "round_id": 1
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/upload-json-eligibility/1 \
  -H "Content-Type: application/json" \
  -d '{
    "eligibility": {
      "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": "1000000000000000000",
      "0x8ba1f109551bD432803012645Hac136c5a2B1A": "500000000000000000"
    }
  }'
```

### Download JSON Eligibility Data
```http
GET /api/v1/download-json-eligibility/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "eligibility": {
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": "1000000000000000000",
    "0x8ba1f109551bD432803012645Hac136c5a2B1A": "500000000000000000"
  }
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/download-json-eligibility/1
```

---

## Merkle Trie Data Management

### Download Trie Data with Merkle Proofs
```http
GET /api/v1/download-trie-data/{round_id}?format={json|hex|base64}
```

**Parameters:**
- `round_id` (path): Round ID
- `format` (query, optional): Output format (`json`, `hex`, `base64`). Default: `json`

**Response:**
```json
{
  "round_id": 1,
  "root_hash": "0x1234567890abcdef...",
  "trie_data": "serialized_data_in_requested_format",
  "format": "json",
  "merkle_proofs": {
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": [
      "0xabcdef1234567890...",
      "0x1234567890abcdef..."
    ]
  }
}
```

**Examples:**
```bash
# JSON format (default)
curl -X GET http://localhost:3000/api/v1/download-trie-data/1

# Hex format
curl -X GET http://localhost:3000/api/v1/download-trie-data/1?format=hex

# Base64 format
curl -X GET http://localhost:3000/api/v1/download-trie-data/1?format=base64
```

### Upload and Compare External Trie Data
```http
POST /api/v1/upload-compare-trie/{round_id}
Content-Type: application/json
```

**Parameters:**
- `round_id` (path): Round ID

**Request Body:**
```json
{
  "round_id": 1,
  "root_hash": "0x1234567890abcdef...",
  "trie_data": "serialized_data",
  "format": "hex"
}
```

**Response:**
```json
{
  "matches": true,
  "local_root_hash": "0x1234567890abcdef...",
  "external_root_hash": "0x1234567890abcdef...",
  "differences": []
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/upload-compare-trie/1 \
  -H "Content-Type: application/json" \
  -d '{
    "round_id": 1,
    "root_hash": "0x1234567890abcdef",
    "trie_data": "0xabcdef123456789",
    "format": "hex"
  }'
```

---

## External Backend Integration

### Fetch Data from External Backend
```http
POST /api/v1/fetch-external-data/{round_id}
Content-Type: application/json
```

**Parameters:**
- `round_id` (path): Round ID

**Request Body:**
```json
{
  "external_url": "https://external-backend.com/api/eligibility/round/1"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Successfully updated round 1 with external data",
  "round_id": 1,
  "external_url": "https://external-backend.com/api/eligibility/round/1"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/fetch-external-data/1 \
  -H "Content-Type: application/json" \
  -d '{
    "external_url": "https://external-backend.com/api/eligibility/round/1"
  }'
```

### Compare with External Trie Data
```http
POST /api/v1/compare-external-trie/{round_id}
Content-Type: application/json
```

**Parameters:**
- `round_id` (path): Round ID

**Request Body:**
```json
{
  "external_url": "https://external-backend.com/api/trie/round/1"
}
```

**Response:**
```json
{
  "success": true,
  "comparison_result": true,
  "message": "Trie data matches",
  "round_id": 1,
  "external_url": "https://external-backend.com/api/trie/round/1"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/compare-external-trie/1 \
  -H "Content-Type: application/json" \
  -d '{
    "external_url": "https://external-backend.com/api/trie/round/1"
  }'
```

---

## Trie Management

### Update Trie
```http
POST /api/v1/update-trie/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "success": true,
  "message": "Trie for round 1 is up to date",
  "round_id": 1,
  "root_hash": "0x1234567890abcdef...",
  "entry_count": 1000,
  "last_updated": "2024-01-15T10:30:00Z"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/update-trie/1
```

### Submit Trie to Blockchain
```http
POST /api/v1/submit-trie/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "success": true,
  "message": "Trie update submitted for round 1",
  "round_id": 1,
  "transaction_hash": "0xabcdef1234567890..."
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/submit-trie/1
```

### Get Trie Information
```http
GET /api/v1/trie-info/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "round_id": 1,
  "root_hash": "0x1234567890abcdef...",
  "entry_count": 1000,
  "created_at": "2024-01-15T10:00:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/trie-info/1
```

---

## Eligibility Verification

### Verify User Eligibility
```http
POST /api/v1/verify-eligibility
Content-Type: application/json
```

**Request Body:**
```json
{
  "round_id": 1,
  "address": "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
  "amount": "1000000000000000000"
}
```

**Response:**
```json
{
  "is_eligible": true,
  "round_id": 1,
  "address": "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
  "amount": "1000000000000000000"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/verify-eligibility \
  -H "Content-Type: application/json" \
  -d '{
    "round_id": 1,
    "address": "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
    "amount": "1000000000000000000"
  }'
```

### Get User Eligibility
```http
GET /api/v1/get-eligibility/{round_id}/{address}
```

**Parameters:**
- `round_id` (path): Round ID
- `address` (path): Ethereum address

**Response:**
```json
{
  "eligible": true,
  "round_id": 1,
  "address": "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
  "amount": "1000000000000000000"
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/get-eligibility/1/0x742C4d97C86bCF0176776C16e073b8c6f9Db4021
```

---

## Contract Information

### Get Contract Information
```http
GET /api/v1/contract/info
```

**Response:**
```json
{
  "contract_address": "0x1234567890123456789012345678901234567890",
  "contract_version": "1.0.0",
  "round_count": "5",
  "interface_type": "json_abi"
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/contract/info
```

### Check Round Active Status
```http
GET /api/v1/rounds/{round_id}/active
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "round_id": 1,
  "is_active": true
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/rounds/1/active
```

### Get Round Metadata
```http
GET /api/v1/rounds/{round_id}/metadata
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "round_id": "1",
  "root_hash": "0x1234567890abcdef...",
  "total_eligible": "1000",
  "total_amount": "1000000000000000000000",
  "start_time": "1641024000",
  "end_time": "1641110400",
  "is_active": true,
  "metadata_uri": "ipfs://QmExample..."
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/rounds/1/metadata
```

### Validate On-Chain Consistency
```http
GET /api/v1/rounds/{round_id}/validate-consistency
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "round_id": 1,
  "is_consistent": true,
  "message": "Local trie root matches on-chain root"
}
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/rounds/1/validate-consistency
```

---

## Statistics and Monitoring

### Get Round Statistics
```http
GET /api/v1/rounds/statistics
```

**Response:**
```json
[
  {
    "round_id": 1,
    "entry_count": 1000,
    "last_updated": "2024-01-15T10:30:00Z"
  },
  {
    "round_id": 2,
    "entry_count": 1500,
    "last_updated": "2024-01-16T10:30:00Z"
  }
]
```

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/rounds/statistics
```

### Get Processing Logs
```http
GET /api/v1/processing-logs?round_id={round_id}
```

**Parameters:**
- `round_id` (query, optional): Filter by round ID

**Response:**
```json
[
  {
    "id": 1,
    "round_id": 1,
    "operation": "csv_processing",
    "status": "completed",
    "message": "Processed 1000 records with root hash: 0x123...",
    "transaction_hash": null,
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

**Examples:**
```bash
# All logs
curl -X GET http://localhost:3000/api/v1/processing-logs

# Logs for specific round
curl -X GET http://localhost:3000/api/v1/processing-logs?round_id=1
```

### Get Round Processing Logs
```http
GET /api/v1/processing-logs/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:** Same as processing logs but filtered by round

**Example:**
```bash
curl -X GET http://localhost:3000/api/v1/processing-logs/1
```

---

## Round Management

### Delete Round
```http
DELETE /api/v1/rounds/{round_id}
```

**Parameters:**
- `round_id` (path): Round ID

**Response:**
```json
{
  "success": true,
  "message": "Round 1 deleted successfully",
  "round_id": 1
}
```

**Example:**
```bash
curl -X DELETE http://localhost:3000/api/v1/rounds/1
```

---

## Error Responses

All endpoints return standard HTTP status codes and JSON error responses:

**400 Bad Request:**
```json
{
  "error": "Invalid input: address format is incorrect"
}
```

**404 Not Found:**
```json
{
  "error": "No trie data found for round 1"
}
```

**500 Internal Server Error:**
```json
{
  "error": "Internal server error"
}
```

---

## Data Formats

### Supported Formats

- **JSON**: Standard JSON format for structured data
- **CSV**: Comma-separated values with headers: `address,amount`
- **Hex**: Hexadecimal encoding (with or without `0x` prefix)
- **Base64**: Base64 encoding for binary data

### Address Format
All Ethereum addresses must be in hexadecimal format with `0x` prefix:
```
0x742C4d97C86bCF0176776C16e073b8c6f9Db4021
```

### Amount Format
All amounts are in Wei (smallest unit of Ether) as decimal strings:
```
"1000000000000000000"  // 1 ETH in Wei
```

---

## External Backend Integration

### Expected External API Formats

**Eligibility Data Endpoint** (`GET`):
```json
{
  "eligibility": {
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": "1000000000000000000",
    "0x8ba1f109551bD432803012645Hac136c5a2B1A": "500000000000000000"
  }
}
```

**Trie Data Endpoint** (`GET`):
```json
{
  "round_id": 1,
  "root_hash": "0x1234567890abcdef",
  "trie_data": "0xabcdef123456789",
  "format": "hex"
}
```

---

## Development

### Running Tests
```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

### Building Docker Image
```bash
docker build -t airdrop-backend:latest .
```

### Database Migrations
```bash
# Run migrations
make migrate

# Reset database
make dev-db-setup
```

---

## Production Deployment

### Environment Variables
```bash
export CONFIG_PATH=/app/config/production.yaml
export RUST_LOG=info
export RUST_BACKTRACE=1
```

### Docker Compose
```bash
# Production deployment
docker-compose up -d

# With monitoring
docker-compose -f docker-compose.yml -f docker-compose.monitoring.yml up -d
```

### Health Checks
The service includes built-in health checks accessible at `/health` endpoint.

---

## Security Considerations

1. **Private Key Management**: Uses AWS KMS envelope encryption
2. **Input Validation**: All inputs are validated and sanitized
3. **Rate Limiting**: Implement rate limiting in production
4. **HTTPS**: Use HTTPS in production environments
5. **Authentication**: Implement proper authentication mechanisms
6. **Audit Logs**: All operations are logged for audit purposes

---

## Support

For issues and questions:

1. Check the [GitHub Issues](https://github.com/andrcmdr/cdk-soa-backend/issues)
2. Review the logs: `docker-compose logs airdrop-backend`
3. Check service health: `curl http://localhost:3000/health`

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

