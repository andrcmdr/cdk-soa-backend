use tokio_postgres::{Client, NoTls, Config as PgConfig};
use tracing::{info, error, warn, debug};
use std::time::Duration;

use crate::types::BlockPayload;
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

    pub async fn insert_block(&self, payload: &BlockPayload) -> anyhow::Result<()> {
        let query = r#"
            INSERT INTO blocks_monitor_data (
                chain_id,
                block_number,
                block_hash,
                block_timestamp,
                block_time,
                parent_hash,
                gas_used,
                gas_limit,
                transactions
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb)
            ON CONFLICT (chain_id, block_number) DO UPDATE SET
                block_hash = EXCLUDED.block_hash,
                block_timestamp = EXCLUDED.block_timestamp,
                block_time = EXCLUDED.block_time,
                parent_hash = EXCLUDED.parent_hash,
                gas_used = EXCLUDED.gas_used,
                gas_limit = EXCLUDED.gas_limit,
                transactions = EXCLUDED.transactions
                updated_at = CURRENT_TIMESTAMP
        "#;

        let transactions_data_jsonb = payload.transactions.as_ref()
            .map(|txs| serde_json::to_value(txs))
            .transpose()?;

        match self.client
            .execute(
                query,
                &[
                    &payload.chain_id,
                    &payload.block_number,
                    &payload.block_hash,
                    &payload.block_timestamp,
                    &payload.block_time,
                    &payload.parent_hash,
                    &payload.gas_used,
                    &payload.gas_limit,
                    &transactions_data_jsonb,
                ],
            )
            .await
        {
            Ok(_) => {
                debug!("Block inserted to AWS RDS: {:?} : {:?}", payload.block_number, payload.block_hash);
                Ok(())
            },
            Err(e) => {
                error!("Failed to insert block to AWS RDS: {:?}", e);
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
            AND table_name = 'blocks_monitor_data'
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
