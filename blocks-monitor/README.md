# Blocks Monitor

A high-performance Ethereum blockchain blocks monitoring and indexing service built with Rust and Alloy. Monitor blocks in real-time or sync historical data with support for both block headers and full blocks with transaction filtering.

## Features

- **Flexible Block Monitoring**
  - Block headers only (lightweight)
  - Full blocks with transaction details
  - Real-time block subscription
  - Historical block synchronization

- **Multiple Transport Protocols**
  - WebSocket (real-time subscriptions)
  - HTTP RPC (polling with configurable intervals)
  - Protocol selection per task (historical vs real-time)

- **Transaction Filtering**
  - Filter by sender addresses
  - Filter by receiver addresses
  - Configurable filter lists

- **Dual Storage Architecture**
  - Local PostgreSQL (primary)
  - AWS RDS PostgreSQL (optional replication)
  - NATS JetStream Object Store (optional)

- **Parallel Processing**
  - Independent historical sync task
  - Independent real-time subscription task
  - Configurable chunking for historical data
  - Non-blocking concurrent operation

- **Production Ready**
  - Comprehensive error handling
  - Automatic reconnection
  - Task isolation
  - Health monitoring
  - API mode for multi-tenant deployments

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage](#usage)
- [CLI Reference](#cli-reference)
- [Configuration Reference](#configuration-reference)
- [Database Schema](#database-schema)
- [API Mode](#api-mode)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Installation

### Prerequisites

- Rust 1.91.0 or higher
- PostgreSQL 17
- NATS Server (optional)
- Access to Ethereum RPC endpoints (HTTP and WebSocket)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd blocks-monitor

# Build the project
cargo build --release

# Binary will be available at
./target/release/blocks-monitor
```

## Quick Start

### 1. Setup PostgreSQL Database

```bash
# Create user, its privileges, database, schema
psql -h localhost -p 5432 -U postgres -f init.sql
# Or under `postgres` user in a system (user should be created beforehand)
sudo -u postgres psql -h localhost -p 5432 -U postgres -f init.sql
```

### 2. Create Database Schema

Create `init_table.sql`:

```sql
CREATE TABLE IF NOT EXISTS blocks_monitor_data (
    id BIGSERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    block_number TEXT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp TEXT NOT NULL,
    block_time TEXT NOT NULL,
    parent_hash TEXT NOT NULL,
    gas_used TEXT NOT NULL,
    gas_limit TEXT NOT NULL,
    transactions JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, block_number, block_hash)
);

CREATE INDEX IF NOT EXISTS idx_chain_id ON blocks_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_block_number ON blocks_monitor_data(block_number);
CREATE INDEX IF NOT EXISTS idx_block_hash ON blocks_monitor_data(block_hash);
CREATE INDEX IF NOT EXISTS idx_block_timestamp ON blocks_monitor_data(block_timestamp);
CREATE INDEX IF NOT EXISTS idx_block_time ON blocks_monitor_data(block_time);
CREATE INDEX IF NOT EXISTS idx_parent_hash ON blocks_monitor_data(parent_hash);
CREATE INDEX IF NOT EXISTS idx_gas_used ON blocks_monitor_data(gas_used);
CREATE INDEX IF NOT EXISTS idx_gas_limit ON blocks_monitor_data(gas_limit);
CREATE INDEX IF NOT EXISTS idx_tx_data_jsonb ON blocks_monitor_data USING gin (transactions);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash ON blocks_monitor_data(chain_id, block_number, block_hash);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp_parent_hash ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp_parent_hash_gas_used_limit ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash, gas_used, gas_limit);
```

### 3. Create Configuration File

Create `blocks_monitor.config.yaml`:

```yaml
name: "my-blocks-monitor"

chain:
  http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
  ws_rpc_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
  chain_id: 1

indexing:
  from_block: 18000000
  to_block: null  # null means sync to latest

  # Historical sync
  historical_blocks_processing: 1
  blocks_sync_protocol: http
  blocks_chunk_size: 100
  full_blocks_historical: false

  # Real-time monitoring
  new_blocks_subscription: 1
  new_blocks_subscription_protocol: ws
  full_blocks_subscription: true
  http_polling_interval_secs: 5
  http_subscription_method: "watch_full_blocks"
  ws_subscription_method: "subscribe_full_blocks"
  ws_subscription_channel_size: 10

postgres:
  dsn: "host=localhost user=blocks_monitor password=passwd dbname=blocks_monitor_db port=5432"
  schema: "./init_table.sql"

nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "blocks_bucket"
```

### 4. Run the Monitor

```bash
# Single task mode
./target/release/blocks-monitor blocks_monitor.config.yaml

# Or with custom schema path
./target/release/blocks-monitor blocks_monitor.config.yaml ./init_table.sql
```

## Configuration

### Minimal Configuration

```yaml
name: "minimal-monitor"

chain:
  http_rpc_url: "https://your-rpc.com"
  ws_rpc_url: "wss://your-rpc.com/ws"
  chain_id: 1

indexing:
  historical_blocks_processing: 0  # Disable historical sync
  new_blocks_subscription: 1       # Only real-time
  new_blocks_subscription_protocol: ws
  full_blocks_subscription: true

postgres:
  dsn: "host=localhost user=postgres password=pass dbname=blocks port=5432"
  schema: "./init_table.sql"

nats:
  nats_enabled: 0
```

### Headers Only (Lightweight)

```yaml
indexing:
  historical_blocks_processing: 1
  blocks_sync_protocol: http
  blocks_chunk_size: 1000  # Larger chunks for headers
  full_blocks_historical: false  # Headers only

  new_blocks_subscription: 1
  new_blocks_subscription_protocol: http
  full_blocks_subscription: false  # Headers only
```

### Full Blocks with Transaction Filtering

```yaml
indexing:
  historical_blocks_processing: 1
  full_blocks_historical: true

  new_blocks_subscription: 1
  full_blocks_subscription: true

  # Filter transactions
  filter_senders:
    - "0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD"
    - "0x1234567890123456789012345678901234567890"

  filter_receivers:
    - "0xdAC17F958D2ee523a2206206994597C13D831ec7"  # USDT
```

### Historical Sync Only

```yaml
indexing:
  from_block: 10000000
  to_block: 20000000  # Specific range

  historical_blocks_processing: 1
  blocks_sync_protocol: http
  blocks_chunk_size: 50
  full_blocks_historical: true

  new_blocks_subscription: 0  # Disable real-time
```

### Real-time Only

```yaml
indexing:
  historical_blocks_processing: 0  # Disable historical

  new_blocks_subscription: 1
  new_blocks_subscription_protocol: ws
  http_polling_interval_secs: 2
  full_blocks_subscription: true
```

## Usage

### Single Task Mode

Monitor blocks for a single blockchain:

```bash
# Basic usage
./blocks-monitor config.yaml

# With custom schema
./blocks-monitor config.yaml schema.sql

# With logging
RUST_LOG=info ./blocks-monitor config.yaml

# Debug mode
RUST_LOG=debug ./blocks-monitor config.yaml
```

### API Mode (Multi-tenant)

Run multiple monitoring tasks via HTTP API:

```bash
# Start API server
./blocks-monitor --api

# Custom bind address
BIND_ADDRESS="0.0.0.0:9090" ./blocks-monitor --api
```

#### API Endpoints

**Start a monitoring task:**
```bash
curl -X POST http://localhost:8080/api/tasks \
  -F "name=ethereum-blocks-monitor" \
  -F "config_yaml=@./config.yaml" \
  -F "db_schema=@./schema.sql"
```

**List all tasks:**
```bash
curl http://localhost:8080/api/tasks
```

**Get task status:**
```bash
curl http://localhost:8080/api/tasks/{task_id}
```

**Stop a task:**
```bash
curl -X POST http://localhost:8080/api/tasks/{task_id}/stop
```

**Delete task:**
```bash
curl -X DELETE http://localhost:8080/api/tasks/{task_id}
```

**Health check:**
```bash
curl http://localhost:8080/api/health
```

### Docker Deployment

```dockerfile
FROM rust:1.91 as builder
WORKDIR /apps
COPY . .
RUN cargo build --release

FROM debian:testing-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /var/cache/apt/archives/*
COPY --from=builder /app/target/release/blocks-monitor /usr/local/bin/
ENTRYPOINT ["blocks-monitor"]
```

```bash
# Build
docker build -t blocks-monitor .

# Run single task
docker run -v $(pwd)/config.yaml:/config.yaml:ro blocks-monitor /config.yaml

# Run API mode
docker run -p 8080:8080 blocks-monitor --api
```

### Docker Compose

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:17
    environment:
      POSTGRES_DB: blocks_monitor_db
      POSTGRES_USER: blocks_monitor
      POSTGRES_PASSWORD: passwd
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"

  nats:
    image: nats:latest
    command: "-js -sd /data"
    volumes:
      - nats_data:/data
    ports:
      - "4222:4222"
      - "8222:8222"

  blocks-monitor:
    image: blocks-monitor
    depends_on:
      - postgres
      - nats
    volumes:
      - ./blocks_monitor.config.yaml:/config.yaml
    environment:
      RUST_LOG: info
    command: /config.yaml

volumes:
  postgres_data:
  nats_data:
```

## CLI Reference

### Commands

```bash
# Single task mode
blocks-monitor [CONFIG_PATH] [SCHEMA_PATH]

# API mode
blocks-monitor --api
```

### Arguments

| Argument | Description | Default | Required |
|----------|-------------|---------|----------|
| `CONFIG_PATH` | Path to YAML configuration file | `./config.yaml` | No |
| `SCHEMA_PATH` | Path to SQL schema file | Value from config | No |
| `--api` | Run in API server mode | - | No |

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `debug` |
| `BIND_ADDRESS` | API server bind address (API mode only) | `0.0.0.0:8080` |

### Examples

```bash
# Default configuration
./blocks-monitor

# Custom config
./blocks-monitor /path/to/config.yaml

# Custom config and schema
./blocks-monitor config.yaml schema.sql

# Info logging
RUST_LOG=info ./blocks-monitor config.yaml

# Debug logging
RUST_LOG=debug ./blocks-monitor config.yaml

# API mode
./blocks-monitor --api

# API mode with custom port
BIND_ADDRESS="0.0.0.0:9090" ./blocks-monitor --api
```

## Configuration Reference

### Complete Configuration Structure

```yaml
# Optional task name for identification
name: "string"

# Blockchain RPC endpoints (required)
chain:
  http_rpc_url: "string"      # HTTP RPC endpoint
  ws_rpc_url: "string"         # WebSocket RPC endpoint
  chain_id: number             # Chain ID

# Indexing configuration (required)
indexing:
  # Block range
  from_block: number | null    # Starting block (null = 0)
  to_block: number | null      # Ending block (null = latest)

  # Historical blocks processing
  historical_blocks_processing: 0|1           # Enable/disable
  blocks_sync_protocol: "http"|"ws"          # Transport protocol
  blocks_chunk_size: number                   # Blocks per request
  full_blocks_historical: true|false          # Headers or full blocks

  # Real-time blocks subscription
  new_blocks_subscription: 0|1                # Enable/disable
  new_blocks_subscription_protocol: "http"|"ws" # Transport protocol
  full_blocks_subscription: true|false        # Headers or full blocks
  http_polling_interval_secs: number          # HTTP polling interval
  http_subscription_method: "watch_full_blocks"|"watch_blocks" # Polling method for HTTP transport
  ws_subscription_method: "subscribe_full_blocks"|"subscribe_blocks" # Polling method for WS transport
  ws_subscription_channel_size: number # Size of the channel

  # Transaction filtering (full blocks only)
  filter_senders: ["address", ...]           # Filter by sender
  filter_receivers: ["address", ...]         # Filter by receiver

# PostgreSQL configuration (required)
postgres:
  dsn: "string"               # Connection DSN string
  schema: "string"            # Schema file path

# AWS RDS configuration (optional)
aws_rds:
  enabled: 0|1                # Enable/disable
  endpoint: "string"          # RDS endpoint
  port: number                # Port (default: 5432)
  database_name: "string"     # Database name
  username: "string"          # Username
  password: "string"          # Password
  region: "string"            # AWS region
  ssl_mode: "string"          # SSL mode
  connection_timeout: number  # Timeout in seconds
  max_connections: number     # Max connections
  schema: "string"            # Schema file path

# NATS configuration (optional)
nats:
  nats_enabled: 0|1           # Enable/disable
  url: "string"               # NATS server URL
  object_store_bucket: "string"  # Bucket name
```

### Configuration Fields

#### `name` (optional)

Task identifier for logging and monitoring.

**Type:** `string`
**Default:** `"monitor-{timestamp}"`
**Example:** `"mainnet-blocks-monitor"`

#### `chain` (required)

Blockchain RPC configuration.

##### `chain.http_rpc_url` (required)

HTTP RPC endpoint URL.

**Type:** `string`
**Example:** `"https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"`

##### `chain.ws_rpc_url` (required)

WebSocket RPC endpoint URL.

**Type:** `string`
**Example:** `"wss://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"`

##### `chain.chain_id` (required)

Blockchain chain ID.

**Type:** `number`
**Examples:**
- Ethereum Mainnet: `1`
- Goerli: `5`
- Polygon: `137`
- Arbitrum: `42161`

#### `indexing` (required)

Block indexing configuration.

##### `indexing.from_block` (optional)

Starting block number for historical sync.

**Type:** `number | null`
**Default:** `0`
**Example:** `18000000`

##### `indexing.to_block` (optional)

Ending block number for historical sync.

**Type:** `number | null`
**Default:** `null` (latest block)
**Example:** `20000000`

##### `indexing.historical_blocks_processing` (required)

Enable historical blocks processing.

**Type:** `0 | 1`
**Values:**
- `0`: Disabled
- `1`: Enabled

##### `indexing.blocks_sync_protocol` (optional)

Transport protocol for historical sync.

**Type:** `"http" | "ws"`
**Default:** `"http"`
**Recommendation:** Use `"http"` for better reliability

##### `indexing.blocks_chunk_size` (optional)

Number of blocks to fetch per request (historical sync).

**Type:** `number`
**Default:** `100`
**Range:** `1-10000`
**Recommendations:**
- Headers only: `500-1000`
- Full blocks: `10-100`
- High-latency RPC: `10-50`

##### `indexing.full_blocks_historical` (optional)

Fetch full blocks with transactions (historical).

**Type:** `boolean`
**Default:** `false`
**Values:**
- `true`: Full blocks with transactions
- `false`: Block headers only

##### `indexing.new_blocks_subscription` (required)

Enable real-time block monitoring.

**Type:** `0 | 1`
**Values:**
- `0`: Disabled
- `1`: Enabled

##### `indexing.new_blocks_subscription_protocol` (optional)

Transport protocol for real-time monitoring.

**Type:** `"ws" | "http"`
**Default:** `"http"`
**Recommendations:**
- `"ws"`: Lower latency, real-time push
- `"http"`: More reliable, works through proxies

##### `indexing.http_polling_interval_secs` (optional)

Polling interval for HTTP real-time monitoring.

**Type:** `number`
**Default:** `5`
**Range:** `1-60`
**Recommendations:**
- Fast networks: `1-2` seconds
- Standard: `5` seconds
- Rate-limited RPC: `10-30` seconds

##### `indexing.full_blocks_subscription` (optional)

Fetch full blocks with transactions (real-time).

**Type:** `boolean`
**Default:** `false`
**Values:**
- `true`: Full blocks with transactions
- `false`: Block headers only

##### `indexing.filter_senders` (optional)

Filter transactions by sender addresses.

**Type:** `array of strings`
**Default:** `[]` (no filtering)
**Example:**
```yaml
filter_senders:
  - "0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD"
  - "0x1234567890123456789012345678901234567890"
```

**Note:** Only applies when using full blocks.

##### `indexing.filter_receivers` (optional)

Filter transactions by receiver addresses.

**Type:** `array of strings`
**Default:** `[]` (no filtering)
**Example:**
```yaml
filter_receivers:
  - "0xdAC17F958D2ee523a2206206994597C13D831ec7"  # USDT
  - "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"  # USDC
```

**Note:** Only applies when using full blocks.

#### `postgres` (required)

PostgreSQL database configuration.

##### `postgres.dsn` (required)

PostgreSQL connection string.

**Type:** `string`
**Format:** `"host={host} port={port} user={user} password={pass} dbname={db}"`
**Example:** `"host=localhost user=monitor password=pass dbname=blocks port=5432"`

##### `postgres.schema` (required)

Path to SQL schema file.

**Type:** `string`
**Example:** `"./init_table.sql"`

#### `aws_rds` (optional)

AWS RDS PostgreSQL replication configuration.

##### `aws_rds.enabled` (required if section present)

Enable AWS RDS replication.

**Type:** `0 | 1`
**Values:**
- `0`: Disabled
- `1`: Enabled

##### `aws_rds.*` (various)

See AWS RDS section for complete configuration.

#### `nats` (required)

NATS JetStream configuration.

##### `nats.nats_enabled` (required)

Enable NATS publishing.

**Type:** `0 | 1`
**Values:**
- `0`: Disabled
- `1`: Enabled

##### `nats.url` (required if enabled)

NATS server URL.

**Type:** `string`
**Example:** `"nats://localhost:4222"`

##### `nats.object_store_bucket` (required if enabled)

NATS object store bucket name.

**Type:** `string`
**Example:** `"blocks_bucket"`

## Database Schema

### Blocks Table

```sql
CREATE TABLE IF NOT EXISTS blocks_monitor_data (
    id BIGSERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    block_number TEXT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp TEXT NOT NULL,
    block_time TEXT NOT NULL,
    parent_hash TEXT NOT NULL,
    gas_used TEXT NOT NULL,
    gas_limit TEXT NOT NULL,
    transactions JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, block_number, block_hash)
);

CREATE INDEX IF NOT EXISTS idx_chain_id ON blocks_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_block_number ON blocks_monitor_data(block_number);
CREATE INDEX IF NOT EXISTS idx_block_hash ON blocks_monitor_data(block_hash);
CREATE INDEX IF NOT EXISTS idx_block_timestamp ON blocks_monitor_data(block_timestamp);
CREATE INDEX IF NOT EXISTS idx_block_time ON blocks_monitor_data(block_time);
CREATE INDEX IF NOT EXISTS idx_parent_hash ON blocks_monitor_data(parent_hash);
CREATE INDEX IF NOT EXISTS idx_gas_used ON blocks_monitor_data(gas_used);
CREATE INDEX IF NOT EXISTS idx_gas_limit ON blocks_monitor_data(gas_limit);
CREATE INDEX IF NOT EXISTS idx_tx_data_jsonb ON blocks_monitor_data USING gin (transactions);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash ON blocks_monitor_data(chain_id, block_number, block_hash);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp_parent_hash ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp_parent_hash_gas_used_limit ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash, gas_used, gas_limit);
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `id` | BIGSERIAL | Primary key |
| `chain_id` | TEXT | Blockchain chain ID |
| `block_number` | TEXT | Block number |
| `block_hash` | TEXT | Block hash (0x prefixed) |
| `block_timestamp` | TEXT | Unix timestamp |
| `block_time` | TEXT | ISO 8601 timestamp |
| `parent_hash` | TEXT | Parent block hash |
| `gas_used` | TEXT | Gas used in block |
| `gas_limit` | TEXT | Gas limit |
| `transactions` | JSONB | Transactions array (if full blocks) |
| `created_at` | TIMESTAMP | Insert timestamp |

### Querying Examples

```sql
-- Get latest 10 blocks
SELECT * FROM blocks_monitor_data
ORDER BY block_number::BIGINT DESC
LIMIT 10;

-- Get blocks in range
SELECT * FROM blocks_monitor_data
WHERE chain_id = '1'
  AND block_number::BIGINT BETWEEN 18000000 AND 18001000;

-- Count transactions in a block
SELECT block_number, jsonb_array_length(transactions) as tx_count
FROM blocks_monitor_data
WHERE transactions IS NOT NULL;

-- Find transactions from specific address
SELECT block_number, tx->>'hash' as tx_hash
FROM blocks_monitor_data,
     jsonb_array_elements(transactions) as tx
WHERE tx->>'from' = '0x742d35Cc6634C0532925a3b8BC342A5b6437AFCD';

-- Average gas used per block
SELECT AVG(gas_used::BIGINT) as avg_gas
FROM blocks_monitor_data
WHERE block_number::BIGINT > 18000000;
```

## Performance Tuning

### RPC Configuration

**Chunk Size Optimization:**

```yaml
# Headers only - larger chunks
blocks_chunk_size: 1000

# Full blocks - smaller chunks
blocks_chunk_size: 50

# Rate-limited RPC
blocks_chunk_size: 10
```

**Protocol Selection:**

```yaml
# Fastest for historical
blocks_sync_protocol: http

# Lowest latency for real-time
new_blocks_subscription_protocol: ws
```

### Database Optimization

**Indexes:**

```sql
-- Additional indexes for specific queries
CREATE INDEX idx_blocks_timestamp_desc
ON blocks_monitor_data(block_timestamp DESC);

CREATE INDEX idx_blocks_gas_used
ON blocks_monitor_data(gas_used);

-- GIN index for transaction queries
CREATE INDEX idx_blocks_transactions
ON blocks_monitor_data USING gin(transactions);
```

**PostgreSQL Configuration:**

```ini
# postgresql.conf recommendations
shared_buffers = 256MB
work_mem = 16MB
maintenance_work_mem = 128MB
effective_cache_size = 1GB
max_connections = 100
```

### System Resources

**Memory Usage:**

- Headers only: ~50MB baseline
- Full blocks: ~200MB baseline
- Per chunk in memory: ~10-100MB

### Concurrent Processing

```yaml
# Multiple chains - use API mode
# Run separate instances for:
# - Different chains
# - Different block ranges
# - Headers vs full blocks
```

## Troubleshooting

### Common Issues

#### 1. Connection Failures

**Symptom:** `Failed to connect to RPC`

**Solutions:**
```yaml
# Check RPC URLs
chain:
  http_rpc_url: "https://..."  # Must be https://
  ws_rpc_url: "wss://..."      # Must be wss://

# Verify API key
http_rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY_HERE"
```

#### 2. Database Connection Issues

**Symptom:** `connection refused` or `authentication failed`

**Solutions:**
```bash
# Test connection
psql "host=localhost user=blocks_monitor password=passwd dbname=blocks_monitor_db port=5432"

# Check PostgreSQL is running
sudo systemctl status postgresql

# Verify all permissions
CREATE USER blocks_monitor WITH PASSWORD 'passwd' CREATEDB;

ALTER USER blocks_monitor CREATEDB;

CREATE DATABASE blocks_monitor_db OWNER blocks_monitor;

\c blocks_monitor_db blocks_monitor;

GRANT ALL PRIVILEGES ON DATABASE blocks_monitor_db TO blocks_monitor;
GRANT CONNECT, CREATE ON DATABASE blocks_monitor_db TO blocks_monitor;
GRANT CREATE ON SCHEMA public TO blocks_monitor;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TYPES TO blocks_monitor;

GRANT ALL ON ALL TABLES IN SCHEMA public TO blocks_monitor;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO blocks_monitor;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO blocks_monitor;
-- GRANT ALL ON ALL TYPES IN SCHEMA public TO blocks_monitor;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO blocks_monitor;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO blocks_monitor;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO blocks_monitor;
-- GRANT ALL PRIVILEGES ON ALL TYPES IN SCHEMA public TO blocks_monitor;
```

#### 3. Rate Limiting

**Symptom:** `429 Too Many Requests` or slow processing

**Solutions:**
```yaml
indexing:
  blocks_chunk_size: 10  # Reduce chunk size
  http_polling_interval_secs: 10  # Increase interval
```

#### 4. Memory Issues

**Symptom:** `Out of memory` or slow performance

**Solutions:**
```yaml
indexing:
  blocks_chunk_size: 50  # Reduce chunk size
  full_blocks_historical: false  # Use headers only
```

#### 5. Duplicate Blocks

**Symptom:** Unique constraint violation

**Solution:**
The schema has `UNIQUE (chain_id, block_number)` with `ON CONFLICT DO UPDATE`, so duplicates are automatically handled.

### Debug Mode

```bash
# Enable detailed logging
RUST_LOG=debug ./blocks-monitor config.yaml

# Trace level (very verbose)
RUST_LOG=trace ./blocks-monitor config.yaml

# Module-specific logging
RUST_LOG=events_monitor::subscriptions=debug ./blocks-monitor config.yaml
```

### Health Checks

**In single task mode:**
```bash
# Check process
ps aux | grep blocks-monitor

# Check database
psql -d blocks_monitor_db -c "SELECT COUNT(*) FROM blocks_monitor_data;"
```

**In API mode:**
```bash
# Health endpoint
curl http://localhost:8080/health

# Task status
curl http://localhost:8080/tasks
```

### Log Analysis

```bash
# Filter errors
./blocks-monitor config.yaml 2>&1 | grep ERROR

# Count processed blocks
./blocks-monitor config.yaml 2>&1 | grep "Persisting block" | wc -l

# Monitor progress
tail -f logs/monitor.log | grep "block range"
```

## Best Practices

### Configuration

1. **Start Small:** Begin with small block ranges and headers only
2. **Test First:** Validate configuration with a short test run
3. **Use HTTP:** Prefer HTTP for historical sync (more reliable)
4. **Use WebSocket:** Use WebSocket for real-time (lower latency)

### Deployment

1. **Separate Tasks:** Use different instances for historical and real-time
2. **Monitor Resources:** Track memory and CPU usage
3. **Database Maintenance:** Regular VACUUM and ANALYZE
4. **Backup Strategy:** Regular PostgreSQL backups

### Monitoring

1. **Log Aggregation:** Use centralized logging (ELK, Loki)
2. **Metrics:** Export Prometheus metrics
3. **Alerts:** Set up alerting for failures
4. **Health Checks:** Regular health endpoint checks

### Security

1. **API Keys:** Store in environment variables
2. **Database Credentials:** Use secrets management
3. **Network Security:** Firewall rules for PostgreSQL
4. **TLS:** Use SSL for database connections

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

