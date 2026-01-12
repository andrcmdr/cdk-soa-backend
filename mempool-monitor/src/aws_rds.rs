use tokio_postgres::{Client, NoTls, Config as PgConfig};
use tracing::{info, error, warn, debug};
use std::time::Duration;

use crate::types::TransactionPayload;
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

    pub async fn insert_transaction(
        &self,
        payload: &TransactionPayload,
    ) -> anyhow::Result<()> {
        let query = r#"
            INSERT INTO mempool_monitor_data (
                chain_id,
                transaction_hash,
                transaction_sender,
                transaction_receiver,
                nonce,
                value,
                gas_limit,
                gas_price,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                input_data,
                transaction_type,
                timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (chain_id, transaction_hash) DO UPDATE SET
                transaction_sender = EXCLUDED.transaction_sender,
                transaction_receiver = EXCLUDED.transaction_receiver,
                nonce = EXCLUDED.nonce,
                value = EXCLUDED.value,
                gas_limit = EXCLUDED.gas_limit,
                gas_price = EXCLUDED.gas_price,
                max_fee_per_gas = EXCLUDED.max_fee_per_gas,
                max_priority_fee_per_gas = EXCLUDED.max_priority_fee_per_gas,
                input_data = EXCLUDED.input_data,
                transaction_type = EXCLUDED.transaction_type,
                timestamp = EXCLUDED.timestamp
        "#;

        match self.client
            .execute(
                query,
                &[
                    &payload.chain_id,
                    &payload.transaction_hash,
                    &payload.transaction_sender,
                    &payload.transaction_receiver,
                    &payload.nonce,
                    &payload.value,
                    &payload.gas_limit,
                    &payload.gas_price,
                    &payload.max_fee_per_gas,
                    &payload.max_priority_fee_per_gas,
                    &payload.input_data,
                    &payload.transaction_type,
                    &payload.timestamp,
                ],
            )
            .await
        {
            Ok(_) => {
                debug!("Transaction inserted to AWS RDS: {:?}", payload.transaction_hash);
                Ok(())
            },
            Err(e) => {
                error!("Failed to insert transaction to AWS RDS: {:?}", e);
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
