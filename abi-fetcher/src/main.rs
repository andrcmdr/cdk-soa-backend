use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tokio;
use tracing_subscriber::{EnvFilter, fmt};
use tracing::{ info, debug, error, trace, warn };

// Configuration structure for the app
#[derive(Debug, Deserialize)]
struct AppConfig {
    blockscout: BlockscoutConfig,
    output: OutputConfig,
}

#[derive(Debug, Deserialize)]
struct BlockscoutConfig {
    server: String,
    api_path: String,
    #[serde(default = "default_request_timeout")]
    request_timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    max_retries: u32,
}

#[derive(Debug, Deserialize)]
struct OutputConfig {
    contracts_file: String,
    abi_directory: String,
}

fn default_request_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }

// API response structures for smart contracts list
#[derive(Debug, Deserialize)]
struct SmartContractsResponse {
    items: Vec<SmartContractItem>,
    next_page_params: Option<NextPageParams>,
}

#[derive(Debug, Deserialize)]
struct SmartContractItem {
    address: ContractAddress,
}

#[derive(Debug, Deserialize)]
struct ContractAddress {
    hash: String,
    implementations: Option<Vec<Implementation>>,
    is_contract: bool,
    is_verified: bool,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Implementation {
    address: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextPageParams {
    smart_contract_id: u64,
}

// API response structure for individual contract details
#[derive(Debug, Deserialize)]
struct ContractDetailsResponse {
    is_verified: bool,
    is_fully_verified: Option<bool>,
    implementations: Option<Vec<Implementation>>,
    name: Option<String>,
    abi: Option<Value>,
}

// Output structures for YAML
#[derive(Debug, Serialize)]
struct ContractsOutput {
    metadata: ContractsMetadata,
    verified_contracts: Vec<ContractInfo>,
}

#[derive(Debug, Serialize)]
struct ContractsMetadata {
    generated_at: String,
    blockscout_server: String,
    total_contracts: usize,
    total_with_abi: usize,
    abi_directory: String,
}

#[derive(Debug, Serialize)]
struct ContractInfo {
    name: Option<String>,
    address: String,
    abi_file: Option<String>,
    implementations: Option<Vec<ImplementationInfo>>,
}

#[derive(Debug, Serialize)]
struct ImplementationInfo {
    name: Option<String>,
    address: String,
    abi_file: Option<String>,
}

struct BlockscoutClient {
    client: reqwest::Client,
    base_url: String,
    max_retries: u32,
}

impl BlockscoutClient {
    fn new(server: &str, api_path: &str, timeout_seconds: u64, max_retries: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = format!("{}/{}", server.trim_end_matches('/'), api_path.trim_start_matches('/'));

        Self {
            client,
            base_url,
            max_retries,
        }
    }

    async fn fetch_all_verified_contracts(&self) -> Result<Vec<SmartContractItem>> {
        let mut all_contracts = Vec::new();
        let mut next_page_id: Option<u64> = None;

        loop {
            let url = if let Some(page_id) = next_page_id {
                format!("{}/smart-contracts?smart_contract_id={}", self.base_url, page_id)
            } else {
                format!("{}/smart-contracts", self.base_url)
            };

            info!("Fetching contracts from: {}", url);

            let response = self.fetch_with_retry(&url).await
                .context("Failed to fetch smart contracts list")?;

            let contracts_response: SmartContractsResponse = response.json().await
                .context("Failed to parse smart contracts response")?;

            let items_count = contracts_response.items.len();
            info!("Fetched {} contracts in this page", items_count);

            all_contracts.extend(contracts_response.items);

            // Check if there's a next page
            if let Some(next_params) = contracts_response.next_page_params {
                next_page_id = Some(next_params.smart_contract_id);
                debug!("Next page ID: {}", next_params.smart_contract_id);
            } else {
                info!("No more pages, pagination complete");
                break;
            }
        }

        info!("Total contracts fetched: {}", all_contracts.len());
        Ok(all_contracts)
    }

    async fn fetch_contract_details(&self, address: &str) -> Result<ContractDetailsResponse> {
        let url = format!("{}/smart-contracts/{}", self.base_url, address);
        
        debug!("Fetching contract details for: {}", address);

        let response = self.fetch_with_retry(&url).await
            .with_context(|| format!("Failed to fetch contract details for {}", address))?;

        let contract_details: ContractDetailsResponse = response.json().await
            .with_context(|| format!("Failed to parse contract details for {}", address))?;

        Ok(contract_details)
    }

    async fn fetch_with_retry(&self, url: &str) -> Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match self.client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else {
                        let status = response.status();
                        let error = anyhow::anyhow!("HTTP error: {}", status);
                        last_error = Some(error);
                        
                        if attempt < self.max_retries {
                            warn!("Request failed with status {}, retrying... (attempt {}/{})", 
                                  status, attempt + 1, self.max_retries);
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e.into());
                    if attempt < self.max_retries {
                        warn!("Request failed: {:?}, retrying... (attempt {}/{})", 
                              last_error, attempt + 1, self.max_retries);
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

fn save_abi_to_file(
    abi: &Value,
    contract_name: Option<&str>,
    contract_address: &str,
    abi_dir: &Path,
) -> Result<String> {
    let filename = if let Some(name) = contract_name {
        format!("{}.json", sanitize_filename(name))
    } else {
        format!("{}.json", contract_address)
    };

    let file_path = abi_dir.join(&filename);

    let final_path = if file_path.exists() {
        let stem = file_path.file_stem().unwrap().to_str().unwrap();
        let unique_filename = format!("{}_{}.json", stem, &contract_address);
        abi_dir.join(unique_filename)
    } else {
        let stem = file_path.file_stem().unwrap().to_str().unwrap();
        let unique_filename = format!("{}_{}.json", stem, &contract_address);
        abi_dir.join(unique_filename)
    };

    let abi_json = serde_json::to_string_pretty(abi)
        .context("Failed to serialize ABI to JSON")?;

    fs::write(&final_path, abi_json)
        .with_context(|| format!("Failed to write ABI file: {:?}", final_path))?;

    Ok(final_path.file_name().unwrap().to_str().unwrap().to_string())
}

async fn process_contract_with_implementations(
    client: &BlockscoutClient,
    contract_item: &SmartContractItem,
    abi_dir: &Path,
    processed_addresses: &mut HashSet<String>,
) -> Result<ContractInfo> {
    let address = &contract_item.address.hash;
    
    // Skip if already processed
    if processed_addresses.contains(address) {
        debug!("Skipping already processed contract: {}", address);
        return Ok(ContractInfo {
            name: contract_item.address.name.clone(),
            address: address.clone(),
            abi_file: None,
            implementations: None,
        });
    }

    processed_addresses.insert(address.clone());

    // Fetch contract details
    let contract_details = client.fetch_contract_details(address).await
        .with_context(|| format!("Failed to get details for contract {}", address))?;

    // Save ABI if available
    let abi_file = if let Some(abi) = &contract_details.abi {
        Some(save_abi_to_file(
            abi,
            contract_details.name.as_deref(),
            address,
            abi_dir,
        )?)
    } else {
        None
    };

    // Process implementations if any
    let implementations = if let Some(impls) = &contract_details.implementations {
        let mut impl_infos = Vec::new();
        
        for implementation in impls {
            let impl_address = &implementation.address;
            
            if !processed_addresses.contains(impl_address) {
                processed_addresses.insert(impl_address.clone());
                
                let impl_details = client.fetch_contract_details(impl_address).await
                    .with_context(|| format!("Failed to get details for implementation {}", impl_address))?;

                let impl_abi_file = if let Some(abi) = &impl_details.abi {
                    Some(save_abi_to_file(
                        abi,
                        impl_details.name.as_deref().or(implementation.name.as_deref()),
                        impl_address,
                        abi_dir,
                    )?)
                } else {
                    None
                };

                impl_infos.push(ImplementationInfo {
                    name: impl_details.name.or(implementation.name.clone()),
                    address: impl_address.clone(),
                    abi_file: impl_abi_file,
                });
            }
        }
        
        if impl_infos.is_empty() { None } else { Some(impl_infos) }
    } else {
        None
    };

    Ok(ContractInfo {
        name: contract_details.name.or(contract_item.address.name.clone()),
        address: address.clone(),
        abi_file,
        implementations,
    })
}

fn ensure_directory_exists<P: AsRef<Path>>(dir_path: P) -> Result<()> {
    let path = dir_path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {:?}", path))?;
        info!("Created directory: {:?}", path);
    }
    Ok(())
}

fn load_config<P: AsRef<Path>>(config_path: P) -> Result<AppConfig> {
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path.as_ref()))?;

    let config: AppConfig = serde_yaml::from_str(&config_content)
        .context("Failed to parse YAML configuration")?;

    Ok(config)
}

fn save_contracts_to_yaml<P: AsRef<Path>>(
    contracts_output: &ContractsOutput,
    output_path: P,
) -> Result<()> {
    let yaml_content = serde_yaml::to_string(contracts_output)
        .context("Failed to serialize contracts to YAML")?;

    fs::write(&output_path, yaml_content)
        .with_context(|| format!("Failed to write contracts to file: {:?}", output_path.as_ref()))?;

    info!("Contracts saved to: {:?}", output_path.as_ref());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize log tracing
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    // Load configuration
    let cfg_path = std::env::args().nth(1).unwrap_or_else(|| "./config.yaml".to_string());
    let config = load_config(&cfg_path)
        .context("Failed to load application configuration")?;

    info!("Loaded configuration from config.yaml");
    info!("Blockscout server: {}", config.blockscout.server);

    // Ensure ABI directory exists
    let abi_dir = Path::new(&config.output.abi_directory);
    ensure_directory_exists(abi_dir)
        .context("Failed to create ABI directory")?;

    // Create Blockscout client
    let client = BlockscoutClient::new(
        &config.blockscout.server,
        &config.blockscout.api_path,
        config.blockscout.request_timeout_seconds,
        config.blockscout.max_retries,
    );

    // Fetch all verified contracts with pagination
    let contract_items = client.fetch_all_verified_contracts().await
        .context("Failed to fetch verified contracts")?;

    info!("Processing {} contracts and their implementations...", contract_items.len());

    // Process each contract and its implementations
    let mut processed_addresses = HashSet::new();
    let mut contract_infos = Vec::new();

    for contract_item in contract_items {
        match process_contract_with_implementations(
            &client,
            &contract_item,
            abi_dir,
            &mut processed_addresses,
        ).await {
            Ok(contract_info) => {
                contract_infos.push(contract_info);
            }
            Err(e) => {
                error!("Failed to process contract {}: {:?}", contract_item.address.hash, e);
                // Continue with other contracts
            }
        }
    }

    // Count contracts with ABI files
    let contracts_with_abi = contract_infos
        .iter()
        .filter(|c| c.abi_file.is_some())
        .count();

    let implementations_with_abi = contract_infos
        .iter()
        .filter_map(|c| c.implementations.as_ref())
        .flatten()
        .filter(|impl_info| impl_info.abi_file.is_some())
        .count();

    // Create output structure
    let contracts_output = ContractsOutput {
        metadata: ContractsMetadata {
            generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            blockscout_server: config.blockscout.server.clone(),
            total_contracts: contract_infos.len(),
            total_with_abi: contracts_with_abi + implementations_with_abi,
            abi_directory: config.output.abi_directory.clone(),
        },
        verified_contracts: contract_infos,
    };

    // Save to YAML file
    save_contracts_to_yaml(&contracts_output, &config.output.contracts_file)
        .context("Failed to save contracts to YAML file")?;

    info!(
        "Successfully processed {} contracts with {} ABI files created",
        contracts_output.metadata.total_contracts,
        contracts_output.metadata.total_with_abi
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml_config = r#"
blockscout:
  server: "https://blockscout.example.com"
  api_path: "/api/v2"
  request_timeout_seconds: 60
  max_retries: 5

output:
  contracts_file: "contracts.yaml"
  abi_directory: "./abi"
        "#;

        let config: AppConfig = serde_yaml::from_str(yaml_config).unwrap();
        assert_eq!(config.blockscout.server, "https://blockscout.example.com");
        assert_eq!(config.blockscout.api_path, "/api/v2");
        assert_eq!(config.blockscout.max_retries, 5);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("MyContract"), "MyContract");
        assert_eq!(sanitize_filename("Contract/Name"), "Contract_Name");
        assert_eq!(sanitize_filename("Contract:Name"), "Contract_Name");
        assert_eq!(sanitize_filename("Contract<>Name"), "Contract__Name");
    }
}
