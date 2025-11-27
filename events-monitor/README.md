# Event Monitor (with Web API and multi-threaded run-time)

A comprehensive blockchain event monitoring service with a RESTful API for managing multiple monitoring tasks. Each task can independently monitor different blockchain contracts and their events.

## Table of Contents

- [Overview](#overview)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [API Endpoints](#api-endpoints)
  - [Create Task](#create-task)
  - [List Tasks](#list-tasks)
  - [Get Task Details](#get-task-details)
  - [Stop Task](#stop-task)
  - [Delete Task](#delete-task)
  - [Health Check](#health-check)
- [Data Models](#data-models)
- [Error Handling](#error-handling)
- [Examples](#examples)
- [Environment Variables](#environment-variables)

## Overview

The Event Monitor provides two operational modes:

1. **Single Task Mode**: Traditional standalone monitoring (original behavior)
2. **API Server Mode**: Multi-task management via REST API

The API server allows you to:
- Create multiple independent monitoring tasks
- Monitor different blockchain networks simultaneously
- Manage task lifecycle (start, stop, monitor status)
- Upload configurations dynamically
- Track task execution status and health

## Getting Started

### Prerequisites

- Rust 1.90.0 or later
- PostgreSQL database
- NATS server (optional, for event streaming)
- Access to blockchain RPC endpoints (HTTP and WebSocket)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd events-monitor

# Build the application
cargo build --release
```

### Running the API Server

```bash
# Start the API server on default port 8080
cargo run -- --api

# Or specify a custom bind address
BIND_ADDRESS=127.0.0.1:3000 cargo run -- --api
```

### Running in Single Task Mode

```bash
# Traditional single task mode
cargo run -- config.yaml init.sql
```

## Configuration

The application uses YAML configuration files. Here's a minimal example:

```yaml
name: "my-blockchain-monitor"  # Optional: Task identifier name

chain:
  http_rpc_url: "https://ethereum-rpc.publicnode.com"
  ws_rpc_url: "wss://ethereum-rpc.publicnode.com"
  chain_id: 1

indexing:
  from_block: 18500000      # Optional: Start from specific block
  to_block:                 # Optional: End at specific block (null for live)
  all_logs_processing: 1    # 1 to process historical logs, 0 to skip

postgres:
  dsn: "host=localhost user=monitor password=secret dbname=events_db port=5432"
  schema: "./init_table.sql"

nats:
  nats_enabled: 1           # 1 to enable NATS, 0 to disable
  url: "nats://localhost:4222"
  object_store_bucket: "events_bucket"

contracts:
  - name: "USDC"
    address: "0xA0b86a33E6BbC172f7dD4aFE71A95d4b0d08c5f"
    abi_path: "./abi/USDC.json"
    implementations: null   # For proxy contracts, list implementation contracts
```

## API Endpoints

### Create Task

Creates a new blockchain monitoring task with the provided configuration.

**Endpoint:** `POST /api/tasks`

**Content-Type:** `multipart/form-data`

#### Request Parameters

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `config_yaml` | string | Yes | Complete YAML configuration content for the monitoring task |
| `name` | string | No | Human-readable task name. If not provided, will use config name or generate timestamp-based name |
| `db_schema` | string | No | SQL schema content for database initialization. If not provided, will read from config schema path |

#### Request Body

Multipart form with the following fields:

- **config_yaml**: The complete YAML configuration as a string
- **name**: Optional task identifier 
- **db_schema**: Optional SQL schema content

#### Response

**Success (201 Created):**
```json
{
  "task_id": "123e4567-e89b-12d3-a456-426614174000",
  "message": "Task 'my-blockchain-monitor' created successfully"
}
```

**Error (4xx/5xx):**
```json
{
  "error": "Invalid YAML configuration: missing required field 'chain'"
}
```

#### Example

```bash
curl -X POST http://localhost:8080/api/tasks \
  -F "name=ethereum-usdc-monitor" \
  -F "config_yaml=@./config.yaml" \
  -F "db_schema=@./schema.sql"
```

```bash
# With inline configuration
curl -X POST http://localhost:8080/api/tasks \
  -F "name=quick-test" \
  -F 'config_yaml=name: "test-monitor"
chain:
  http_rpc_url: "https://eth.publicnode.com"
  ws_rpc_url: "wss://eth.publicnode.com"
  chain_id: 1
indexing:
  from_block: 18500000
postgres:
  dsn: "postgresql://user:pass@localhost/events"
  schema: "./init.sql"
nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "events"
contracts: []'
```

---

### List Tasks

Retrieves a list of all monitoring tasks and their current status.

**Endpoint:** `GET /api/tasks`

#### Request Parameters

None

#### Response

**Success (200 OK):**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "ethereum-usdc-monitor",
    "status": "Running",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:05Z"
  },
  {
    "id": "987fcdeb-51d3-47b8-9c2a-8b5d4e7f6a9b",
    "name": "polygon-staking-monitor",
    "status": "Stopped",
    "created_at": "2024-01-15T09:15:00Z",
    "updated_at": "2024-01-15T10:25:00Z"
  }
]
```

#### Example

```bash
curl http://localhost:8080/api/tasks
```

```bash
# With pretty formatting
curl -s http://localhost:8080/api/tasks | jq '.'
```

---

### Get Task Details

Retrieves detailed information about a specific monitoring task.

**Endpoint:** `GET /api/tasks/{task_id}`

#### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `task_id` | string | Yes | UUID of the monitoring task |

#### Response

**Success (200 OK):**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "name": "ethereum-usdc-monitor",
  "status": "Running",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:05Z"
}
```

**Error (404 Not Found):**
```json
{
  "error": "Task not found: 123e4567-e89b-12d3-a456-426614174000"
}
```

#### Example

```bash
curl http://localhost:8080/api/tasks/123e4567-e89b-12d3-a456-426614174000
```

---

### Stop Task

Gracefully stops a running monitoring task.

**Endpoint:** `POST /api/tasks/{task_id}/stop`

#### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `task_id` | string | Yes | UUID of the monitoring task to stop |

#### Response

**Success (200 OK):**
```json
{
  "message": "Task 123e4567-e89b-12d3-a456-426614174000 stop signal sent"
}
```

**Error (404 Not Found):**
```json
{
  "error": "Task not found: 123e4567-e89b-12d3-a456-426614174000"
}
```

#### Example

```bash
curl -X POST http://localhost:8080/api/tasks/123e4567-e89b-12d3-a456-426614174000/stop
```

---

### Delete Task

Stops and removes a monitoring task from the system.

**Endpoint:** `DELETE /api/tasks/{task_id}`

#### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `task_id` | string | Yes | UUID of the monitoring task to delete |

#### Response

**Success (200 OK):**
```json
{
  "message": "Task 123e4567-e89b-12d3-a456-426614174000 deletion requested"
}
```

#### Example

```bash
curl -X DELETE http://localhost:8080/api/tasks/123e4567-e89b-12d3-a456-426614174000
```

---

### Health Check

Returns the health status of the API server.

**Endpoint:** `GET /api/health`

#### Response

**Success (200 OK):**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Example

```bash
curl http://localhost:8080/api/health
```

## Data Models

### TaskInfo

Represents the state and metadata of a monitoring task.

```json
{
  "id": "string",           // UUID of the task
  "name": "string",         // Human-readable task name
  "status": "TaskStatus",   // Current task status (see below)
  "created_at": "string",   // ISO 8601 timestamp of creation
  "updated_at": "string"    // ISO 8601 timestamp of last update
}
```

### TaskStatus

Possible task status values:

- **`Starting`**: Task is initializing and setting up connections
- **`Running`**: Task is actively monitoring blockchain events
- **`Stopping`**: Task received stop signal and is shutting down gracefully
- **`Stopped`**: Task completed execution or was stopped
- **`Failed(string)`**: Task encountered an error (error message included)

### EventPayload

Structure of blockchain events processed by the monitor:

```json
{
  "contract_name": "string",           // Name of the contract
  "contract_address": "string",        // Contract address (0x...)
  "implementation_name": "string",     // Implementation name (for proxy contracts)
  "implementation_address": "string",  // Implementation address (for proxy contracts)
  "chain_id": "string",               // Blockchain chain ID
  "block_number": "string",           // Block number containing the event
  "block_hash": "string",             // Block hash (0x...)
  "block_timestamp": "string",        // Block timestamp (Unix timestamp)
  "block_time": "string",             // Block time (ISO 8601)
  "transaction_hash": "string",       // Transaction hash (0x...)
  "transaction_sender": "string",     // Transaction sender address
  "transaction_receiver": "string",   // Transaction receiver address
  "transaction_index": "string",      // Transaction index in block
  "log_index": "string",              // Log index in transaction
  "log_hash": "string",               // Unique log identifier hash
  "event_name": "string",             // Name of the emitted event
  "event_signature": "string",        // Event signature hash (0x...)
  "event_data": "object"              // Decoded event parameters
}
```

## Error Handling

The API uses standard HTTP status codes and returns consistent error responses:

### Status Codes

- **200 OK**: Successful request
- **201 Created**: Resource created successfully
- **400 Bad Request**: Invalid request parameters or body
- **404 Not Found**: Requested resource not found
- **500 Internal Server Error**: Server error

### Error Response Format

```json
{
  "error": "Descriptive error message"
}
```

### Common Errors

| Error | Description | Solution |
|-------|-------------|----------|
| `Invalid YAML configuration: ...` | Malformed or invalid YAML config | Check YAML syntax and required fields |
| `Task not found: {id}` | Task ID doesn't exist | Verify task ID from `/api/tasks` endpoint |
| `Database connection failed: ...` | Cannot connect to PostgreSQL | Check database credentials and availability |
| `Failed to read database schema: ...` | Cannot read schema file | Ensure schema file exists and is readable |

## Examples

### Complete Workflow Example

```bash
#!/bin/bash

API_BASE="http://localhost:8080"

# 1. Check API health
echo "Checking API health..."
curl -s "$API_BASE/api/health" | jq '.'

# 2. Create a new monitoring task
echo "Creating monitoring task..."
TASK_RESPONSE=$(curl -s -X POST "$API_BASE/api/tasks" \
  -F "name=ethereum-events-monitor" \
  -F "config_yaml=@./ethereum-config.yaml" \
  -F "db_schema=@./init_schema.sql")

TASK_ID=$(echo "$TASK_RESPONSE" | jq -r '.task_id')
echo "Created task: $TASK_ID"

# 3. Monitor task status
echo "Task details:"
curl -s "$API_BASE/api/tasks/$TASK_ID" | jq '.'

# 4. List all tasks
echo "All tasks:"
curl -s "$API_BASE/api/tasks" | jq '.'

# 5. Let it run for 30 seconds
echo "Letting task run for 30 seconds..."
sleep 30

# 6. Stop the task
echo "Stopping task..."
curl -s -X POST "$API_BASE/api/tasks/$TASK_ID/stop" | jq '.'

# 7. Verify task stopped
sleep 2
curl -s "$API_BASE/api/tasks/$TASK_ID" | jq '.'

# 8. Clean up - delete task
echo "Deleting task..."
curl -s -X DELETE "$API_BASE/api/tasks/$TASK_ID" | jq '.'
```

### Python Client Example

```python
import requests
import json
import time

API_BASE = "http://localhost:8080"

def create_task(name, config_yaml, db_schema=None):
    """Create a new monitoring task"""
    files = {
        'name': (None, name),
        'config_yaml': (None, config_yaml),
    }
    if db_schema:
        files['db_schema'] = (None, db_schema)

    response = requests.post(f"{API_BASE}/api/tasks", files=files)
    return response.json()

def get_task_status(task_id):
    """Get task status"""
    response = requests.get(f"{API_BASE}/api/tasks/{task_id}")
    return response.json()

def stop_task(task_id):
    """Stop a task"""
    response = requests.post(f"{API_BASE}/api/tasks/{task_id}/stop")
    return response.json()

def list_tasks():
    """List all tasks"""
    response = requests.get(f"{API_BASE}/api/tasks")
    return response.json()

# Example usage
config = """
name: "python-created-monitor"
chain:
  http_rpc_url: "https://eth.publicnode.com"
  ws_rpc_url: "wss://eth.publicnode.com"
  chain_id: 1
indexing:
  from_block: 18500000
postgres:
  dsn: "postgresql://user:pass@localhost/events"
  schema: "./init.sql"
nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "events"
contracts: []
"""

# Create task
result = create_task("python-monitor", config)
task_id = result['task_id']
print(f"Created task: {task_id}")

# Monitor status
for i in range(5):
    status = get_task_status(task_id)
    print(f"Status: {status['status']}")
    time.sleep(2)

# Stop task
stop_result = stop_task(task_id)
print(f"Stop result: {stop_result}")
```

### JavaScript/Node.js Client Example

```javascript
const axios = require('axios');
const FormData = require('form-data');

const API_BASE = 'http://localhost:8080';

async function createTask(name, configYaml, dbSchema = null) {
    const form = new FormData();
    form.append('name', name);
    form.append('config_yaml', configYaml);
    if (dbSchema) {
        form.append('db_schema', dbSchema);
    }

    const response = await axios.post(`${API_BASE}/api/tasks`, form, {
        headers: form.getHeaders()
    });
    return response.data;
}

async function getTaskStatus(taskId) {
    const response = await axios.get(`${API_BASE}/api/tasks/${taskId}`);
    return response.data;
}

async function stopTask(taskId) {
    const response = await axios.post(`${API_BASE}/api/tasks/${taskId}/stop`);
    return response.data;
}

async function listTasks() {
    const response = await axios.get(`${API_BASE}/api/tasks`);
    return response.data;
}

// Example usage
(async () => {
    try {
        const config = `
name: "nodejs-monitor"
chain:
  http_rpc_url: "https://eth.publicnode.com"
  ws_rpc_url: "wss://eth.publicnode.com"
  chain_id: 1
indexing:
  from_block: 18500000
postgres:
  dsn: "postgresql://user:pass@localhost/events"
  schema: "./init.sql"
nats:
  nats_enabled: 0
  url: "nats://localhost:4222"
  object_store_bucket: "events"
contracts: []
        `;

        // Create task
        const createResult = await createTask('nodejs-monitor', config);
        console.log('Created task:', createResult.task_id);

        // Check status
        const status = await getTaskStatus(createResult.task_id);
        console.log('Task status:', status);

        // List all tasks
        const tasks = await listTasks();
        console.log('All tasks:', tasks.length);

        // Stop task after 10 seconds
        setTimeout(async () => {
            const stopResult = await stopTask(createResult.task_id);
            console.log('Stop result:', stopResult);
        }, 10000);

    } catch (error) {
        console.error('Error:', error.response?.data || error.message);
    }
})();
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDRESS` | `0.0.0.0:8080` | Server bind address and port |
| `RUST_LOG` | `debug` | Logging level (error, warn, info, debug, trace) |

## Security Considerations

- **Network Access**: The API server binds to `0.0.0.0` by default. For production, consider binding to `127.0.0.1` or specific interfaces
- **Input Validation**: All YAML configurations are validated before task creation
- **Resource Limits**: Consider implementing rate limiting for production deployments
- **Database Security**: Ensure PostgreSQL credentials are properly secured
- **File System Access**: The service reads ABI files from the local filesystem - ensure proper file permissions

## Monitoring and Observability

The service includes comprehensive logging via the `tracing` crate:

```bash
# Set logging level
export RUST_LOG=info

# Enable debug logging for specific modules
export RUST_LOG=events_monitor=debug,tower_http=info
```

Key log events:
- Task creation and lifecycle events
- Database connection status
- Blockchain event processing
- Error conditions and recoveries

## Troubleshooting

### Common Issues

1. **"Database connection failed"**
   - Verify PostgreSQL is running and accessible
   - Check database credentials in configuration
   - Ensure database exists and user has proper permissions

2. **"Failed to read database schema"**
   - Check schema file path exists
   - Verify file permissions
   - Provide schema content via API instead of file path

3. **"Invalid YAML configuration"**
   - Validate YAML syntax using online tools
   - Check all required fields are present
   - Verify ABI file paths exist

4. **Task stuck in "Starting" status**
   - Check RPC endpoint connectivity
   - Verify blockchain network accessibility
   - Review logs for connection errors

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
RUST_LOG=debug cargo run -- --api
```

## Key Features of This Implementation:

### 1. **Proper Lifetime Management**
- Uses `Arc` to share the `TaskManager` between web handlers
- Each task gets its own `EventProcessor` instance with owned data
- Shutdown channels for graceful task termination
- Automatic cleanup of finished tasks

### 2. **Thread Safety**
- `TaskManager` uses `Arc<RwLock<HashMap>>` for thread-safe task storage
- Each task runs in its own `tokio::spawn` with proper error handling
- Shutdown signals use `oneshot` channels for clean termination

### 3. **Robust Task Management**
- Tasks have proper status tracking (Starting, Running, Stopping, Stopped, Failed)
- UUID-based task IDs for uniqueness
- Graceful shutdown handling with `tokio::select!`
- Database and NATS connections are properly initialized per task

### 4. **Web API Features**
- RESTful API with proper HTTP status codes
- Multipart form support for configuration uploads
- CORS and tracing middleware
- Health check endpoint
- Comprehensive error handling

### 5. **Configuration Flexibility**
- Tasks can be created with YAML configurations via API
- Optional task naming from config or API
- Database schema can be provided via API or file
- Environment variable support for server binding

### 6. **Database Connection Handling**
- Since `tokio_postgres::Client` is not clonable, each task creates its own connection
- Proper connection lifecycle management
- Error handling for database connection failures
