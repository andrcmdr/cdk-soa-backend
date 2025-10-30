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
    #[serde(default)]
    max_implementations_per_contract: Option<usize>,
    #[serde(default)]
    max_implementation_nesting_depth: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
struct OutputConfig {
    contracts_file: String,
    abi_directory: String,
    events_directory: String,
    events_file: String,
    contracts_events_file: String,
}

fn default_request_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }

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

// Structure for events - includes address, verified_at, and contract name
#[derive(Debug, Serialize, Clone)]
struct ContractSource {
    address: String,
    verified_at: Option<String>,
    contract_name: Option<String>,
}

// Structure for contracts_events.yaml - simpler, just address and verified_at
#[derive(Debug, Serialize, Clone)]
struct ContractAddress {
    address: String,
    verified_at: Option<String>,
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

// API response structures for smart contracts list
#[derive(Debug, Deserialize)]
struct SmartContractsResponse {
    items: Vec<SmartContractItem>,
    next_page_params: Option<NextPageParams>,
}

#[derive(Debug, Deserialize)]
struct SmartContractItem {
    address: ContractAddressResponse,
    verified_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContractAddressResponse {
    hash: String,
    implementations: Option<Vec<Implementation>>,
    is_contract: Option<bool>,
    is_verified: Option<bool>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Implementation {
    address: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextPageParams {
    items_count: u64,
    hash: String,
    #[serde(default)]
    coin_balance: Option<String>,
    #[serde(default)]
    transaction_count: Option<u64>,
    #[serde(default)]
    transactions_count: Option<u64>,
}

// API response structure for individual contract details
#[derive(Debug, Deserialize)]
struct ContractDetailsResponse {
    is_verified: Option<bool>,
    is_fully_verified: Option<bool>,
    implementations: Option<Vec<Implementation>>,
    name: Option<String>,
    abi: Option<Value>,
    verified_at: Option<String>,
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
    total_verified_with_abi: usize,
    total_unverified_with_abi: usize,
    total_verified_implementations_with_abi: usize,
    total_unverified_implementations_with_abi: usize,
    abi_directory: String,
}

#[derive(Debug, Serialize)]
struct ContractInfo {
    name: Option<String>,
    address: String,
    abi_file: Option<String>,
    is_verified: bool,
    is_fully_verified: Option<bool>,
    verified_at: Option<String>,
    implementations: Option<Vec<ImplementationInfo>>,
}

#[derive(Debug, Serialize)]
struct ImplementationInfo {
    name: Option<String>,
    address: String,
    abi_file: Option<String>,
    is_verified: bool,
    is_fully_verified: Option<bool>,
    verified_at: Option<String>,
    implementations: Option<Vec<ImplementationInfo>>,
}

// Structure to track contract events for the contracts-events YAML
#[derive(Debug, Clone)]
struct ContractEventInfo {
    contract_name: Option<String>,
    contract_address: String,
    verified_at: Option<String>,
    events: Vec<String>, // Extended event signatures
}

// Counter for ABI files
#[derive(Debug, Default)]
struct AbiFileCounters {
    verified_contracts: usize,
    unverified_contracts: usize,
    verified_implementations: usize,
    unverified_implementations: usize,
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
        let mut next_page_params: Option<NextPageParams> = None;
        let mut initial_page_size: Option<u64> = None;
        let mut page_multiplier: u64 = 1;

        loop {
            let url = if let Some(ref params) = next_page_params {
                // Use initial page size from first response multiplied by page number
                let page_size = initial_page_size.unwrap_or(params.items_count);
                page_multiplier += 1;
                let items_count = page_size * page_multiplier;

                format!("{}/smart-contracts?items_count={}&hash={}",
                    self.base_url, items_count, params.hash)
            } else {
                // First request without pagination parameters
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
                // Save initial page size from the first response
                if initial_page_size.is_none() {
                    initial_page_size = Some(next_params.items_count);
                    info!("Initial page size: {}", next_params.items_count);
                }

                debug!("Next page params - items_count: {}, hash: {}, page_multiplier: {}",
                    next_params.items_count, next_params.hash, page_multiplier + 1);
                next_page_params = Some(next_params);
            } else {
                info!("No more pages (next_page_params is null), pagination complete");
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

// Helper function to parse RFC3339 timestamp and convert to Unix timestamp for comparison
fn parse_verified_at_timestamp(verified_at: &Option<String>) -> Option<i64> {
    verified_at.as_ref().and_then(|timestamp_str| {
        chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .map(|dt| dt.timestamp())
            .ok()
    })
}

// Helper function to sort contract sources by verified_at in descending order (most recent first)
fn sort_contract_sources_by_verified_at_desc(contract_sources: &mut Vec<ContractSource>) {
    contract_sources.sort_by(|a, b| {
        match (parse_verified_at_timestamp(&a.verified_at), parse_verified_at_timestamp(&b.verified_at)) {
            (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts), // Descending order (b > a)
            (Some(_), None) => std::cmp::Ordering::Less, // Verified contracts first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.address.cmp(&b.address), // Fallback to address comparison
        }
    });
}

// Helper function to sort contract addresses by verified_at in descending order (most recent first)
fn sort_contract_addresses_by_verified_at_desc(contract_addresses: &mut Vec<ContractAddress>) {
    contract_addresses.sort_by(|a, b| {
        match (parse_verified_at_timestamp(&a.verified_at), parse_verified_at_timestamp(&b.verified_at)) {
            (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts), // Descending order (b > a)
            (Some(_), None) => std::cmp::Ordering::Less, // Verified contracts first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.address.cmp(&b.address), // Fallback to address comparison
        }
    });
}

// Event processing functions
fn parse_abi_events(
    abi: &Value,
    contract_address: &str,
    contract_name: Option<&str>,
    verified_at: &Option<String>,
    events_map: &mut HashMap<String, EventDefinition>,
    contract_events: &mut Vec<ContractEventInfo>,
    events_dir: &str,
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

                let signature_filename = format!("{}.txt", sanitize_filename(&signature));
                let signature_file_path = format!("{}/{}", events_dir, signature_filename);

                // Use signature as key to group events from different contracts
                if let Some(existing_event) = events_map.get_mut(&signature) {
                    // Add this contract to the sources if not already present
                    let contract_source = ContractSource {
                        address: contract_address.to_string(),
                        verified_at: verified_at.clone(),
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
                            verified_at: verified_at.clone(),
                            contract_name: contract_name.map(|s| s.to_string()),
                        }],
                        signature_file: signature_file_path,
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
            verified_at: verified_at.clone(),
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
    let filename = event.signature_file.split('/').last().unwrap_or(&event.signature_file);
    let file_path = events_dir.join(filename);

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

    content.push_str("\nContract Sources (sorted by verification time, most recent first):\n");
    for source in &event.contract_sources {
        let verified_at_str = source.verified_at
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("N/A");
        let contract_name_str = source.contract_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        content.push_str(&format!("  - {} (contract_name: {}, verified_at: {})\n",
                                 source.address, contract_name_str, verified_at_str));
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
            verified_at: contract_event_info.verified_at,
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

    // Sort events within each contract and addresses by verified_at in descending order
    for contract in &mut contracts {
        contract.events.sort_by(|a, b| a.event.cmp(&b.event));
        sort_contract_addresses_by_verified_at_desc(&mut contract.address);
    }

    ContractsEventsOutput { contracts }
}

fn is_contract_verified(is_verified: Option<bool>, is_fully_verified: Option<bool>) -> bool {
    is_verified.unwrap_or(false) || is_fully_verified.unwrap_or(false)
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
    parent_address: Option<&str>,
    abi_dir_name: &str,
) -> Result<String> {
    let base_filename = if let Some(name) = contract_name {
        sanitize_filename(name)
    } else {
        contract_address.to_string()
    };

    let filename = if let Some(parent) = parent_address {
        format!("{}_{}_parent_{}.json", base_filename, contract_address, parent)
    } else {
        format!("{}_{}.json", base_filename, contract_address)
    };

    let file_path = abi_dir.join(&filename);

    let abi_json = serde_json::to_string_pretty(abi)
        .context("Failed to serialize ABI to JSON")?;

    fs::write(&file_path, abi_json)
        .with_context(|| format!("Failed to write ABI file: {:?}", file_path))?;

    Ok(format!("{}/{}", abi_dir_name, filename))
}

async fn process_implementations_recursively(
    client: &BlockscoutClient,
    implementations: Vec<Implementation>,
    parent_address: &str,
    abi_dir: &Path,
    abi_dir_name: &str,
    events_map: &mut HashMap<String, EventDefinition>,
    contract_events_list: &mut Vec<ContractEventInfo>,
    depth: usize,
    max_depth: Option<usize>,
    max_per_level: Option<usize>,
    events_dir: &str,
    counters: &mut AbiFileCounters,
) -> Result<Vec<ImplementationInfo>> {
    // Check max depth limit
    if let Some(max) = max_depth {
        if depth >= max {
            warn!("Maximum implementation nesting depth ({}) reached, stopping implementation processing", max);
            return Ok(Vec::new());
        }
    } else if depth > 10 {
        // Fallback to hardcoded limit if no config provided
        warn!("Maximum recursion depth reached, stopping implementation processing");
        return Ok(Vec::new());
    }

    // Limit the number of implementations to process at this level
    let impls_to_process = if let Some(max) = max_per_level {
        if implementations.len() > max {
            info!("Limited implementations at depth {} to {} (from {})", depth, max, implementations.len());
        }
        implementations.into_iter().take(max).collect::<Vec<_>>()
    } else {
        implementations
    };

    let mut impl_infos = Vec::new();

    for implementation in impls_to_process {
        let impl_address = &implementation.address;

        match client.fetch_contract_details(impl_address).await {
            Ok(impl_details) => {
                let is_verified = is_contract_verified(impl_details.is_verified, impl_details.is_fully_verified);

                let impl_abi_file = if is_verified {
                    if let Some(abi) = &impl_details.abi {
                        // Parse events from this ABI
                        let final_contract_name = impl_details.name.as_deref().or(implementation.name.as_deref());
                        if let Err(e) = parse_abi_events(
                            abi,
                            impl_address,
                            final_contract_name,
                            &impl_details.verified_at,
                            events_map,
                            contract_events_list,
                            events_dir,
                        ) {
                            warn!("Failed to parse events from implementation {}: {:?}", impl_address, e);
                        }

                        let abi_file_path = save_abi_to_file(
                            abi,
                            final_contract_name,
                            impl_address,
                            abi_dir,
                            Some(parent_address),
                            abi_dir_name,
                        )?;

                        // Update counters
                        counters.verified_implementations += 1;

                        Some(abi_file_path)
                    } else {
                        None
                    }
                } else {
                    if impl_details.abi.is_some() {
                        counters.unverified_implementations += 1;
                    }
                    None
                };

                // Recursively process nested implementations
                let nested_implementations = if let Some(nested_impls) = impl_details.implementations {
                    let nested_impl_infos = Box::pin(process_implementations_recursively(
                        client,
                        nested_impls,
                        impl_address,
                        abi_dir,
                        abi_dir_name,
                        events_map,
                        contract_events_list,
                        depth + 1,
                        max_depth,
                        max_per_level,
                        events_dir,
                        counters,
                    )).await?;

                    if nested_impl_infos.is_empty() { None } else { Some(nested_impl_infos) }
                } else {
                    None
                };

                impl_infos.push(ImplementationInfo {
                    name: impl_details.name.or(implementation.name.clone()),
                    address: impl_address.clone(),
                    abi_file: impl_abi_file,
                    is_verified,
                    is_fully_verified: impl_details.is_fully_verified,
                    verified_at: impl_details.verified_at,
                    implementations: nested_implementations,
                });
            }
            Err(e) => {
                error!("Failed to fetch implementation details for {}: {:?}", impl_address, e);
                // Continue with other implementations
            }
        }
    }

    Ok(impl_infos)
}

async fn process_contract_with_implementations(
    client: &BlockscoutClient,
    contract_item: &SmartContractItem,
    abi_dir: &Path,
    abi_dir_name: &str,
    events_map: &mut HashMap<String, EventDefinition>,
    contract_events_list: &mut Vec<ContractEventInfo>,
    max_depth: Option<usize>,
    max_per_level: Option<usize>,
    events_dir: &str,
    counters: &mut AbiFileCounters,
) -> Result<ContractInfo> {
    let address = &contract_item.address.hash;

    // Fetch contract details
    let contract_details = client.fetch_contract_details(address).await
        .with_context(|| format!("Failed to get details for contract {}", address))?;

    let is_verified = is_contract_verified(contract_details.is_verified, contract_details.is_fully_verified);

    // Save ABI if available and contract is verified
    let abi_file = if is_verified {
        if let Some(abi) = &contract_details.abi {
            // Parse events from this ABI
            let final_verified_at = contract_details.verified_at.as_ref().or(contract_item.verified_at.as_ref());
            let final_contract_name = contract_details.name.as_deref().or(contract_item.address.name.as_deref());
            if let Err(e) = parse_abi_events(
                abi,
                address,
                final_contract_name,
                &final_verified_at.cloned(),
                events_map,
                contract_events_list,
                events_dir,
            ) {
                warn!("Failed to parse events from contract {}: {:?}", address, e);
            }

            let abi_file_path = save_abi_to_file(
                abi,
                final_contract_name,
                address,
                abi_dir,
                None,
                abi_dir_name,
            )?;

            // Update counters
            counters.verified_contracts += 1;

            Some(abi_file_path)
        } else {
            None
        }
    } else {
        if contract_details.abi.is_some() {
            counters.unverified_contracts += 1;
        }
        None
    };

    // Process implementations recursively if any
    let implementations = if let Some(impls) = contract_details.implementations {
        let impl_infos = process_implementations_recursively(
            client,
            impls,
            address,
            abi_dir,
            abi_dir_name,
            events_map,
            contract_events_list,
            0, // Start at depth 0
            max_depth,
            max_per_level,
            events_dir,
            counters,
        ).await?;

        if impl_infos.is_empty() { None } else { Some(impl_infos) }
    } else {
        None
    };

    Ok(ContractInfo {
        name: contract_details.name.or(contract_item.address.name.clone()),
        address: address.clone(),
        abi_file,
        is_verified,
        is_fully_verified: contract_details.is_fully_verified,
        verified_at: contract_details.verified_at.or(contract_item.verified_at.clone()),
        implementations,
    })
}

// Add sorting functions - changed to descending order for contracts output
fn sort_contracts_by_verified_at(contracts: &mut Vec<ContractInfo>) {
    contracts.sort_by(|a, b| {
        match (parse_verified_at_timestamp(&a.verified_at), parse_verified_at_timestamp(&b.verified_at)) {
            (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts), // Descending order (most recent first)
            (Some(_), None) => std::cmp::Ordering::Less, // Verified contracts first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.address.cmp(&b.address), // Fallback to address comparison
        }
    });

    // Sort implementations recursively
    for contract in contracts {
        if let Some(ref mut implementations) = contract.implementations {
            sort_implementations_by_verified_at(implementations);
        }
    }
}

fn sort_implementations_by_verified_at(implementations: &mut Vec<ImplementationInfo>) {
    implementations.sort_by(|a, b| {
        match (parse_verified_at_timestamp(&a.verified_at), parse_verified_at_timestamp(&b.verified_at)) {
            (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts), // Descending order (most recent first)
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.address.cmp(&b.address),
        }
    });

    // Sort nested implementations recursively
    for implementation in implementations {
        if let Some(ref mut nested_implementations) = implementation.implementations {
            sort_implementations_by_verified_at(nested_implementations);
        }
    }
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

// Helper function to ensure strings are properly quoted in YAML
fn ensure_quoted_yaml(yaml_content: String) -> String {
    let lines: Vec<&str> = yaml_content.lines().collect();
    let mut result = String::new();

    for line in lines {
        if line.trim_start().starts_with("name:") ||
           line.trim_start().starts_with("- name:") ||
           line.trim_start().starts_with("address:") ||
           line.trim_start().starts_with("- address:") ||
           line.trim_start().starts_with("abi_file:") ||
           line.trim_start().starts_with("signature_file:") {

            // Check if the line already has quotes or is null
            if line.contains("null") || line.contains("~") {
                result.push_str(line);
                result.push('\n');
                continue;
            } else if let Some(colon_pos) = line.find(':') {
                let key_part = &line[..=colon_pos];
                let value_part = line[colon_pos + 1..].trim();

                // Skip if empty value
                if value_part.is_empty() {
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }

                // Remove existing quotes if any
                let clean_value = value_part.trim_matches(|c| c == '"' || c == '\'');

                // Add properly quoted value
                result.push_str(&format!("{} \"{}\"\n", key_part, clean_value));
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}

fn save_contracts_to_yaml<P: AsRef<Path>>(
    contracts_output: &ContractsOutput,
    output_path: P,
) -> Result<()> {
    let yaml_content = serde_yaml::to_string(contracts_output)
        .context("Failed to serialize contracts to YAML")?;

    // Post-process to ensure proper quoting
    let quoted_yaml = ensure_quoted_yaml(yaml_content);

    fs::write(&output_path, quoted_yaml)
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

    // Post-process to ensure proper quoting for signature_file
    let quoted_yaml = ensure_quoted_yaml(yaml_content);

    fs::write(&output_path, quoted_yaml)
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

    // Log implementation limits if configured
    if let Some(max_impls) = config.blockscout.max_implementations_per_contract {
        info!("Max implementations per contract: {}", max_impls);
    } else {
        info!("Max implementations per contract: unlimited");
    }
    if let Some(max_depth) = config.blockscout.max_implementation_nesting_depth {
        info!("Max implementation nesting depth: {}", max_depth);
    } else {
        info!("Max implementation nesting depth: unlimited (fallback to 10)");
    }

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
    );

    // Fetch all verified contracts with pagination
    let contract_items = client.fetch_all_verified_contracts().await
        .context("Failed to fetch verified contracts")?;

    info!("Processing {} contracts and their implementations...", contract_items.len());

    // Process each contract and its implementations
    let mut contract_infos = Vec::new();
    let mut events_map: HashMap<String, EventDefinition> = HashMap::new();
    let mut contract_events_list: Vec<ContractEventInfo> = Vec::new();
    let mut counters = AbiFileCounters::default();

    for contract_item in contract_items {
        match process_contract_with_implementations(
            &client,
            &contract_item,
            abi_dir,
            &config.output.abi_directory,
            &mut events_map,
            &mut contract_events_list,
            config.blockscout.max_implementation_nesting_depth,
            config.blockscout.max_implementations_per_contract,
            &config.output.events_directory,
            &mut counters,
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

    // Separate verified and unverified contracts
    let mut verified_contracts = Vec::new();
    let mut unverified_contracts = Vec::new();

    for contract_info in contract_infos {
        if contract_info.is_verified {
            verified_contracts.push(contract_info);
        } else {
            unverified_contracts.push(contract_info);
        }
    }

    // Sort contracts by verified_at timestamp (descending order - most recent first)
    sort_contracts_by_verified_at(&mut verified_contracts);
    sort_contracts_by_verified_at(&mut unverified_contracts);

    // Save event signature files and prepare events output
    let mut events_list: Vec<EventDefinition> = events_map.into_values().collect();
    events_list.sort_by(|a, b| a.name.cmp(&b.name));

    // Sort contract sources within each event by verified_at in descending order
    for event in &mut events_list {
        sort_contract_sources_by_verified_at_desc(&mut event.contract_sources);
    }

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
            total_verified_with_abi: counters.verified_contracts,
            total_unverified_with_abi: counters.unverified_contracts,
            total_verified_implementations_with_abi: counters.verified_implementations,
            total_unverified_implementations_with_abi: counters.unverified_implementations,
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
        "Successfully processed {} verified and {} unverified contracts",
        contracts_output.metadata.total_verified,
        contracts_output.metadata.total_unverified
    );

    info!(
        "ABI files created - Verified contracts: {}, Unverified contracts: {}, Verified implementations: {}, Unverified implementations: {}",
        counters.verified_contracts,
        counters.unverified_contracts,
        counters.verified_implementations,
        counters.unverified_implementations
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_extended_event_signature() {
        let inputs = vec![
            AbiInput {
                name: "userPercentage".to_string(),
                input_type: "uint256".to_string(),
                indexed: Some(true),
                components: None,
            },
            AbiInput {
                name: "repPercentage".to_string(),
                input_type: "uint256".to_string(),
                indexed: Some(true),
                components: None,
            },
            AbiInput {
                name: "artifactPercentage".to_string(),
                input_type: "uint256".to_string(),
                indexed: Some(true),
                components: None,
            },
        ];

        let extended_signature = generate_extended_event_signature("RewardPercentagesUpdated", &inputs, false);
        assert_eq!(extended_signature, "RewardPercentagesUpdated(uint256 indexed userPercentage, uint256 indexed repPercentage, uint256 indexed artifactPercentage)");
    }

    #[test]
    fn test_generate_event_signature() {
        let inputs = vec![
            AbiInput {
                name: "from".to_string(),
                input_type: "address".to_string(),
                indexed: Some(true),
                components: None,
            },
            AbiInput {
                name: "to".to_string(),
                input_type: "address".to_string(),
                indexed: Some(true),
                components: None,
            },
            AbiInput {
                name: "value".to_string(),
                input_type: "uint256".to_string(),
                indexed: Some(false),
                components: None,
            },
        ];

        let signature = generate_event_signature("Transfer", &inputs, false);
        assert_eq!(signature, "Transfer(address,address,uint256)");

        let anon_signature = generate_event_signature("Transfer", &inputs, true);
        assert_eq!(anon_signature, "Transfer(address,address,uint256) [anonymous]");
    }

    #[test]
    fn test_generate_topic_hash() {
        let hash = generate_topic_hash("Transfer(address,address,uint256)");
        assert_eq!(hash, "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    }

    #[test]
    fn test_parse_verified_at_timestamp() {
        let valid_timestamp = Some("2023-09-11T10:30:45Z".to_string());
        let invalid_timestamp = Some("invalid".to_string());
        let none_timestamp: Option<String> = None;

        assert!(parse_verified_at_timestamp(&valid_timestamp).is_some());
        assert!(parse_verified_at_timestamp(&invalid_timestamp).is_none());
        assert!(parse_verified_at_timestamp(&none_timestamp).is_none());
    }
}
