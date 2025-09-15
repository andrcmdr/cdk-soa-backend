use serde::{Deserialize, Serialize};

// These structs would be used to store reports in the database
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

// These structs would be used to store received data from the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendRevenueData {
    pub artifact_address: String,
    pub revenue: String,
    pub timestamp_eff_from: i64,
    pub timestamp_eff_to: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendUsageData {
    pub artifact_address: String,
    pub usage: String,
    pub timestamp_eff_from: i64,
    pub timestamp_eff_to: i64,
}

impl BackendRevenueData {
    pub fn new(artifact_address: String, revenue: String, timestamp_eff_from: i64, timestamp_eff_to: i64) -> Self {
        Self { 
            artifact_address, 
            revenue, 
            timestamp_eff_from, 
            timestamp_eff_to,
        }
    }
}

impl BackendUsageData {
    pub fn new(artifact_address: String, usage: String, timestamp_eff_from: i64, timestamp_eff_to: i64) -> Self {
        Self { 
            artifact_address, 
            usage, 
            timestamp_eff_from, 
            timestamp_eff_to
        }
    }
}