use anyhow::Result;
use async_nats::jetstream;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use futures::StreamExt;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTrieData {
    pub round_id: u32,
    pub root_hash: String,
    pub trie_data: Vec<u8>,
    pub metadata: TrieMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrieMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: u32,
    pub entry_count: usize,
}

pub struct NatsObjectStorage {
    jetstream: jetstream::Context,
    object_store: jetstream::object_store::ObjectStore,
}

impl NatsObjectStorage {
    pub async fn new(nats_url: &str, bucket_name: String) -> Result<Self> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = jetstream::new(client);

        let object_store = match jetstream.get_object_store(&bucket_name).await {
            Ok(store) => store,
            Err(_) => {
                jetstream
                    .create_object_store(jetstream::object_store::Config {
                        bucket: bucket_name.clone(),
                        description: Some("Airdrop Merkle Trie storage".to_string()),
                        ..Default::default()
                    })
                    .await?
            }
        };

        Ok(Self {
            jetstream,
            object_store,
        })
    }

    pub async fn store_trie_data(&self, round_id: u32, data: &StoredTrieData) -> Result<()> {
        let object_name = format!("trie_round_{}", round_id);
        let serialized = serde_json::to_vec(data)?;
        let mut cursor = Cursor::new(serialized);

        self.object_store
            .put(object_name.as_str(), &mut cursor)
            .await?;

        tracing::info!("Stored trie data for round {} in NATS", round_id);
        Ok(())
    }

    pub async fn get_trie_data(&self, round_id: u32) -> Result<StoredTrieData> {
        let object_name = format!("trie_round_{}", round_id);

        let mut object = self.object_store.get(&object_name).await?;

        let mut data = Vec::new();
        object.read_to_end(&mut data).await?;

        let stored_data: StoredTrieData = serde_json::from_slice(&data)?;
        Ok(stored_data)
    }

    pub async fn delete_trie_data(&self, round_id: u32) -> Result<()> {
        let object_name = format!("trie_round_{}", round_id);
        self.object_store.delete(&object_name).await?;
        tracing::info!("Deleted trie data for round {} from NATS", round_id);
        Ok(())
    }

    pub async fn store_csv_data(&self, round_id: u32, csv_data: &[u8]) -> Result<String> {
        let object_name = format!("csv_round_{}", round_id);
        let mut cursor = Cursor::new(csv_data.to_vec());

        self.object_store
            .put(object_name.as_str(), &mut cursor)
            .await?;

        tracing::info!("Stored CSV data for round {} in NATS", round_id);
        Ok(object_name)
    }

    pub async fn get_csv_data(&self, round_id: u32) -> Result<Vec<u8>> {
        let object_name = format!("csv_round_{}", round_id);

        let mut object = self.object_store.get(&object_name).await?;

        let mut data = Vec::new();
        object.read_to_end(&mut data).await?;

        Ok(data)
    }

    pub async fn list_trie_objects(&self) -> Result<Vec<String>> {
        let mut list = self.object_store.list().await?;
        let mut names = Vec::new();

        while let Some(object) = list.next().await {
            let object = object?;
            if object.name.starts_with("trie_round_") {
                names.push(object.name);
            }
        }

        Ok(names)
    }
}
