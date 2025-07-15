use std::collections::HashMap;
use tokio_postgres::Client;

pub async fn insert_event(
    client: &Client,
    contract_name: &str,
    contract_address: String,
    tx_hash: &str,
    block_number: i64,
    params: HashMap<String, String>,
) -> anyhow::Result<()> {
    let query = r#"
        INSERT INTO contract_events (
            contract_name,
            contract_address,
            transaction_hash,
            block_number,
            params
        ) VALUES ($1, $2, $3, $4, $5)
    "#;

    let params_json = serde_json::to_value(&params)?;

    client
        .execute(query, &[&contract_name, &contract_address, &tx_hash, &block_number, &params_json])
        .await?;

    Ok(())
}
