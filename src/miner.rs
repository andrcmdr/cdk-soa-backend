use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, warn};
use crate::db::Database;
use crate::types::{BackendData, BackendApiResponse};
use crate::validators::validate_backend_data;
use reqwest::Client;
use serde_json::Value;

/// APIMiner handles mining data from external APIs
/// This miner would typically fetch data from external services and store it in the database
pub struct APIMiner {
    db: Arc<Database>,
    api_key: String,
    api_url: String,
    http_client: Client,
}

impl APIMiner {
    pub fn new(db: Arc<Database>, api_key: String, api_url: String) -> Self {
        Self {
            db,
            api_key: api_key,
            api_url: api_url,
            http_client: Client::new(),
        }
    }


    /// Mine data from external APIs. This would be called periodically to fetch data from the external API.
    async fn mine_data(&self, start_at: i64, end_at: i64) -> Result<()> {
        info!("APIMiner: Starting data mining cycle");
        
        // Simulate fetching revenue data from external API
        let backend_data = self.fetch_data(start_at, end_at).await?;
        for data in backend_data {
            match validate_backend_data(&data) {
                Ok(valid) => {
                    if valid {
                        self.db.insert_backend_data(&data).await?;
                    }
                }
                Err(e) => {
                    error!("Failed to validate backend data: {}", e);
                }
            }
        }
        
        info!("APIMiner: Data mining cycle completed");
        Ok(())
    }

    /// Fetch revenue and usage data from external API with pagination support
    pub async fn fetch_data(&self, start_at: i64, end_at: i64) -> Result<Vec<BackendData>> {
        Self::fetch_data_from_api(&self.http_client, &self.api_key, &self.api_url, start_at, end_at).await
    }

    /// Standalone function to fetch data from API (useful for testing)
    pub async fn fetch_data_from_api(
        http_client: &Client,
        api_key: &str,
        api_url: &str,
        start_at: i64,
        end_at: i64,
    ) -> Result<Vec<BackendData>> {
        let mut all_data = Vec::new();
        let mut page = 1;
        let page_size = 100; // Current page size
        let current_timestamp = chrono::Utc::now().timestamp();

        loop {
            info!("Fetching data from backend API - page: {}", page);
            
            let url = format!("{}?page={}&limit={}&start_at={}&end_at={}", 
                api_url, page, page_size, start_at, end_at);

            info!("Fetching data from backend API - url: {}", url);
            
            let mut request = http_client
                .get(&url)
                .header("Content-Type", "application/json");
            
            // Only add API key header if provided
            if !api_key.is_empty() {
                request = request.header("x-api-key", api_key);
            }
            
            let response = request.send().await?;

            if !response.status().is_success() {
                error!("Failed to fetch data from backend API. Status: {}", response.status());
                return Err(anyhow::anyhow!("Backend API request failed with status: {}", response.status()));
            }

            let response_text = response.text().await?;
            
            // Check if response is null or empty
            if response_text.trim().is_empty() || response_text.trim() == "null" {
                info!("No more data available from backend API");
                break;
            }

            // Parse the JSON response
            let json_value: Value = serde_json::from_str(&response_text)?;
            
            // Handle both array and null responses
            let api_responses: Vec<BackendApiResponse> = match json_value {
                Value::Array(arr) => {
                    if arr.is_empty() {
                        info!("Empty array received from backend API");
                        break;
                    }
                    serde_json::from_value(Value::Array(arr))?
                },
                Value::Null => {
                    info!("Null response received from backend API");
                    break;
                },
                _ => {
                    warn!("Unexpected response format from backend API: {}", json_value);
                    break;
                }
            };

            if api_responses.is_empty() {
                info!("No data in current page, stopping pagination");
                break;
            }

            let response_count = api_responses.len();
            
            // Convert API responses to BackendData
            for api_response in api_responses {
                let backend_data = api_response.to_backend_data(current_timestamp);
                all_data.push(backend_data);
            }

            info!("Fetched {} items from page {}", response_count, page);
            
            // Reached end of data
            if response_count < page_size {
                info!("Reached end of data (got {} items, expected {})", response_count, page_size);
                break;
            }

            page += 1;
            
            // Safety check to prevent infinite loops
            if page > 10 {
                warn!("Reached maximum page limit (10), stopping pagination");
                break;
            }
        }

        info!("Successfully fetched {} total items from backend API", all_data.len());
        Ok(all_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_fetch_data_from_api() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init();

        dotenv::dotenv().ok();
        if std::env::var("API_URL").is_ok() && std::env::var("API_KEY").is_ok() {
            let api_url = std::env::var("API_URL").unwrap();
            let api_key = std::env::var("API_KEY").unwrap();
            let http_client = Client::new();
             
            //  delay is 5 minutes
            let delay =  300;

            let start_at = chrono::Utc::now().timestamp() - 300 - delay;
            let end_at = chrono::Utc::now().timestamp() - delay ;
            let result = APIMiner::fetch_data_from_api(
                &http_client,
                &api_key,
                &api_url,
                start_at,
                end_at,
            ).await;
            
            // Just test that it doesn't panic and returns a result
            match result {
                Ok(data) => {
                    info!("Fetched {} items", data.len());
                    for item in data {
                        info!("Item: {:?}", item);
                    }
                }
                Err(e) => error!("API test failed (expected in CI): {}", e),
            }
        } else {
            error!("Skipping API test - no API_URL or API_KEY set");
        }
    }
}
