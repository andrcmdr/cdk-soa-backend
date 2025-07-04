use sqlx::{Pool, Postgres};
use serde_json::Value;

pub async fn init_db(db_url: &str) -> Pool<Postgres> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Failed to connect to DB")
}

pub async fn insert_event(
    pool: &Pool<Postgres>,
    contract_address: &str,
    event_name: &str,
    params: &Value,
) {
    let _ = sqlx::query(
        "INSERT INTO events (contract_address, event_name, parameters) VALUES ($1, $2, $3)",
    )
    .bind(contract_address)
    .bind(event_name)
    .bind(params)
    .execute(pool)
    .await;
}
