use anyhow::Result;
use tokio_postgres::{Client, NoTls};
use tracing::{info, error};

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn new(db_url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;
        
        // Spawn the connection to run it in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres error: {}", e);
            }
        });

        info!("Database connection established");
        Ok(Database { client })
    }

    pub async fn test_connection(&self) -> Result<()> {
        let row = self.client.query_one("SELECT 1", &[]).await?;
        let value: i32 = row.get(0);
        info!("Database connection test successful: {}", value);
        Ok(())
    }

    pub async fn get_revenue_reports_count(&self) -> Result<i64> {
        let row = self.client.query_one("SELECT COUNT(*) FROM revenue_reports", &[]).await?;
        let count: i64 = row.get(0);
        Ok(count)
    }

    pub async fn get_usage_reports_count(&self) -> Result<i64> {
        let row = self.client.query_one("SELECT COUNT(*) FROM usage_reports", &[]).await?;
        let count: i64 = row.get(0);
        Ok(count)
    }
}
