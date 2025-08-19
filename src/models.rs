use serde::{Deserialize, Serialize};

// These structs would be used to store received reports from the API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReport {
    pub artifact_address: String,
    pub revenue: String, // Using String for NUMERIC(78,0) to handle large numbers
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub artifact_address: String,
    pub usage: String, // Using String for NUMERIC(78,0) to handle large numbers
    pub timestamp: i64,
}

impl RevenueReport {
    pub fn new(artifact_address: String, revenue: String, timestamp: i64) -> Self {
        Self {
            artifact_address,
            revenue,
            timestamp,
        }
    }

}

impl UsageReport {
    pub fn new(artifact_address: String, usage: String, timestamp: i64) -> Self {
        Self {
            artifact_address,
            usage,
            timestamp,
        }
    }

}