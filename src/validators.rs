use crate::types::{BackendRevenueData, BackendUsageData};
use alloy::primitives::Address;
use std::str::FromStr;


// Placeholder for now
pub fn validate_revenue_data(revenue_data: &BackendRevenueData) -> Result<bool, String> {
    if revenue_data.revenue.is_empty() {
        return Err("Revenue is empty".to_string());
    }
    if revenue_data.timestamp_eff_from < 0 || revenue_data.timestamp_eff_to < 0 {
        return Err("Timestamp eff from or timestamp eff to is less than 0".to_string());
    }
    if revenue_data.timestamp_eff_from > revenue_data.timestamp_eff_to {
        return Err("Timestamp eff from is greater than timestamp eff to".to_string());
    }
    if revenue_data.artifact_address.is_empty() {
        return Err("Artifact address is empty".to_string());
    }
    // check address is valid
    if Address::from_str(&revenue_data.artifact_address).is_err() {
        return Err("Artifact address is not valid".to_string());
    }
    if revenue_data.revenue.parse::<i64>().unwrap() < 0 {
        return Err("Revenue is less than 0".to_string());
    }
    
    Ok(true)
}

// Placeholder for now
pub fn validate_usage_data(usage_data: &BackendUsageData) -> Result<bool, String> {
    if usage_data.usage.is_empty() {
        return Err("Usage is empty".to_string());
    }
    if usage_data.timestamp_eff_from < 0 || usage_data.timestamp_eff_to < 0 {
        return Err("Timestamp eff from or timestamp eff to is less than 0".to_string());
    }
    if usage_data.timestamp_eff_from > usage_data.timestamp_eff_to {
        return Err("Timestamp eff from is greater than timestamp eff to".to_string());
    }
    if usage_data.artifact_address.is_empty() {
        return Err("Artifact address is empty".to_string());
    }
    // check address is valid
    if Address::from_str(&usage_data.artifact_address).is_err() {
        return Err("Artifact address is not valid".to_string());
    }
    if usage_data.usage.parse::<i64>().unwrap() < 0 {
        return Err("Usage is less than 0".to_string());
    }
    Ok(true)
}