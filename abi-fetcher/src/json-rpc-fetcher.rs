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
use sha3::{Digest, Keccak256};
use hex;

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
    #[serde(default = "default_abi_fetch_attempts")]
    abi_fetch_attempts: u32,
}

#[derive(Debug, Deserialize)]
struct OutputConfig {
    contracts_file: String,
    abi_directory: String,
    events_directory: String,
    events_file: String,
    contracts_events_file: String,
}

fn default_request_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_abi_fetch_attempts() -> u32 { 5 }

// API response structures for contract list endpoints
#[derive(Debug, Deserialize)]
struct ContractListResponse {
    message: String,
    result: Vec<ContractListItem>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct ContractListItem {
    #[serde(rename = "ABI")]
    abi: Option<String>,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "CompilerVersion")]
    compiler_version: Option<String>,
    #[serde(rename = "ContractName")]
    contract_name: Option<String>,
    #[serde(rename = "OptimizationUsed")]
    optimization_used: Option<String>,
}

// API response structure for individual ABI retrieval
#[derive(Debug, Deserialize)]
struct AbiResponse {
    message: String,
    result: Option<Value>,
    status: String,
}

// ABI-specific structures for event parsing
#[derive(Debug, Deserialize)]
struct AbiItem {
    #[serde(rename = "type")]
    item_type: String,
    name: Option<String>,
    inputs: Option<Vec<AbiInput>>,
    anonymous: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AbiInput {
    name: String,
    #[serde(rename = "type")]
    input_type: String,
    indexed: Option<bool>,
    components: Option<Vec<AbiInput>>, // For tuple types
}

// Event-related output structures
#[derive(Debug, Serialize)]
struct EventsOutput {
    metadata: EventsMetadata,
    events: Vec<EventDefinition>,
}

#[derive(Debug, Serialize)]
struct EventsMetadata {
    generated_at: String,
    blockscout_server: String,
    total_events: usize,
    total_unique_signatures: usize,
    events_directory: String,
}

#[derive(Debug, Serialize)]
struct EventDefinition {
    name: String,
    signature: String,
    topic_hash: String,
    anonymous: bool,
    inputs: Vec<EventInput>,
    contract_sources: Vec<ContractSource>,
    signature_file: String,
}

#[derive(Debug, Serialize)]
struct EventInput {
    name: String,
    input_type: String,
    indexed: bool,
}

#[derive(Debug, Serialize, Clone)]
struct ContractSource {
    address: String,
    contract_name: Option<String>,
}

// Structure for contracts_events.yaml
#[derive(Debug, Serialize, Clone)]
struct ContractAddress {
    address: String,
}

// Contract events output structures
#[derive(Debug, Serialize)]
struct ContractsEventsOutput {
    contracts: Vec<ContractEvents>,
}

#[derive(Debug, Serialize)]
struct ContractEvents {
    name: Option<String>,
    address: Vec<ContractAddress>,
    events: Vec<EventSignature>,
}

#[derive(Debug, Serialize)]
struct EventSignature {
    event: String, // Extended event signature
}

// Output structures for YAML
#[derive(Debug, Serialize)]
struct ContractsOutput {
    metadata: ContractsMetadata,
    verified_contracts: Vec<ContractInfo>,
    unverified_contracts: Vec<ContractInfo>,
}

#[derive(Debug, Serialize)]
struct ContractsMetadata {
    generated_at: String,
    blockscout_server: String,
    total_verified: usize,
    total_unverified: usize,
    total_with_abi: usize,
    abi_directory: String,
}

#[derive(Debug, Serialize)]
struct ContractInfo {
    name: Option<String>,
    address: String,
    abi_file: Option<String>,
    is_verified: bool,
}

// Structure to track contract events for the contracts-events YAML
#[derive(Debug, Clone)]
struct ContractEventInfo {
    contract_name: Option<String>,
    contract_address: String,
    events: Vec<String>, // Extended event signatures
}

struct BlockscoutClient {
    client: reqwest::Client,
    base_url: String,
    max_retries: u32,
    abi_fetch_attempts: u32,
}

impl BlockscoutClient {
    fn new(server: &str, api_path: &str, timeout_seconds: u64, max_retries: u32, abi_fetch_attempts: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = format!("{}/{}", server.trim_end_matches('/'), api_path.trim_start_matches('/'));

        Self {
            client,
            base_url,
            max_retries,
            abi_fetch_attempts,
        }
    }

    async fn fetch_verified_contracts(&self) -> Result<Vec<ContractListItem>> {
        let url = format!("{}?module=contract&action=listcontracts&filter=verified", self.base_url);
        info!("Fetching verified contracts from: {}", url);

        let response = self.fetch_with_retry(&url).await
            .context("Failed to fetch verified contracts list")?;

        let contracts_response: ContractListResponse = response.json().await
            .context("Failed to parse verified contracts response")?;

        if contracts_response.status != "1" {
            return Err(anyhow::anyhow!("API returned error status: {}", contracts_response.message));
        }

        info!("Fetched {} verified contracts", contracts_response.result.len());
        Ok(contracts_response.result)
    }

    async fn fetch_unverified_contracts(&self) -> Result<Vec<ContractListItem>> {
        let url = format!("{}?module=contract&action=listcontracts&filter=unverified", self.base_url);
        info!("Fetching unverified contracts from: {}", url);

        let response = self.fetch_with_retry(&url).await
            .context("Failed to fetch unverified contracts list")?;

        let contracts_response: ContractListResponse = response.json().await
            .context("Failed to parse unverified contracts response")?;

        if contracts_response.status != "1" {
            return Err(anyhow::anyhow!("API returned error status: {}", contracts_response.message));
        }

        info!("Fetched {} unverified contracts", contracts_response.result.len());
        Ok(contracts_response.result)
    }

    async fn fetch_contract_abi(&self, address: &str) -> Result<Option<Value>> {
        let url = format!("{}?module=contract&action=getabi&address={}", self.base_url, address);

        for attempt in 1..=self.abi_fetch_attempts {
            debug!("Fetching ABI for contract {} (attempt {}/{})", address, attempt, self.abi_fetch_attempts);

            match self.fetch_with_retry(&url).await {
                Ok(response) => {
                    match response.json::<AbiResponse>().await {
                        Ok(abi_response) => {
                            if abi_response.status == "1" {
                                if let Some(result) = abi_response.result {
                                    // Check if result is a valid ABI JSON array or null
                                    match &result {
                                        Value::Array(_) => {
                                            debug!("Successfully fetched ABI for contract {} on attempt {}", address, attempt);
                                            return Ok(Some(result));
                                        }
                                        Value::Null => {
                                            debug!("ABI is null for contract {}", address);
                                            return Ok(None);
                                        }
                                        Value::String(s) if s == "Contract source code not verified" => {
                                            debug!("Contract {} source code not verified", address);
                                            return Ok(None);
                                        }
                                        _ => {
                                            if attempt < self.abi_fetch_attempts {
                                                debug!("Invalid ABI format for contract {}, retrying... (attempt {}/{})", 
                                                      address, attempt, self.abi_fetch_attempts);
                                                tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
                                                continue;
                                            } else {
                                                warn!("Invalid ABI format for contract {} after {} attempts", address, self.abi_fetch_attempts);
                                                return Ok(None);
                                            }
                                        }
                                    }
                                } else {
                                    if attempt < self.abi_fetch_attempts {
                                        debug!("No result in ABI response for contract {}, retrying... (attempt {}/{})", 
                                              address, attempt, self.abi_fetch_attempts);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
                                        continue;
                                    } else {
                                        debug!("No ABI available for contract {} after {} attempts", address, self.abi_fetch_attempts);
                                        return Ok(None);
                                    }
                                }
                            } else {
                                if attempt < self.abi_fetch_attempts {
                                    debug!("API error for contract {}: {}, retrying... (attempt {}/{})", 
                                          address, abi_response.message, attempt, self.abi_fetch_attempts);
                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
                                    continue;
                                } else {
                                    warn!("API error for contract {} after {} attempts: {}", address, self.abi_fetch_attempts, abi_response.message);
                                    return Ok(None);
                                }
                            }
                        }
                        Err(e) => {
                            if attempt < self.abi_fetch_attempts {
                                debug!("Failed to parse ABI response for contract {}: {:?}, retrying... (attempt {}/{})", 
                                      address, e, attempt, self.abi_fetch_attempts);
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
                                continue;
                            } else {
                                warn!("Failed to parse ABI response for contract {} after {} attempts: {:?}", 
                                     address, self.abi_fetch_attempts, e);
                                return Ok(None);
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt < self.abi_fetch_attempts {
                        debug!("HTTP request failed for contract {}: {:?}, retrying... (attempt {}/{})", 
                              address, e, attempt, self.abi_fetch_attempts);
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
                        continue;
                    } else {
                        warn!("HTTP request failed for contract {} after {} attempts: {:?}", 
                             address, self.abi_fetch_attempts, e);
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None)
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

// Event processing functions
fn parse_abi_events(
    abi: &Value,
    contract_address: &str,
    contract_name: Option<&str>,
    events_map: &mut HashMap<String, EventDefinition>,
    contract_events: &mut Vec<ContractEventInfo>
) -> Result<()> {
    let abi_array: Vec<AbiItem> = serde_json::from_value(abi.clone())
        .context("Failed to parse ABI")?;

    let mut current_contract_events = Vec::new();

    for item in abi_array {
        if item.item_type == "event" {
            if let Some(event_name) = item.name {
                let anonymous = item.anonymous.unwrap_or(false);
                let inputs = item.inputs.unwrap_or_default();

                let signature = generate_event_signature(&event_name, &inputs, anonymous);
                let extended_signature = generate_extended_event_signature(&event_name, &inputs, anonymous);
                let topic_hash = if !anonymous {
                    generate_topic_hash(&signature)
                } else {
                    "N/A (anonymous)".to_string()
                };

                let event_inputs: Vec<EventInput> = inputs
                    .into_iter()
                    .map(|input| EventInput {
                        name: input.name.clone(),
                        input_type: format_type_string(&input),
                        indexed: input.indexed.unwrap_or(false),
                    })
                    .collect();

                // Add to contract-specific events list
                current_contract_events.push(extended_signature.clone());

                // Use signature as key to group events from different contracts
                if let Some(existing_event) = events_map.get_mut(&signature) {
                    // Add this contract to the sources if not already present
                    let contract_source = ContractSource {
                        address: contract_address.to_string(),
                        contract_name: contract_name.map(|s| s.to_string()),
                    };

                    if !existing_event.contract_sources.iter().any(|cs| cs.address == contract_address) {
                        existing_event.contract_sources.push(contract_source);
                    }
                } else {
                    events_map.insert(signature.clone(), EventDefinition {
                        name: event_name,
                        signature: signature.clone(),
                        topic_hash,
                        anonymous,
                        inputs: event_inputs,
                        contract_sources: vec![ContractSource {
                            address: contract_address.to_string(),
                            contract_name: contract_name.map(|s| s.to_string()),
                        }],
                        signature_file: format!("{}.txt", sanitize_filename(&signature)),
                    });
                }
            }
        }
    }

    // Add contract events info if there are any events
    if !current_contract_events.is_empty() {
        contract_events.push(ContractEventInfo {
            contract_name: contract_name.map(|s| s.to_string()),
            contract_address: contract_address.to_string(),
            events: current_contract_events,
        });
    }

    Ok(())
}

fn generate_event_signature(name: &str, inputs: &[AbiInput], anonymous: bool) -> String {
    let param_types: Vec<String> = inputs
        .iter()
        .map(|input| format_type_string(input))
        .collect();

    let signature = format!("{}({})", name, param_types.join(","));

    if anonymous {
        format!("{} [anonymous]", signature)
    } else {
        signature
    }
}

fn generate_extended_event_signature(name: &str, inputs: &[AbiInput], anonymous: bool) -> String {
    let param_parts: Vec<String> = inputs
        .iter()
        .map(|input| {
            let indexed_str = if input.indexed.unwrap_or(false) {
                " indexed"
            } else {
                ""
            };
            format!("{}{} {}", format_type_string(input), indexed_str, input.name)
        })
        .collect();

    let signature = format!("{}({})", name, param_parts.join(", "));

    if anonymous {
        format!("{} [anonymous]", signature)
    } else {
        signature
    }
}

fn format_type_string(input: &AbiInput) -> String {
    let base_type = if input.input_type == "tuple" {
        if let Some(components) = &input.components {
            let component_types: Vec<String> = components
                .iter()
                .map(|comp| format_type_string(comp))
                .collect();
            format!("({})", component_types.join(","))
        } else {
            input.input_type.clone()
        }
    } else {
        input.input_type.clone()
    };

    // Handle arrays
    if base_type.ends_with("[]") || base_type.contains("[") {
        base_type
    } else {
        base_type
    }
}

fn generate_topic_hash(signature: &str) -> String {
    // Remove [anonymous] suffix if present for hash calculation
    let clean_signature = signature.replace(" [anonymous]", "");
    let mut hasher = Keccak256::new();
    hasher.update(clean_signature.as_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

fn save_event_signature_to_file(
    event: &EventDefinition,
    events_dir: &Path,
) -> Result<()> {
    let file_path = events_dir.join(&event.signature_file);

    let mut content = String::new();
    content.push_str(&format!("Event Name: {}\n", event.name));
    content.push_str(&format!("Signature: {}\n", event.signature));
    content.push_str(&format!("Topic Hash: {}\n", event.topic_hash));
    content.push_str(&format!("Anonymous: {}\n", event.anonymous));
    content.push_str("\nInputs:\n");

    for (i, input) in event.inputs.iter().enumerate() {
        content.push_str(&format!(
            "  {}: {} {} {}\n",
            i,
            input.name,
            input.input_type,
            if input.indexed { "(indexed)" } else { "(not indexed)" }
        ));
    }

    content.push_str("\nContract Sources:\n");
    for source in &event.contract_sources {
        let contract_name_str = source.contract_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        content.push_str(&format!("  - {} (contract_name: {})\n",
                                 source.address, contract_name_str));
    }

    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write event signature file: {:?}", file_path))?;

    Ok(())
}

fn build_contracts_events_output(contract_events_list: Vec<ContractEventInfo>) -> ContractsEventsOutput {
    // Group contracts by name, combining addresses for contracts with the same name
    let mut contracts_map: HashMap<String, ContractEvents> = HashMap::new();

    for contract_event_info in contract_events_list {
        let contract_key = contract_event_info.contract_name
            .clone()
            .unwrap_or_else(|| format!("unnamed_{}", &contract_event_info.contract_address[2..8]));

        let contract_address = ContractAddress {
            address: contract_event_info.contract_address.clone(),
        };

        if let Some(existing_contract) = contracts_map.get_mut(&contract_key) {
            // Add address if not already present
            if !existing_contract.address.iter().any(|ca| ca.address == contract_event_info.contract_address) {
                existing_contract.address.push(contract_address);
            }

            // Add events, avoiding duplicates
            for event in contract_event_info.events {
                if !existing_contract.events.iter().any(|e| e.event == event) {
                    existing_contract.events.push(EventSignature { event });
                }
            }
        } else {
            let events: Vec<EventSignature> = contract_event_info.events
                .into_iter()
                .map(|event| EventSignature { event })
                .collect();

            contracts_map.insert(contract_key.clone(), ContractEvents {
                name: contract_event_info.contract_name,
                address: vec![contract_address],
                events,
            });
        }
    }

    // Convert to sorted vector
    let mut contracts: Vec<ContractEvents> = contracts_map.into_values().collect();
    contracts.sort_by(|a, b| {
        let a_name = a.name.as_deref().unwrap_or("unnamed");
        let b_name = b.name.as_deref().unwrap_or("unnamed");
        a_name.cmp(b_name)
    });

    // Sort events within each contract and addresses
    for contract in &mut contracts {
        contract.events.sort_by(|a, b| a.event.cmp(&b.event));
        contract.address.sort_by(|a, b| a.address.cmp(&b.address));
    }

    ContractsEventsOutput { contracts }
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

fn save_events_to_yaml<P: AsRef<Path>>(
    events_output: &EventsOutput,
    output_path: P,
) -> Result<()> {
    let yaml_content = serde_yaml::to_string(events_output)
        .context("Failed to serialize events to YAML")?;

    fs::write(&output_path, yaml_content)
        .with_context(|| format!("Failed to write events to file: {:?}", output_path.as_ref()))?;

    info!("Events saved to: {:?}", output_path.as_ref());
    Ok(())
}

fn save_contracts_events_to_yaml<P: AsRef<Path>>(
    contracts_events_output: &ContractsEventsOutput,
    output_path: P,
) -> Result<()> {
    let yaml_content = serde_yaml::to_string(contracts_events_output)
        .context("Failed to serialize contracts events to YAML")?;

    fs::write(&output_path, yaml_content)
        .with_context(|| format!("Failed to write contracts events to file: {:?}", output_path.as_ref()))?;

    info!("Contracts events saved to: {:?}", output_path.as_ref());
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

    // Ensure directories exist
    let abi_dir = Path::new(&config.output.abi_directory);
    let events_dir = Path::new(&config.output.events_directory);
    ensure_directory_exists(abi_dir)
        .context("Failed to create ABI directory")?;
    ensure_directory_exists(events_dir)
        .context("Failed to create events directory")?;

    // Create Blockscout client
    let client = BlockscoutClient::new(
        &config.blockscout.server,
        &config.blockscout.api_path,
        config.blockscout.request_timeout_seconds,
        config.blockscout.max_retries,
        config.blockscout.abi_fetch_attempts,
    );

    // Fetch verified contracts
    let verified_contract_items = client.fetch_verified_contracts().await
        .context("Failed to fetch verified contracts")?;

    // Fetch unverified contracts
    let unverified_contract_items = client.fetch_unverified_contracts().await
        .context("Failed to fetch unverified contracts")?;

    info!("Processing {} verified and {} unverified contracts...", 
          verified_contract_items.len(), unverified_contract_items.len());

    let mut verified_contracts = Vec::new();
    let mut unverified_contracts = Vec::new();
    let mut events_map: HashMap<String, EventDefinition> = HashMap::new();
    let mut contract_events_list: Vec<ContractEventInfo> = Vec::new();
    let mut total_abi_files = 0;

    // Process verified contracts
    for contract_item in verified_contract_items {
        let mut abi_file = None;
        let mut abi_value: Option<Value> = None;

        // Parse ABI from the contract list response
        if let Some(abi_str) = &contract_item.abi {
            if abi_str != "Contract source code not verified" {
                match serde_json::from_str::<Value>(abi_str) {
                    Ok(abi) => {
                        abi_value = Some(abi.clone());
                        abi_file = Some(save_abi_to_file(
                            &abi,
                            contract_item.contract_name.as_deref(),
                            &contract_item.address,
                            abi_dir,
                        )?);
                        total_abi_files += 1;
                    }
                    Err(e) => {
                        warn!("Failed to parse ABI for verified contract {}: {:?}", contract_item.address, e);
                    }
                }
            }
        }

        // Parse events if ABI is available
        if let Some(abi) = &abi_value {
            if let Err(e) = parse_abi_events(
                abi,
                &contract_item.address,
                contract_item.contract_name.as_deref(),
                &mut events_map,
                &mut contract_events_list
            ) {
                warn!("Failed to parse events from verified contract {}: {:?}", contract_item.address, e);
            }
        }

        verified_contracts.push(ContractInfo {
            name: contract_item.contract_name,
            address: contract_item.address,
            abi_file,
            is_verified: true,
        });
    }

    // Process unverified contracts
    for contract_item in unverified_contract_items {
        let mut abi_file = None;

        // Try to fetch ABI for unverified contract
        match client.fetch_contract_abi(&contract_item.address).await {
            Ok(Some(abi)) => {
                abi_file = Some(save_abi_to_file(
                    &abi,
                    contract_item.contract_name.as_deref(),
                    &contract_item.address,
                    abi_dir,
                )?);
                total_abi_files += 1;

                // Parse events from this ABI
                if let Err(e) = parse_abi_events(
                    &abi,
                    &contract_item.address,
                    contract_item.contract_name.as_deref(),
                    &mut events_map,
                    &mut contract_events_list
                ) {
                    warn!("Failed to parse events from unverified contract {}: {:?}", contract_item.address, e);
                }
            }
            Ok(None) => {
                debug!("No ABI available for unverified contract {}", contract_item.address);
            }
            Err(e) => {
                warn!("Failed to fetch ABI for unverified contract {}: {:?}", contract_item.address, e);
            }
        }

        unverified_contracts.push(ContractInfo {
            name: contract_item.contract_name,
            address: contract_item.address,
            abi_file,
            is_verified: false,
        });
    }

    // Sort contracts by address
    verified_contracts.sort_by(|a, b| a.address.cmp(&b.address));
    unverified_contracts.sort_by(|a, b| a.address.cmp(&b.address));

    // Save event signature files and prepare events output
    let mut events_list: Vec<EventDefinition> = events_map.into_values().collect();
    events_list.sort_by(|a, b| a.name.cmp(&b.name));

    for event in &events_list {
        if let Err(e) = save_event_signature_to_file(event, events_dir) {
            warn!("Failed to save event signature file for {}: {:?}", event.name, e);
        }
    }

    let unique_signatures = events_list.len();

    // Create events output structure
    let events_output = EventsOutput {
        metadata: EventsMetadata {
            generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            blockscout_server: config.blockscout.server.clone(),
            total_events: events_list.len(),
            total_unique_signatures: unique_signatures,
            events_directory: config.output.events_directory.clone(),
        },
        events: events_list,
    };

    // Create contracts events output structure
    let contracts_events_output = build_contracts_events_output(contract_events_list);

    // Create contracts output structure
    let contracts_output = ContractsOutput {
        metadata: ContractsMetadata {
            generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            blockscout_server: config.blockscout.server.clone(),
            total_verified: verified_contracts.len(),
            total_unverified: unverified_contracts.len(),
            total_with_abi: total_abi_files,
            abi_directory: config.output.abi_directory.clone(),
        },
        verified_contracts,
        unverified_contracts,
    };

    // Save to YAML files
    save_contracts_to_yaml(&contracts_output, &config.output.contracts_file)
        .context("Failed to save contracts to YAML file")?;

    save_events_to_yaml(&events_output, &config.output.events_file)
        .context("Failed to save events to YAML file")?;

    save_contracts_events_to_yaml(&contracts_events_output, &config.output.contracts_events_file)
        .context("Failed to save contracts events to YAML file")?;

    info!(
        "Successfully processed {} verified and {} unverified contracts with {} ABI files created",
        contracts_output.metadata.total_verified,
        contracts_output.metadata.total_unverified,
        contracts_output.metadata.total_with_abi
    );

    info!(
        "Extracted {} unique event signatures from all contracts",
        events_output.metadata.total_unique_signatures
    );

    info!(
        "Generated contracts-events YAML with {} contracts",
        contracts_events_output.contracts.len()
    );

    Ok(())
}
