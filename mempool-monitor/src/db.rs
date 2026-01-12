use tokio_postgres::{Client, NoTls};
use tracing::{info, error, warn};

use crate::types::TransactionPayload;
use crate::config::AwsRdsCfg;
use crate::aws_rds::{AwsRdsClient, create_aws_rds_client};

pub struct DatabaseClients {
    pub local_pg: Client,
    pub aws_rds: Option<AwsRdsClient>,
}

impl DatabaseClients {
    pub async fn new(
        local_dsn: &str,
        local_schema: &str,
        aws_rds_config: Option<&AwsRdsCfg>
    ) -> anyhow::Result<Self> {
        // Connect to local PostgreSQL
        let local_pg = connect_pg(local_dsn, local_schema).await?;

        // Connect to AWS RDS if enabled
        let aws_rds = if let Some(rds_config) = aws_rds_config {
            if rds_config.enabled.unwrap_or(0) > 0 {
                info!("AWS RDS is enabled, connecting...");
                match create_aws_rds_client(rds_config).await {
                    Ok(client) => {
                        info!("Successfully connected to AWS RDS");
                        Some(client)
                    },
                    Err(e) => {
                        error!("Failed to connect to AWS RDS: {:?}", e);
                        warn!("Continuing without AWS RDS support");
                        None
                    }
                }
            } else {
                info!("AWS RDS is disabled in configuration");
                None
            }
        } else {
            info!("AWS RDS configuration not found");
            None
        };

        Ok(Self {
            local_pg,
            aws_rds,
        })
    }

    pub async fn insert_transaction(&self, payload: &TransactionPayload) -> anyhow::Result<()> {
        // Always insert to local PostgreSQL first
        if let Err(e) = insert_transaction(&self.local_pg, payload).await {
            error!("Failed to insert transaction to local PostgreSQL: {:?}", e);
            return Err(e);
        }

        // Optionally insert to AWS RDS
        if let Some(aws_rds) = &self.aws_rds {
            if let Err(e) = aws_rds.insert_transaction(payload).await {
                // Log error but don't fail the entire operation
                // AWS RDS is an additional data availability layer
                error!("Failed to insert transaction to AWS RDS (non-critical): {:?}", e);
                warn!("Transaction was saved to local PostgreSQL but failed to replicate to AWS RDS");
            } else {
                info!("Transaction successfully replicated to AWS RDS: {}", payload.transaction_hash);
            }
        }

        Ok(())
    }

    pub async fn test_connections(&self) -> anyhow::Result<()> {
        // Test local PostgreSQL
        match self.local_pg.execute("SELECT 1", &[]).await {
            Ok(_) => info!("Local PostgreSQL connection test successful"),
            Err(e) => {
                error!("Local PostgreSQL connection test failed: {:?}", e);
                return Err(anyhow::anyhow!("Local PostgreSQL connection failed: {}", e));
            }
        }

        // Test AWS RDS if available
        if let Some(aws_rds) = &self.aws_rds {
            match aws_rds.test_connection().await {
                Ok(_) => info!("AWS RDS connection test successful"),
                Err(e) => {
                    warn!("AWS RDS connection test failed: {:?}", e);
                    // Don't fail here as AWS RDS is optional
                }
            }
        }

        Ok(())
    }
}

pub async fn connect_pg(dsn: &str, schema: &str) -> anyhow::Result<Client> {
    let (client, connection) = tokio_postgres::connect(dsn, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Postgres connection error: {:?}", e);
        }
    });

    // Create schema if not exists
    client.batch_execute(schema).await?;

    info!("Local PostgreSQL ready");

    Ok(client)
}

pub async fn insert_transaction(
    client: &Client,
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

    client
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
        .await?;

    info!("Transaction inserted to local PostgreSQL");

    Ok(())
}
