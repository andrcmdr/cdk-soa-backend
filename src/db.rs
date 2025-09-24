use anyhow::Result;
use tokio_postgres::{Client, NoTls};
use tracing::{info, error};
use crate::types::BackendData;

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

    pub async fn insert_backend_data(&self, data: &BackendData) -> Result<()> {
        let usage_query = r#"
        INSERT INTO usage_reports (
            artifact_address,
            usage,
            timestamp
        ) VALUES ($1, $2, $3)
        ON CONFLICT (artifact_address, timestamp) DO NOTHING
    "#;
        self.client.execute(usage_query, &[&data.artifact_address, &data.usage, &data.timestamp]).await?;
        let revenue_query = r#"
        INSERT INTO revenue_reports (
            artifact_address,
            revenue,
            timestamp
        ) VALUES ($1, $2, $3)
        ON CONFLICT (artifact_address, timestamp) DO NOTHING
    "#;
        self.client.execute(revenue_query, &[&data.artifact_address, &data.revenue, &data.timestamp]).await?;
        Ok(())
    }

    // Gets unsubmitted revenue reports from the database
    pub async fn get_unsubmitted_revenue_reports(&self, limit: i32) -> Result<Vec<tokio_postgres::Row>> {
        let query = r#"
            SELECT id, artifact_address, revenue, timestamp
            FROM revenue_reports
            WHERE submitted_to_chain = FALSE
            ORDER BY timestamp ASC
            LIMIT $1
        "#;

        let rows = self.client.query(query, &[&(limit as i64)]).await?;
        Ok(rows)
    }

    // Gets unsubmitted usage reports from the database
    pub async fn get_unsubmitted_usage_reports(&self, limit: i32) -> Result<Vec<tokio_postgres::Row>> {
        let query = r#"
            SELECT id, artifact_address, usage, timestamp
            FROM usage_reports
            WHERE submitted_to_chain = FALSE
            ORDER BY timestamp ASC
            LIMIT $1
        "#;

        let rows = self.client.query(query, &[&(limit as i64)]).await?;
        Ok(rows)
    }

    pub async fn update_revenue_report_submitted_to_chain(&self, id: Vec<i32>) -> Result<()> {
        let query = r#"
            UPDATE revenue_reports
            SET submitted_to_chain = TRUE
            WHERE id = ANY($1)
        "#;
        self.client.execute(query, &[&id]).await?;
        Ok(())
    }

    pub async fn update_usage_report_submitted_to_chain(&self, id: Vec<i32>) -> Result<()> {
        let query = r#"
            UPDATE usage_reports
            SET submitted_to_chain = TRUE
            WHERE id = ANY($1)
        "#;
        self.client.execute(query, &[&id]).await?;
        Ok(())
    }

    /// Mark usage reports as submitted to blockchain
    pub async fn mark_usage_reports_submitted(&self, ids: Vec<i32>) -> Result<()> {
        self.update_usage_report_submitted_to_chain(ids).await
    }

    /// Mark revenue reports as submitted to blockchain
    pub async fn mark_revenue_reports_submitted(&self, ids: Vec<i32>) -> Result<()> {
        self.update_revenue_report_submitted_to_chain(ids).await
    }

}
