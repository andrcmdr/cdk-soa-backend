mod config;
mod event_processor;
mod event_decoder;
mod types;
mod db;
mod nats;

use tokio_postgres::NoTls;
use config::AppConfig;
use event_processor::EventProcessor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load_from_file("config.toml")?;

    let (client, connection) = tokio_postgres::connect(&config.db_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    let nats_store = nats::init_nats(&config.nats_url, &config.nats_bucket).await?;

    let processor = EventProcessor::new(&config, client, nats_store).await?;
    processor.process_logs().await?;

    Ok(())
}
