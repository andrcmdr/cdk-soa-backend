use anyhow::Result;
use tokio_postgres::{Client, NoTls};
use tracing::{info, error};
use crate::models::{RevenueReport, UsageReport};

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

    pub async fn insert_revenue_report(&self, report: &RevenueReport) -> Result<()> {
        let query = r#"
        INSERT INTO revenue_reports (
            artifact_address,
            revenue,
            timestamp
        ) VALUES ($1, $2, $3)
    "#;

    self.client
        .execute(
            query,
            &[
                &report.artifact_address,
                &report.revenue,
                &report.timestamp,
            ],
        ).await?;

        Ok(())
    }

    pub async fn insert_usage_report(&self, report: &UsageReport) -> Result<()> {
        let query = r#"
        INSERT INTO usage_reports (
            artifact_address,
            usage,
            timestamp
        ) VALUES ($1, $2, $3)
    "#;

    self.client
        .execute(
            query,
            &[
                &report.artifact_address,
                &report.usage,
                &report.timestamp,
            ],
        ).await?;

        Ok(())
    }

    // Gets unsubmitted revenue reports from the database
    pub async fn get_unsubmitted_revenue_reports(&self, limit: i32) -> Result<Vec<RevenueReport>> {
        let query = r#"
            SELECT artifact_address, revenue, timestamp
            FROM revenue_reports
            WHERE submitted_to_chain = FALSE
            ORDER BY timestamp ASC
            LIMIT $1
        "#;

        let rows = self.client.query(query, &[&limit]).await?;
        
        let mut reports = Vec::new();
        for row in rows {
            let report = RevenueReport {
                artifact_address: row.get(0),
                revenue: row.get(1),
                timestamp: row.get(2),
            };
            reports.push(report);
        }

        Ok(reports)
    }

    // Gets unsubmitted usage reports from the database
    pub async fn get_unsubmitted_usage_reports(&self, limit: i32) -> Result<Vec<UsageReport>> {
        let query = r#"
            SELECT artifact_address, usage, timestamp
            FROM usage_reports
            WHERE submitted_to_chain = FALSE
            ORDER BY timestamp ASC
            LIMIT $1
        "#;

        let rows = self.client.query(query, &[&limit]).await?;
        
        let mut reports = Vec::new();
        for row in rows {
            let report = UsageReport {
                artifact_address: row.get(0),
                usage: row.get(1),
                timestamp: row.get(2),
            };
            reports.push(report);
        }

        Ok(reports)
    }

}
