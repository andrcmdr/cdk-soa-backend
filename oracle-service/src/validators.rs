use crate::types::{BackendData};
use alloy::primitives::Address;
use std::str::FromStr;


pub fn validate_backend_data(revenue_data: &BackendData) -> Result<bool, String> {

    // Non-empty checks

    if revenue_data.revenue.is_empty() {
        return Err("Revenue is empty".to_string());
    }
    if revenue_data.usage.is_empty() {
        return Err("Usage is empty".to_string());
    }
    if revenue_data.artifact_address.is_empty() {
        return Err("Artifact address is empty".to_string());
    }

    // Validity checks
    // check address is valid
    if Address::from_str(&revenue_data.artifact_address).is_err() {
        return Err("Artifact address is not valid".to_string());
    }
    if revenue_data.timestamp < 0 {
        return Err("Timestamp is less than 0".to_string());
    }
    if revenue_data.revenue.parse::<i64>().unwrap() < 0 {
        return Err("Revenue is less than 0".to_string());
    }
    if revenue_data.usage.parse::<i64>().unwrap() < 0 {
        return Err("Usage is less than 0".to_string());
    }
    
    Ok(true)
}
