# Mempool Monitor - Comprehensive Documentation

## Table of Contents
1. [Overview](#overview)
2. [Installation](#installation)
3. [Configuration Reference](#configuration-reference)
4. [CLI Usage](#cli-usage)
5. [API Reference](#api-reference)
6. [Database Schema](#database-schema)
7. [Logging](#logging)
8. [Examples](#examples)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The **Mempool Monitor** is a high-performance Rust application designed to monitor and capture pending transactions from blockchain networks in real-time. It supports both WebSocket and HTTP RPC protocols with configurable transaction filtering.

### Key Features

- ✅ **Real-time monitoring** of pending transactions (mempool)
- ✅ **Dual protocol support**: WebSocket and HTTP RPC
- ✅ **Flexible subscription modes**: Transaction hashes only or full transaction bodies
- ✅ **Address filtering**: Filter by sender and/or receiver addresses
- ✅ **Data persistence**: PostgreSQL (local) and optional AWS RDS
- ✅ **Event streaming**: NATS JetStream Object Store support
- ✅ **API mode**: RESTful API for managing multiple monitoring tasks
- ✅ **Production-ready**: Built with Tokio async runtime

---

## Installation

### Prerequisites

- Rust 1.91.0 or later
- PostgreSQL 12+
- NATS Server (optional)
- Ethereum-compatible RPC endpoint (WebSocket and/or HTTP)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/andrcmdr/cdk-dev-stack
cd cdk-dev-stack/mempool-monitor/

# Build release binary
cargo build --release

# Binary location
./target/release/mempool-monitor
```

### Database Setup

```bash
# Create database
createdb mempool_monitor_db

# Create user (optional)
psql -c "CREATE USER mempool_monitor WITH PASSWORD 'passwd';"
psql -c "GRANT ALL PRIVILEGES ON DATABASE mempool_monitor_db TO mempool_monitor;"
```

---

## Configuration Reference

### Complete Configuration Example

```yaml
name: "eth-mainnet-mempool"

chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY"
  chain_id: 1

indexing:
  # Protocol for mempool monitoring
  new_tx_subscription_protocol: ws  # ws | http | http_watcher

  # HTTP polling interval (seconds) - only for HTTP protocol
  http_polling_interval_secs: 5

  # Subscribe to full transaction bodies or hashes only
  mempool_full_transactions: false  # false = hashes, true = full bodies

  # Optional: Filter by sender addresses
  filter_senders: []
  # filter_senders:
  #   - "0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD"

  # Optional: Filter by receiver addresses
  filter_receivers: []
  # filter_receivers:
  #   - "0x1234567890123456789012345678901234567890"

postgres:
  dsn: "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db port=5432"
  schema: "./init_mempool.sql"

# Optional: AWS RDS configuration
aws_rds:
  enabled: 0  # 0 = disabled, 1 = enabled
  endpoint: "your-instance.region.rds.amazonaws.com"
  port: 5432
  database_name: "mempool_monitor_rds"
  username: "mempool_user"
  password: "secure-password"
  region: "us-west-2"
  ssl_mode: "require"
  connection_timeout: 30
  max_connections: 10
  schema: "./init_mempool.sql"

nats:
  nats_enabled: 1  # 0 = disabled, 1 = enabled
  url: "nats://localhost:4222"
  object_store_bucket: "mempool_bucket"

# Not used for mempool monitoring
contracts: []
```

### Configuration Parameters

#### Chain Configuration

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `http_rpc_url` | string | Yes | HTTP RPC endpoint URL |
| `ws_rpc_url` | string | Yes | WebSocket RPC endpoint URL |
| `chain_id` | u64 | Yes | Blockchain chain ID |

#### Indexing Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `new_tx_subscription_protocol` | string | `ws` | Protocol: `ws`, `http`, `http_watcher` |
| `http_polling_interval_secs` | u64 | `5` | Polling interval (HTTP only) |
| `mempool_full_transactions` | bool | `false` | `false` = hashes only, `true` = full bodies |
| `filter_senders` | array | `[]` | Filter by sender addresses |
| `filter_receivers` | array | `[]` | Filter by receiver addresses |

**Protocol Options:**
- **`ws`**: WebSocket subscription (recommended)
- **`http`**: Manual HTTP polling
- **`http_watcher`**: HTTP polling with watcher

#### PostgreSQL Configuration

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `dsn` | string | Yes | libpq connection string |
| `schema` | string | Yes | Path to SQL schema file |

**DSN Format:**
```
host=<host> user=<user> password=<pass> dbname=<db> port=<port> [sslmode=<mode>]
```

#### NATS Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `nats_enabled` | u8 | `0` | 0 = disabled, 1 = enabled |
| `url` | string | - | NATS server URL |
| `object_store_bucket` | string | - | JetStream bucket name |

---

## CLI Usage

### Command Syntax

```bash
mempool-monitor [OPTIONS] [CONFIG_FILE] [SCHEMA_FILE]
```

### Options

| Option | Description |
|--------|-------------|
| `--api` | Run in API mode (HTTP server) |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

### Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `CONFIG_FILE` | `./config.yaml` | Path to configuration file |
| `SCHEMA_FILE` | `./init_mempool.sql` | Path to database schema file |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDRESS` | `0.0.0.0:8080` | API server bind address (API mode only) |
| `RUST_LOG` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |

### Usage Examples

#### Single Task Mode (Default)

```bash
# Use default config file
./mempool-monitor

# Use custom config file
./mempool-monitor /path/to/config.yaml

# Specify both config and schema files
./mempool-monitor /path/to/config.yaml /path/to/schema.sql
```

#### API Mode

```bash
# Start API server (default port 8080)
./mempool-monitor --api

# Custom bind address
BIND_ADDRESS="0.0.0.0:3000" ./mempool-monitor --api
```

---

## API Reference

### Starting the API Server

```bash
./mempool-monitor --api
```

Default endpoint: `http://localhost:8080`

---

### 1. Health Check

Check if the API server is running.

**Endpoint:** `GET /health`

**Method:** `GET`

**Content-Type:** N/A

**Request Parameters:** None

**Response Format:**

```json
{
  "status": "ok"
}
```

**Example (cURL):**

```bash
curl -X GET http://localhost:8080/health
```

**Response:**

```json
{
  "status": "ok"
}
```

---

### 2. Start Monitoring Task

Create and start a new mempool monitoring task.

**Endpoint:** `POST /tasks`

**Method:** `POST`

**Content-Type:** `multipart/form-data`

**Request Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `config` | file | Yes | YAML configuration file |
| `schema` | file | No | SQL schema file (optional) |

**Response Format:**

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "name": "eth-mainnet-mempool"
}
```

**Example (cURL):**

```bash
curl -X POST http://localhost:8080/tasks \
  -F "config=@mempool_config.yaml" \
  -F "schema=@init_mempool.sql"
```

**Response:**

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "name": "eth-mainnet-mempool"
}
```

---

### 3. List All Tasks

Retrieve a list of all running monitoring tasks.

**Endpoint:** `GET /tasks`

**Method:** `GET`

**Content-Type:** N/A

**Request Parameters:** None

**Response Format:**

```json
{
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "eth-mainnet-mempool",
      "status": "running",
      "started_at": "2025-11-09T10:30:00Z"
    },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "name": "polygon-mempool",
      "status": "running",
      "started_at": "2025-11-09T11:15:00Z"
    }
  ]
}
```

**Example (cURL):**

```bash
curl -X GET http://localhost:8080/tasks
```

**Response:**

```json
{
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "eth-mainnet-mempool",
      "status": "running",
      "started_at": "2025-11-09T10:30:00Z"
    }
  ]
}
```

---

### 4. Stop Task

Stop a specific monitoring task.

**Endpoint:** `DELETE /tasks/{task_id}`

**Method:** `DELETE`

**Content-Type:** N/A

**Request Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `task_id` | string (UUID) | Yes | Task ID to stop |

**Response Format:**

```json
{
  "message": "Task stopped successfully"
}
```

**Example (cURL):**

```bash
curl -X DELETE http://localhost:8080/tasks/550e8400-e29b-41d4-a716-446655440000
```

**Response:**

```json
{
  "message": "Task stopped successfully"
}
```

**Error Response (Task Not Found):**

```json
{
  "error": "Task not found"
}
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

### Query Examples

```sql
-- Get recent transactions
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

-- Filter by sender
SELECT COUNT(*)
FROM mempool_transactions
WHERE transaction_sender = '0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD';

-- High-value transactions
SELECT
    transaction_hash,
    value,
    transaction_sender
FROM mempool_transactions
WHERE CAST(value AS NUMERIC) > 1000000000000000000  -- > 1 ETH
ORDER BY CAST(value AS NUMERIC) DESC;
```

---

## Logging

The Mempool Monitor uses the `tracing` crate for structured logging with multiple log levels.

### Log Levels

| Level | Description | Usage |
|-------|-------------|-------|
| `error` | Critical errors requiring immediate attention | System failures, database errors |
| `warn` | Warning conditions that should be reviewed | Connection issues, retries |
| `info` | Informational messages (default) | Task start/stop, configuration loaded |
| `debug` | Detailed debugging information | Transaction processing, protocol details |
| `trace` | Very detailed trace information | Raw data, internal state changes |

### Setting Log Level

#### Via Environment Variable (Recommended)

```bash
# Info level (default)
RUST_LOG=info ./mempool-monitor

# Debug level
RUST_LOG=debug ./mempool-monitor

# Trace level (verbose)
RUST_LOG=trace ./mempool-monitor

# Error level only
RUST_LOG=error ./mempool-monitor

# Module-specific logging
RUST_LOG=mempool_monitor=debug,tokio=info ./mempool-monitor

# Multiple modules
RUST_LOG=mempool_monitor::processor=trace,mempool_monitor::db=debug ./mempool-monitor
```

#### Via System Environment

```bash
# Set globally for session
export RUST_LOG=debug
./mempool-monitor

# Unset
unset RUST_LOG
```

### Log Output Format

The monitor uses a **compact** log format by default:

```
2025-11-09T10:30:15.123Z  INFO mempool_monitor: Starting Mempool Monitor in single task mode
2025-11-09T10:30:15.456Z  INFO mempool_monitor::config: Loaded configuration from config.yaml
2025-11-09T10:30:15.789Z  INFO mempool_monitor::db: Local PostgreSQL ready
2025-11-09T10:30:16.012Z  INFO mempool_monitor::processor: Starting mempool monitoring (protocol: ws)
2025-11-09T10:30:16.345Z DEBUG mempool_monitor::processor: Received transaction hash: 0xabcd...
```

### Log Fields

Each log entry contains:

| Field | Description |
|-------|-------------|
| **Timestamp** | ISO 8601 format with millisecond precision |
| **Level** | Log level (ERROR, WARN, INFO, DEBUG, TRACE) |
| **Module** | Source module path |
| **Message** | Log message |
| **Fields** | Additional structured data (if present) |

### Common Log Messages

#### Startup

```
INFO mempool_monitor: Starting Mempool Monitor in single task mode
INFO mempool_monitor::config: Loaded configuration from config.yaml
INFO mempool_monitor::db: Local PostgreSQL ready
INFO mempool_monitor::nats: Connected to NATS at nats://localhost:4222
```

#### Monitoring

```
INFO mempool_monitor::processor: Starting mempool monitoring (protocol: ws)
DEBUG mempool_monitor::processor: Subscribed to pending transactions
DEBUG mempool_monitor::processor: Received transaction: 0xabcd1234...
INFO mempool_monitor::db: Transaction inserted: 0xabcd1234...
```

#### Errors

```
ERROR mempool_monitor::db: Failed to insert transaction: connection refused
WARN mempool_monitor::processor: WebSocket connection lost, reconnecting...
ERROR mempool_monitor::nats: NATS connection error: timeout
```

### Filtering Logs

#### By Module

```bash
# Only show database logs
RUST_LOG=mempool_monitor::db=debug ./mempool-monitor

# Show processor and database logs
RUST_LOG=mempool_monitor::processor=info,mempool_monitor::db=info ./mempool-monitor
```

#### By Level

```bash
# Show warnings and errors only
RUST_LOG=warn ./mempool-monitor

# Show info and above (info, warn, error)
RUST_LOG=info ./mempool-monitor
```

### Redirecting Logs

#### To File

```bash
# Redirect stdout and stderr
./mempool-monitor > mempool.log 2>&1

# With log level
RUST_LOG=debug ./mempool-monitor > mempool.log 2>&1

# Append to file
RUST_LOG=info ./mempool-monitor >> mempool.log 2>&1
```

#### Using tee (Display + Save)

```bash
# Display and save
RUST_LOG=info ./mempool-monitor 2>&1 | tee mempool.log

# Append mode
RUST_LOG=info ./mempool-monitor 2>&1 | tee -a mempool.log
```

### Log Rotation

For production use, consider using a log rotation tool:

```bash
# Using logrotate (Linux)
cat > /etc/logrotate.d/mempool-monitor << EOF
/var/log/mempool-monitor/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0640 mempool_monitor mempool_monitor
}
EOF

# Using logrotate manually
logrotate -f /etc/logrotate.d/mempool-monitor
```

### Troubleshooting with Logs

#### Enable Full Debug Logging

```bash
RUST_LOG=trace ./mempool-monitor 2>&1 | tee debug.log
```

#### Filter Specific Issues

```bash
# Database issues
RUST_LOG=mempool_monitor::db=trace ./mempool-monitor

# Connection issues
RUST_LOG=mempool_monitor::processor=debug ./mempool-monitor

# NATS issues
RUST_LOG=mempool_monitor::nats=debug ./mempool-monitor
```

---

## Examples

### Example 1: Monitor All Transactions (WebSocket)

**config.yaml:**
```yaml
name: "all-tx-ws"
chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  chain_id: 1
indexing:
  new_tx_subscription_protocol: ws
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
RUST_LOG=info ./mempool-monitor config.yaml
```

### Example 2: Filter by Sender (HTTP Polling)

**config.yaml:**
```yaml
name: "filtered-sender"
chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
  chain_id: 1
indexing:
  new_tx_subscription_protocol: http
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
RUST_LOG=debug ./mempool-monitor config.yaml
```

---

## Troubleshooting

### Common Issues

#### 1. Connection Refused

**Problem:** Cannot connect to RPC endpoint

**Solution:**
```bash
# Test RPC connection
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  https://your-rpc-url.com

# Enable debug logging
RUST_LOG=debug ./mempool-monitor config.yaml
```

#### 2. Database Connection Error

**Problem:** Cannot connect to PostgreSQL

**Solution:**
```bash
# Test connection
psql "host=localhost user=mempool_monitor password=passwd dbname=mempool_monitor_db"

# Check PostgreSQL status
sudo systemctl status postgresql

# View logs
RUST_LOG=mempool_monitor::db=trace ./mempool-monitor
```

#### 3. No Transactions Received

**Problem:** Monitor running but no transactions stored

**Solution:**
- Check if RPC supports mempool APIs
- Verify `mempool_full_transactions` setting
- Check filters are not too restrictive
- Enable debug logging: `RUST_LOG=debug ./mempool-monitor`

#### 4. High Memory Usage

**Problem:** Application consuming too much memory

**Solution:**
- Set `mempool_full_transactions: false`
- Monitor with: `ps aux | grep mempool-monitor`
- Check logs: `RUST_LOG=warn ./mempool-monitor`

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

