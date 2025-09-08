use anyhow::Result;
use async_nats::jetstream::{self, object_store::{ObjectStore, Config as ObjectStoreConfig}};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

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
    object_store: ObjectStore,
    bucket_name: String,
}

impl NatsObjectStorage {
    pub async fn new(nats_url: &str, bucket_name: String) -> Result<Self> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = jetstream::new(client);

        // Create or get object store
        let object_store = jetstream
            .get_or_create_object_store(ObjectStoreConfig {
                bucket: bucket_name.clone(),
                ..Default::default()
            })
            .await?;

        Ok(Self {
            object_store,
            bucket_name,
        })
    }

    pub async fn store_trie_data(&self, round_id: u32, data: &StoredTrieData) -> Result<String> {
        let object_name = format!("trie_round_{}", round_id);
        let serialized_data = bincode::serialize(data)?;

        let object_info = self.object_store
            .put(&object_name, serialized_data.into())
            .await?;

        tracing::info!(
            "Stored trie data for round {} in NATS object store: {}",
            round_id,
            object_info.name
        );

        Ok(object_name)
    }

    pub async fn get_trie_data(&self, round_id: u32) -> Result<Option<StoredTrieData>> {
        let object_name = format!("trie_round_{}", round_id);

        match self.object_store.get(&object_name).await {
            Ok(object) => {
                let data = object.bytes().await?;
                let trie_data: StoredTrieData = bincode::deserialize(&data)?;
                Ok(Some(trie_data))
            }
            Err(async_nats::Error::RequestError { .. }) => {
                // Object not found
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn store_csv_data(&self, round_id: u32, csv_data: &[u8]) -> Result<String> {
        let object_name = format!("csv_round_{}_{}", round_id, Uuid::new_v4());

        let object_info = self.object_store
            .put(&object_name, csv_data.to_vec().into())
            .await?;

        tracing::info!(
            "Stored CSV data for round {} in NATS object store: {}",
            round_id,
            object_info.name
        );

        Ok(object_name)
    }

    pub async fn get_csv_data(&self, object_name: &str) -> Result<Option<Vec<u8>>> {
        match self.object_store.get(object_name).await {
            Ok(object) => {
                let data = object.bytes().await?;
                Ok(Some(data.to_vec()))
            }
            Err(async_nats::Error::RequestError { .. }) => {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn list_trie_objects(&self) -> Result<Vec<String>> {
        let objects = self.object_store.list().await?;
        let mut names = Vec::new();

        while let Some(object) = objects.recv().await {
            if object.name.starts_with("trie_round_") {
                names.push(object.name);
            }
        }

        Ok(names)
    }

    pub async fn delete_trie_data(&self, round_id: u32) -> Result<()> {
        let object_name = format!("trie_round_{}", round_id);
        self.object_store.delete(&object_name).await?;

        tracing::info!("Deleted trie data for round {} from NATS object store", round_id);
        Ok(())
    }
}
