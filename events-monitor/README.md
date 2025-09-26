# Event Monitor API

This application can run in two modes:

## Single Task Mode (Original)
```bash
cargo run -- config.yaml init.sql
```

## API Server Mode
```bash
cargo run -- --api
```

The server binds to `0.0.0.0:8080` by default. Use the `BIND_ADDRESS` environment variable to change this:

```bash
BIND_ADDRESS=127.0.0.1:3000 cargo run -- --api
```

## API Endpoints

### Create Task
```http
POST /api/tasks
Content-Type: multipart/form-data

Fields:
- config_yaml: YAML configuration content
- name: (optional) Task name
- db_schema: (optional) Database schema SQL content
```

### List Tasks
```http
GET /api/tasks
```

### Get Task Details
```http
GET /api/tasks/{task_id}
```

### Stop Task
```http
POST /api/tasks/{task_id}/stop
```

### Delete Task
```http
DELETE /api/tasks/{task_id}
```

### Health Check
```http
GET /api/health
```

## Example Usage

```bash
# Start the API server
cargo run -- --api

# Create a task using curl
curl -X POST http://localhost:8080/api/tasks \
  -F "name=my-monitor" \
  -F "config_yaml=@config.yaml" \
  -F "db_schema=@init.sql"

# List all tasks
curl http://localhost:8080/api/tasks

# Stop a task
curl -X POST http://localhost:8080/api/tasks/{task_id}/stop
```

## Configuration

Add an optional `name` field to your YAML configuration:

```yaml
name: "testnet-events-monitor"
chain:
  http_rpc_url: "https://rpc.polygon-cdk-chain.example"
  ws_rpc_url:   "wss://rpc.polygon-cdk-chain.example/ws"
  chain_id:  1101
  # ... rest of configuration
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
