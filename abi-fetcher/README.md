# ABI Fetcher Tools Documentation

## Overview

This crate provides two command-line tools for fetching and managing Ethereum smart contract ABIs and events from Blockscout API:

1. **`contracts-fetcher`** - Fetches all contracts (verified and unverified), their ABIs, and extracts event definitions
2. **`abi-fetcher`** - Fetches individual contract details and ABIs from specific addresses

Both tools interact with Blockscout-compatible blockchain explorers to retrieve contract information, parse ABIs, extract event signatures, and generate structured output files.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Tools Overview](#tools-overview)
  - [contracts-fetcher](#contracts-fetcher)
  - [abi-fetcher](#abi-fetcher)
- [CLI Reference](#cli-reference)
- [Configuration File Reference](#configuration-file-reference)
- [Output Files](#output-files)
- [Authentication](#authentication)
- [Logging](#logging)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

---

## Installation

### Build from Source

```bash
cargo build --release
```

The compiled binaries will be available at:
- `./target/release/contracts-fetcher`
- `./target/release/abi-fetcher`

### Build for Production

The crate is optimized for small binary sizes with the following profile:

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = "z"
```

---

## Quick Start

### 1. Create Configuration File

Create a `config.yaml` file in your project directory:

```yaml
blockscout:
  server: "https://explorer.example.com"
  api_path: "/api"
  request_timeout_seconds: 30
  max_retries: 3
  abi_fetch_attempts: 5
  pagination_offset: 1000
  # Optional: uncomment for authenticated access
  # auth_user: "your_username"
  # auth_password: "your_password"

output:
  contracts_file: "./output/contracts.yaml"
  abi_directory: "./output/abis"
  events_directory: "./output/events"
  events_file: "./output/events.yaml"
  contracts_events_file: "./output/contracts-events.yaml"
```

### 2. Run contracts-fetcher

```bash
./contracts-fetcher config.yaml
```

### 3. Run abi-fetcher

```bash
./abi-fetcher config.yaml
```

---

## Configuration

### Configuration File Location

Both tools accept a configuration file path as the first command-line argument:

```bash
./contracts-fetcher /path/to/config.yaml
./abi-fetcher /path/to/custom-config.yaml
```

**Default:** If no argument is provided, both tools look for `./config.yaml` in the current directory.

### Environment Variables

Authentication credentials can be provided via environment variables (overrides config file):

```bash
export BLOCKSCOUT_AUTH_USER="your_username"
export BLOCKSCOUT_AUTH_PASSWORD="your_password"
```

---

## Tools Overview

### contracts-fetcher

**Purpose:** Comprehensive contract and event data extraction from a Blockscout instance.

**Key Features:**
- Fetches all verified contracts from Blockscout API
- Fetches all unverified contracts from Blockscout API
- Attempts to retrieve ABIs for unverified contracts
- Extracts and parses event definitions from all ABIs
- Generates Keccak256 topic hashes for non-anonymous events
- Creates individual signature files for each unique event
- Outputs structured YAML files with metadata
- Groups contracts by name in contracts-events output

**Workflow:**
1. Connects to Blockscout API with configured settings
2. Paginates through all verified contracts (using `listcontracts` endpoint)
3. Paginates through all unverified contracts
4. Saves each contract's ABI as a separate JSON file
5. Parses events from ABIs and extracts signatures
6. Generates Keccak256 topic hashes for event signatures
7. Creates individual `.txt` files for each unique event signature
8. Outputs three YAML files:
   - `contracts.yaml` - All contracts with metadata
   - `events.yaml` - All unique events with sources
   - `contracts-events.yaml` - Contract-to-events mapping

**Output:**
- `contracts.yaml` - Complete contract list with verification status
- `events.yaml` - Unique event definitions with topic hashes
- `contracts-events.yaml` - Contract-event relationships
- `abis/*.json` - Individual ABI files for each contract
- `events/*.txt` - Event signature details

---

### abi-fetcher

**Purpose:** Fetch detailed contract information for specific addresses from a database list or API.

**Key Features:**
- Fetches contract details from Blockscout smart-contracts endpoint
- Retrieves comprehensive contract metadata
- Saves contract ABIs individually
- Designed for targeted contract analysis
- Supports batch processing from address lists

**Workflow:**
1. Reads contract addresses from a configured source
2. Fetches detailed contract information via `/smart-contracts/{address}` endpoint
3. Retrieves and saves ABIs
4. Processes contract metadata

**Output:**
- Individual contract detail files
- ABI JSON files
- Structured metadata output

---

## CLI Reference

### contracts-fetcher

#### Syntax

```bash
contracts-fetcher [CONFIG_FILE]
```

#### Arguments

| Argument | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `CONFIG_FILE` | String (path) | No | `./config.yaml` | Path to YAML configuration file |

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success - All contracts and events processed successfully |
| 1 | Error - Configuration loading failed, API errors, or I/O errors |

#### Examples

```bash
# Use default config.yaml
./contracts-fetcher

# Specify custom config
./contracts-fetcher /etc/myapp/blockscout-config.yaml

# With authentication via environment
export BLOCKSCOUT_AUTH_USER="admin"
export BLOCKSCOUT_AUTH_PASSWORD="secret123"
./contracts-fetcher production-config.yaml

# With custom logging level
RUST_LOG=info ./contracts-fetcher config.yaml
RUST_LOG=debug ./contracts-fetcher config.yaml
RUST_LOG=trace ./contracts-fetcher config.yaml
```

---

### abi-fetcher

#### Syntax

```bash
abi-fetcher [CONFIG_FILE]
```

#### Arguments

| Argument | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `CONFIG_FILE` | String (path) | No | `./config.yaml` | Path to YAML configuration file |

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success - All contract details fetched successfully |
| 1 | Error - Configuration loading failed, API errors, or I/O errors |

#### Examples

```bash
# Use default config.yaml
./abi-fetcher

# Specify custom config
./abi-fetcher /opt/configs/abi-config.yaml

# With authentication and debug logging
export BLOCKSCOUT_AUTH_USER="viewer"
export BLOCKSCOUT_AUTH_PASSWORD="pass456"
RUST_LOG=debug ./abi-fetcher config.yaml

# Production mode with minimal logging
RUST_LOG=warn ./abi-fetcher production.yaml
```

---

## Configuration File Reference

### Complete Configuration Structure

```yaml
blockscout:
  # Required: Base URL of Blockscout instance (without trailing slash)
  server: "https://explorer.example.com"

  # Required: API path (with or without leading slash)
  api_path: "/api"

  # Optional: HTTP request timeout in seconds (default: 30)
  request_timeout_seconds: 30

  # Optional: Maximum retry attempts for failed requests (default: 3)
  max_retries: 3

  # Optional: Retry attempts specifically for ABI fetching (default: 5)
  abi_fetch_attempts: 5

  # Optional: Pagination size for contract list endpoints (default: 1000)
  pagination_offset: 1000

  # Optional: HTTP Basic Authentication username
  auth_user: "your_username"

  # Optional: HTTP Basic Authentication password
  auth_password: "your_password"

output:
  # Required: Path to output contracts YAML file
  contracts_file: "./output/contracts.yaml"

  # Required: Directory for storing individual ABI JSON files
  abi_directory: "./output/abis"

  # Required: Directory for storing individual event signature files
  events_directory: "./output/events"

  # Required: Path to output events YAML file
  events_file: "./output/events.yaml"

  # Required: Path to output contracts-events mapping YAML file
  contracts_events_file: "./output/contracts-events.yaml"
```

### Configuration Parameters

#### `blockscout` Section

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `server` | String (URL) | **Yes** | - | Blockscout server base URL (e.g., `https://explorer.polygon.com`) |
| `api_path` | String (path) | **Yes** | - | API endpoint path (typically `/api`) |
| `request_timeout_seconds` | Integer | No | `30` | HTTP request timeout in seconds |
| `max_retries` | Integer | No | `3` | Maximum retry attempts for failed HTTP requests |
| `abi_fetch_attempts` | Integer | No | `5` | Specific retry attempts for ABI fetching operations |
| `pagination_offset` | Integer | No | `1000` | Number of items per page for contract list pagination |
| `auth_user` | String | No | - | HTTP Basic Authentication username (can be set via `BLOCKSCOUT_AUTH_USER` env var) |
| `auth_password` | String | No | - | HTTP Basic Authentication password (can be set via `BLOCKSCOUT_AUTH_PASSWORD` env var) |

**Notes:**
- `abi_fetch_attempts` is separate from `max_retries` to allow more aggressive retrying for ABI operations
- Pagination automatically continues until no more results are returned
- Environment variables `BLOCKSCOUT_AUTH_USER` and `BLOCKSCOUT_AUTH_PASSWORD` override config file values

#### `output` Section

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `contracts_file` | String (path) | **Yes** | - | Output file path for contracts YAML |
| `abi_directory` | String (path) | **Yes** | - | Directory where individual ABI JSON files will be saved |
| `events_directory` | String (path) | **Yes** | - | Directory where individual event signature text files will be saved |
| `events_file` | String (path) | **Yes** | - | Output file path for events YAML |
| `contracts_events_file` | String (path) | **Yes** | - | Output file path for contracts-events mapping YAML |

**Notes:**
- Directories are created automatically if they don't exist
- ABI files are named: `{ContractName}_{Address}.json`
- Event files are named: `{EventSignature}.txt` (sanitized)

---

## Output Files

### 1. contracts.yaml

Contains all fetched contracts with metadata.

**Structure:**
```yaml
metadata:
  generated_at: "2025-11-08T12:34:56.789012Z"
  blockscout_server: "https://explorer.example.com"
  total_verified: 150
  total_unverified: 50
  total_with_abi: 175
  abi_directory: "./output/abis"

verified_contracts:
  - name: "MyToken"
    address: "0x1234567890123456789012345678901234567890"
    abi_file: "MyToken_0x1234567890123456789012345678901234567890.json"
    is_verified: true
  - name: "AnotherContract"
    address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    abi_file: "AnotherContract_0xabcdefabcdefabcdefabcdefabcdefabcdefabcd.json"
    is_verified: true

unverified_contracts:
  - name: null
    address: "0x9876543210987654321098765432109876543210"
    abi_file: null
    is_verified: false
```

### 2. events.yaml

Contains all unique event signatures with their sources.

**Structure:**
```yaml
metadata:
  generated_at: "2025-11-08T12:34:56.789012Z"
  blockscout_server: "https://explorer.example.com"
  total_events: 45
  total_unique_signatures: 45
  events_directory: "./output/events"

events:
  - name: "Transfer"
    signature: "Transfer(address,address,uint256)"
    topic_hash: "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
    anonymous: false
    inputs:
      - name: "from"
        input_type: "address"
        indexed: true
      - name: "to"
        input_type: "address"
        indexed: true
      - name: "value"
        input_type: "uint256"
        indexed: false
    contract_sources:
      - address: "0x1234567890123456789012345678901234567890"
        contract_name: "MyToken"
      - address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        contract_name: "AnotherToken"
    signature_file: "Transfer_address_address_uint256_.txt"
```

### 3. contracts-events.yaml

Maps contracts to their events (grouped by contract name).

**Structure:**
```yaml
contracts:
  - name: "MyToken"
    address:
      - address: "0x1234567890123456789012345678901234567890"
      - address: "0x2222222222222222222222222222222222222222"
    events:
      - event: "Transfer(address indexed from, address indexed to, uint256 value)"
      - event: "Approval(address indexed owner, address indexed spender, uint256 value)"

  - name: "MyNFT"
    address:
      - address: "0x3333333333333333333333333333333333333333"
    events:
      - event: "Transfer(address indexed from, address indexed to, uint256 indexed tokenId)"
      - event: "Approval(address indexed owner, address indexed approved, uint256 indexed tokenId)"
```

### 4. Individual ABI Files (abis/)

Each contract's ABI is saved as a separate JSON file:

**Filename:** `{ContractName}_{Address}.json`

**Example:** `MyToken_0x1234567890123456789012345678901234567890.json`

```json
[
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      {
        "name": "from",
        "type": "address",
        "indexed": true
      },
      {
        "name": "to",
        "type": "address",
        "indexed": true
      },
      {
        "name": "value",
        "type": "uint256",
        "indexed": false
      }
    ],
    "anonymous": false
  }
]
```

### 5. Individual Event Signature Files (events/)

Each unique event signature is saved with details:

**Filename:** `{SanitizedSignature}.txt`

**Example:** `Transfer_address_address_uint256_.txt`

```
Event Name: Transfer
Signature: Transfer(address,address,uint256)
Topic Hash: 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef
Anonymous: false

Inputs:
  0: from address (indexed)
  1: to address (indexed)
  2: value uint256 (not indexed)

Contract Sources:
  - 0x1234567890123456789012345678901234567890 (contract_name: MyToken)
  - 0xabcdefabcdefabcdefabcdefabcdefabcdefabcd (contract_name: AnotherToken)
```

---

## Authentication

### HTTP Basic Authentication

Both tools support HTTP Basic Authentication for protected Blockscout instances.

#### Method 1: Configuration File

```yaml
blockscout:
  auth_user: "your_username"
  auth_password: "your_password"
```

#### Method 2: Environment Variables (Recommended for Production)

```bash
export BLOCKSCOUT_AUTH_USER="your_username"
export BLOCKSCOUT_AUTH_PASSWORD="your_password"
./contracts-fetcher config.yaml
```

**Security Note:** Environment variables override config file values and are more secure for production deployments.

---

## Logging

Both tools use the `tracing` framework with environment-based log level control.

### Log Levels

Set the `RUST_LOG` environment variable:

```bash
# Error only (minimal output)
RUST_LOG=error ./contracts-fetcher

# Warnings and errors
RUST_LOG=warn ./contracts-fetcher

# Info, warnings, and errors (recommended)
RUST_LOG=info ./contracts-fetcher

# Debug output (verbose)
RUST_LOG=debug ./contracts-fetcher

# Trace output (very verbose, for troubleshooting)
RUST_LOG=trace ./contracts-fetcher
```

**Default:** If `RUST_LOG` is not set, the default level is `debug`.

### Log Output Examples

**Info Level:**
```
2025-11-08T12:34:56.789Z INFO  Loaded configuration from config.yaml
2025-11-08T12:34:56.790Z INFO  Blockscout server: https://explorer.example.com
2025-11-08T12:34:56.791Z INFO  HTTP Basic Authentication is enabled
2025-11-08T12:34:57.123Z INFO  Fetching verified contracts from: https://explorer.example.com/api?module=contract&action=listcontracts&filter=verified&offset=1000&page=1 (page 1)
2025-11-08T12:34:58.456Z INFO  Fetched 1000 verified contracts on page 1
```

**Debug Level:**
```
2025-11-08T12:34:59.789Z DEBUG Fetching ABI for contract 0x1234... (attempt 1/5)
2025-11-08T12:35:00.123Z DEBUG Successfully fetched ABI for contract 0x1234... on attempt 1
```

---

## Examples

### Example 1: Basic Usage with Default Config

```bash
# Create config.yaml
cat > config.yaml << EOF
blockscout:
  server: "https://polygon-explorer.example.com"
  api_path: "/api"
output:
  contracts_file: "./contracts.yaml"
  abi_directory: "./abis"
  events_directory: "./events"
  events_file: "./events.yaml"
  contracts_events_file: "./contracts-events.yaml"
EOF

# Run contracts-fetcher
./contracts-fetcher
```

### Example 2: Production Setup with Authentication

```bash
# Set credentials via environment
export BLOCKSCOUT_AUTH_USER="prod_user"
export BLOCKSCOUT_AUTH_PASSWORD="SecurePassword123"

# Run with custom config and info logging
RUST_LOG=info ./contracts-fetcher /etc/blockchain/prod-config.yaml
```

### Example 3: Testing with Custom Retry Settings

```yaml
blockscout:
  server: "https://testnet.example.com"
  api_path: "/api"
  request_timeout_seconds: 60
  max_retries: 5
  abi_fetch_attempts: 10
  pagination_offset: 500
output:
  contracts_file: "./test-output/contracts.yaml"
  abi_directory: "./test-output/abis"
  events_directory: "./test-output/events"
  events_file: "./test-output/events.yaml"
  contracts_events_file: "./test-output/contracts-events.yaml"
```

### Example 4: Processing Specific Network

```bash
# Polygon Mainnet
cat > polygon-config.yaml << EOF
blockscout:
  server: "https://polygon.blockscout.com"
  api_path: "/api"
  pagination_offset: 1000
output:
  contracts_file: "./polygon-contracts.yaml"
  abi_directory: "./polygon-abis"
  events_directory: "./polygon-events"
  events_file: "./polygon-events.yaml"
  contracts_events_file: "./polygon-contracts-events.yaml"
EOF

RUST_LOG=info ./contracts-fetcher polygon-config.yaml
```

### Example 5: Using abi-fetcher for Targeted Analysis

```bash
# Configure for specific contract details
./abi-fetcher config.yaml
```

---

## Troubleshooting

### Common Issues

#### 1. Configuration File Not Found

**Error:**
```
Failed to load application configuration: Failed to read config file: "./config.yaml"
```

**Solution:**
- Ensure `config.yaml` exists in the current directory, or
- Specify the full path: `./contracts-fetcher /path/to/config.yaml`

#### 2. Authentication Failures

**Error:**
```
HTTP error: 401 Unauthorized
```

**Solution:**
- Verify credentials in config file or environment variables
- Check that `BLOCKSCOUT_AUTH_USER` and `BLOCKSCOUT_AUTH_PASSWORD` are correctly set
- Ensure the Blockscout instance requires authentication

#### 3. API Rate Limiting

**Symptoms:**
```
Request failed with status 429, retrying... (attempt 1/3)
```

**Solution:**
- Increase `request_timeout_seconds` in config
- Reduce `pagination_offset` to make smaller requests
- Increase `max_retries` and `abi_fetch_attempts`

#### 4. Network Timeouts

**Error:**
```
HTTP request failed for contract 0x1234...: operation timed out
```

**Solution:**
- Increase `request_timeout_seconds` (default: 30)
- Check network connectivity to Blockscout server
- Verify the server URL in configuration

#### 5. Invalid ABI Format

**Warning:**
```
Failed to parse ABI response for contract 0x1234...: expected value at line 1 column 1
```

**Solution:**
- This is expected for unverified contracts
- Tool automatically retries based on `abi_fetch_attempts`
- Check Blockscout API response manually if persistent

#### 6. Output Directory Permissions

**Error:**
```
Failed to create directory: "./output/abis": Permission denied
```

**Solution:**
- Ensure write permissions for output directories
- Create directories manually: `mkdir -p output/{abis,events}`
- Check file system permissions

### Debug Mode

For detailed troubleshooting, enable trace-level logging:

```bash
RUST_LOG=trace ./contracts-fetcher config.yaml 2>&1 | tee debug.log
```

This captures all HTTP requests, responses, and internal processing details.

---

## API Endpoints Used

### contracts-fetcher

| Endpoint | Purpose | Parameters |
|----------|---------|------------|
| `GET /api?module=contract&action=listcontracts` | List contracts | `filter=verified/unverified`, `offset={pagination_offset}`, `page={page_number}` |
| `GET /api?module=contract&action=getabi` | Fetch contract ABI | `address={contract_address}` |

### abi-fetcher

| Endpoint | Purpose | Parameters |
|----------|---------|------------|
| `GET /api/smart-contracts/{address}` | Fetch contract details | Address in URL path |

---

## Performance Considerations

### Optimization Tips

1. **Pagination Size**: Larger `pagination_offset` values (e.g., 1000) reduce the number of API calls but may hit server limits
2. **Retry Strategy**: Balance `max_retries` and `abi_fetch_attempts` for reliability vs. speed
3. **Timeout Values**: Increase `request_timeout_seconds` for slower networks or large responses
4. **Parallel Processing**: Both tools currently process sequentially; consider running multiple instances with different filters if needed

### Expected Runtime

- **Small networks** (<1000 contracts): 5-15 minutes
- **Medium networks** (1000-10000 contracts): 30-120 minutes
- **Large networks** (>10000 contracts): 2+ hours

Runtime depends on:
- Network latency to Blockscout server
- Number of contracts
- API rate limits
- Number of unverified contracts requiring ABI fetching

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

---

## Support

For issues or questions:
1. Check the [Troubleshooting](#troubleshooting) section
2. Enable debug logging: `RUST_LOG=debug`
3. Review Blockscout API documentation for your specific instance
4. Verify network connectivity and authentication

---
