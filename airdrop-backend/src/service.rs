use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, error};
use alloy_primitives::{Address, B256, U256};

use crate::config::Config;
use crate::database::{Database, TrieState, EligibilityRecord, ProcessingLog};
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
    config_path: String,
}

impl AirdropService {
    pub async fn new(mut config: Config, config_path: String) -> AppResult<Self> {
        // Initialize KMS encryption
        let encryption = KmsEnvelopeEncryption::new(&config.aws.region, config.aws.kms_key_id.clone())
            .await
            .map_err(|e| AppError::Encryption(format!("KMS initialization failed: {}", e)))?;

        // Handle private key - get existing or create new
        let private_key = if config.needs_key_generation() {
            info!("No encrypted private key found in config, generating new one...");

            let encrypted_key = encryption.generate_and_encrypt_private_key()
                .await
                .map_err(|e| AppError::Encryption(format!("Failed to generate private key: {}", e)))?;

            // Save the encrypted key to config
            config.set_encrypted_private_key(encrypted_key.clone());
            config.save_to_file(&config_path)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save config: {}", e)))?;

            info!("Generated new private key and saved encrypted version to config");

            // Decrypt for use
            encryption.decrypt_private_key(&encrypted_key)
                .await
                .map_err(|e| AppError::Encryption(format!("Failed to decrypt newly generated key: {}", e)))?
        } else {
            info!("Decrypting existing private key from config");
            encryption.decrypt_private_key(&config.wallet.encrypted_private_key)
                .await
                .map_err(|e| AppError::Encryption(format!("Failed to decrypt private key: {}", e)))?
        };

        // Load contract ABI
        let abi = config.load_contract_abi()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to load contract ABI: {}", e)))?;

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
            abi,
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
            config_path,
        };

        // Load existing tries from database (primary source)
        service.load_tries_from_database().await?;

        Ok(service)
    }

    async fn load_tries_from_database(&self) -> AppResult<()> {
        let trie_states = self.database.get_all_trie_states().await?;
        let mut tries = self.tries.write().await;

        for trie_state in trie_states {
            let trie = MerklePatriciaTrie::deserialize(&trie_state.trie_data)
                .map_err(|e| AppError::Internal(e))?;
            tries.insert(trie_state.round_id, trie);
            info!("Loaded trie for round {} from database", trie_state.round_id);
        }

        info!("Loaded {} tries from database", tries.len());
        Ok(())
    }

    pub async fn process_csv_data(&self, csv_data: &[u8], round_id: u32) -> AppResult<()> {
        info!("Processing CSV data for round {}", round_id);

        // Log processing start
        let log_id = self.database.log_processing_operation(&ProcessingLog {
            id: 0,
            round_id,
            operation: "csv_processing".to_string(),
            status: "started".to_string(),
            message: Some("Processing CSV data".to_string()),
            transaction_hash: None,
            created_at: chrono::Utc::now(),
        }).await?;

        // Store CSV in NATS for audit trail and data availability
        let csv_object_name = self.nats_storage.store_csv_data(round_id, csv_data).await
            .map_err(|e| {
                tokio::spawn({
                    let db = self.database.clone();
                    async move {
                        let _ = db.update_processing_log_status(log_id, "failed", Some(&format!("Failed to store CSV: {}", e))).await;
                    }
                });
                e
            })?;
        info!("Stored CSV data as object: {}", csv_object_name);

        // Process CSV data
        let eligibility_data = CsvProcessor::process_csv_bytes(csv_data)
            .map_err(|e| {
                tokio::spawn({
                    let db = self.database.clone();
                    async move {
                        let _ = db.update_processing_log_status(log_id, "failed", Some(&format!("CSV processing failed: {}", e))).await;
                    }
                });
                e
            })?;

        CsvProcessor::validate_csv_data(&eligibility_data)
            .map_err(|e| {
                tokio::spawn({
                    let db = self.database.clone();
                    async move {
                        let _ = db.update_processing_log_status(log_id, "failed", Some(&format!("CSV validation failed: {}", e))).await;
                    }
                });
                e
            })?;

        info!("Processed {} eligibility records", eligibility_data.len());

        // Update or create trie for this round
        let mut trie = self.get_or_create_trie(round_id).await?;
        trie.update_eligibility_data(eligibility_data.clone())
            .map_err(|e| AppError::Internal(e))?;

        // Store updated trie in memory
        {
            let mut tries = self.tries.write().await;
            tries.insert(round_id, trie.clone());
        }

        // Save to database (primary storage)
        let trie_state = TrieState {
            round_id,
            root_hash: trie.get_root_hash(),
            trie_data: trie.serialize().map_err(|e| AppError::Internal(e))?,
            entry_count: eligibility_data.len() as i32,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.database.save_trie_state(&trie_state).await?;

        // Save eligibility records to database
        let records: Vec<EligibilityRecord> = eligibility_data
            .iter()
            .map(|(address, amount)| EligibilityRecord {
                id: None,
                address: *address,
                amount: *amount,
                round_id,
                created_at: None,
            })
            .collect();

        self.database.save_eligibility_records(&records).await?;

        // Also backup to NATS object storage for data availability
        let stored_data = StoredTrieData {
            round_id,
            root_hash: hex::encode(trie.get_root_hash()),
            trie_data: trie_state.trie_data.clone(),
            metadata: TrieMetadata {
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
                entry_count: eligibility_data.len(),
            },
        };

        self.nats_storage.store_trie_data(round_id, &stored_data).await?;

        // Update processing log
        self.database.update_processing_log_status(
            log_id,
            "completed",
            Some(&format!("Processed {} records with root hash: 0x{}",
                         eligibility_data.len(),
                         hex::encode(trie.get_root_hash())))
        ).await?;

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

        // Load from database (primary storage)
        if let Some(trie_state) = self.database.get_trie_state(round_id).await? {
            let trie = MerklePatriciaTrie::deserialize(&trie_state.trie_data)
                .map_err(|e| AppError::Internal(e))?;
            return Ok(trie);
        }

        // Create new trie if not found
        Ok(MerklePatriciaTrie::new())
    }

    pub async fn submit_trie_update(&self, round_id: u32) -> AppResult<B256> {
        // Log submission start
        let log_id = self.database.log_processing_operation(&ProcessingLog {
            id: 0,
            round_id,
            operation: "blockchain_submission".to_string(),
            status: "started".to_string(),
            message: Some("Submitting trie to blockchain".to_string()),
            transaction_hash: None,
            created_at: chrono::Utc::now(),
        }).await?;

        let trie = self.get_or_create_trie(round_id).await?;
        let root_hash = trie.get_root_hash();

        // Check if root hash already exists on-chain
        if self.contract_client.is_root_hash_exists(root_hash).await? {
            warn!("Root hash 0x{} already exists on-chain for round {}",
                  hex::encode(root_hash), round_id);

            self.database.update_processing_log_status(
                log_id,
                "skipped",
                Some("Root hash already exists on-chain")
            ).await?;

            return Err(AppError::InvalidInput(format!("Root hash already exists for round {}", round_id)));
        }

        // Submit to smart contract
        let trie_data = trie.serialize().map_err(|e| AppError::Internal(e))?;
        let tx_hash = self.contract_client
            .submit_trie_update(round_id, root_hash, trie_data)
            .await
            .map_err(|e| {
                tokio::spawn({
                    let db = self.database.clone();
                    async move {
                        let _ = db.update_processing_log_status(log_id, "failed", Some(&format!("Blockchain submission failed: {}", e))).await;
                    }
                });
                e
            })?;

        // Update processing log with success
        self.database.update_processing_log_status(
            log_id,
            "completed",
            Some(&format!("Successfully submitted with transaction: 0x{}", hex::encode(tx_hash)))
        ).await?;

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
        // Try database first (faster lookup)
        if let Some(amount) = self.database.get_user_eligibility(round_id, &address).await? {
            return Ok(Some(amount));
        }

        // Fallback to trie lookup
        let trie = self.get_or_create_trie(round_id).await?;
        trie.get_value(&address).map_err(|e| AppError::Internal(e))
    }

    pub async fn get_trie_info(&self, round_id: u32) -> AppResult<Option<TrieState>> {
        self.database.get_trie_state(round_id).await.map_err(AppError::Database)
    }

    pub async fn get_all_round_statistics(&self) -> AppResult<Vec<(u32, i32, chrono::DateTime<chrono::Utc>)>> {
        self.database.get_round_statistics().await.map_err(AppError::Database)
    }

    pub async fn get_processing_logs(&self, round_id: Option<u32>) -> AppResult<Vec<ProcessingLog>> {
        self.database.get_processing_logs(round_id).await.map_err(AppError::Database)
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

    pub async fn delete_round(&self, round_id: u32) -> AppResult<()> {
        // Remove from memory
        {
            let mut tries = self.tries.write().await;
            tries.remove(&round_id);
        }

        // Delete from database
        self.database.delete_round_data(round_id).await?;

        // Delete from NATS (optional, for cleanup)
        if let Err(e) = self.nats_storage.delete_trie_data(round_id).await {
            warn!("Failed to delete NATS data for round {}: {}", round_id, e);
        }

        info!("Deleted all data for round {}", round_id);
        Ok(())
    }
}
