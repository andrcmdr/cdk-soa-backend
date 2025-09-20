use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls};
use alloy_primitives::{B256, Address, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrieState {
    pub round_id: u32,
    pub root_hash: B256,
    pub trie_data: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityRecord {
    pub address: Address,
    pub amount: U256,
    pub round_id: u32,
}

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
        self.client.execute(
            "CREATE TABLE IF NOT EXISTS trie_states (
                round_id INTEGER PRIMARY KEY,
                root_hash BYTEA NOT NULL,
                trie_data BYTEA NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )",
            &[],
        ).await?;

        self.client.execute(
            "CREATE TABLE IF NOT EXISTS eligibility_records (
                id SERIAL PRIMARY KEY,
                address BYTEA NOT NULL,
                amount NUMERIC NOT NULL,
                round_id INTEGER NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(address, round_id)
            )",
            &[],
        ).await?;

        Ok(())
    }

    pub async fn save_trie_state(&self, trie_state: &TrieState) -> Result<()> {
        self.client.execute(
            "INSERT INTO trie_states (round_id, root_hash, trie_data, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (round_id)
             DO UPDATE SET root_hash = $2, trie_data = $3, updated_at = NOW()",
            &[
                &(trie_state.round_id as i32),
                &trie_state.root_hash.as_slice(),
                &trie_state.trie_data,
            ],
        ).await?;
        Ok(())
    }

    pub async fn get_trie_state(&self, round_id: u32) -> Result<Option<TrieState>> {
        let row = self.client.query_opt(
            "SELECT round_id, root_hash, trie_data, created_at, updated_at
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
                created_at: row.get(3),
                updated_at: row.get(4),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn save_eligibility_records(&self, records: &[EligibilityRecord]) -> Result<()> {
        let mut params = Vec::new();
        let mut placeholders = Vec::new();

        for (i, record) in records.iter().enumerate() {
            let base_idx = i * 3 + 1;
            placeholders.push(format!("(${}, ${}, ${})", base_idx, base_idx + 1, base_idx + 2));
            params.push(&record.address.as_slice() as &(dyn tokio_postgres::types::ToSql + Sync));

            let amount_str = record.amount.to_string();
            params.push(&amount_str as &(dyn tokio_postgres::types::ToSql + Sync));
            params.push(&(record.round_id as i32) as &(dyn tokio_postgres::types::ToSql + Sync));
        }

        if !placeholders.is_empty() {
            let query = format!(
                "INSERT INTO eligibility_records (address, amount, round_id)
                 VALUES {} ON CONFLICT (address, round_id)
                 DO UPDATE SET amount = EXCLUDED.amount",
                placeholders.join(", ")
            );

            self.client.execute(&query, &params).await?;
        }

        Ok(())
    }
}
