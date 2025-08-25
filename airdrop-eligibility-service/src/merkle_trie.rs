use anyhow::Result;
use alloy_primitives::{B256, Address, U256, keccak256};
use memory_db::MemoryDB;
use trie_db::{TrieDBMut, TrieDB, Trie, TrieMut};
use keccak_hasher::KeccakHasher;
use rlp::RlpStream;
use std::collections::HashMap;

pub type KeccakTrieDBMut<'a> = TrieDBMut<'a, KeccakHasher>;
pub type KeccakTrieDB<'a> = TrieDB<'a, KeccakHasher>;
pub type KeccakMemoryDB = MemoryDB<KeccakHasher>;

#[derive(Debug, Clone)]
pub struct MerklePatriciaTrie {
    db: KeccakMemoryDB,
    root: B256,
}

impl MerklePatriciaTrie {
    pub fn new() -> Self {
        let db = KeccakMemoryDB::default();
        let root = B256::ZERO;
        Self { db, root }
    }

    pub fn from_data(db: KeccakMemoryDB, root: B256) -> Self {
        Self { db, root }
    }

    pub fn update_eligibility_data(&mut self, eligibility_data: HashMap<Address, U256>) -> Result<()> {
        let mut trie = KeccakTrieDBMut::new(&mut self.db, &mut self.root.0)?;

        for (address, amount) in eligibility_data {
            let key = keccak256(address.as_slice());
            let value = self.encode_value(amount)?;
            trie.insert(&key, &value)?;
        }

        Ok(())
    }

    pub fn get_root_hash(&self) -> B256 {
        self.root
    }

    pub fn get_value(&self, address: &Address) -> Result<Option<U256>> {
        let trie = KeccakTrieDB::new(&self.db, &self.root.0)?;
        let key = keccak256(address.as_slice());

        if let Some(value_bytes) = trie.get(&key)? {
            let value = self.decode_value(&value_bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        // Serialize root hash
        data.extend_from_slice(self.root.as_slice());

        // Serialize database
        let db_data = bincode::serialize(&self.db.drain())?;
        data.extend_from_slice(&db_data);

        Ok(data)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(anyhow::anyhow!("Invalid trie data: too short"));
        }

        let root = B256::from_slice(&data[0..32]);
        let db_data = &data[32..];

        let db_items: Vec<_> = bincode::deserialize(db_data)?;
        let mut db = KeccakMemoryDB::default();
        for (key, (value, ref_count)) in db_items {
            db.insert(key, value, ref_count);
        }

        Ok(Self { db, root })
    }

    fn encode_value(&self, amount: U256) -> Result<Vec<u8>> {
        let mut stream = RlpStream::new();
        stream.append(&amount);
        Ok(stream.out().to_vec())
    }

    fn decode_value(&self, data: &[u8]) -> Result<U256> {
        let rlp = rlp::Rlp::new(data);
        Ok(rlp.as_val()?)
    }

    pub fn compute_merkle_proof(&self, address: &Address) -> Result<Vec<Vec<u8>>> {
        let trie = KeccakTrieDB::new(&self.db, &self.root.0)?;
        let key = keccak256(address.as_slice());

        // Generate merkle proof
        let proof = trie.get_proof(&key)?;
        Ok(proof)
    }
}

impl Default for MerklePatriciaTrie {
    fn default() -> Self {
        Self::new()
    }
}
