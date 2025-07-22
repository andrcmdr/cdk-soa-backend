use std::collections::HashMap;
use tokio_postgres::Client;

use crate::types::EventPayload;

pub async fn insert_event(
    client: &Client,
    payload: &EventPayload,
) -> anyhow::Result<()> {
    let query = r#"
        INSERT INTO contract_events (
            contract_name,
            contract_address,
            block_number,
            transaction_hash,
            event_name,
            params
        ) VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    let params_json = serde_json::to_value(&payload.params)?;

    client
        .execute(
            query,
            &[
                &payload.contract_name,
                &payload.contract_address,
                &payload.block_number,
                &payload.transaction_hash,
                &payload.event_name,
                &params_json,
            ],
        )
        .await?;

    Ok(())
}
