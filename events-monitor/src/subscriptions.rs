use std::collections::BTreeMap;
use futures_util::StreamExt;
use tracing::{info, error, debug};

use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{Filter, FilterBlockOption, BlockNumberOrTag, Log as RpcLog},
    primitives::Address,
    json_abi::JsonAbi,
};
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy::providers::{Identity, RootProvider};
use alloy::consensus::Transaction;
use alloy::network::TransactionResponse;

use tokio_postgres::Client as DbClient;
use async_nats::jetstream::object_store::ObjectStore;

use crate::{abi::ContractAbi, db, nats, nats::Nats};
use crate::config::AppCfg as AppConfig;
use crate::event_decoder::EventDecoder;
use crate::types::EventPayload;

use std::ops::{Range, RangeFrom};
use std::str::FromStr;
use std::sync::Arc;
use anyhow::anyhow;

type RPCProvider = FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider>;

pub struct EventProcessor {
    addr_abi_map: BTreeMap<Address, ContractAbi>,
    db_pool: DbClient,
    nats_store: Option<Nats>,
    config: AppConfig,
    ws_rpc_provider: RPCProvider,
    http_rpc_provider: RPCProvider,
    chain_id: u64,
}

impl EventProcessor {
    pub async fn new(config: &AppConfig, db_pool: DbClient, nats_store: Option<Nats>) -> anyhow::Result<Self> {
        // ABIs
        let mut contracts = Vec::with_capacity(config.contracts.len());
        for c in config.contracts.iter() {
            let abi = ContractAbi::load(&c.name, &c.address, &c.abi_path)?;
            contracts.push(abi);
        }

        info!("Loaded contracts: {:?}", contracts.len());

        // index contracts by address for a quick lookup
        use std::collections::BTreeMap;
        let mut addr_abi_map: BTreeMap<Address, ContractAbi> = BTreeMap::new();
        for c in contracts { addr_abi_map.insert(c.address, c); }

        let ws = WsConnect::new(&config.chain.ws_rpc_url);
        let http_rpc = reqwest::Url::from_str(&config.chain.http_rpc_url)?;
        let (ws_rpc_provider, http_rpc_provider) = build_providers(ws, http_rpc).await?;

        let chain_id = http_rpc_provider.get_chain_id().await?;
        if chain_id != config.chain.chain_id {
            anyhow::bail!("Chain ID mismatch: expected {}, got {}", config.chain.chain_id, chain_id);
        }
        info!("Chain ID: {}", chain_id);

        Ok(Self {
            addr_abi_map,
            db_pool,
            nats_store,
            config: config.clone(),
            ws_rpc_provider,
            http_rpc_provider,
            chain_id,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let provider = self.ws_rpc_provider.clone();

        let from_block = self.config.indexing.from_block.unwrap_or(0u64);
        let to_block = self.config.indexing.to_block;

        // build a single filter for all addresses
        let addresses: Vec<Address> = self.addr_abi_map.iter().map(|(_addr, c)| c.address).collect();

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

        // Grab logs from all contracts according to filer and subscribe to new ones
        let logs = self.http_rpc_provider.get_logs(&filter).await?;
        debug!("Received {} logs from {} contracts", logs.len(), addresses.len());
        logs.iter().for_each(|log| {
            debug!("Received log from contract: {}", log.address());
            debug!("Log: {:?}", log);
        });

        // Grab logs from all contracts according to filer and subscribe to new ones
        let logs = self.ws_rpc_provider.get_logs(&filter).await?;
        debug!("Received {} logs from {} contracts", logs.len(), addresses.len());
        logs.iter().for_each(|log| {
            debug!("Received log from contract: {}", log.address());
            debug!("Log: {:?}", log);
        });

        let sub = provider.subscribe_logs(&filter).await?;
        info!("Subscribed to logs for {} contracts", addresses.len());
        let mut sub_stream = sub.into_stream();
        info!("Subscribed to logs for {} contracts", addresses.len());
        while let Some(log) = sub_stream.next().await {
            debug!("Received log from contract: {}", log.address());
            if let Err(e) = self.handle_log(log).await {
                error!("Failed to handle log: {:?}", e);
                eprintln!("Log error: {:?}", e);
            }
        }
        Ok(())
    }

    async fn handle_log(
        &self,
        log: RpcLog,
    ) -> anyhow::Result<()> {
        let addr = log.address();
        debug!("Received log from contract: {}", addr);
        let Some(contract) = self.addr_abi_map.get(&addr) else { return Ok(()); };

        let abi = Arc::new(contract.abi.clone());
        let decoder = EventDecoder::new(abi)?;
        let parsed_event = decoder.decode_log(&log.inner)?;
        let parsed_event_value = parsed_event.to_json()?;

        let contract_name = contract.name.as_str();
        let contract_address = contract.address.to_string();
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

        // Compute unique log hash using the Log's `hash()` with SHA3-256 hasher
        let mut hasher = Sha3_256StdHasher::default();
        log.inner.hash(&mut hasher);
        let log_hash_bytes = hasher.finalize_bytes();
        let log_hash = format!("0x{}", hex::encode(log_hash_bytes));

        let payload = EventPayload {
            contract_name: contract_name.to_string(),
            contract_address,
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

        // Persist to Postgres
        db::insert_event(&self.db_pool, &payload).await?;
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
