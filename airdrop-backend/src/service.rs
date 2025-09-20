use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, error};
use alloy_primitives::{Address, B256, U256};

use crate::config::Config;
use crate::database::{Database, TrieState, EligibilityRecord};
use crate::merkle_trie::MerklePatriciaTrie;
use crate::csv_processor::CsvProcessor;
use crate::contract_client::ContractClient;

pub struct AirdropService {
    database: Database,
    contract_client: ContractClient,
    tries: HashMap<u32, MerklePatriciaTrie>,
}

impl AirdropService {
    pub async fn new(config: Config) -> Result<Self> {
        let database = Database::new(&config.database_url).await?;
        let contract_address = config.contract_address.parse()?;
        let contract_client = ContractClient::new(
            &config.rpc_url,
            contract_address,
            &config.private_key,
            config.chain_id,
        ).await?;

        let mut service = Self {
            database,
            contract_client,
            tries: HashMap::new(),
        };

        // Load existing tries from database
        service.load_tries_from_database().await?;

        Ok(service)
    }

    async fn load_tries_from_database(&mut self) -> Result<()> {
        // In a real implementation, we'd query all rounds from the database
        // For now, we'll load tries as needed
        Ok(())
    }

    pub async fn process_csv_and_update_trie(&mut self, csv_path: &str, round_id: u32) -> Result<()> {
        info!("Processing CSV file: {} for round {}", csv_path, round_id);

        // Process CSV data
        let eligibility_data = CsvProcessor::process_csv(csv_path)?;
        CsvProcessor::validate_csv_data(&eligibility_data)?;

        info!("Processed {} eligibility records", eligibility_data.len());

        // Update or create trie for this round
        let mut trie = self.get_or_create_trie(round_id).await?;
        trie.update_eligibility_data(eligibility_data.clone())?;

        // Store updated trie
        self.tries.insert(round_id, trie.clone());

        // Save to database
        let trie_state = TrieState {
            round_id,
            root_hash: trie.get_root_hash(),
            trie_data: trie.serialize()?,
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
              round_id, hex::encode(trie_state.root_hash));

        Ok(())
    }

    async fn get_or_create_trie(&mut self, round_id: u32) -> Result<MerklePatriciaTrie> {
        if let Some(trie) = self.tries.get(&round_id) {
            return Ok(trie.clone());
        }

        // Try to load from database
        if let Some(trie_state) = self.database.get_trie_state(round_id).await? {
            let trie = MerklePatriciaTrie::deserialize(&trie_state.trie_data)?;
            return Ok(trie);
        }

        // Create new trie
        Ok(MerklePatriciaTrie::new())
    }

    pub async fn submit_trie_update(&mut self, round_id: u32) -> Result<B256> {
        let trie = self.get_or_create_trie(round_id).await?;
        let root_hash = trie.get_root_hash();

        // Check if root hash already exists on-chain
        if self.contract_client.is_root_hash_exists(root_hash).await? {
            warn!("Root hash 0x{} already exists on-chain for round {}",
                  hex::encode(root_hash), round_id);
            return Ok(B256::ZERO);
        }

        // Submit to smart contract
        let trie_data = trie.serialize()?;
        let tx_hash = self.contract_client
            .submit_trie_update(round_id, root_hash, trie_data)
            .await?;

        info!("Submitted trie update for round {} with transaction: 0x{}",
              round_id, hex::encode(tx_hash));

        Ok(tx_hash)
    }

    pub async fn verify_eligibility(
        &mut self,
        round_id: u32,
        address: Address,
        amount: U256
    ) -> Result<bool> {
        let trie = self.get_or_create_trie(round_id).await?;

        // Generate merkle proof
        let proof = trie.compute_merkle_proof(&address)?;

        // Verify on-chain
        let is_valid = self.contract_client
            .verify_eligibility(round_id, address, amount, proof)
            .await?;

        info!("Eligibility verification for {} in round {}: {}",
              address, round_id, is_valid);

        Ok(is_valid)
    }

    pub async fn get_eligibility(&mut self, round_id: u32, address: Address) -> Result<Option<U256>> {
        let trie = self.get_or_create_trie(round_id).await?;
        trie.get_value(&address)
    }

    pub fn get_trie_root_hash(&self, round_id: u32) -> Option<B256> {
        self.tries.get(&round_id).map(|trie| trie.get_root_hash())
    }

    pub async fn validate_on_chain_consistency(&mut self, round_id: u32) -> Result<bool> {
        if let Some(local_root) = self.get_trie_root_hash(round_id) {
            let on_chain_root = self.contract_client.get_trie_root(round_id).await?;
            Ok(local_root == on_chain_root)
        } else {
            Ok(false)
        }
    }
}
