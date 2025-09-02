use alloy::providers::WsConnect;
use alloy::providers::ProviderBuilder;
use alloy_provider::{Provider, RootProvider};
use alloy::pubsub::PubSubFrontend;
use alloy::rpc::types::eth::{Log, Filter, FilterBlockOption};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::json_abi::JsonAbi;

use std::collections::HashMap;
use std::ops::{Range, RangeFrom};
use std::str::FromStr;
use std::sync::Arc;
use futures::{Stream, StreamExt};

use crate::config::AppConfig;
use crate::types::EventPayload;
use crate::{db, nats};
use crate::event_decoder::EventDecoder;

use tokio_postgres::Client as DbClient;
use async_nats::jetstream::object_store::ObjectStore;

pub struct EventProcessor {
    provider: RootProvider<PubSubFrontend>,
    abi_map: HashMap<Address, Arc<JsonAbi>>,
    contract_names: HashMap<Address, String>,
    db_pool: DbClient,
    nats_store: ObjectStore,
    config: AppConfig,
}

impl EventProcessor {
    pub async fn new(config: &AppConfig, db_pool: DbClient, nats_store: ObjectStore) -> anyhow::Result<Self> {
        let ws = WsConnect::new(&config.ws_provider);
        let provider = ProviderBuilder::new().on_ws(ws).await?;

        let mut abi_map = HashMap::new();
        let mut contract_names = HashMap::new();

        for (name, c) in &config.contracts {
            let addr = Address::from_str(&c.address)?;
            let abi_json = std::fs::read_to_string(&c.abi_path)?;
            // Preprocess the JSON to add missing anonymous fields
            let preprocessed_json = EventDecoder::preprocess_abi_json_from_str(&abi_json)?;
            // Safely deserialize with JsonAbi
            let abi: JsonAbi = serde_json::from_str(&preprocessed_json)?;
            abi_map.insert(addr, Arc::new(abi));
            contract_names.insert(addr, name.clone());
        }

        Ok(Self {
            provider,
            abi_map,
            contract_names,
            db_pool,
            nats_store,
            config: config.clone(),
        })
    }

    pub async fn process_logs(&self) -> anyhow::Result<()> {
        let from_block = self.config.indexing.from_block.unwrap_or(0u64);
        let to_block = self.config.indexing.to_block;

        let mut filter = Filter::new().select(0u64..);
        if let Some(to_block) = to_block {
            filter = Filter::new().select(BlockRange(from_block..to_block));
        } else {
            filter = Filter::new().select(BlockRangeFrom(from_block..));
        }

        let sub = self.provider.subscribe_logs(&filter).await?;
        let mut sub_stream = sub.into_stream();

        while let Some(log) = sub_stream.next().await {
            if let Err(err) = self.handle_log(log).await {
                tracing::error!("Failed to handle log: {:?}", err);
                eprintln!("Log error: {:?}", err);
            }
        }

        Ok(())
    }

    async fn handle_log(&self, log: Log) -> anyhow::Result<()> {
        let address = log.address();
        let abi = match self.abi_map.get(&address) {
            Some(abi) => abi.clone(),
            None => return Ok(()),
        };

        let decoder = EventDecoder::new(abi)?;
        let parsed_event = decoder.decode_log(&log.inner)?;
        let parsed_event_value = parsed_event.to_json()?;
        let event_name = parsed_event.name.as_str();

        let contract_name = self.contract_names.get(&address).cloned().unwrap_or_default();
        let contract_name_str = contract_name.as_str();
        let block_hash = log.block_hash.unwrap_or_default().to_string();
        let block_ts = log.block_timestamp.unwrap_or_default().to_string();
        let tx_index = log.transaction_index.unwrap_or_default().to_string();
        let log_index = log.log_index.unwrap_or_default().to_string();
        let event_signature = parsed_event.signature.unwrap_or_default().to_string();

        let payload = EventPayload {
            contract_name: contract_name_str.to_string(),
            contract_address: address.to_string(),
            block_number: log.block_number.unwrap_or_default().to_string(),
            block_hash,
            block_timestamp: block_ts,
            transaction_hash: log.transaction_hash.unwrap_or_default().to_string(),
            transaction_index: tx_index,
            log_index,
            event_name: event_name.to_string(),
            event_signature,
            event_data: parsed_event_value,
        };

        db::insert_event(&self.db_pool, &payload).await?;
        nats::publish_event(&self.nats_store, &payload).await?;

        tracing::info!("Inserted event '{}' from contract '{}'", event_name, contract_name_str);

        Ok(())
    }
}

/// Range from/to block type conversion helpers
pub struct BlockRange(pub Range<u64>);
impl From<BlockRange> for FilterBlockOption {
    fn from(value: BlockRange) -> Self {
        FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(value.0.start)),
            to_block: Some(BlockNumberOrTag::Number(value.0.end)),
        }
    }
}

/// Range from.. block type conversion helpers
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
