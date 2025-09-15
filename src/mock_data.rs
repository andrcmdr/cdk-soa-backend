use crate::types::{BackendRevenueData, BackendUsageData};
use serde_json;
use std::fs;
use anyhow::Result;


/// Load mock revenue reports from JSON file
pub fn load_mock_revenue_reports() -> Result<Vec<BackendRevenueData>> {
    let data = fs::read_to_string("tests/mock_data/revenue_reports.json")?;
    let reports: Vec<BackendRevenueData> = serde_json::from_str(&data)?;
    Ok(reports)
}

/// Load mock usage reports from JSON file
pub fn load_mock_usage_reports() -> Result<Vec<BackendUsageData>> {
    let data = fs::read_to_string("tests/mock_data/usage_reports.json")?;
    let reports: Vec<BackendUsageData> = serde_json::from_str(&data)?;
    Ok(reports)
}

/// Load invalid revenue data for testing validation
pub fn load_invalid_revenue_data() -> Result<Vec<serde_json::Value>> {
    let data = fs::read_to_string("tests/mock_data/invalid_revenue_data.json")?;
    let invalid_data: Vec<serde_json::Value> = serde_json::from_str(&data)?;
    Ok(invalid_data)
}

