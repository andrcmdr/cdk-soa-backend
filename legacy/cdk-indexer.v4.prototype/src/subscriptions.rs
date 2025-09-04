use crate::abi::{AbiIndex, ContractAbi};
use crate::db::Db;
use crate::messaging::Nats;
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::primitives::{Address, B256, Bytes};
use alloy::rpc::types::eth::{Filter, Log};
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use sha3::{Digest, Sha3_256};

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

pub struct Subscriptions<N: alloy::network::Network> {
    http: RootProvider<N>,
    ws: RootProvider<N>,
    db: Db,
    nats: Option<Nats>,
    abi_index: AbiIndex,
}

impl<N: alloy::network::Network> Subscriptions<N> {
    pub fn new(http: RootProvider<N>, ws: RootProvider<N>, db: Db, nats: Option<Nats>, abi_index: AbiIndex) -> Self {
        Self { http, ws, db, nats, abi_index }
    }

    pub async fn run(&self, addresses: Vec<Address>, from_block: Option<B256>) -> anyhow::Result<()> {
        let mut filter = Filter::new().address(addresses.clone());
        if let Some(_fb) = from_block { /* Alloy Filter supports from_block via block hash/number on builder; left as-is for realtime */ }

        let mut sub = self.ws.subscribe_logs(&filter).await?;
        tracing::info!("Subscribed to logs for {} contracts", addresses.len());

        while let Some(item) = sub.next().await {
            match item {
                Ok(log) => {
                    if let Err(err) = self.handle_log(log).await {
                        tracing::warn!(?err, "handle_log failed");
                    }
                }
                Err(e) => tracing::error!(error = %e, "subscription error"),
            }
        }
        Ok(())
    }

    async fn handle_log(&self, log: Log) -> anyhow::Result<()> {
        // Compute unique log hash using the Log's `hash()` with SHA3-256 hasher
        let mut hasher = Sha3_256StdHasher::default();
        log.hash(&mut hasher);
        let log_hash = hasher.finalize_bytes();

        // Resolve contract
        let addr = log.address;
        let contract = self.abi_index.get(&addr);

        // Decode event (name, event_hash, params)
        let (event_name, event_hash, decoded_params) = if let Some(c) = contract {
            match c.decode_log(&log) {
                Some((name, evh, params)) => (Some(name), Some(evh), Some(params)),
                None => (None, None, None),
            }
        } else { (None, None, None) };

        // Extract chain_id (per instruction do this within subscription path)
        let chain_id_u64 = self.http.get_chain_id().await? as i64;

        // Extract enriched metadata from Log
        let block_hash = log.block_hash;
        let block_number = log.block_number.map(|n| i64::try_from(n).unwrap_or_default());
        let block_timestamp = log.block_timestamp.map(|ts| ts as i64);
        let tx_hash = log.transaction_hash;
        let tx_index = log.transaction_index.map(|i| i as i32);
        let log_index = log.log_index.map(|i| i as i32);
        let removed = log.removed.unwrap_or(false);

        // Retrieve tx sender using transaction hash
        let tx_sender = if let Some(h) = tx_hash {
            match self.http.get_transaction_by_hash(h).await? {
                Some(tx) => Some(tx.from()),
                None => None,
            }
        } else { None };

        // Prepare raw payloads for DB and NATS
        let topics_json = serde_json::Value::Array(
            log.topics.iter().map(|t| json!(format!("0x{}", hex::encode(*t)))).collect()
        );
        let data: Bytes = log.data.unwrap_or_default();

        // Insert into Postgres
        self.db
            .insert_log(
                chain_id_u64,
                contract,
                event_name.as_deref(),
                event_hash,
                topics_json.clone(),
                data.clone(),
                block_hash,
                block_number,
                block_timestamp,
                tx_hash,
                tx_index,
                log_index,
                removed,
                tx_sender,
                log_hash,
                decoded_params.clone(),
            )
            .await?;

        // Publish to NATS if configured
        if let Some(nats) = &self.nats {
            let payload = json!({
                "chain_id": chain_id_u64,
                "contract_name": contract.map(|c| c.name.clone()),
                "contract_address": format!("0x{}", hex::encode(addr)),
                "event_name": event_name,
                "event_hash": event_hash.map(|h| format!("0x{}", hex::encode(h)) ),
                "topics": topics_json,
                "data": format!("0x{}", hex::encode(&data)),
                "block_hash": block_hash.map(|h| format!("0x{}", hex::encode(h)) ),
                "block_number": block_number,
                "block_timestamp": block_timestamp,
                "transaction_hash": tx_hash.map(|h| format!("0x{}", hex::encode(h)) ),
                "transaction_index": tx_index,
                "log_index": log_index,
                "removed": removed,
                "tx_sender": tx_sender.map(|a| a.to_string()),
                "log_hash": format!("0x{}", hex::encode(log_hash)),
                "decoded_params": decoded_params,
            });
            // subject suffix per-contract for easy routing
            let suffix = contract.map(|c| c.name.as_str()).unwrap_or("unknown");
            nats.publish_json(&format!("{}.events", suffix), &payload).await?;
        }

        Ok(())
    }
}

/// Build HTTP and WS providers using Alloy
pub async fn build_providers(http_url: &str, ws_url: &str) -> anyhow::Result<(RootProvider, RootProvider)> {
    let http = ProviderBuilder::new().on_http(http_url.parse()?)?;
    let ws = ProviderBuilder::new().on_ws(ws_url).await?;
    Ok((http, ws))
}
