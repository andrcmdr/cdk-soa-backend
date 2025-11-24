use std::collections::BTreeMap;
use futures_util::StreamExt;
use tracing::{info, error, debug};

use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    transports::ws::WebSocketConfig,
    rpc::types::{BlockNumberOrTag, BlockTransactionsKind, BlockId},
    primitives::Address,
};
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy::providers::{Identity, RootProvider, WatchBlocks};
use alloy::consensus::{BlockBody, Transaction};
use alloy::network::TransactionResponse;

use crate::{db::{self, DatabaseClients}, nats::{self, Nats}};
use crate::config::AppCfg as AppConfig;
use crate::types::BlockPayload;

use std::ops::{Range, RangeFrom};
use std::str::FromStr;
use std::sync::Arc;
use alloy::eips::RpcBlockHash;
use alloy::rpc::types::TransactionTrait;
use anyhow::anyhow;
use tokio::task::JoinHandle;

type RPCProvider = FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider>;

pub struct BlockProcessor {
    db_clients: DatabaseClients,
    nats_store: Option<Nats>,
    config: AppConfig,
    ws_rpc_provider: RPCProvider,
    http_rpc_provider: RPCProvider,
    chain_id: u64,
    filter_senders: Option<Vec<Address>>,
    filter_receivers: Option<Vec<Address>>,
}

impl BlockProcessor {
    pub async fn new(config: &AppConfig, db_clients: DatabaseClients, nats_store: Option<Nats>) -> anyhow::Result<Self> {
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

        let mut handles: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

        // Task 1: Process historical blocks, if enabled
        let process_historical_blocks = self_arc.config.indexing.historical_blocks_processing.is_some_and(|process_blocks| process_blocks > 0);
        if process_historical_blocks {
            let processor_for_history = Arc::clone(&self_arc);

            let historical_task = tokio::spawn(async move {
                info!("Starting historical blocks processing task");

                let blocks_sync_protocol = processor_for_history.config.indexing.blocks_sync_protocol.clone();
                let full_blocks = processor_for_history.config.indexing.full_blocks_historical.unwrap_or(false);

                // Get chunk size from config, default to 100 blocks
                let chunk_size = processor_for_history.config.indexing.blocks_chunk_size.unwrap_or(100);

                // Determine the actual end block for chunking
                let end_block = if let Some(to) = to_block {
                    to
                } else {
                    // Fetch the latest block number if to_block is not specified
                    match processor_for_history.http_rpc_provider.get_block_number().await {
                        Ok(latest) => latest,
                        Err(e) => {
                            error!("Failed to get latest block number: {:?}", e);
                            return Err(anyhow!("Failed to get latest block number: {:?}", e));
                        }
                    }
                };

                info!(
                    "Processing historical blocks from block {} to {} with chunk size of {} blocks ({})",
                    from_block, end_block, chunk_size,
                    if full_blocks { "full blocks" } else { "headers only" }
                );

                // Process blocks in chunks
                let mut current_block = from_block;
                let mut total_blocks_processed = 0usize;

                while current_block < end_block {
                    let chunk_end = std::cmp::min(current_block + chunk_size, end_block);

                    info!("Fetching blocks for block range {}..{}", current_block, chunk_end);

                    // Process each block in the chunk
                    for block_num in current_block..chunk_end {
                        let block_id = BlockId::Number(BlockNumberOrTag::Number(block_num));

                        // Fetch block using the configured protocol
                        let block = match blocks_sync_protocol {
                            Some(ref protocol) if protocol.to_lowercase() == "http" => {
                                if full_blocks {
                                    processor_for_history.http_rpc_provider
                                        .get_block(block_id).full()
                                        .await?
                                } else {
                                    processor_for_history.http_rpc_provider
                                        .get_block(block_id)
                                        .await?
                                }
                            },
                            Some(ref protocol) if protocol.to_lowercase() == "ws" => {
                                if full_blocks {
                                    processor_for_history.ws_rpc_provider
                                        .get_block(block_id).full()
                                        .await?
                                } else {
                                    processor_for_history.ws_rpc_provider
                                        .get_block(block_id)
                                        .await?
                                }
                            },
                            _ => {
                                debug!("Invalid or missing block sync protocol, using 'http' as fallback");
                                if full_blocks {
                                    processor_for_history.http_rpc_provider
                                        .get_block(block_id).full()
                                        .await?
                                } else {
                                    processor_for_history.http_rpc_provider
                                        .get_block(block_id)
                                        .await?
                                }
                            }
                        };

                        if let Some(block) = block {
                            debug!("Received historical block: {}", block.header.number.to_string());
                            total_blocks_processed += 1;

                            if let Err(e) = processor_for_history.handle_block(block).await {
                                error!("Failed to handle historical block: {:?}", e);
                                eprintln!("Historical block error: {:?}", e);
                            }
                        }
                    }

                    // Move to the next chunk
                    current_block = chunk_end;

                    // Optional: Add a small delay between chunks to avoid overwhelming the RPC
                    if current_block < end_block {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }

                info!(
                    "Historical blocks processing completed: processed {} total blocks from {} to {}",
                    total_blocks_processed, from_block, end_block
                );

                Ok(())
            });
            handles.push(historical_task);
        }

        // Task 2: Subscribe to new blocks, if enabled
        let subscribe_new_blocks = self_arc.config.indexing.new_blocks_subscription.is_some_and(|subscribe_blocks| subscribe_blocks > 0);
        if subscribe_new_blocks {
            let processor_for_subscription = Arc::clone(&self_arc);

            // Determine subscription protocol (default to HTTP for backward compatibility)
            let subscription_protocol = processor_for_subscription.config.indexing.new_blocks_subscription_protocol
                .clone()
                .unwrap_or_else(|| "http".to_string());

            let full_blocks = processor_for_subscription.config.indexing.full_blocks_subscription.unwrap_or(false);

            if subscription_protocol.to_lowercase() == "http" {
                // HTTP polling mode for watching blocks
                let polling_interval_secs = processor_for_subscription.config.indexing.http_polling_interval_secs.unwrap_or(5);
                let http_subscription_method = processor_for_subscription.config.indexing.http_subscription_method
                    .clone()
                    .unwrap_or_else(|| "watch_full_blocks".to_string());

                let subscription_task = tokio::spawn(async move {
                    info!("Starting HTTP watch blocks task for new blocks (interval: {}s, {}, method: {})",
                        polling_interval_secs,
                        if full_blocks { "full blocks" } else { "headers only" },
                        http_subscription_method,
                    );

                    if full_blocks {
                        if http_subscription_method.to_lowercase() == "watch_full_blocks" {
                            let poller = processor_for_subscription.http_rpc_provider.watch_full_blocks().await?;

                            let mut block_stream = poller.into_stream().flat_map(futures::stream::iter);

                            info!("Started watching blocks (watch_full_blocks) via HTTP polling");

                            while let Some(block) = block_stream.next().await {
                                debug!("Received (watch_full_blocks) block: {}", block.header.number.to_string());
                                if let Err(e) = processor_for_subscription.handle_block(block).await {
                                    error!("Failed to handle (watch_full_blocks) block: {:?}", e);
                                    eprintln!("Failed to handle (watch_full_blocks) block: {:?}", e);
                                }
                            }
                        } else if http_subscription_method.to_lowercase() == "watch_blocks" {
                            let poller = processor_for_subscription.http_rpc_provider.watch_blocks().await?;

                            let mut block_stream = poller.into_stream().flat_map(futures::stream::iter);

                            info!("Started watching blocks (watch_blocks) via HTTP polling");

                            while let Some(new_block_hash_bytes) = block_stream.next().await {
                                let block_hash = format!("0x{}", hex::encode(new_block_hash_bytes.0.as_slice()));
                                debug!("Received (watch_blocks) block hash: {}", block_hash);
                                let block = processor_for_subscription.http_rpc_provider.get_block(BlockId::Hash(RpcBlockHash::from_hash(new_block_hash_bytes, Some(false)))).await?;
                                if let Some(block) = block {
                                    debug!("Received (watch_blocks + get_block) block: {}", block.header.number.to_string());
                                    if let Err(e) = processor_for_subscription.handle_block(block).await {
                                        error!("Failed to handle (watch_blocks + get_block) block: {:?}", e);
                                        eprintln!("Failed to handle (watch_blocks + get_block) block: {:?}", e);
                                    }
                                } else {
                                    error!("Failed to get block for hash {}", block_hash);
                                    eprintln!("Failed to get block for hash {}", block_hash);
                                };
                            }
                        }
                    } else {
                        if http_subscription_method.to_lowercase() == "watch_full_blocks" {
                            let poller = processor_for_subscription.http_rpc_provider.watch_full_blocks().await?;

                            let mut block_stream = poller.into_stream().flat_map(futures::stream::iter);

                            info!("Started watching blocks (watch_full_blocks) via HTTP polling");

                            while let Some(block) = block_stream.next().await {
                                debug!("Received (watch_full_blocks) block: {}", block.header.number.to_string());
                                // Create and reconstruct block from the header to match the expected Block type (w/o transactions w/ txs hashes only)
                                let block = alloy::rpc::types::Block {
                                    header: block.header.clone(),
                                    uncles: block.uncles.clone(),
                                    transactions: alloy::rpc::types::BlockTransactions::<_>::Hashes(block.transactions.as_hashes().unwrap_or(vec![].as_ref()).to_vec()),
                                    withdrawals: block.withdrawals.clone(),
                                };
                                if let Err(e) = processor_for_subscription.handle_block(block).await {
                                    error!("Failed to handle (watch_full_blocks) block: {:?}", e);
                                    eprintln!("Failed to handle (watch_full_blocks) block: {:?}", e);
                                }
                            }
                        } else if http_subscription_method.to_lowercase() == "watch_blocks" {
                            let poller = processor_for_subscription.http_rpc_provider.watch_blocks().await?;

                            let mut block_stream = poller.into_stream().flat_map(futures::stream::iter);

                            info!("Started watching blocks (watch_blocks) via HTTP polling");

                            while let Some(new_block_hash_bytes) = block_stream.next().await {
                                let block_hash = format!("0x{}", hex::encode(new_block_hash_bytes.0.as_slice()));
                                debug!("Received (watch_blocks) block hash: {}", block_hash);
                                let block = processor_for_subscription.http_rpc_provider.get_block(BlockId::Hash(RpcBlockHash::from_hash(new_block_hash_bytes, Some(false)))).await?;
                                if let Some(block) = block {
                                    debug!("Received (watch_blocks + get_block) block: {}", block.header.number.to_string());
                                    // Create and reconstruct block from the header to match the expected Block type (w/o transactions w/ txs hashes only)
                                    let block = alloy::rpc::types::Block {
                                        header: block.header.clone(),
                                        uncles: block.uncles.clone(),
                                        transactions: alloy::rpc::types::BlockTransactions::<_>::Hashes(block.transactions.as_hashes().unwrap_or(vec![].as_ref()).to_vec()),
                                        withdrawals: block.withdrawals.clone(),
                                    };
                                    if let Err(e) = processor_for_subscription.handle_block(block).await {
                                        error!("Failed to handle (watch_blocks + get_block) block: {:?}", e);
                                        eprintln!("Failed to handle (watch_blocks + get_block) block: {:?}", e);
                                    }
                                } else {
                                    error!("Failed to get block for hash {}", block_hash);
                                    eprintln!("Failed to get block for hash {}", block_hash);
                                };
                            }
                        }
                    };

                    info!("Watch blocks task completed");
                    Ok(())
                });
                handles.push(subscription_task);
            } else {
                // WebSocket subscription mode
                let ws_subscription_channel_size = processor_for_subscription.config.indexing.ws_subscription_channel_size.unwrap_or(10);
                let ws_subscription_method = processor_for_subscription.config.indexing.ws_subscription_method
                    .clone()
                    .unwrap_or_else(|| "subscribe_full_blocks".to_string());

                let subscription_task = tokio::spawn(async move {
                    info!("Starting WebSocket subscription task for blocks ({}, method: {}, channel size: {})",
                        if full_blocks { "full blocks" } else { "headers only" },
                        ws_subscription_method,
                        ws_subscription_channel_size
                    );

                    let provider = processor_for_subscription.ws_rpc_provider.clone();

                    if ws_subscription_method.to_lowercase() == "subscribe_full_blocks" {
                        let sub = if full_blocks {
                            provider.subscribe_full_blocks().full().channel_size(ws_subscription_channel_size as usize)
                        } else {
                            provider.subscribe_full_blocks().hashes().channel_size(ws_subscription_channel_size as usize)
                        };

                        info!("Subscribed to new blocks (subscribe_full_blocks) via WebSocket");

                        let mut sub_stream = sub.into_stream().await?;

                        while let Some(block) = sub_stream.next().await.transpose()? {
                            debug!("Received subscription block: {}", block.header.number.to_string());
                            if let Err(e) = processor_for_subscription.handle_block(block).await {
                                error!("Failed to handle subscription block: {:?}", e);
                                eprintln!("Failed to handle subscription block: {:?}", e);
                            }
                        }
                    } else if ws_subscription_method.to_lowercase() == "subscribe_blocks" {
                        let sub = provider.subscribe_blocks().channel_size(ws_subscription_channel_size as usize).await?;

                        info!("Subscribed to new blocks (subscribe_blocks) via WebSocket");

                        let mut sub_stream = sub.into_stream();

                        while let Some(block_header) = sub_stream.next().await {
                            debug!("Received subscription block header of block number: {}", block_header.number.to_string());
                            if full_blocks {
                                let block_hash = block_header.hash.clone();
                                let block = provider.get_block(BlockId::Hash(RpcBlockHash::from_hash(block_hash, Some(false)))).await?;
                                if let Some(block) = block {
                                    debug!("Received (subscribe_blocks + get_block) block: {}", block.header.number.to_string());
                                    if let Err(e) = processor_for_subscription.handle_block(block).await {
                                        error!("Failed to handle subscription block: {:?}", e);
                                        eprintln!("Failed to handle subscription block: {:?}", e);
                                    }
                                } else {
                                    error!("Failed to get block for hash {}", block_hash);
                                    eprintln!("Failed to get block for hash {}", block_hash);
                                }
                            } else {
                                let block_hash = block_header.hash.clone();
                                let block = provider.get_block(BlockId::Hash(RpcBlockHash::from_hash(block_hash, Some(false)))).await?;
                                if let Some(block) = block {
                                    debug!("Received (subscribe_blocks + get_block) block: {}", block.header.number.to_string());
                                    // Create and reconstruct block from the header to match the expected Block type (w/o transactions w/ txs hashes only)
                                    let block_txless = alloy::rpc::types::Block {
                                        header: block.header.clone(),
                                        uncles: block.uncles.clone(),
                                        transactions: alloy::rpc::types::BlockTransactions::<_>::Hashes(block.transactions.as_hashes().unwrap_or(vec![].as_ref()).to_vec()),
                                        withdrawals: block.withdrawals.clone(),
                                    };
                                    if let Err(e) = processor_for_subscription.handle_block(block_txless).await {
                                        error!("Failed to handle subscription block (constructed block from the header and txs hashes taken by get_block): {:?}", e);
                                        eprintln!("Failed to handle subscription block (constructed block from the header and txs hashes taken by get_block): {:?}", e);
                                    }
                                } else {
                                    error!("Failed to get block for hash {}", block_hash);
                                    eprintln!("Failed to get block for hash {}", block_hash);
                                }
                            }
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

    async fn handle_block(&self, block: alloy::rpc::types::Block) -> anyhow::Result<()> {
        let block_number = block.header.number;
        debug!("Received block number: {}", block_number);

        let block_hash = format!("0x{}", hex::encode(block.header.hash.0.as_slice()));
        let block_timestamp = block.header.timestamp;
        let block_time = chrono::DateTime::from_timestamp(block_timestamp as i64, 0)
            .unwrap_or_default()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Process transactions if full block
        let transactions = match block.transactions {
            alloy::rpc::types::BlockTransactions::Full(txs) => {
                let mut filtered_txs = Vec::new();

                for tx in txs {
                    let tx_sender = tx.from();
                    let tx_receiver = tx.to();

                    // Apply sender filtering if configured
                    if let Some(filter_senders) = &self.filter_senders {
                        if !filter_senders.is_empty() {
                            if !filter_senders.contains(&tx_sender) {
                                debug!("Filtering out transaction: sender {} not in filter list", tx_sender);
                                continue;
                            }
                        }
                    }

                    // Apply receiver filtering if configured
                    if let Some(filter_receivers) = &self.filter_receivers {
                        if !filter_receivers.is_empty() {
                            if let Some(receiver) = tx_receiver {
                                if !filter_receivers.contains(&receiver) {
                                    debug!("Filtering out transaction: receiver {} not in filter list", receiver);
                                    continue;
                                }
                            } else {
                                debug!("Filtering out transaction: no receiver found");
                                continue;
                            }
                        }
                    }

                    let tx_hash = format!("0x{}", hex::encode(tx.tx_hash().0.as_slice()));
                    let tx_sender_str = tx_sender.to_string();
                    let tx_receiver_str = tx_receiver.map(|addr| addr.to_string()).unwrap_or_default();
                    let tx_value = tx.value().to_string();
                    let tx_gas_price = TransactionTrait::gas_price(&tx).map(|p| p.to_string()).unwrap_or_default();
                    let tx_gas = tx.gas_limit().to_string();

                    filtered_txs.push(serde_json::json!({
                        "hash": tx_hash,
                        "from": tx_sender_str,
                        "to": tx_receiver_str,
                        "value": tx_value,
                        "gas_price": tx_gas_price,
                        "gas": tx_gas,
                    }));
                }

                Some(filtered_txs)
            },
            alloy::rpc::types::BlockTransactions::Hashes(hashes) => {
                Some(hashes.iter().map(|h| serde_json::json!({
                    "hash": format!("0x{}", hex::encode(h.0.as_slice()))
                })).collect())
            },
            alloy::rpc::types::BlockTransactions::Uncle => None,
        };

        let payload = BlockPayload {
            chain_id: self.chain_id.to_string(),
            block_number: block_number.to_string(),
            block_hash,
            block_timestamp: block_timestamp.to_string(),
            block_time,
            parent_hash: format!("0x{}", hex::encode(block.header.parent_hash.0.as_slice())),
            gas_used: block.header.gas_used.to_string(),
            gas_limit: block.header.gas_limit.to_string(),
            transactions,
        };

        debug!("Persisting block: {:?}", payload);

        // Persist to databases (local PostgreSQL + AWS RDS if enabled)
        self.db_clients.insert_block(&payload).await?;

        // Persist to NATS Object Store
        if let Some(nats_store) = &self.nats_store {
            nats::publish_block(&nats_store.object_store, &payload).await?;
        };

        Ok(())
    }
}

/// Build HTTP and WS providers using Alloy
pub async fn build_providers(ws_rpc_url: WsConnect, http_rpc_url: reqwest::Url) -> anyhow::Result<(RPCProvider, RPCProvider)> {
    let ws_rpc_provider = ProviderBuilder::new().connect_ws(ws_rpc_url.clone()).await?;
    let http_rpc_provider = ProviderBuilder::new().connect_http(http_rpc_url.clone());
    info!("Connecting to RPC endpoints: ws: {:?}, http: {:?}", ws_rpc_url, http_rpc_url);

    Ok((ws_rpc_provider, http_rpc_provider))
}
