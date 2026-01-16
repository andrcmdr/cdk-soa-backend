# Mempool Monitor - Comprehensive Guide

## Table of Contents
1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [CLI Usage](#cli-usage)
6. [Configuration Reference](#configuration-reference)
7. [Database Schema](#database-schema)
8. [API Reference](#api-reference)
9. [Examples](#examples)
10. [Troubleshooting](#troubleshooting)

---

## Overview

The **Mempool Monitor** is a high-performance Rust application designed to monitor and capture pending transactions from blockchain networks in real-time. It supports both WebSocket and HTTP RPC protocols, with configurable transaction filtering by sender and receiver addresses.

### Key Features

- ✅ **Real-time monitoring** of pending transactions (mempool)
- ✅ **Dual protocol support**: WebSocket and HTTP RPC
- ✅ **Flexible subscription modes**: Transaction hashes only or full transaction bodies
- ✅ **Address filtering**: Filter by sender and/or receiver addresses
- ✅ **Data persistence**: PostgreSQL (local) and optional AWS RDS
- ✅ **Event streaming**: NATS JetStream Object Store support
- ✅ **API mode**: RESTful API for managing multiple monitoring tasks
- ✅ **Production-ready**: Built with Tokio async runtime and comprehensive error handling

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                  Mempool Monitor                     │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────┐        ┌──────────────┐            │
│  │  WebSocket   │        │   HTTP RPC   │            │
│  │ Subscription │        │   Polling    │            │
│  └──────┬───────┘        └──────┬───────┘            │
│         │                       │                    │
│         └───────────┬───────────┘                    │
│                     ▼                                │
│           ┌──────────────────┐                       │
│           │  TxProcessor     │                       │
│           │  - Filter        │                       │
│           │  - Process       │                       │
│           └────────┬─────────┘                       │
│                    │                                 │
│         ┌──────────┴──────────┐                      │
│         ▼                     ▼                      │
│  ┌─────────────┐      ┌─────────────┐                │
│  │ PostgreSQL  │      │    NATS     │                │
│  │ (Local/RDS) │      │  Object     │                │
│  │             │      │   Store     │                │
│  └─────────────┘      └─────────────┘                │
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

## Installation

### Prerequisites

- Rust 1.91.0 or later
- PostgreSQL 12+
- NATS Server (optional, for event streaming)
- Access to an Ethereum-compatible RPC endpoint (WebSocket and/or HTTP)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/andrcmdr/cdk-dev-stack
cd cdk-dev-stack/mempool-monitor/

# Build the release binary
cargo build --release

# The binary will be available at:
./target/release/mempool-monitor
```

### Database Setup

```bash
# Create PostgreSQL database
createdb mempool_monitor_db

# Create user (optional)
psql -c "CREATE USER mempool_monitor WITH PASSWORD 'passwd';"
psql -c "GRANT ALL PRIVILEGES ON DATABASE mempool_monitor_db TO mempool_monitor;"

# Initialize schema (automatically done on first run)
# Or manually:
psql -d mempool_monitor_db -f init_mempool.sql
```

---

## Configuration

### Quick Start Configuration

Create a `mempool_config.yaml` file:

```yaml
name: "mainnet-mempool-monitor"

chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY"
  chain_id: 1

indexing:
  # WebSocket or HTTP protocol for mempool monitoring
  new_logs_subscription_protocol: ws  # ws, http, or http_watcher
  
  # Polling interval (only used with HTTP protocol)
  http_polling_interval_secs: 5
  
  # Subscribe to full transaction bodies or hashes only
  mempool_full_transactions: false  # false = hashes only, true = full bodies
  
  # Optional: Filter transactions by sender addresses
  filter_senders: []
  # filter_senders:
  #   - "0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD"
  
  # Optional: Filter transactions by receiver addresses
  filter_receivers: []
  # filter_receivers:
  #   - "0x1234567890123456789012345678901234567890"

postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"

# Optional: AWS RDS for additional data layer
aws_rds:
  enabled: 0  # 0 = disabled, 1 = enabled

nats:
  nats_enabled: 1
  url: "nats://localhost:4222"
  object_store_bucket: "mempool_bucket"

# Not used for mempool monitoring
contracts: []
```

---

## CLI Usage

### Basic Commands

#### 1. Single Task Mode (Default)

Run a single mempool monitoring task:

```bash
# Using default config file (./config.yaml)
./mempool-monitor

# Using custom config file
./mempool-monitor /path/to/mempool_config.yaml

# With custom database schema file
./mempool-monitor /path/to/mempool_config.yaml /path/to/init_mempool.sql
```

#### 2. API Mode

Run as an HTTP API server to manage multiple monitoring tasks:

```bash
# Start API server on default port (8080)
./mempool-monitor --api

# Custom bind address
BIND_ADDRESS="0.0.0.0:3000" ./mempool-monitor --api
```

### Command-Line Arguments

```
USAGE:
    mempool-monitor [OPTIONS] [CONFIG_FILE] [SCHEMA_FILE]

OPTIONS:
    --api              Run in API mode (HTTP server)
    -h, --help         Print help information
    -V, --version      Print version information

ARGS:
    <CONFIG_FILE>      Path to configuration file [default: ./config.yaml]
    <SCHEMA_FILE>      Path to database schema file [default: ./init_mempool.sql]
```

### Environment Variables

```bash
# API Mode
BIND_ADDRESS="0.0.0.0:8080"       # API server bind address

# Logging
RUST_LOG="info"                    # Log level: error, warn, info, debug, trace
```

### Example Usage Scenarios

#### Monitor Ethereum Mainnet (WebSocket, Full Transactions)

```bash
# config.yaml
cat > config.yaml << EOF
name: "eth-mainnet-full-tx"
chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  chain_id: 1
indexing:
  new_logs_subscription_protocol: ws
  mempool_full_transactions: true
  filter_senders: []
  filter_receivers: []
postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"
nats:
  nats_enabled: 1
  url: "nats://localhost:4222"
  object_store_bucket: "eth_mempool"
contracts: []
EOF

./mempool-monitor config.yaml
```

#### Monitor Specific Addresses (HTTP Polling, Hashes Only)

```bash
# config_filtered.yaml
cat > config_filtered.yaml << EOF
name: "uniswap-router-monitor"
chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  chain_id: 1
indexing:
  new_logs_subscription_protocol: http
  http_polling_interval_secs: 3
  mempool_full_transactions: false
  filter_senders: []
  filter_receivers:
    - "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"  # Uniswap V3 Router
postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"
nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "mempool"
contracts: []
EOF

./mempool-monitor config_filtered.yaml
```

---

## Configuration Reference

### Chain Configuration

```yaml
chain:
  # HTTP RPC endpoint URL
  http_rpc_url: "https://rpc-endpoint.example.com"
  
  # WebSocket RPC endpoint URL (required for WS protocol)
  ws_rpc_url: "wss://rpc-endpoint.example.com"
  
  # Chain ID for validation
  chain_id: 1
```

**Parameters:**
- `http_rpc_url` (required): HTTP RPC endpoint
- `ws_rpc_url` (required): WebSocket RPC endpoint
- `chain_id` (required): Blockchain chain ID

---

### Indexing Configuration

```yaml
indexing:
  # Protocol for mempool monitoring
  new_logs_subscription_protocol: ws  # ws | http | http_watcher
  
  # HTTP polling interval (seconds) - only used with http/http_watcher
  http_polling_interval_secs: 5
  
  # Subscribe to full transaction bodies or hashes only
  mempool_full_transactions: false  # true | false
  
  # Filter by sender addresses (optional)
  filter_senders:
    - "0xAddress1"
    - "0xAddress2"
  
  # Filter by receiver addresses (optional)
  filter_receivers:
    - "0xAddress3"
    - "0xAddress4"
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `new_logs_subscription_protocol` | string | `ws` | Protocol: `ws`, `http`, `http_watcher` |
| `http_polling_interval_secs` | u64 | `5` | Polling interval (HTTP only) |
| `mempool_full_transactions` | bool | `false` | `true` = full bodies, `false` = hashes only |
| `filter_senders` | array | `[]` | Filter by sender addresses |
| `filter_receivers` | array | `[]` | Filter by receiver addresses |

**Protocol Details:**

- **`ws`**: WebSocket subscription using `subscribe_pending_transactions()` or `subscribe_full_pending_transactions()`
- **`http`**: HTTP polling using manual interval checks
- **`http_watcher`**: HTTP polling using `watch_pending_transactions()` or `watch_full_pending_transactions()`

---

### PostgreSQL Configuration

```yaml
postgres:
  # libpq connection string
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  
  # Path to SQL schema initialization file
  schema: "./init_mempool.sql"
```

**DSN Format:**
```
host=<host> user=<user> password=<pass> dbname=<db> port=<port> [sslmode=<mode>]
```

---

### AWS RDS Configuration (Optional)

```yaml
aws_rds:
  enabled: 0  # 0 = disabled, 1 = enabled
  endpoint: "your-instance.abcd1234.us-west-2.rds.amazonaws.com"
  port: 5432
  database_name: "mempool_monitor_rds"
  username: "mempool_user"
  password: "secure-password"
  region: "us-west-2"
  ssl_mode: "require"  # disable, prefer, require, verify-ca, verify-full
  connection_timeout: 30
  max_connections: 10
  schema: "./init_mempool.sql"
```

---

### NATS Configuration

```yaml
nats:
  nats_enabled: 1  # 0 = disabled, 1 = enabled
  url: "nats://localhost:4222"
  object_store_bucket: "mempool_bucket"
```

**NATS Object Store Key Format:**
```
tx::<chain_id>::<tx_hash>::<sender>::<timestamp>
```

---

## Database Schema

### Table: `mempool_transactions`

```sql
CREATE TABLE IF NOT EXISTS mempool_transactions (
    id BIGSERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    transaction_hash TEXT NOT NULL UNIQUE,
    transaction_sender TEXT NOT NULL,
    transaction_receiver TEXT,
    nonce TEXT NOT NULL,
    value TEXT NOT NULL,
    gas_limit TEXT NOT NULL,
    gas_price TEXT,
    max_fee_per_gas TEXT,
    max_priority_fee_per_gas TEXT,
    input_data TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_mempool_tx_hash ON mempool_transactions(transaction_hash);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_sender ON mempool_transactions(transaction_sender);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_receiver ON mempool_transactions(transaction_receiver);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_timestamp ON mempool_transactions(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id ON mempool_transactions(chain_id);
```

### Transaction Payload Structure

```rust
pub struct TransactionPayload {
    pub chain_id: String,
    pub transaction_hash: String,
    pub transaction_sender: String,
    pub transaction_receiver: Option<String>,
    pub nonce: String,
    pub value: String,
    pub gas_limit: String,
    pub gas_price: Option<String>,
    pub max_fee_per_gas: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
    pub input_data: String,
    pub transaction_type: String,
    pub timestamp: String,
}
```

---

## API Reference

### Run in API Mode

```bash
./mempool-monitor --api
```

### Endpoints

#### 1. Health Check
```http
GET /health
```

**Response:**
```json
{
  "status": "ok"
}
```

#### 2. Start Monitoring Task
```http
POST /tasks
Content-Type: multipart/form-data

config: <config.yaml file>
schema: <init_mempool.sql file> (optional)
```

**Response:**
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "name": "mainnet-mempool-monitor"
}
```

#### 3. List Tasks
```http
GET /tasks
```

**Response:**
```json
{
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "mainnet-mempool-monitor",
      "status": "running",
      "started_at": "2025-11-09T10:30:00Z"
    }
  ]
}
```

#### 4. Stop Task
```http
DELETE /tasks/{task_id}
```

**Response:**
```json
{
  "message": "Task stopped successfully"
}
```

---

## Examples

### Example 1: Monitor All Transactions (WebSocket)

```yaml
name: "all-transactions-ws"
chain:
  http_rpc_url: "https://rpc.example.com"
  ws_rpc_url: "wss://rpc.example.com"
  chain_id: 1
indexing:
  new_logs_subscription_protocol: ws
  mempool_full_transactions: true
  filter_senders: []
  filter_receivers: []
postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"
nats:
  nats_enabled: 1
  url: "nats://localhost:4222"
  object_store_bucket: "mempool"
contracts: []
```

```bash
./mempool-monitor config.yaml
```

### Example 2: Monitor Specific Sender (HTTP Polling)

```yaml
name: "specific-sender-http"
chain:
  http_rpc_url: "https://rpc.example.com"
  ws_rpc_url: "wss://rpc.example.com"
  chain_id: 1
indexing:
  new_logs_subscription_protocol: http
  http_polling_interval_secs: 3
  mempool_full_transactions: false
  filter_senders:
    - "0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD"
  filter_receivers: []
postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"
nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "mempool"
contracts: []
```

```bash
./mempool-monitor config.yaml
```

### Example 3: Query Transactions from Database

```bash
# Connect to database
psql -d mempool_monitor_db

# Get recent transactions
SELECT 
    transaction_hash,
    transaction_sender,
    transaction_receiver,
    value,
    gas_price,
    timestamp
FROM mempool_transactions
ORDER BY timestamp DESC
LIMIT 10;

# Filter by sender
SELECT COUNT(*) 
FROM mempool_transactions 
WHERE transaction_sender = '0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD';

# Get high-value transactions
SELECT 
    transaction_hash,
    value,
    transaction_sender,
    transaction_receiver
FROM mempool_transactions
WHERE CAST(value AS NUMERIC) > 1000000000000000000  -- > 1 ETH
ORDER BY CAST(value AS NUMERIC) DESC
LIMIT 20;
```

---

## Troubleshooting

### Common Issues

#### 1. Connection Refused

**Problem:** Cannot connect to RPC endpoint

**Solution:**
```bash
# Check RPC URL
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  https://your-rpc-url.com

# Verify WebSocket connection
wscat -c wss://your-ws-url.com
```

#### 2. Database Connection Error

**Problem:** Cannot connect to PostgreSQL

**Solution:**
```bash
# Test connection
psql "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db"

# Check PostgreSQL is running
sudo systemctl status postgresql

# Check PostgreSQL logs
sudo tail -f /var/log/postgresql/postgresql-*.log
```

#### 3. No Transactions Received

**Problem:** Monitor running but no transactions stored

**Solution:**
- Check if RPC endpoint supports mempool APIs
- Verify `mempool_full_transactions` setting
- Check filters are not too restrictive
- Enable debug logging: `RUST_LOG=debug ./mempool-monitor config.yaml`

#### 4. High Memory Usage

**Problem:** Application consuming too much memory

**Solution:**
- Set `mempool_full_transactions: false` to reduce memory
- Increase PostgreSQL `max_connections` if needed
- Monitor with: `ps aux | grep mempool-monitor`

### Logging

Enable detailed logging:

```bash
# Info level (default)
RUST_LOG=info ./mempool-monitor

# Debug level
RUST_LOG=debug ./mempool-monitor

# Trace level (verbose)
RUST_LOG=trace ./mempool-monitor
```

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

