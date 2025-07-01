use sqlx::{Pool, Postgres};
use serde_json::Value;

pub async fn insert_event(
    pool: &Pool<Postgres>,
    contract: &str,
    event_name: &str,
    params: &Value,
) {
    let _ = sqlx::query(
        "INSERT INTO events (contract_address, event_name, parameters) VALUES ($1, $2, $3)"
    )
    .bind(contract)
    .bind(event_name)
    .bind(params)
    .execute(pool)
    .await;
}
