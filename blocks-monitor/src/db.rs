use tokio_postgres::{Client, NoTls};
use tracing::{info, error, warn};

use crate::types::BlockPayload;
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

    pub async fn insert_block(&self, payload: &BlockPayload) -> anyhow::Result<()> {
        // Always insert to local PostgreSQL first
        if let Err(e) = insert_block(&self.local_pg, payload).await {
            error!("Failed to insert block to local PostgreSQL: {:?}", e);
            return Err(e);
        }

        // Optionally insert to AWS RDS
        if let Some(aws_rds) = &self.aws_rds {
            if let Err(e) = aws_rds.insert_block(payload).await {
                // Log error but don't fail the entire operation
                error!("Failed to insert block to AWS RDS (non-critical): {:?}", e);
                warn!("Block was saved to local PostgreSQL but failed to replicate to AWS RDS");
            } else {
                info!("Block successfully replicated to AWS RDS: {}", payload.block_hash);
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

pub async fn insert_block(
    client: &Client,
    payload: &BlockPayload,
) -> anyhow::Result<()> {
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
    "#;

    let transactions_jsonb = payload.transactions.as_ref()
        .map(|txs| serde_json::to_value(txs))
        .transpose()?;

    client
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
                &transactions_jsonb,
            ],
        )
        .await?;

    info!("Block inserted to local PostgreSQL");

    Ok(())
}
