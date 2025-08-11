use crate::types::AirdropEntry;
use trie_db::{Trie, TrieMut, TrieDBMut, DBValue};
use hash_db::Hasher;
use keccak_hasher::KeccakHasher;
use memory_db::{MemoryDB, HashKey};
use rlp::RlpStream;
use alloy_primitives::{Address, B256};

pub struct TrieResult {
    pub root_hash: B256,
    pub trie_nodes: Vec<u8>, // RLP-encoded trie root
}

pub fn build_trie(entries: &[AirdropEntry]) -> TrieResult {
    let mut db = MemoryDB::<KeccakHasher, HashKey<_>, DBValue>::default();
    let mut root = Default::default();
    let mut trie = TrieDBMut::<KeccakHasher>::new(&mut db, &mut root);

    for entry in entries {
        let key = entry.address.as_slice();
        let mut value_stream = RlpStream::new_list(3);
        value_stream.append(&entry.amount);
        value_stream.append(&entry.round);
        value_stream.append(&entry.address); // optional redundancy
        trie.insert(key, &value_stream.out()).unwrap();
    }

    let root_hash = B256::from_slice(trie.root().as_bytes());
    let trie_encoded = rlp::encode(trie.root());

    TrieResult {
        root_hash,
        trie_nodes: trie_encoded.to_vec(),
    }
}
