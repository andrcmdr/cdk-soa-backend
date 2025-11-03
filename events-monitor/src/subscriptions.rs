use std::collections::BTreeMap;
use futures_util::StreamExt;
use tracing::{info, error, debug};

use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    transports::ws::WebSocketConfig,
    rpc::types::{Filter, FilterBlockOption, BlockNumberOrTag, Log as RpcLog},
    primitives::Address,
    json_abi::JsonAbi,
};
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy::providers::{Identity, RootProvider};
use alloy::consensus::Transaction;
use alloy::network::TransactionResponse;

use async_nats::jetstream::object_store::ObjectStore;

use crate::{abi::ContractAbi, db::{self, DatabaseClients}, nats, nats::Nats};
use crate::config::AppCfg as AppConfig;
use crate::event_decoder::EventDecoder;
use crate::types::EventPayload;

use std::ops::{Range, RangeFrom};
use std::str::FromStr;
use std::sync::Arc;
use anyhow::anyhow;
use tokio::task::JoinHandle;

type RPCProvider = FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider>;

pub struct EventProcessor {
    addr_abi_map: BTreeMap<Address, ContractAbi>,
    db_clients: DatabaseClients,
    nats_store: Option<Nats>,
    config: AppConfig,
    ws_rpc_provider: RPCProvider,
    http_rpc_provider: RPCProvider,
    chain_id: u64,
    filter_senders: Option<Vec<Address>>,
    filter_receivers: Option<Vec<Address>>,
}

impl EventProcessor {
    pub async fn new(config: &AppConfig, db_clients: DatabaseClients, nats_store: Option<Nats>) -> anyhow::Result<Self> {
        // Get all contracts including implementations
        let all_contracts = config.get_all_contracts();

        let mut contracts = Vec::with_capacity(all_contracts.len());
        for c in all_contracts.iter() {
            let abi = ContractAbi::from_contract_with_implementation(c)?;
            contracts.push(abi);
        }

        info!("Loaded contracts: {} (including implementations)", contracts.len());

        // Index contracts by address for a quick lookup
        // For proxy contracts, we need to map the proxy address to implementation ABI
        let mut addr_abi_map: BTreeMap<Address, ContractAbi> = BTreeMap::new();
        for c in contracts {
            if c.is_implementation() {
                // For implementations, use the parent (proxy) contract address as key
                // but keep the implementation ABI for decoding
                let proxy_address = c.get_effective_contract_address();

                // Check if we already have a contract for this address
                // If so, we might want to merge or handle multiple implementations
                if addr_abi_map.contains_key(&proxy_address) {
                    debug!("Multiple implementations found for proxy address: {}", proxy_address);
                    // For now, use the last implementation loaded
                    // In a more sophisticated setup, we might want to merge or handle all implementations
                }

                addr_abi_map.insert(proxy_address, c);
            } else {
                // Regular contracts use their own address
                addr_abi_map.insert(c.address, c);
            }
        }

        let ws_config = WebSocketConfig::default()
            .read_buffer_size(256 * 1024)
            .write_buffer_size(256 * 1024)
            .max_message_size(Some(1024 * 1024 * 1024))
            .max_frame_size(Some(256 * 1024 * 1024))
            .accept_unmasked_frames(false);
        let ws = WsConnect::new(&config.chain.ws_rpc_url).with_config(ws_config);
        let http_rpc = reqwest::Url::from_str(&config.chain.http_rpc_url)?;
        let (ws_rpc_provider, http_rpc_provider) = build_providers(ws, http_rpc).await?;

        let chain_id = http_rpc_provider.get_chain_id().await?;
        if chain_id != config.chain.chain_id {
            anyhow::bail!("Chain ID mismatch: expected {}, got {}", config.chain.chain_id, chain_id);
        }
        info!("Chain ID: {}", chain_id);

        // Parse sender filtering addresses from configuration
        let filter_senders = if let Some(senders) = &config.indexing.filter_senders {
            if !senders.is_empty() {
                let parsed: Result<Vec<Address>, _> = senders.iter()
                    .map(|s| Address::from_str(s))
                    .collect();
                let parsed = parsed?;
                info!("Filter senders configured: {} addresses", parsed.len());
                Some(parsed)
            } else {
                None
            }
        } else {
            None
        };

        // Parse receiver filtering addresses from configuration
        let filter_receivers = if let Some(receivers) = &config.indexing.filter_receivers {
            if !receivers.is_empty() {
                let parsed: Result<Vec<Address>, _> = receivers.iter()
                    .map(|s| Address::from_str(s))
                    .collect();
                let parsed = parsed?;
                info!("Filter receivers configured: {} addresses", parsed.len());
                Some(parsed)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            addr_abi_map,
            db_clients,
            nats_store,
            config: config.clone(),
            ws_rpc_provider,
            http_rpc_provider,
            chain_id,
            filter_senders,
            filter_receivers,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let self_arc = Arc::new(self);

        let from_block = self_arc.config.indexing.from_block.unwrap_or(0u64);
        let to_block = self_arc.config.indexing.to_block;

        // build a single filter for all addresses
        let addresses: Vec<Address> = self_arc.addr_abi_map.iter().map(|(addr, _c)| *addr).collect();

        let mut filter = Filter::new()
            .address(addresses.clone())
            .select(0u64..);

        if let Some(to_block) = to_block {
            filter = Filter::new()
                .address(addresses.clone())
                .select(BlockRange(from_block..to_block));
        } else {
            filter = Filter::new()
                .address(addresses.clone())
                .select(BlockRangeFrom(from_block..));
        }

        let mut handles: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

        // Task 1: Process historical logs, if enabled
        let process_historical_logs = self_arc.config.indexing.historical_logs_processing.is_some_and(|process_logs| process_logs > 0);
        if process_historical_logs {
            let processor_for_history = Arc::clone(&self_arc);
            let filter_for_history = filter.clone();
            let addresses_for_history = addresses.clone();

            let historical_task = tokio::spawn(async move {
                info!("Starting historical logs processing task");

                let logs_sync_protocol = processor_for_history.config.indexing.logs_sync_protocol.clone();
                if let Some(ref sync_protocol) = logs_sync_protocol && sync_protocol.to_lowercase() == "http_watcher" {
                    info!("Starting HTTP watch_logs task for historical logs");

                    // Start historical watching logs using HTTP polling
                    let poller = processor_for_history.http_rpc_provider
                        .watch_logs(&filter_for_history)
                        .await?;

                    // Convert poller to stream
                    let mut log_stream = poller.into_stream().flat_map(futures::stream::iter);

                    info!("Started historical watching logs via HTTP polling");

                    // Process logs as they arrive
                    while let Some(log) = log_stream.next().await {
                        debug!("Received historical watch_logs log from contract: {}", log.address());
                        if let Err(e) = processor_for_history.handle_log(log).await {
                            error!("Failed to handle historical watch_logs log: {:?}", e);
                            eprintln!("Historical watch logs error: {:?}", e);
                        }
                    }

                    info!("Historical watch logs task completed");
                } else {
                    let logs = match logs_sync_protocol {
                        Some(protocol) if protocol.to_lowercase() == "http" => {
                            processor_for_history.http_rpc_provider.get_logs(&filter_for_history).await?
                        },
                        Some(protocol) if protocol.to_lowercase() == "ws" => {
                            processor_for_history.ws_rpc_provider.get_logs(&filter_for_history).await?
                        },
                        _ => {
                            error!("Invalid log sync protocol (must be 'http' or 'ws'): {:?}", logs_sync_protocol);
                            info!("Fallback to 'http' RPC protocol for logs sync");
                            processor_for_history.http_rpc_provider.get_logs(&filter_for_history).await?
                        }
                    };
                    debug!("Received {} logs from {} contracts", logs.len(), addresses_for_history.len());

                    for log in logs.iter() {
                        debug!("Received historical log from contract: {}", log.address());
                        debug!("Historical log: {:?}", log);
                        if let Err(e) = processor_for_history.handle_log(log.clone()).await {
                            error!("Failed to handle historical log: {:?}", e);
                            eprintln!("Historical log error: {:?}", e);
                        }
                    }
                }

                info!("Historical logs processing task completed");
                Ok(())
            });
            handles.push(historical_task);
        }

        // Task 2: Subscribe to new logs, if enabled
        let subscribe_new_logs = self_arc.config.indexing.new_logs_subscription.is_some_and(|subscribe_logs| subscribe_logs > 0);
        if subscribe_new_logs {
            let processor_for_subscription = Arc::clone(&self_arc);
            let addresses_for_subscription = addresses.clone();

            // Determine subscription protocol (default to WS for backward compatibility)
            let subscription_protocol = processor_for_subscription.config.indexing.new_logs_subscription_protocol
                .clone()
                .unwrap_or_else(|| "http".to_string()); // fetch new logs via HTTP RPC by default

            if subscription_protocol.to_lowercase() == "http" {
                // HTTP polling mode
                let polling_interval_secs = processor_for_subscription.config.indexing.http_polling_interval_secs.unwrap_or(5);

                let subscription_task = tokio::spawn(async move {
                    info!("Starting HTTP polling task for new logs (interval: {}s)", polling_interval_secs);

                    // Start watching from the current block or configured block
                    let start_block = match processor_for_subscription.http_rpc_provider.get_block_number().await {
                        Ok(block) => block,
                        Err(e) => {
                            error!("Failed to get latest block number (as starting block): {:?}", e);
                            BlockNumberOrTag::Latest.as_number().unwrap_or(0)
                        }
                    };
                    let mut current_block = start_block;
                    info!("Starting HTTP polling from block {}", current_block);

                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(polling_interval_secs));

                    loop {
                        interval.tick().await;

                        // Get the latest block number
                        let latest_block = match processor_for_subscription.http_rpc_provider.get_block_number().await {
                            Ok(block) => block,
                            Err(e) => {
                                error!("Failed to get latest block number: {:?}", e);
                                continue;
                            }
                        };

                        // If there are new blocks, fetch logs
                        if latest_block > current_block {
                            debug!("Polling for logs from block {} to {}", current_block + 1, latest_block);

                            // Create a filter for the new blocks
                            let poll_filter = Filter::new()
                                .address(addresses_for_subscription.clone())
                                .select(BlockRange((current_block + 1)..latest_block + 1));

                            match processor_for_subscription.http_rpc_provider.get_logs(&poll_filter).await {
                                Ok(logs) => {
                                    debug!("Received {} new logs via HTTP polling", logs.len());

                                    for log in logs {
                                        debug!("Received polling log from contract: {}", log.address());
                                        if let Err(e) = processor_for_subscription.handle_log(log).await {
                                            error!("Failed to handle polling log: {:?}", e);
                                            eprintln!("Polling log error: {:?}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to get logs via HTTP polling: {:?}", e);
                                }
                            }

                            // Update current block
                            current_block = latest_block;
                        } else {
                            debug!("No new blocks (current: {}, latest: {})", current_block, latest_block);
                        }
                    }
                });
                handles.push(subscription_task);
            } else if subscription_protocol.to_lowercase() == "http_watcher" {
                // HTTP polling mode using watch_logs
                let subscription_task = tokio::spawn(async move {
                    info!("Starting HTTP watch_logs task for new logs");

                    // Create filter for new logs (from latest block)
                    let watch_filter = Filter::new()
                        .address(addresses_for_subscription.clone())
                        .from_block(BlockNumberOrTag::Latest);

                    // Start watching logs using HTTP polling
                    let poller = processor_for_subscription.http_rpc_provider
                        .watch_logs(&watch_filter)
                        .await?;

                    // Convert poller to stream
                    let mut log_stream = poller.into_stream().flat_map(futures::stream::iter);

                    info!("Started watching logs via HTTP polling");

                    // Process logs as they arrive
                    while let Some(log) = log_stream.next().await {
                        debug!("Received watch_logs log from contract: {}", log.address());
                        if let Err(e) = processor_for_subscription.handle_log(log).await {
                            error!("Failed to handle watch_logs log: {:?}", e);
                            eprintln!("Watch logs error: {:?}", e);
                        }
                    }

                    info!("Watch logs task completed");
                    Ok(())
                });
                handles.push(subscription_task);
            } else {
                // WebSocket subscription mode (original initial implementation using WS subscribe_logs method)
                let subscription_task = tokio::spawn(async move {
                    info!("Starting WebSocket subscription task");

                    let provider = processor_for_subscription.ws_rpc_provider.clone();
                    let sub = provider.subscribe_logs(&filter).await?;
                    info!("Subscribed to logs for {} contracts", addresses_for_subscription.len());

                    let mut sub_stream = sub.into_stream();
                    while let Some(log) = sub_stream.next().await {
                        debug!("Received subscription log from contract: {}", log.address());
                        if let Err(e) = processor_for_subscription.handle_log(log).await {
                            error!("Failed to handle subscription log: {:?}", e);
                            eprintln!("Subscription log error: {:?}", e);
                        }
                    }

                    info!("Subscription task completed");
                    Ok(())
                });
                handles.push(subscription_task);
            }
        }

        // Wait for all tasks to complete
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => info!("Task completed successfully"),
                Ok(Err(e)) => {
                    error!("Task failed with error: {:?}", e);
                    return Err(e);
                }
                Err(join_err) => {
                    error!("Task panicked: {:?}", join_err);
                    return Err(anyhow!("Task panicked: {:?}", join_err));
                }
            }
        }

        Ok(())
    }

    async fn handle_log(&self, log: RpcLog) -> anyhow::Result<()> {
        let addr = log.address();
        debug!("Received log from contract: {}", addr);

        // Retrieve tx sender using transaction hash
        let tx_sender = if let Some(h) = log.transaction_hash {
            match self.http_rpc_provider.get_transaction_by_hash(h).await? {
                Some(tx) => Some(tx.from()),
                None => None,
            }
        } else { None };

        // Retrieve tx receiver using transaction hash
        let tx_receiver = if let Some(h) = log.transaction_hash {
            match self.http_rpc_provider.get_transaction_by_hash(h).await? {
                Some(tx) => if let Some(addr) = tx.to() { Some(addr) } else { None },
                None => None,
            }
        } else { None };

        // Apply sender filtering if configured
        if let Some(filter_senders) = &self.filter_senders {
            if !filter_senders.is_empty() {
                if let Some(sender) = tx_sender {
                    if !filter_senders.contains(&sender) {
                        debug!("Filtering out log: sender {} not in filter list", sender);
                        return Ok(());
                    }
                } else {
                    debug!("Filtering out log: no sender found in transaction");
                    return Ok(());
                }
            }
        }

        // Apply receiver filtering if configured
        if let Some(filter_receivers) = &self.filter_receivers {
            if !filter_receivers.is_empty() {
                if let Some(receiver) = tx_receiver {
                    if !filter_receivers.contains(&receiver) {
                        debug!("Filtering out log: receiver {} not in filter list", receiver);
                        return Ok(());
                    }
                } else {
                    debug!("Filtering out log: no receiver found in transaction");
                    return Ok(());
                }
            }
        }

        let transaction_sender = tx_sender
            .map(|addr| addr.to_string())
            .ok_or_else(|| {
                error!("Missing sender address in transaction data");
                anyhow!("Missing sender address in transaction data")
            })
            .unwrap_or("".to_string());

        let transaction_receiver = tx_receiver
            .map(|addr| addr.to_string())
            .ok_or_else(|| {
                error!("Missing receiver address in transaction data");
                anyhow!("Missing receiver address in transaction data")
            })
            .unwrap_or("".to_string());

        let Some(contract) = self.addr_abi_map.get(&addr) else { return Ok(()); };

        let abi = Arc::new(contract.abi.clone());
        let decoder = EventDecoder::new(abi)?;
        let parsed_event = decoder.decode_log(&log.inner)?;
        let parsed_event_value = parsed_event.to_json()?;

        // Determine contract and implementation details
        let (contract_name, contract_address, implementation_name, implementation_address) =
            if contract.is_implementation() {
                // This is an implementation, so we have proxy -> implementation mapping
                (
                    contract.get_effective_contract_name().to_string(),
                    contract.get_effective_contract_address().to_string(),
                    contract.implementation_name.clone(),
                    contract.implementation_address.map(|addr| addr.to_string()),
                )
            } else {
                // This is a regular contract
                (
                    contract.name.clone(),
                    contract.address.to_string(),
                    None,
                    None,
                )
            };

        let block_number = log.block_number.unwrap_or_default().to_string();
        let block_hash = log.block_hash
            .map(|bh| format!("0x{}", hex::encode(bh.0.as_slice())))
            .ok_or_else(|| {
                error!("Missing block hash in a log");
                anyhow!("Missing block hash in a log")
            })
            .unwrap_or("0x".to_string());
        let block_timestamp = log.block_timestamp.unwrap_or_default();
        let block_time = chrono::DateTime::from_timestamp(block_timestamp as i64, 0)
            .unwrap_or_default()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let transaction_hash = log.transaction_hash
            .map(|th| format!("0x{}", hex::encode(th.0.as_slice())))
            .ok_or_else(|| {
                error!("Missing transaction hash in a log");
                anyhow!("Missing transaction hash in a log")
            })
            .unwrap_or("0x".to_string());
        let tx_index = log.transaction_index.unwrap_or_default().to_string();
        let log_index = log.log_index.unwrap_or_default().to_string();
        let event_name = parsed_event.name.as_str();
        let event_signature = parsed_event.signature
            .map(|h| format!("0x{}", hex::encode(h.0.as_slice())))
            .ok_or_else(|| {
                error!("Missing event signature/hash in parsed event data: anonymous event");
                anyhow!("Missing event signature/hash in parsed event data: anonymous event")
            })
            .unwrap_or("0x".to_string());

        // Compute unique log hash using the Log's `hash()` with SHA3-256 hasher
        let mut hasher = Sha3_256StdHasher::default();
        log.inner.hash(&mut hasher);
        let log_hash_bytes = hasher.finalize_bytes();
        let log_hash = format!("0x{}", hex::encode(log_hash_bytes));

        let payload = EventPayload {
            contract_name,
            contract_address,
            implementation_name,
            implementation_address,
            chain_id: self.chain_id.to_string(),
            block_number,
            block_hash,
            block_timestamp: block_timestamp.to_string(),
            block_time,
            transaction_hash,
            transaction_sender,
            transaction_receiver,
            transaction_index: tx_index,
            log_index,
            log_hash,
            event_name: event_name.to_string(),
            event_signature,
            event_data: parsed_event_value,
        };

        debug!("Persisting event: {:?}", payload);

        // Persist to databases (local PostgreSQL + AWS RDS if enabled)
        self.db_clients.insert_event(&payload).await?;

        // Persist to NATS Object Store
        if let Some(nats_store) = &self.nats_store {
            nats::publish_event(&nats_store.object_store, &payload).await?;
        };

        Ok(())
    }
}

/// Range (from..to) block type conversion helpers
pub struct BlockRange(pub Range<u64>);
impl From<BlockRange> for FilterBlockOption {
    fn from(value: BlockRange) -> Self {
        FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(value.0.start)),
            to_block: Some(BlockNumberOrTag::Number(value.0.end)),
        }
    }
}

/// Range (from..) block type conversion helpers
pub struct BlockRangeFrom(pub RangeFrom<u64>);
impl From<BlockRangeFrom> for FilterBlockOption {
    fn from(value: BlockRangeFrom) -> Self {
        FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(value.0.start)),
            to_block: None,
//          to_block: Some(BlockNumberOrTag::Latest),
        }
    }
}

use sha3::{Digest, Sha3_256};
use std::hash::{Hash, Hasher};

/// Custom hasher adapter so we can use `Log::hash(&mut hasher)` and also get full 32-byte SHA3-256
#[derive(Default)]
struct Sha3_256StdHasher {
    inner: Sha3_256,
}

impl Hasher for Sha3_256StdHasher {
    fn write(&mut self, bytes: &[u8]) { self.inner.update(bytes); }
    fn finish(&self) -> u64 {
        // not used for our logic; provide first 8 bytes of the digest for trait compliance
        let mut clone = self.inner.clone();
        let digest = clone.finalize();
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&digest[..8]);
        u64::from_be_bytes(arr)
    }
}

impl Sha3_256StdHasher {
    fn finalize_bytes(self) -> [u8; 32] {
        let digest = self.inner.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }
}

/// Build HTTP and WS providers using Alloy
pub async fn build_providers(ws_rpc_url: WsConnect, http_rpc_url: reqwest::Url) -> anyhow::Result<(RPCProvider, RPCProvider)> {
    let ws_rpc_provider = ProviderBuilder::new().connect_ws(ws_rpc_url.clone()).await?;
    let http_rpc_provider = ProviderBuilder::new().connect_http(http_rpc_url.clone());
    info!("Connecting to RPC endpoints: ws: {:?}, http: {:?}", ws_rpc_url, http_rpc_url);

    Ok((ws_rpc_provider, http_rpc_provider))
}
