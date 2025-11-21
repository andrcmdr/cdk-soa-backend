use tokio_postgres::{Client, NoTls};
use tracing::{info, error};
use crate::config::AwsRdsCfg;
use crate::types::{EventPayload, BlockPayload};

pub struct AwsRdsClient {
    client: Client,
}

impl AwsRdsClient {
    pub async fn insert_event(&self, payload: &EventPayload) -> anyhow::Result<()> {
        crate::db::insert_event(&self.client, payload).await
    }

    pub async fn insert_block(&self, payload: &BlockPayload) -> anyhow::Result<()> {
        crate::db::insert_block(&self.client, payload).await
    }

    pub async fn test_connection(&self) -> anyhow::Result<()> {
        self.client.execute("SELECT 1", &[]).await?;
        Ok(())
    }
}

pub async fn create_aws_rds_client(config: &AwsRdsCfg) -> anyhow::Result<AwsRdsClient> {
    let ssl_mode = config.ssl_mode.as_deref().unwrap_or("require");
    let port = config.port.unwrap_or(5432);

    let connection_string = format!(
        "host={} port={} dbname={} user={} password={} sslmode={}",
        config.endpoint,
        port,
        config.database_name,
        config.username,
        config.password,
        ssl_mode
    );

    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("AWS RDS connection error: {:?}", e);
        }
    });

    // Create schema if provided
    if let Some(schema_path) = &config.schema {
        let schema = std::fs::read_to_string(schema_path)?;
        client.batch_execute(&schema).await?;
    }

    info!("AWS RDS PostgreSQL ready");

    Ok(AwsRdsClient { client })
}
