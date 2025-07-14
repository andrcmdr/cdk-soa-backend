use alloy::providers::Provider;
use alloy::providers::Ws;
use alloy::rpc::types::Log;
use alloy::sol_types::Address;
use alloy_json_abi::{AbiItem, JsonAbi};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::db::insert_event;
use crate::config::{AppConfig, ContractConfig};

type SharedAbi = Arc<JsonAbi>;

pub struct EventProcessor {
    provider: Provider<Ws>,
    abi_map: HashMap<Address, SharedAbi>,
    contract_names: HashMap<Address, String>,
    db_pool: tokio_postgres::Client,
}

impl EventProcessor {
    pub async fn new(config: &AppConfig, db_pool: tokio_postgres::Client) -> anyhow::Result<Self> {
        let ws = Ws::connect(&config.ws_provider).await?;
        let provider = Provider::new(ws);

        let mut abi_map = HashMap::new();
        let mut contract_names = HashMap::new();

        for (name, ContractConfig { address, abi_path }) in &config.contracts {
            let addr = address.parse::<Address>()?;
            let abi_json = std::fs::read_to_string(abi_path)?;
            let abi: JsonAbi = serde_json::from_str(&abi_json)?;
            abi_map.insert(addr, Arc::new(abi));
            contract_names.insert(addr, name.clone());
        }

        Ok(Self {
            provider,
            abi_map,
            contract_names,
            db_pool,
        })
    }

    pub async fn process_logs(&self) -> anyhow::Result<()> {
        let filter = alloy::rpc::types::Filter::new().event(None);
        let mut sub = self.provider.subscribe_logs(&filter).await?;

        while let Some(log) = sub.next().await {
            if let Ok(event) = log {
                if let Err(err) = self.handle_log(event).await {
                    tracing::warn!("Failed to handle log: {:?}", err);
                }
            }
        }

        Ok(())
    }

    async fn handle_log(&self, log: Log) -> anyhow::Result<()> {
        let address = log.address;
        let abi = match self.abi_map.get(&address) {
            Some(abi) => abi.clone(),
            None => return Ok(()),
        };

        let contract_name = self.contract_names.get(&address).cloned().unwrap_or_default();
        let topics = log.topics.iter().map(|t| t.0).collect::<Vec<_>>();

        let decoded = abi.decode_log(&topics, log.data.0.clone())?;

        let mut params = HashMap::new();
        for (name, value) in decoded.params.iter() {
            params.insert(name.clone(), format!("{:?}", value));
        }

        insert_event(
            &self.db_pool,
            &contract_name,
            address.to_string(),
            &log.transaction_hash.map(|h| format!("{:?}", h)).unwrap_or_default(),
            log.block_number.unwrap_or_default().as_u64() as i64,
            params,
        )
        .await?;

        info!("Inserted event from {}", address);
        Ok(())
    }
}
