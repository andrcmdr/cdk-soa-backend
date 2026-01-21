use serde::{Deserialize, Serialize};

// These structs would be used to store received data from the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendData {
    pub artifact_address: String,
    pub revenue: String,
    pub usage: String,
    pub timestamp: i64,
}

// Structs for backend API response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendApiResponse {
    pub art_address: String,
    pub usage: ValueData,
    pub revenue: ValueData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueData {
    pub value: i64,
}

impl BackendApiResponse {
    pub fn to_backend_data(&self, timestamp: i64) -> BackendData {
        BackendData {
            artifact_address: self.art_address.clone(),
            revenue: self.revenue.value.to_string(),
            usage: self.usage.value.to_string(),
            timestamp,
        }
    }
}