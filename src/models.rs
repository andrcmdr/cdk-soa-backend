use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReport {
    pub id: Option<i32>,
    pub artifact_address: String,
    pub revenue: String, // Using String for NUMERIC(78,0) to handle large numbers
    pub timestamp: i64,
    pub submitted_to_chain: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub id: Option<i32>,
    pub artifact_address: String,
    pub usage: String, // Using String for NUMERIC(78,0) to handle large numbers
    pub timestamp: i64,
    pub submitted_to_chain: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl RevenueReport {
    pub fn new(artifact_address: String, revenue: String, timestamp: i64) -> Self {
        Self {
            id: None,
            artifact_address,
            revenue,
            timestamp,
            submitted_to_chain: false,
            created_at: None,
        }
    }

}

impl UsageReport {
    pub fn new(artifact_address: String, usage: String, timestamp: i64) -> Self {
        Self {
            id: None,
            artifact_address,
            usage,
            timestamp,
            submitted_to_chain: false,
            created_at: None,
        }
    }

}