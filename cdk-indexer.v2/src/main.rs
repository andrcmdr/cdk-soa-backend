mod config;
mod db;
mod event_processor;

use config::AppConfig;
use event_processor::EventProcessor;
use tokio_postgres::NoTls;
use tracing_subscriber;

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

    let processor = EventProcessor::new(&config, client).await?;
    processor.process_logs().await?;

    Ok(())
}
