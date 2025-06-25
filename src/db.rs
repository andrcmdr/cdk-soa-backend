use sqlx::postgres::PgPoolOptions;

pub async fn init_db(db_url: &str) -> sqlx::Pool<sqlx::Postgres> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Failed to connect to DB")
}

pub async fn insert_event(pool: &sqlx::Pool<sqlx::Postgres>, setter: &str, value: &str) {
    let _ = sqlx::query("INSERT INTO events (setter, value) VALUES ($1, $2)")
        .bind(setter)
        .bind(value)
        .execute(pool)
        .await;
}
