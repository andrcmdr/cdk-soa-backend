# ABI Fetcher Tools Documentation

## Overview

This crate provides two specialized command-line tools for fetching and managing Ethereum smart contract ABIs and events from Blockscout API:

1. **`abi-fetcher`** - Fetches contracts from Blockscout v2 API with support for proxy implementations and recursive nesting
2. **`contracts-fetcher`** - Fetches contracts from Blockscout v1 API (compatible with Etherscan-like endpoints)

Both tools interact with Blockscout-compatible blockchain explorers to retrieve contract information, parse ABIs, extract event signatures with Keccak256 topic hashes, and generate structured YAML output files.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Tools Overview](#tools-overview)
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
- `./target/release/abi-fetcher`
- `./target/release/contracts-fetcher`

### Build for Production

The crate is optimized for small binary sizes:

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = "z"
```

---

## Quick Start

### For abi-fetcher (Blockscout v2 API)

#### 1. Create Configuration File

Create `abi_fetcher.config.yaml`:

```yaml
blockscout:
  server: "https://blockscout.server"
  api_path: "/api/v2"
  request_timeout_seconds: 10
  max_retries: 10
  max_implementations_per_contract: null  # unlimited
  max_implementation_nesting_depth: null  # unlimited (defaults to 10)
  # Optional authentication
  # auth_user: "username"
  # auth_password: "password"

output:
  contracts_file: "contracts_output.yaml"
  abi_directory: "./abi"
  events_directory: "./events"
  events_file: "events_output.yaml"
  contracts_events_file: "contracts_events.yaml"
```

#### 2. Run abi-fetcher

```bash
./abi-fetcher abi_fetcher.config.yaml
```

---

### For contracts-fetcher (Blockscout v1/Etherscan-like API)

#### 1. Create Configuration File

Create `contracts_fetcher.config.yaml`:

```yaml
blockscout:
  server: "https://explorer.example.com"
  api_path: "/api"
  request_timeout_seconds: 30
  max_retries: 3
  abi_fetch_attempts: 5
  pagination_offset: 1000
  # Optional authentication
  # auth_user: "username"
  # auth_password: "password"

output:
  contracts_file: "./output/contracts.yaml"
  abi_directory: "./output/abis"
  events_directory: "./output/events"
  events_file: "./output/events.yaml"
  contracts_events_file: "./output/contracts-events.yaml"
```

#### 2. Run contracts-fetcher

```bash
./contracts-fetcher contracts_fetcher.config.yaml
```

---

## Tools Overview

### abi-fetcher

**Purpose:** Fetch contracts from Blockscout v2 API with comprehensive proxy implementation support.

**API Compatibility:** Blockscout v2 API (`/api/v2/smart-contracts`)

**Key Features:**
- ✅ Fetches all verified contracts from Blockscout v2 API
- ✅ Recursively processes proxy implementation contracts
- ✅ Configurable implementation nesting depth limits
- ✅ Configurable maximum implementations per contract
- ✅ Tracks verification timestamps (`verified_at`)
- ✅ Extracts and parses event definitions with Keccak256 topic hashes
- ✅ Generates individual signature files for each unique event
- ✅ Supports HTTP Basic Authentication
- ✅ Automatic pagination handling

**Workflow:**
1. Connects to Blockscout v2 API
2. Paginates through all smart contracts
3. Fetches detailed contract information including implementations
4. Recursively processes proxy implementations up to configured depth
5. Saves ABIs with parent-child relationships preserved
6. Extracts events from all ABIs
7. Generates Keccak256 topic hashes for events
8. Outputs structured YAML files with metadata

**Output:**
- `contracts_output.yaml` - All contracts sorted by verification time
- `events_output.yaml` - Unique events with topic hashes and sources
- `contracts_events.yaml` - Contract-to-events mapping
- `abi/*.json` - Individual ABI files (including implementations)
- `events/*.txt` - Event signature details

---

### contracts-fetcher

**Purpose:** Fetch contracts from Blockscout v1/Etherscan-compatible API.

**API Compatibility:** Blockscout v1 API / Etherscan-like API (`/api?module=contract&action=...`)

**Key Features:**
- ✅ Fetches verified contracts via `listcontracts` endpoint
- ✅ Fetches unverified contracts via `listcontracts` endpoint
- ✅ Attempts to retrieve ABIs for unverified contracts via `getabi` endpoint
- ✅ Configurable retry attempts for ABI fetching
- ✅ Configurable pagination size
- ✅ Extracts and parses event definitions with Keccak256 topic hashes
- ✅ Generates individual signature files for each unique event
- ✅ Supports HTTP Basic Authentication
- ✅ Automatic pagination with verification

**Workflow:**
1. Connects to Blockscout v1 API
2. Paginates through all verified contracts
3. Paginates through all unverified contracts
4. Attempts ABI retrieval for unverified contracts
5. Saves each contract's ABI as a separate JSON file
6. Extracts events from all ABIs
7. Generates Keccak256 topic hashes for events
8. Outputs structured YAML files with metadata

**Output:**
- `contracts.yaml` - All contracts with verification status
- `events.yaml` - Unique events with topic hashes and sources
- `contracts-events.yaml` - Contract-to-events mapping
- `abis/*.json` - Individual ABI files
- `events/*.txt` - Event signature details

---

## CLI Reference

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
| 0 | Success - All contracts and implementations processed |
| 1 | Error - Configuration loading failed, API errors, or I/O errors |

#### Examples

```bash
# Use custom config
./abi-fetcher abi_fetcher.config.yaml

# With authentication via environment
export BLOCKSCOUT_AUTH_USER="admin"
export BLOCKSCOUT_AUTH_PASSWORD="secret123"
./abi-fetcher abi_fetcher.config.yaml

# With debug logging
RUST_LOG=debug ./abi-fetcher abi_fetcher.config.yaml
```

---

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
| 0 | Success - All contracts processed successfully |
| 1 | Error - Configuration loading failed, API errors, or I/O errors |

#### Examples

```bash
# Use custom config
./contracts-fetcher contracts_fetcher.config.yaml

# With authentication and info logging
export BLOCKSCOUT_AUTH_USER="viewer"
export BLOCKSCOUT_AUTH_PASSWORD="pass456"
RUST_LOG=info ./contracts-fetcher contracts_fetcher.config.yaml

# Production mode
RUST_LOG=warn ./contracts-fetcher production.yaml
```

---

## Configuration File Reference

### abi-fetcher Configuration

```yaml
blockscout:
  # Required: Base URL of Blockscout instance (without trailing slash)
  server: "https://blockscout.server"
  
  # Required: API path for v2 API
  api_path: "/api/v2"
  
  # Optional: HTTP request timeout in seconds (default: 30)
  request_timeout_seconds: 10
  
  # Optional: Maximum retry attempts for failed requests (default: 3)
  max_retries: 10
  
  # Optional: Maximum implementations per contract (null = unlimited)
  max_implementations_per_contract: null
  
  # Optional: Maximum implementation nesting depth (null = unlimited, defaults to 10)
  max_implementation_nesting_depth: null
  
  # Optional: HTTP Basic Authentication
  auth_user: null
  auth_password: null

output:
  # Required: Path to output contracts YAML file
  contracts_file: "contracts_output.yaml"
  
  # Required: Directory for storing individual ABI JSON files
  abi_directory: "./abi"
  
  # Required: Directory for storing individual event signature files
  events_directory: "./events"
  
  # Required: Path to output events YAML file
  events_file: "events_output.yaml"
  
  # Required: Path to output contracts-events mapping YAML file
  contracts_events_file: "contracts_events.yaml"
```

#### abi-fetcher Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `server` | String (URL) | **Yes** | - | Blockscout v2 server base URL |
| `api_path` | String (path) | **Yes** | - | API endpoint path (typically `/api/v2`) |
| `request_timeout_seconds` | Integer | No | `30` | HTTP request timeout in seconds |
| `max_retries` | Integer | No | `3` | Maximum retry attempts for failed requests |
| `max_implementations_per_contract` | Integer or null | No | `null` | Limit implementations processed per contract |
| `max_implementation_nesting_depth` | Integer or null | No | `null` | Maximum depth for recursive implementation processing (defaults to 10 if null) |
| `auth_user` | String or null | No | `null` | HTTP Basic Authentication username |
| `auth_password` | String or null | No | `null` | HTTP Basic Authentication password |

---

### contracts-fetcher Configuration

```yaml
blockscout:
  # Required: Base URL of Blockscout instance (without trailing slash)
  server: "https://explorer.example.com"
  
  # Required: API path for v1/Etherscan-like API
  api_path: "/api"
  
  # Optional: HTTP request timeout in seconds (default: 30)
  request_timeout_seconds: 30
  
  # Optional: Maximum retry attempts for failed requests (default: 3)
  max_retries: 3
  
  # Optional: Retry attempts specifically for ABI fetching (default: 5)
  abi_fetch_attempts: 5
  
  # Optional: Pagination size for contract list endpoints (default: 1000)
  pagination_offset: 1000
  
  # Optional: HTTP Basic Authentication
  auth_user: null
  auth_password: null

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

#### contracts-fetcher Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `server` | String (URL) | **Yes** | - | Blockscout v1 server base URL |
| `api_path` | String (path) | **Yes** | - | API endpoint path (typically `/api`) |
| `request_timeout_seconds` | Integer | No | `30` | HTTP request timeout in seconds |
| `max_retries` | Integer | No | `3` | Maximum retry attempts for failed requests |
| `abi_fetch_attempts` | Integer | No | `5` | Specific retry attempts for ABI fetching operations |
| `pagination_offset` | Integer | No | `1000` | Number of items per page for contract list pagination |
| `auth_user` | String or null | No | `null` | HTTP Basic Authentication username |
| `auth_password` | String or null | No | `null` | HTTP Basic Authentication password |

---

## Output Files

### abi-fetcher Output Files

#### 1. contracts_output.yaml

Contains all contracts with implementations, sorted by verification time (most recent first).

```yaml
metadata:
  generated_at: "2025-11-08T12:34:56.789012Z"
  blockscout_server: "https://blockscout.server"
  total_verified: 150
  total_unverified: 50
  total_verified_with_abi: 145
  total_unverified_with_abi: 5
  total_verified_implementations_with_abi: 89
  total_unverified_implementations_with_abi: 2
  abi_directory: "./abi"

verified_contracts:
  - name: "MyToken"
    address: "0x1234567890123456789012345678901234567890"
    abi_file: "./abi/MyToken_0x1234567890123456789012345678901234567890.json"
    is_verified: true
    is_fully_verified: true
    verified_at: "2025-11-08T10:30:45Z"
    implementations:
      - name: "TokenImplementation"
        address: "0xabcd...1234"
        abi_file: "./abi/TokenImplementation_0xabcd...1234_parent_0x1234...7890.json"
        is_verified: true
        is_fully_verified: true
        verified_at: "2025-11-07T15:20:30Z"
        implementations: null

unverified_contracts:
  - name: null
    address: "0x9876543210987654321098765432109876543210"
    abi_file: null
    is_verified: false
    is_fully_verified: null
    verified_at: null
    implementations: null
```

#### 2. events_output.yaml

Contains all unique event signatures with sources sorted by verification time.

```yaml
metadata:
  generated_at: "2025-11-08T12:34:56.789012Z"
  blockscout_server: "https://blockscout.server"
  total_events: 45
  total_unique_signatures: 45
  events_directory: "./events"

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
        verified_at: "2025-11-08T10:30:45Z"
        contract_name: "MyToken"
      - address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        verified_at: "2025-11-07T15:20:30Z"
        contract_name: "AnotherToken"
    signature_file: "./events/Transfer_address_address_uint256_.txt"
```

#### 3. contracts_events.yaml

Maps contracts to their events, with addresses sorted by verification time (most recent first).

```yaml
contracts:
  - name: "MyToken"
    address:
      - address: "0x1234567890123456789012345678901234567890"
        verified_at: "2025-11-08T10:30:45Z"
      - address: "0x2222222222222222222222222222222222222222"
        verified_at: "2025-11-06T08:15:20Z"
    events:
      - event: "Transfer(address indexed from, address indexed to, uint256 value)"
      - event: "Approval(address indexed owner, address indexed spender, uint256 value)"
```

#### 4. Individual ABI Files (abi/)

**Filename format:**
- Main contracts: `{ContractName}_{Address}.json`
- Implementations: `{ContractName}_{Address}_parent_{ParentAddress}.json`

**Example:** `TokenImplementation_0xabcd1234_parent_0x12345678.json`

#### 5. Individual Event Signature Files (events/)

**Filename:** `{SanitizedSignature}.txt`

```
Event Name: Transfer
Signature: Transfer(address,address,uint256)
Topic Hash: 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef
Anonymous: false

Inputs:
  0: from address (indexed)
  1: to address (indexed)
  2: value uint256 (not indexed)

Contract Sources (sorted by verification time, most recent first):
  - 0x1234567890123456789012345678901234567890 (contract_name: MyToken, verified_at: 2025-11-08T10:30:45Z)
  - 0xabcdefabcdefabcdefabcdefabcdefabcdefabcd (contract_name: AnotherToken, verified_at: 2025-11-07T15:20:30Z)
```

---

### contracts-fetcher Output Files

#### 1. contracts.yaml

Contains all fetched contracts with verification status.

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

unverified_contracts:
  - name: null
    address: "0x9876543210987654321098765432109876543210"
    abi_file: null
    is_verified: false
```

#### 2. events.yaml

Contains all unique event signatures.

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
    signature_file: "Transfer_address_address_uint256_.txt"
```

#### 3. contracts-events.yaml

Maps contracts to their events.

```yaml
contracts:
  - name: "MyToken"
    address:
      - address: "0x1234567890123456789012345678901234567890"
    events:
      - event: "Transfer(address indexed from, address indexed to, uint256 value)"
      - event: "Approval(address indexed owner, address indexed spender, uint256 value)"
```

#### 4. Individual ABI Files (abis/)

**Filename:** `{ContractName}_{Address}.json`

#### 5. Individual Event Signature Files (events/)

**Filename:** `{SanitizedSignature}.txt`

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
```

---

## Authentication

Both tools support HTTP Basic Authentication for protected Blockscout instances.

### Method 1: Configuration File

**For abi-fetcher:**
```yaml
blockscout:
  auth_user: "your_username"
  auth_password: "your_password"
```

**For contracts-fetcher:**
```yaml
blockscout:
  auth_user: "your_username"
  auth_password: "your_password"
```

### Method 2: Environment Variables (Recommended for Production)

```bash
export BLOCKSCOUT_AUTH_USER="your_username"
export BLOCKSCOUT_AUTH_PASSWORD="your_password"

# For abi-fetcher
./abi-fetcher config.yaml

# For contracts-fetcher
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
RUST_LOG=error ./abi-fetcher config.yaml
RUST_LOG=error ./contracts-fetcher config.yaml

# Warnings and errors
RUST_LOG=warn ./abi-fetcher config.yaml
RUST_LOG=warn ./contracts-fetcher config.yaml

# Info, warnings, and errors (recommended)
RUST_LOG=info ./abi-fetcher config.yaml
RUST_LOG=info ./contracts-fetcher config.yaml

# Debug output (verbose)
RUST_LOG=debug ./abi-fetcher config.yaml
RUST_LOG=debug ./contracts-fetcher config.yaml

# Trace output (very verbose, for troubleshooting)
RUST_LOG=trace ./abi-fetcher config.yaml
RUST_LOG=trace ./contracts-fetcher config.yaml
```

**Default:** If `RUST_LOG` is not set, the default level is `debug`.

### Log Output Examples

**Info Level (abi-fetcher):**
```
2025-11-08T12:34:56.789Z INFO  Loaded configuration from config.yaml
2025-11-08T12:34:56.790Z INFO  Blockscout server: https://blockscout.server
2025-11-08T12:34:56.791Z INFO  HTTP Basic Authentication is enabled
2025-11-08T12:34:56.792Z INFO  Max implementations per contract: unlimited
2025-11-08T12:34:56.793Z INFO  Max implementation nesting depth: unlimited (fallback to 10)
2025-11-08T12:34:57.123Z INFO  Fetching contracts from: https://blockscout.server/api/v2/smart-contracts
2025-11-08T12:34:58.456Z INFO  Fetched 50 contracts in this page
```

**Info Level (contracts-fetcher):**
```
2025-11-08T12:34:56.789Z INFO  Loaded configuration from config.yaml
2025-11-08T12:34:56.790Z INFO  Blockscout server: https://explorer.example.com
2025-11-08T12:34:56.791Z INFO  HTTP Basic Authentication is disabled
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

### Example 1: abi-fetcher with Implementation Limits

```yaml
blockscout:
  server: "https://blockscout.mainnet.com"
  api_path: "/api/v2"
  max_implementations_per_contract: 3
  max_implementation_nesting_depth: 2
output:
  contracts_file: "contracts.yaml"
  abi_directory: "./abi"
  events_directory: "./events"
  events_file: "events.yaml"
  contracts_events_file: "contracts_events.yaml"
```

```bash
RUST_LOG=info ./abi-fetcher config.yaml
```

### Example 2: abi-fetcher with Authentication

```bash
export BLOCKSCOUT_AUTH_USER="admin"
export BLOCKSCOUT_AUTH_PASSWORD="SecurePass123"
RUST_LOG=info ./abi-fetcher abi_fetcher.config.yaml
```

### Example 3: contracts-fetcher with Custom Retry Settings

```yaml
blockscout:
  server: "https://testnet.explorer.com"
  api_path: "/api"
  request_timeout_seconds: 60
  max_retries: 5
  abi_fetch_attempts: 10
  pagination_offset: 500
output:
  contracts_file: "./output/contracts.yaml"
  abi_directory: "./output/abis"
  events_directory: "./output/events"
  events_file: "./output/events.yaml"
  contracts_events_file: "./output/contracts-events.yaml"
```

```bash
RUST_LOG=debug ./contracts-fetcher testnet-config.yaml
```

### Example 4: contracts-fetcher with Authentication

```bash
export BLOCKSCOUT_AUTH_USER="viewer"
export BLOCKSCOUT_AUTH_PASSWORD="ViewPass456"
RUST_LOG=info ./contracts-fetcher production.yaml
```

### Example 5: Processing Specific Network (Polygon)

**For abi-fetcher:**
```bash
cat > polygon-abi-config.yaml << EOF
blockscout:
  server: "https://polygon.blockscout.com"
  api_path: "/api/v2"
  pagination_offset: 1000
output:
  contracts_file: "./polygon-contracts.yaml"
  abi_directory: "./polygon-abis"
  events_directory: "./polygon-events"
  events_file: "./polygon-events.yaml"
  contracts_events_file: "./polygon-contracts-events.yaml"
EOF

RUST_LOG=info ./abi-fetcher polygon-abi-config.yaml
```

**For contracts-fetcher:**
```bash
cat > polygon-contracts-config.yaml << EOF
blockscout:
  server: "https://polygon-explorer.example.com"
  api_path: "/api"
  pagination_offset: 1000
output:
  contracts_file: "./polygon-contracts.yaml"
  abi_directory: "./polygon-abis"
  events_directory: "./polygon-events"
  events_file: "./polygon-events.yaml"
  contracts_events_file: "./polygon-contracts-events.yaml"
EOF

RUST_LOG=info ./contracts-fetcher polygon-contracts-config.yaml
```

### Example 6: Both Tools with Different APIs

```bash
# Using abi-fetcher for Blockscout v2
RUST_LOG=info ./abi-fetcher abi_fetcher.config.yaml

# Using contracts-fetcher for Blockscout v1/Etherscan-like
RUST_LOG=info ./contracts-fetcher contracts_fetcher.config.yaml
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
- Ensure config file exists in the current directory, or
- Specify the full path: `./abi-fetcher /path/to/config.yaml` or `./contracts-fetcher /path/to/config.yaml`

#### 2. Authentication Failures

**Error:**
```
HTTP error: 401 Unauthorized
```

**Solution:**
- Verify credentials in config file or environment variables
- Check that `BLOCKSCOUT_AUTH_USER` and `BLOCKSCOUT_AUTH_PASSWORD` are correctly set
- Ensure the Blockscout instance requires authentication

#### 3. API Version Mismatch

**Symptoms:**
- `abi-fetcher` fails with 404 errors
- `contracts-fetcher` returns unexpected JSON structure

**Solution:**
- **abi-fetcher** requires Blockscout v2 API (`/api/v2/smart-contracts`)
- **contracts-fetcher** requires Blockscout v1 API (`/api?module=contract&action=...`)
- Verify the API version of your Blockscout instance
- Use the correct tool for your API version

#### 4. API Rate Limiting

**Symptoms:**
```
Request failed with status 429, retrying... (attempt 1/3)
```

**Solution:**
- Increase `request_timeout_seconds` in config
- Increase `max_retries`
- For **contracts-fetcher**: reduce `pagination_offset` to make smaller requests
- For **contracts-fetcher**: increase `abi_fetch_attempts` for better retry handling

#### 5. Network Timeouts

**Error:**
```
HTTP request failed for contract 0x1234...: operation timed out
```

**Solution:**
- Increase `request_timeout_seconds` (default: 30)
- Check network connectivity to Blockscout server
- Verify the server URL in configuration

#### 6. Invalid ABI Format (contracts-fetcher)

**Warning:**
```
Failed to parse ABI response for contract 0x1234...: expected value at line 1 column 1
```

**Solution:**
- This is expected for unverified contracts
- Tool automatically retries based on `abi_fetch_attempts`
- Check Blockscout API response manually if persistent

#### 7. Implementation Recursion Limits (abi-fetcher)

**Warning:**
```
Maximum implementation nesting depth (2) reached, stopping implementation processing
```

**Solution:**
- Increase `max_implementation_nesting_depth` in config
- Set to `null` for unlimited (defaults to hardcoded limit of 10)

#### 8. Output Directory Permissions

**Error:**
```
Failed to create directory: "./output/abis": Permission denied
```

**Solution:**
- Ensure write permissions for output directories
- Create directories manually: `mkdir -p output/{abis,events}` or `mkdir -p abi events`
- Check file system permissions

### Debug Mode

For detailed troubleshooting of both tools, enable trace-level logging:

**abi-fetcher:**
```bash
RUST_LOG=trace ./abi-fetcher config.yaml 2>&1 | tee abi-fetcher-debug.log
```

**contracts-fetcher:**
```bash
RUST_LOG=trace ./contracts-fetcher config.yaml 2>&1 | tee contracts-fetcher-debug.log
```

This captures all HTTP requests, responses, and internal processing details.

---

## API Endpoints Used

### abi-fetcher (Blockscout v2)

| Endpoint | Purpose | Parameters |
|----------|---------|------------|
| `GET /api/v2/smart-contracts` | List all smart contracts with pagination | `items_count`, `hash` |
| `GET /api/v2/smart-contracts/{address}` | Fetch contract details and implementations | Address in URL path |

### contracts-fetcher (Blockscout v1)

| Endpoint | Purpose | Parameters |
|----------|---------|------------|
| `GET /api?module=contract&action=listcontracts` | List contracts | `filter=verified/unverified`, `offset={pagination_offset}`, `page={page_number}` |
| `GET /api?module=contract&action=getabi` | Fetch contract ABI | `address={contract_address}` |

---

## Performance Considerations

### Optimization Tips

1. **Pagination Size (contracts-fetcher)**: Larger `pagination_offset` values (e.g., 1000) reduce the number of API calls but may hit server limits
2. **Retry Strategy**: Balance `max_retries` and `abi_fetch_attempts` for reliability vs. speed
3. **Timeout Values**: Increase `request_timeout_seconds` for slower networks or large responses
4. **Implementation Limits (abi-fetcher)**: Use `max_implementations_per_contract` and `max_implementation_nesting_depth` to control processing time
5. **Parallel Processing**: Both tools currently process sequentially; consider running multiple instances with different filters if needed

### Expected Runtime

#### abi-fetcher
- **Small networks** (<1000 contracts): 10-30 minutes
- **Medium networks** (1000-10000 contracts): 1-3 hours
- **Large networks** (>10000 contracts): 3+ hours

Runtime depends on:
- Network latency to Blockscout server
- Number of contracts and their implementations
- Implementation nesting depth
- API rate limits

#### contracts-fetcher
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
2. Enable debug logging: `RUST_LOG=debug` or `RUST_LOG=trace`
3. Review Blockscout API documentation for your specific instance
4. Verify network connectivity and authentication
5. Ensure you're using the correct tool for your API version

---
