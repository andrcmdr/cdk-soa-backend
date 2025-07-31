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
            event_data
        ) VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    let event_data_jsonb = serde_json::to_string_pretty(&payload.event_data)?;

    client
        .execute(
            query,
            &[
                &payload.contract_name,
                &payload.contract_address,
                &payload.block_number,
                &payload.transaction_hash,
                &payload.event_name,
                &event_data_jsonb,
            ],
        )
        .await?;

    Ok(())
}
