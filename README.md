# Oracle Service 

## API Endpoints

This section describes the available API endpoints for the Oracle Service.

### Base URL
The service runs on the configured host and port (default: `localhost:8080`)

### Endpoints

#### Health Check
- **GET** `/health`
- **Description**: Health check endpoint to verify the service is running
- **Response**: `200 OK`

#### Six Months Revenue
- **GET** `/api/v1/artifacts/{address}/six-month-revenue`
- **Description**: Get the six-month revenue for a specific artifact address
- **Parameters**:
  - `address` (path): The artifact address (e.g., "0x123...")
- **Response**:
```json
{
    "artifactAddress": "0x123...",
    "sixMonthRevenue": "5000000000000000000",
    "calculatedAt": 1640995200
}
```

#### Total Usage
- **GET** `/api/v1/artifacts/{address}/total-usage`
- **Description**: Get the total usage for a specific artifact address
- **Parameters**:
  - `address` (path): The artifact address (e.g., "0x123...")
- **Response**:
```json
{
    "artifactAddress": "0x123...",
    "totalUsage": "15000",
    "calculatedAt": 1640995200
}
```

### Error Responses
All endpoints may return error responses in the following format:
```json
{
    "error": "500 Internal Server Error",
    "message": "Database connection failed"
}
```

### Example Usage

```bash
# Health check
curl http://localhost:8080/health

# Get six months revenue for an artifact
curl http://localhost:8080/api/v1/artifacts/0x1234567890abcdef/six-month-revenue

# Get total usage for an artifact
curl http://localhost:8080/api/v1/artifacts/0x1234567890abcdef/total-usage
```

### Implementation Notes

The current implementation includes placeholder logic for the database queries. The actual database query logic needs to be implemented in the following functions:

1. `get_six_months_revenue` in `src/api.rs` - Calculate revenue for the last 6 months
2. `get_total_usage` in `src/api.rs` - Calculate total usage across all time

These functions should query the `revenue_reports` and `usage_reports` tables respectively, filtering by the provided artifact address.
