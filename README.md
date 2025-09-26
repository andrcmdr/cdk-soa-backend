# Oracle Service

A Rust-based blockchain oracle service that mines usage and revenue data from external APIs and submits batched reports to smart contracts.

## Architecture

The service consists of three main components:

### Core Components

1. **API Miner**: Periodically fetches usage and revenue data from external APIs with configurable pagination and delay mechanisms to ensure data consistency.

2. **Database Layer**: PostgreSQL database storing revenue reports, usage reports, and mining state tracking with proper indexing for efficient queries.

3. **Blockchain Client**: Handles batched submission of aggregated data to smart contracts using the Alloy library for Ethereum interactions.

4. **REST API**: Provides endpoints for querying artifact revenue and usage data. (In Version 1 of oracle, they are just placeholders)

### Data Flow

```
External APIs → API Miner → PostgreSQL → Blockchain Client → Smart Contract
                    ↓
              REST API (Query Interface)
```

The service runs three concurrent tasks:
- **Mining Task**: Fetches data from external APIs every 5 minutes with a 2-minute delay buffer
- **Batching Task**: Submits accumulated data to blockchain every 10 minutes in batches of 40 records
- **API Server**: Serves HTTP requests on port 8080

## Deployment

### Prerequisites

- Docker and Docker Compose
- Environment variables for sensitive configuration

### Environment Variables

Copy `.env.example` as `.env`. Override placeholder values in `.env` with appropriate values


### Docker Deployment

```bash
# Create environment file
cp .env.example .env
# Edit .env with your configuration

# Deploy with Docker Compose
docker-compose up -d

# Check logs
docker-compose logs -f oracle-service
```

### Local Development

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run PostgreSQL
docker-compose up -d db

# Update config.toml for local database
# Change database.host from "db" to "localhost"

# Run the service
cargo run
```

## API Endpoints (Placeholders for now)

- `GET /health` - Health check
- `GET /api/v1/artifacts/{address}/six-month-revenue` - Get 6-month revenue for artifact
- `GET /api/v1/artifacts/{address}/total-usage` - Get total usage for artifact

## Configuration

Key configuration parameters in `config.toml`:

- **Mining Interval**: 300 seconds (5 minutes)
- **Mining Delay**: 120 seconds buffer for data consistency
- **Batch Size**: 40 records per blockchain transaction
- **Batch Interval**: 600 seconds (10 minutes)
- **Bootstrap Lookback**: 86400 seconds (24 hours) for initial data mining

## Assumptions

1. **External API Stability**: The external API provides consistent data format and maintains reasonable uptime
2. **Blockchain Connectivity**: Reliable RPC endpoint access with sufficient gas for batch transactions
3. **Data Consistency**: 2-minute delay buffer is sufficient for external API data settlement
4. **Artifact Addresses**: All artifact addresses are valid Ethereum addresses (42 characters, 0x-prefixed)
5. **Database Persistence**: PostgreSQL data persists across service restarts via Docker volumes

## Future Implementation

1. Check the working end-to-end with actual API with Sentient Chat data.
2. Integrate nats-based indexer as another mining method.
3. Implement API functions.