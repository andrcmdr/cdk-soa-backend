use tokio_postgres::{Client, NoTls};
use tracing::{info, error};

use crate::types::EventPayload;

pub async fn connect_pg(dsn: &str, schema: &str) -> anyhow::Result<Client> {
    let (client, connection) = tokio_postgres::connect(dsn, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await { error!("Postgres connection error: {:?}", e); }
    });

    // Create schema if not exists
    client.batch_execute(schema).await?;

    info!("Postgres ready");

    Ok(client)
}

pub async fn insert_event(
    client: &Client,
    payload: &EventPayload,
) -> anyhow::Result<()> {
    let query = r#"
        INSERT INTO contract_events (
            contract_name,
            contract_address,
            block_number,
            block_hash,
            block_timestamp,
            block_time,
            transaction_hash,
            transaction_index,
            log_index,
            event_name,
            event_signature,
            event_data
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::jsonb)
    "#;

    let event_data_jsonb = serde_json::to_string_pretty(&payload.event_data)?;

    client
        .execute(
            query,
            &[
                &payload.contract_name,
                &payload.contract_address,
                &payload.block_number,
                &payload.block_hash,
                &payload.block_timestamp,
                &payload.block_time,
                &payload.transaction_hash,
                &payload.transaction_index,
                &payload.log_index,
                &payload.event_name,
                &payload.event_signature,
                &event_data_jsonb,
            ],
        )
        .await?;

    Ok(())
}
