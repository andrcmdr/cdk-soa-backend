use std::collections::BTreeMap;
use futures_util::StreamExt;
use tracing::{info, error};
use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{Filter, FilterBlockOption, BlockNumberOrTag, Log as RpcLog},
    primitives::Address,
};

use tokio_postgres::Client as DbClient;
use async_nats::jetstream::object_store::ObjectStore;

use crate::{abi::ContractAbi, db, nats, nats::Nats};
use crate::config::AppCfg as AppConfig;
use crate::event_decoder::EventDecoder;
use crate::types::EventPayload;

use std::ops::{Range, RangeFrom};
use std::sync::Arc;
use anyhow::anyhow;

pub struct EventProcessor {
    addr_abi_map: BTreeMap<Address, ContractAbi>,
    db_pool: DbClient,
    nats_store: Nats,
    config: AppConfig,
}

impl EventProcessor {
    pub async fn new(config: &AppConfig, db_pool: DbClient, nats_store: Nats) -> anyhow::Result<Self> {
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

        Ok(Self {
            addr_abi_map,
            db_pool,
            nats_store,
            config: config.clone(),
        })
    }


    pub async fn run(&self) -> anyhow::Result<()> {
        let ws = WsConnect::new(&self.config.chain.ws_rpc_url);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

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

        let sub = provider.subscribe_logs(&filter).await?;
        info!("Subscribed to logs for {} contracts", addresses.len());
        let mut sub_stream = sub.into_stream();

        while let Some(log) = sub_stream.next().await {
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

        let payload = EventPayload {
            contract_name: contract_name.to_string(),
            contract_address,
            block_number,
            block_hash,
            block_timestamp: block_timestamp.to_string(),
            block_time,
            transaction_hash,
            transaction_index: tx_index,
            log_index,
            event_name: event_name.to_string(),
            event_signature,
            event_data: parsed_event_value,
        };

        // Persist to Postgres
        db::insert_event(&self.db_pool, &payload).await?;
        // Persist to NATS Object Store
        nats::publish_event(&self.nats_store.object_store, &payload).await?;

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
