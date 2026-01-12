use std::collections::BTreeMap;
use futures_util::StreamExt;
use tracing::{info, error, debug};

use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    transports::ws::WebSocketConfig,
    rpc::types::{Filter, FilterBlockOption, BlockNumberOrTag, Log as RpcLog},
    primitives::Address,
    json_abi::JsonAbi,
    network::TransactionResponse,
    consensus::Transaction as ConsensusTx,
};
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy::providers::{Identity, RootProvider};

use async_nats::jetstream::object_store::ObjectStore;

use crate::{db::{self, DatabaseClients}, nats, nats::Nats};
use crate::config::AppCfg as AppConfig;
use crate::types::TransactionPayload;

use std::ops::{Range, RangeFrom};
use std::str::FromStr;
use std::sync::Arc;
use anyhow::anyhow;
use tokio::task::JoinHandle;

type RPCProvider = FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider>;

pub struct TxProcessor {
    db_clients: DatabaseClients,
    nats_store: Option<Nats>,
    config: AppConfig,
    ws_rpc_provider: RPCProvider,
    http_rpc_provider: RPCProvider,
    chain_id: u64,
    filter_senders: Option<Vec<Address>>,
    filter_receivers: Option<Vec<Address>>,
}

impl TxProcessor {
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

        let mut handles: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

        // Only real-time monitoring task for mempool
        let subscription_protocol = self_arc.config.indexing.new_tx_subscription_protocol
            .clone()
            .unwrap_or_else(|| "http".to_string()); // HTTP RPC by default for mempool-monitor

        let use_full_transactions = self_arc.config.indexing.mempool_full_transactions
            .unwrap_or(false);

        if subscription_protocol.to_lowercase() == "ws" {
            // WebSocket subscription mode
            let processor_for_subscription = Arc::clone(&self_arc);

            let subscription_task = tokio::spawn(async move {
                info!("Starting WebSocket subscription task for pending transactions (full: {})", use_full_transactions);

                if use_full_transactions {
                    // Subscribe to full pending transactions
                    let sub = processor_for_subscription.ws_rpc_provider.subscribe_full_pending_transactions().await?;
                    info!("Subscribed to full pending transactions");

                    let mut sub_stream = sub.into_stream();
                    while let Some(tx) = sub_stream.next().await {
                        debug!("Received full pending transaction: {}", tx.tx_hash());
                        if let Err(e) = processor_for_subscription.handle_transaction(tx).await {
                            error!("Failed to handle full pending transaction: {:?}", e);
                        }
                    }
                } else {
                    // Subscribe to pending transaction hashes only
                    let sub = processor_for_subscription.ws_rpc_provider.subscribe_pending_transactions().await?;
                    info!("Subscribed to pending transaction hashes");

                    let mut sub_stream = sub.into_stream();
                    while let Some(tx_hash) = sub_stream.next().await {
                        debug!("Received pending transaction hash: {}", tx_hash);

                        // Fetch full transaction details
                        match processor_for_subscription.http_rpc_provider.get_transaction_by_hash(tx_hash).await? {
                            Some(tx) => {
                                if let Err(e) = processor_for_subscription.handle_transaction(tx).await {
                                    error!("Failed to handle pending transaction: {:?}", e);
                                }
                            }
                            None => {
                                debug!("Transaction not found: {}", tx_hash);
                            }
                        }
                    }
                }

                info!("Subscription task completed");
                Ok(())
            });
            handles.push(subscription_task);
        } else if subscription_protocol.to_lowercase() == "http" || subscription_protocol.to_lowercase() == "http_watcher" {
            // HTTP polling mode for mempool
            let processor_for_subscription = Arc::clone(&self_arc);
            let polling_interval_secs = processor_for_subscription.config.indexing.http_polling_interval_secs.unwrap_or(5);

            let subscription_task = tokio::spawn(async move {
                info!("Starting HTTP polling task for pending transactions (interval: {}s, full: {})",
                      polling_interval_secs, use_full_transactions);

                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(polling_interval_secs));

                loop {
                    interval.tick().await;

                    if use_full_transactions {
                        // Watch full pending transactions
                        match processor_for_subscription.http_rpc_provider.watch_full_pending_transactions().await {
                            Ok(watcher) => {
                                let mut tx_stream = watcher.into_stream().flat_map(futures::stream::iter);

                                while let Some(tx) = tx_stream.next().await {
                                    debug!("Received full pending transaction via HTTP: {}", tx.tx_hash());
                                    if let Err(e) = processor_for_subscription.handle_transaction(tx).await {
                                        error!("Failed to handle full pending transaction: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to watch full pending transactions: {:?}", e);
                            }
                        }
                    } else {
                        // Watch pending transaction hashes
                        match processor_for_subscription.http_rpc_provider.watch_pending_transactions().await {
                            Ok(watcher) => {
                                let mut hash_stream = watcher.into_stream().flat_map(futures::stream::iter);

                                while let Some(tx_hash) = hash_stream.next().await {
                                    debug!("Received pending transaction hash via HTTP: {}", tx_hash);

                                    // Fetch full transaction details
                                    match processor_for_subscription.http_rpc_provider.get_transaction_by_hash(tx_hash).await {
                                        Ok(Some(tx)) => {
                                            if let Err(e) = processor_for_subscription.handle_transaction(tx).await {
                                                error!("Failed to handle pending transaction: {:?}", e);
                                            }
                                        }
                                        Ok(None) => {
                                            debug!("Transaction not found: {}", tx_hash);
                                        }
                                        Err(e) => {
                                            error!("Failed to get transaction details: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to watch pending transactions: {:?}", e);
                            }
                        }
                    }
                }
            });
            handles.push(subscription_task);
        } else {
            anyhow::bail!("Unsupported subscription protocol: {}", subscription_protocol);
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

    async fn handle_transaction<T>(&self, tx: T) -> anyhow::Result<()>
    where
        T: TransactionResponse + ConsensusTx,
    {
        let tx_hash = tx.tx_hash();
        debug!("Processing transaction: {}", tx_hash);

        let sender = tx.from();
        let receiver = tx.to();

        // Apply sender filtering if configured
        if let Some(filter_senders) = &self.filter_senders {
            if !filter_senders.is_empty() {
                if !filter_senders.contains(&sender) {
                    debug!("Filtering out transaction: sender {} not in filter list", sender);
                    return Ok(());
                }
            }
        }

        // Apply receiver filtering if configured
        if let Some(filter_receivers) = &self.filter_receivers {
            if !filter_receivers.is_empty() {
                if let Some(to_addr) = receiver {
                    if !filter_receivers.contains(&to_addr) {
                        debug!("Filtering out transaction: receiver {} not in filter list", to_addr);
                        return Ok(());
                    }
                } else {
                    // Contract creation (no receiver)
                    debug!("Filtering out transaction: no receiver (contract creation)");
                    return Ok(());
                }
            }
        }

        let tx_sender = sender.to_string();
        let tx_receiver = receiver.map(|addr| addr.to_string());
        let tx_hash_str = format!("0x{}", hex::encode(tx_hash.0.as_slice()));
        let nonce = tx.nonce();
        let value = tx.value();
        let gas_limit = tx.gas_limit();
        let gas_price = TransactionResponse::gas_price(&tx);
        let input_data = tx.input();
        let tx_type = tx.transaction_type().map(|t| t as u8).unwrap_or(0);

        // Get max_fee_per_gas and max_priority_fee_per_gas for EIP-1559 transactions
        let (max_fee_per_gas, max_priority_fee_per_gas) = if tx_type == alloy::consensus::TxType::Eip1559 as u8 {
            // For EIP-1559 transactions, we can get these fields
            // Note: The exact method names might differ based on the trait implementation
            (TransactionResponse::max_fee_per_gas(&tx), tx.max_priority_fee_per_gas())
        } else {
            (None, None)
        };

        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let payload = TransactionPayload {
            chain_id: self.chain_id.to_string(),
            transaction_hash: tx_hash_str,
            transaction_sender: tx_sender,
            transaction_receiver: tx_receiver,
            nonce: nonce.to_string(),
            value: value.to_string(),
            gas_limit: gas_limit.to_string(),
            gas_price: gas_price.map(|p| p.to_string()),
            max_fee_per_gas: max_fee_per_gas.map(|f| f.to_string()),
            max_priority_fee_per_gas: max_priority_fee_per_gas.map(|f| f.to_string()),
            input_data: format!("0x{}", hex::encode(input_data)),
            transaction_type: tx_type.to_string(),
            timestamp,
        };

        debug!("Persisting transaction: {:?}", payload);

        // Persist to databases (local PostgreSQL + AWS RDS if enabled)
        self.db_clients.insert_transaction(&payload).await?;

        // Persist to NATS Object Store
        if let Some(nats_store) = &self.nats_store {
            nats::publish_transaction(&nats_store.object_store, &payload).await?;
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
