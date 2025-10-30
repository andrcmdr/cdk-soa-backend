use tokio_postgres::{Client, NoTls};
use tracing::{info, error, warn};

use crate::types::EventPayload;
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

    pub async fn insert_event(&self, payload: &EventPayload) -> anyhow::Result<()> {
        // Always insert to local PostgreSQL first
        if let Err(e) = insert_event(&self.local_pg, payload).await {
            error!("Failed to insert event to local PostgreSQL: {:?}", e);
            return Err(e);
        }

        // Optionally insert to AWS RDS
        if let Some(aws_rds) = &self.aws_rds {
            if let Err(e) = aws_rds.insert_event(payload).await {
                // Log error but don't fail the entire operation
                // AWS RDS is an additional data availability layer
                error!("Failed to insert event to AWS RDS (non-critical): {:?}", e);
                warn!("Event was saved to local PostgreSQL but failed to replicate to AWS RDS");
            } else {
                info!("Event successfully replicated to AWS RDS: {}", payload.log_hash);
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

pub async fn insert_event(
    client: &Client,
    payload: &EventPayload,
) -> anyhow::Result<()> {
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
    "#;

    // let event_data_jsonb = serde_json::to_string_pretty(&payload.event_data)?;
    let event_data_jsonb = serde_json::to_value(&payload.event_data)?;

    client
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
        .await?;

    info!("Event inserted to local PostgreSQL");

    Ok(())
}
