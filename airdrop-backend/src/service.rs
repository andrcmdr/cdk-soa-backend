use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, error};
use alloy_primitives::{Address, B256, U256};

use crate::config::Config;
use crate::database::{Database, TrieState, EligibilityRecord};
use crate::merkle_trie::MerklePatriciaTrie;
use crate::csv_processor::CsvProcessor;
use crate::contract_client::ContractClient;
use crate::encryption::KmsEnvelopeEncryption;
use crate::nats_storage::{NatsObjectStorage, StoredTrieData, TrieMetadata};
use crate::error::{AppError, AppResult};

pub struct AirdropService {
    database: Database,
    contract_client: ContractClient,
    nats_storage: NatsObjectStorage,
    encryption: KmsEnvelopeEncryption,
    tries: tokio::sync::RwLock<HashMap<u32, MerklePatriciaTrie>>,
}

impl AirdropService {
    pub async fn new(config: Config) -> AppResult<Self> {
        // Initialize KMS encryption
        let encryption = KmsEnvelopeEncryption::new(&config.aws.region, config.aws.kms_key_id)
            .await
            .map_err(|e| AppError::Encryption(format!("KMS initialization failed: {}", e)))?;

        // Decrypt private key
        let private_key = encryption
            .decrypt_private_key(&config.wallet.encrypted_private_key)
            .await
            .map_err(|e| AppError::Encryption(format!("Failed to decrypt private key: {}", e)))?;

        // Initialize components
        let database = Database::new(&config.database.url)
            .await
            .map_err(AppError::Database)?;

        let contract_address = config.blockchain.contract_address.parse()
            .map_err(|e| AppError::InvalidInput(format!("Invalid contract address: {}", e)))?;

        let contract_client = ContractClient::new(
            &config.blockchain.rpc_url,
            contract_address,
            &private_key,
        ).await?;

        let nats_storage = NatsObjectStorage::new(
            &config.nats.url,
            config.nats.object_store.bucket_name.clone(),
        )
        .await
        .map_err(AppError::Nats)?;

        let mut service = Self {
            database,
            contract_client,
            nats_storage,
            encryption,
            tries: tokio::sync::RwLock::new(HashMap::new()),
        };

        // Load existing tries from storage
        service.load_tries_from_storage().await?;

        Ok(service)
    }

    async fn load_tries_from_storage(&self) -> AppResult<()> {
        let object_names = self.nats_storage.list_trie_objects().await?;
        let mut tries = self.tries.write().await;

        for object_name in object_names {
            if let Some(round_id_str) = object_name.strip_prefix("trie_round_") {
                if let Ok(round_id) = round_id_str.parse::<u32>() {
                    if let Some(stored_data) = self.nats_storage.get_trie_data(round_id).await? {
                        let trie = MerklePatriciaTrie::deserialize(&stored_data.trie_data)
                            .map_err(|e| AppError::Internal(e))?;
                        tries.insert(round_id, trie);
                        info!("Loaded trie for round {} from NATS storage", round_id);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn process_csv_data(&self, csv_data: &[u8], round_id: u32) -> AppResult<()> {
        info!("Processing CSV data for round {}", round_id);

        // Store CSV in NATS for audit trail
        let csv_object_name = self.nats_storage.store_csv_data(round_id, csv_data).await?;
        info!("Stored CSV data as object: {}", csv_object_name);

        // Process CSV data
        let eligibility_data = CsvProcessor::process_csv_bytes(csv_data)?;
        CsvProcessor::validate_csv_data(&eligibility_data)?;

        info!("Processed {} eligibility records", eligibility_data.len());

        // Update or create trie for this round
        let mut trie = self.get_or_create_trie(round_id).await?;
        trie.update_eligibility_data(eligibility_data.clone())
            .map_err(|e| AppError::Internal(e))?;

        // Store updated trie
        {
            let mut tries = self.tries.write().await;
            tries.insert(round_id, trie.clone());
        }

        // Save to NATS object storage
        let stored_data = StoredTrieData {
            round_id,
            root_hash: hex::encode(trie.get_root_hash()),
            trie_data: trie.serialize().map_err(|e| AppError::Internal(e))?,
            metadata: TrieMetadata {
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
                entry_count: eligibility_data.len(),
            },
        };

        self.nats_storage.store_trie_data(round_id, &stored_data).await?;

        // Also save to PostgreSQL for backup
        let trie_state = TrieState {
            round_id,
            root_hash: trie.get_root_hash(),
            trie_data: stored_data.trie_data.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.database.save_trie_state(&trie_state).await?;

        // Save eligibility records
        let records: Vec<EligibilityRecord> = eligibility_data
            .into_iter()
            .map(|(address, amount)| EligibilityRecord {
                address,
                amount,
                round_id,
            })
            .collect();

        self.database.save_eligibility_records(&records).await?;

        info!("Updated trie for round {} with root hash: 0x{}",
              round_id, hex::encode(trie.get_root_hash()));

        Ok(())
    }

    async fn get_or_create_trie(&self, round_id: u32) -> AppResult<MerklePatriciaTrie> {
        // Check in-memory cache first
        {
            let tries = self.tries.read().await;
            if let Some(trie) = tries.get(&round_id) {
                return Ok(trie.clone());
            }
        }

        // Try to load from NATS storage
        if let Some(stored_data) = self.nats_storage.get_trie_data(round_id).await? {
            let trie = MerklePatriciaTrie::deserialize(&stored_data.trie_data)
                .map_err(|e| AppError::Internal(e))?;
            return Ok(trie);
        }

        // Try to load from database as fallback
        if let Some(trie_state) = self.database.get_trie_state(round_id).await? {
            let trie = MerklePatriciaTrie::deserialize(&trie_state.trie_data)
                .map_err(|e| AppError::Internal(e))?;
            return Ok(trie);
        }

        // Create new trie
        Ok(MerklePatriciaTrie::new())
    }

    pub async fn submit_trie_update(&self, round_id: u32) -> AppResult<B256> {
        let trie = self.get_or_create_trie(round_id).await?;
        let root_hash = trie.get_root_hash();

        // Check if root hash already exists on-chain
        if self.contract_client.is_root_hash_exists(root_hash).await? {
            warn!("Root hash 0x{} already exists on-chain for round {}",
                  hex::encode(root_hash), round_id);
            return Err(AppError::InvalidInput(format!("Root hash already exists for round {}", round_id)));
        }

        // Submit to smart contract
        let trie_data = trie.serialize().map_err(|e| AppError::Internal(e))?;
        let tx_hash = self.contract_client
            .submit_trie_update(round_id, root_hash, trie_data)
            .await?;

        info!("Submitted trie update for round {} with transaction: 0x{}",
              round_id, hex::encode(tx_hash));

        Ok(tx_hash)
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256
    ) -> AppResult<bool> {
        let trie = self.get_or_create_trie(round_id).await?;

        // Generate merkle proof
        let proof = trie.compute_merkle_proof(&address)
            .map_err(|e| AppError::Internal(e))?;

        // Verify on-chain
        let is_valid = self.contract_client
            .verify_eligibility(round_id, address, amount, proof)
            .await?;

        info!("Eligibility verification for {} in round {}: {}",
              address, round_id, is_valid);

        Ok(is_valid)
    }

    pub async fn get_eligibility(&self, round_id: u32, address: Address) -> AppResult<Option<U256>> {
        let trie = self.get_or_create_trie(round_id).await?;
        trie.get_value(&address).map_err(|e| AppError::Internal(e))
    }

    pub async fn get_trie_info(&self, round_id: u32) -> AppResult<Option<StoredTrieData>> {
        self.nats_storage.get_trie_data(round_id).await
    }

    pub async fn validate_on_chain_consistency(&self, round_id: u32) -> AppResult<bool> {
        let tries = self.tries.read().await;
        if let Some(local_trie) = tries.get(&round_id) {
            let local_root = local_trie.get_root_hash();
            let on_chain_root = self.contract_client.get_trie_root(round_id).await?;
            Ok(local_root == on_chain_root)
        } else {
            Ok(false)
        }
    }
}
