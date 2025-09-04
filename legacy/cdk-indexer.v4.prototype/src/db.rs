use crate::abi::ContractAbi;
use alloy::primitives::{Address, B256, Bytes};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use tokio_postgres::{Client, NoTls};

#[derive(Clone)]
pub struct Db {
    client: Client,
}

impl Db {
    pub async fn connect(pg_url: &str) -> anyhow::Result<Self> {
        let (client, connection) = tokio_postgres::connect(pg_url, NoTls).await?;
        // Spawn the connection driver
        tokio::spawn(async move {
            if let Err(e) = connection.await { eprintln!("postgres connection error: {e}"); }
        });
        Ok(Self { client })
    }

    pub async fn init_schema(&self, schema_sql: &str) -> anyhow::Result<()> {
        self.client.batch_execute(schema_sql).await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    pub async fn insert_log(
        &self,
        chain_id: i64,
        contract: Option<&ContractAbi>,
        event_name: Option<&str>,
        event_hash: Option<B256>,
        topics_json: serde_json::Value,
        data: Bytes,
        block_hash: Option<B256>,
        block_number: Option<i64>,
        block_timestamp: Option<i64>, // seconds since epoch
        tx_hash: Option<B256>,
        tx_index: Option<i32>,
        log_index: Option<i32>,
        removed: bool,
        tx_sender: Option<Address>,
        log_hash: [u8; 32],
        decoded_params: Option<Value>,
    ) -> anyhow::Result<()> {
        let contract_name = contract.map(|c| c.name.as_str());
        let contract_address = contract.map(|c| c.address);

        let ts: Option<DateTime<Utc>> = block_timestamp.map(|s| Utc.timestamp_opt(s as i64, 0).single()).flatten();

        self.client
            .execute(
                r#"
                INSERT INTO logs (
                    log_hash, chain_id, contract_name, contract_address,
                    event_name, event_hash, topics, data,
                    block_hash, block_number, block_timestamp,
                    transaction_hash, transaction_index, log_index, removed,
                    tx_sender, decoded_params
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8,
                    $9, $10, $11,
                    $12, $13, $14, $15,
                    $16, $17
                ) ON CONFLICT (log_hash) DO NOTHING
                "#,
                &[
                    &log_hash.as_slice(),
                    &chain_id,
                    &contract_name,
                    &contract_address.map(|a| a.as_slice().to_vec()),
                    &event_name,
                    &event_hash.map(|h| h.as_slice().to_vec()),
                    &topics_json,
                    &data.as_ref(),
                    &block_hash.map(|h| h.as_slice().to_vec()),
                    &block_number,
                    &ts,
                    &tx_hash.map(|h| h.as_slice().to_vec()),
                    &tx_index,
                    &log_index,
                    &removed,
                    &tx_sender.map(|a| a.as_slice().to_vec()),
                    &decoded_params,
                ],
            )
            .await?;
        Ok(())
    }
}
