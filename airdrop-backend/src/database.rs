use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls};
use alloy_primitives::{B256, Address, U256};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrieState {
    pub round_id: u32,
    pub root_hash: B256,
    pub trie_data: Vec<u8>,
    pub entry_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityRecord {
    pub id: Option<i32>,
    pub address: Address,
    pub amount: U256,
    pub round_id: u32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingLog {
    pub id: i32,
    pub round_id: u32,
    pub operation: String,
    pub status: String,
    pub message: Option<String>,
    pub transaction_hash: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct Database {
    client: Client,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        let db = Database { client };
        db.init_tables().await?;
        Ok(db)
    }

    async fn init_tables(&self) -> Result<()> {
        // Trie states table - main storage for trie data
        self.client.execute(
            "CREATE TABLE IF NOT EXISTS trie_states (
                round_id INTEGER PRIMARY KEY,
                root_hash BYTEA NOT NULL,
                trie_data BYTEA NOT NULL,
                entry_count INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )",
            &[],
        ).await?;

        // Eligibility records table - individual user eligibility data
        self.client.execute(
            "CREATE TABLE IF NOT EXISTS eligibility_records (
                id SERIAL PRIMARY KEY,
                address BYTEA NOT NULL,
                amount NUMERIC NOT NULL,
                round_id INTEGER NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(address, round_id),
                FOREIGN KEY (round_id) REFERENCES trie_states(round_id) ON DELETE CASCADE
            )",
            &[],
        ).await?;

        // Processing logs table - audit trail
        self.client.execute(
            "CREATE TABLE IF NOT EXISTS processing_logs (
                id SERIAL PRIMARY KEY,
                round_id INTEGER NOT NULL,
                operation VARCHAR(50) NOT NULL,
                status VARCHAR(20) NOT NULL,
                message TEXT,
                transaction_hash VARCHAR(66),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )",
            &[],
        ).await?;

        // Indexes for better performance
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_eligibility_round_id ON eligibility_records(round_id)",
            &[],
        ).await?;

        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_eligibility_address ON eligibility_records(address)",
            &[],
        ).await?;

        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_processing_logs_round_id ON processing_logs(round_id)",
            &[],
        ).await?;

        Ok(())
    }

    pub async fn save_trie_state(&self, trie_state: &TrieState) -> Result<()> {
        self.client.execute(
            "INSERT INTO trie_states (round_id, root_hash, trie_data, entry_count, updated_at)
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT (round_id)
             DO UPDATE SET
                root_hash = $2,
                trie_data = $3,
                entry_count = $4,
                updated_at = NOW()",
            &[
                &(trie_state.round_id as i32),
                &trie_state.root_hash.as_slice(),
                &trie_state.trie_data,
                &trie_state.entry_count,
            ],
        ).await?;
        Ok(())
    }

    pub async fn get_trie_state(&self, round_id: u32) -> Result<Option<TrieState>> {
        let row = self.client.query_opt(
            "SELECT round_id, root_hash, trie_data, entry_count, created_at, updated_at
             FROM trie_states WHERE round_id = $1",
            &[&(round_id as i32)],
        ).await?;

        if let Some(row) = row {
            let root_hash_bytes: &[u8] = row.get(1);
            let root_hash = B256::from_slice(root_hash_bytes);

            Ok(Some(TrieState {
                round_id: row.get::<_, i32>(0) as u32,
                root_hash,
                trie_data: row.get(2),
                entry_count: row.get(3),
                created_at: row.get(4),
                updated_at: row.get(5),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_trie_states(&self) -> Result<Vec<TrieState>> {
        let rows = self.client.query(
            "SELECT round_id, root_hash, trie_data, entry_count, created_at, updated_at
             FROM trie_states ORDER BY round_id",
            &[],
        ).await?;

        let mut trie_states = Vec::new();
        for row in rows {
            let root_hash_bytes: &[u8] = row.get(1);
            let root_hash = B256::from_slice(root_hash_bytes);

            trie_states.push(TrieState {
                round_id: row.get::<_, i32>(0) as u32,
                root_hash,
                trie_data: row.get(2),
                entry_count: row.get(3),
                created_at: row.get(4),
                updated_at: row.get(5),
            });
        }

        Ok(trie_states)
    }

    pub async fn save_eligibility_records(&self, records: &[EligibilityRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut query = String::from(
            "INSERT INTO eligibility_records (address, amount, round_id) VALUES "
        );

        // Create storage vectors for the data columns
        let mut address_vecs: Vec<Vec<u8>> = Vec::with_capacity(records.len());
        let mut amount_strs: Vec<String> = Vec::with_capacity(records.len());
        let mut round_ids: Vec<i32> = Vec::with_capacity(records.len());

        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut placeholders = Vec::new();

        for (i, record) in records.iter().enumerate() {
            let base_idx = i * 3 + 1;
            placeholders.push(format!("(${}, ${}, ${})", base_idx, base_idx + 1, base_idx + 2));

            // Store the data
            address_vecs.push(record.address.to_vec());
            amount_strs.push(record.amount.to_string());
            round_ids.push(record.round_id as i32);
        }

        // Build params by referencing the stored data
        for i in 0..records.len() {
            params.push(&address_vecs[i]);
            params.push(&amount_strs[i]);
            params.push(&round_ids[i]);
        }

        query.push_str(&placeholders.join(", "));
        query.push_str(" ON CONFLICT (address, round_id) DO UPDATE SET amount = EXCLUDED.amount");

        self.client.execute(&query, &params).await?;
        Ok(())
    }

    pub async fn get_eligibility_records(&self, round_id: u32) -> Result<Vec<EligibilityRecord>> {
        let rows = self.client.query(
            "SELECT id, address, amount, round_id, created_at
             FROM eligibility_records WHERE round_id = $1 ORDER BY id",
            &[&(round_id as i32)],
        ).await?;

        let mut records = Vec::new();
        for row in rows {
            let address_bytes: &[u8] = row.get(1);
            let address = Address::from_slice(address_bytes);
            let amount_str: String = row.get(2);
            let amount = U256::from_str(&amount_str)?;

            records.push(EligibilityRecord {
                id: Some(row.get(0)),
                address,
                amount,
                round_id: row.get::<_, i32>(3) as u32,
                created_at: Some(row.get(4)),
            });
        }

        Ok(records)
    }

    pub async fn get_user_eligibility(&self, round_id: u32, address: &Address) -> Result<Option<U256>> {
        let row = self.client.query_opt(
            "SELECT amount FROM eligibility_records WHERE round_id = $1 AND address = $2",
            &[&(round_id as i32), &address.as_slice()],
        ).await?;

        if let Some(row) = row {
            let amount_str: String = row.get(0);
            let amount = U256::from_str(&amount_str)?;
            Ok(Some(amount))
        } else {
            Ok(None)
        }
    }

    pub async fn log_processing_operation(&self, log: &ProcessingLog) -> Result<i32> {
        let row = self.client.query_one(
            "INSERT INTO processing_logs (round_id, operation, status, message, transaction_hash)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
            &[
                &(log.round_id as i32),
                &log.operation,
                &log.status,
                &log.message,
                &log.transaction_hash,
            ],
        ).await?;

        Ok(row.get(0))
    }

    pub async fn get_processing_logs(&self, round_id: Option<u32>) -> Result<Vec<ProcessingLog>> {
        let query: &str;
        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;

        // Create a binding for the converted i32 so it lives for the entire scope
        let round_id_i32;

        if let Some(rid) = round_id {
            round_id_i32 = rid as i32;
            query = "SELECT id, round_id, operation, status, message, transaction_hash, created_at
                     FROM processing_logs WHERE round_id = $1 ORDER BY created_at DESC";
            params = vec![&round_id_i32];
        } else {
            query = "SELECT id, round_id, operation, status, message, transaction_hash, created_at
                     FROM processing_logs ORDER BY created_at DESC LIMIT 100";
            params = vec![];
        }

        let rows = self.client.query(query, &params).await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(ProcessingLog {
                id: row.get(0),
                round_id: row.get::<_, i32>(1) as u32,
                operation: row.get(2),
                status: row.get(3),
                message: row.get(4),
                transaction_hash: row.get(5),
                created_at: row.get(6),
            });
        }

        Ok(logs)
    }

    pub async fn update_processing_log_status(&self, log_id: i32, status: &str, message: Option<&str>) -> Result<()> {
        self.client.execute(
            "UPDATE processing_logs SET status = $1, message = $2 WHERE id = $3",
            &[&status, &message, &log_id],
        ).await?;
        Ok(())
    }

    pub async fn delete_round_data(&self, round_id: u32) -> Result<()> {
        // Delete eligibility records first (due to foreign key)
        self.client.execute(
            "DELETE FROM eligibility_records WHERE round_id = $1",
            &[&(round_id as i32)],
        ).await?;

        // Delete trie state
        self.client.execute(
            "DELETE FROM trie_states WHERE round_id = $1",
            &[&(round_id as i32)],
        ).await?;

        tracing::info!("Deleted all data for round {}", round_id);
        Ok(())
    }

    pub async fn get_round_statistics(&self) -> Result<Vec<(u32, i32, chrono::DateTime<chrono::Utc>)>> {
        let rows = self.client.query(
            "SELECT round_id, entry_count, updated_at FROM trie_states ORDER BY round_id",
            &[],
        ).await?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push((
                row.get::<_, i32>(0) as u32,
                row.get(1),
                row.get(2),
            ));
        }

        Ok(stats)
    }
}
