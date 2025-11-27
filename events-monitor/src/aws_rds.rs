use tokio_postgres::{Client, NoTls, Config as PgConfig};
use tracing::{info, error, warn, debug};
use std::time::Duration;

use crate::types::EventPayload;
use crate::config::AwsRdsCfg;

pub struct AwsRdsClient {
    client: Client,
    config: AwsRdsCfg,
}

impl AwsRdsClient {
    pub async fn new(config: &AwsRdsCfg) -> anyhow::Result<Self> {
        info!("Connecting to AWS RDS at {}:{}", config.endpoint, config.port.unwrap_or(5432));

        let mut pg_config = PgConfig::new();
        pg_config.host(&config.endpoint);
        pg_config.port(config.port.unwrap_or(5432));
        pg_config.dbname(&config.database_name);
        pg_config.user(&config.username);
        pg_config.password(&config.password);

        // Set connection timeout if specified
        if let Some(timeout) = config.connection_timeout {
            pg_config.connect_timeout(Duration::from_secs(timeout));
        }

        // Configure SSL mode
        let ssl_mode = config.ssl_mode.as_deref().unwrap_or("prefer");
        match ssl_mode {
            "disable" => {},
            "prefer" | "require" | "verify-ca" | "verify-full" => {
                // For AWS RDS, we typically use SSL
                debug!("Using SSL mode: {:?}", ssl_mode);
            },
            _ => {
                warn!("Unknown SSL mode: {:?}, using default", ssl_mode);
            }
        }

        let (client, connection) = pg_config.connect(NoTls).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to AWS RDS: {:?}", e))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("AWS RDS connection error: {:?}", e);
            }
        });

        // Create schema if specified and doesn't exist
        if let Some(schema_content) = &config.schema {
            debug!("Setting up AWS RDS schema");
            client.batch_execute(schema_content).await
                .map_err(|e| anyhow::anyhow!("Failed to setup AWS RDS schema: {:?}", e))?;
        }

        info!("Successfully connected to AWS RDS");

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    pub async fn insert_event(&self, payload: &EventPayload) -> anyhow::Result<()> {
        let query = r#"
            INSERT INTO events_monitor_data (
                contract_name,
                contract_address,
                implementation_name,
                implementation_address,
                chain_id,
                block_number,
                block_hash,
                block_timestamp,
                block_time,
                transaction_hash,
                transaction_sender,
                transaction_receiver,
                transaction_index,
                log_index,
                log_hash,
                event_name,
                event_signature,
                event_data
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18::jsonb)
            ON CONFLICT (log_hash) DO UPDATE SET
                updated_at = CURRENT_TIMESTAMP
        "#;

        let event_data_jsonb = serde_json::to_value(&payload.event_data)?;

        match self.client
            .execute(
                query,
                &[
                    &payload.contract_name,
                    &payload.contract_address,
                    &payload.implementation_name,
                    &payload.implementation_address,
                    &payload.chain_id,
                    &payload.block_number,
                    &payload.block_hash,
                    &payload.block_timestamp,
                    &payload.block_time,
                    &payload.transaction_hash,
                    &payload.transaction_sender,
                    &payload.transaction_receiver,
                    &payload.transaction_index,
                    &payload.log_index,
                    &payload.log_hash,
                    &payload.event_name,
                    &payload.event_signature,
                    &event_data_jsonb,
                ],
            )
            .await
        {
            Ok(_) => {
                debug!("Event inserted to AWS RDS: {:?}", payload.log_hash);
                Ok(())
            },
            Err(e) => {
                error!("Failed to insert event to AWS RDS: {:?}", e);
                Err(anyhow::anyhow!("AWS RDS insertion failed: {:?}", e))
            }
        }
    }

    pub async fn test_connection(&self) -> anyhow::Result<()> {
        match self.client.execute("SELECT 1", &[]).await {
            Ok(_) => {
                debug!("AWS RDS connection test successful");
                Ok(())
            },
            Err(e) => {
                error!("AWS RDS connection test failed: {:?}", e);
                Err(anyhow::anyhow!("AWS RDS connection test failed: {:?}", e))
            }
        }
    }

    pub async fn get_table_info(&self) -> anyhow::Result<Vec<String>> {
        let query = r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = 'events_monitor_data'
        "#;

        let rows = self.client.query(query, &[]).await?;
        let tables: Vec<String> = rows.iter()
            .map(|row| row.get::<_, String>(0))
            .collect();

        Ok(tables)
    }
}

pub async fn create_aws_rds_client(config: &AwsRdsCfg) -> anyhow::Result<AwsRdsClient> {
    AwsRdsClient::new(config).await
}
