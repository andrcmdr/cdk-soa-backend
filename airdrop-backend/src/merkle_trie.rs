use anyhow::Result;
use alloy_primitives::{B256, Address, U256};
use keccak_hasher::KeccakHasher;
use hash_db::Hasher as HashDbHasher;
use std::collections::BTreeMap;
use serde_json;

/// Keccak256 hash using keccak-hasher
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    KeccakHasher::hash(data)
}

/// Hash a pair of nodes with sorting (lexicographic order)
pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let (first, second) = if left >= right {
        (right, left)
    } else {
        (left, right)
    };

    let mut packed = Vec::with_capacity(64);
    packed.extend_from_slice(first);
    packed.extend_from_slice(second);

    keccak256(&packed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
    pub data: Option<Vec<u8>>,
    pub index: Option<usize>,
}

impl MerkleNode {
    pub fn new_leaf(data: Vec<u8>, index: usize) -> Self {
        let hash = keccak256(&data);
        MerkleNode {
            hash,
            left: None,
            right: None,
            data: Some(data),
            index: Some(index),
        }
    }

    pub fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let hash = hash_pair(&left.hash, &right.hash);
        MerkleNode {
            hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            data: None,
            index: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_data: Vec<u8>,
    pub leaf_hash: [u8; 32],
    pub siblings: Vec<ProofElement>,
}

#[derive(Debug, Clone)]
pub struct ProofElement {
    pub hash: [u8; 32],
    pub is_right_sibling: bool,
}

#[derive(Debug, Clone)]
pub struct MerkleTrie {
    root: Option<MerkleNode>,
    ordered_leaves: Vec<Vec<u8>>,
    leaf_index_map: BTreeMap<Vec<u8>, usize>,
}

impl MerkleTrie {
    pub fn new() -> Self {
        MerkleTrie {
            root: None,
            ordered_leaves: Vec::new(),
            leaf_index_map: BTreeMap::new(),
        }
    }

    /// Generate leaf data from address and amount (viem-compatible encoding)
    fn encode_leaf_data(address: &Address, amount: &U256) -> Vec<u8> {
        let mut packed = Vec::with_capacity(52);
        // Address: 20 bytes
        packed.extend_from_slice(address.as_slice());
        // Amount: 32 bytes big-endian
        let mut amount_bytes = [0u8; 32];
        amount.to_be_bytes_vec().iter().rev().enumerate().for_each(|(i, &b)| {
            if i < 32 {
                amount_bytes[31 - i] = b;
            }
        });
        packed.extend_from_slice(&amount_bytes);
        packed
    }

    pub fn update_eligibility_data(&mut self, eligibility_data: std::collections::HashMap<Address, U256>) -> Result<()> {
        // Clear existing leaves
        self.ordered_leaves.clear();
        self.leaf_index_map.clear();

        // Add all leaves (will be automatically sorted by BTreeMap key ordering)
        for (address, amount) in eligibility_data {
            let leaf_data = Self::encode_leaf_data(&address, &amount);
            if !self.leaf_index_map.contains_key(&leaf_data) {
                let index = self.ordered_leaves.len();
                self.ordered_leaves.push(leaf_data.clone());
                self.leaf_index_map.insert(leaf_data, index);
            }
        }

        // Build the tree
        self.build_tree()?;

        Ok(())
    }

    fn build_tree(&mut self) -> Result<()> {
        if self.ordered_leaves.is_empty() {
            self.root = None;
            return Ok(());
        }

        let mut current_level: Vec<MerkleNode> = self
            .ordered_leaves
            .iter()
            .enumerate()
            .map(|(i, data)| MerkleNode::new_leaf(data.clone(), i))
            .collect();

        if current_level.len() % 2 == 1 {
            let last_node = current_level.last().unwrap().clone();
            current_level.push(last_node);
        }

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let internal_node = MerkleNode::new_internal(chunk[0].clone(), chunk[1].clone());
                    next_level.push(internal_node);
                } else {
                    next_level.push(chunk[0].clone());
                }
            }

            if next_level.len() % 2 == 1 && next_level.len() > 1 {
                let last_node = next_level.last().unwrap().clone();
                next_level.push(last_node);
            }

            current_level = next_level;
        }

        self.root = current_level.into_iter().next();
        Ok(())
    }

    pub fn get_root_hash(&self) -> B256 {
        self.root
            .as_ref()
            .map(|node| B256::from_slice(&node.hash))
            .unwrap_or_else(|| B256::ZERO)
    }

    pub fn get_value(&self, address: &Address) -> Result<Option<U256>> {
        // Search through leaves for this address
        for leaf_data in &self.ordered_leaves {
            if leaf_data.len() >= 20 && &leaf_data[0..20] == address.as_slice() {
                // Extract amount from leaf data (bytes 20-52)
                if leaf_data.len() >= 52 {
                    let amount_bytes = &leaf_data[20..52];
                    let amount = U256::from_be_slice(amount_bytes);
                    return Ok(Some(amount));
                }
            }
        }
        Ok(None)
    }

    pub fn compute_merkle_proof(&self, address: &Address) -> Result<Vec<Vec<u8>>> {
        // Find the leaf index for this address
        let mut leaf_index = None;
        for (idx, leaf_data) in self.ordered_leaves.iter().enumerate() {
            if leaf_data.len() >= 20 && &leaf_data[0..20] == address.as_slice() {
                leaf_index = Some(idx);
                break;
            }
        }

        let leaf_index = leaf_index.ok_or_else(|| anyhow::anyhow!("Address not found in trie"))?;

        let proof = self.generate_proof_by_index(leaf_index)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate proof"))?;

        // Convert proof to Vec<Vec<u8>>
        Ok(proof.siblings.iter().map(|p| p.hash.to_vec()).collect())
    }

    fn generate_proof_by_index(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.ordered_leaves.len() {
            return None;
        }

        let root = self.root.as_ref()?;
        let mut siblings = Vec::new();

        let mut actual_tree_size = self.ordered_leaves.len();
        if actual_tree_size % 2 == 1 {
            actual_tree_size += 1;
        }

        self.collect_siblings(root, leaf_index, actual_tree_size, 0, &mut siblings);

        let leaf_data = self.ordered_leaves[leaf_index].clone();
        let leaf_hash = keccak256(&leaf_data);

        Some(MerkleProof {
            leaf_index,
            leaf_data,
            leaf_hash,
            siblings,
        })
    }

    fn collect_siblings(
        &self,
        node: &MerkleNode,
        target_index: usize,
        tree_width: usize,
        _level: usize,
        siblings: &mut Vec<ProofElement>,
    ) {
        if node.is_leaf() {
            return;
        }

        let left_child = node.left.as_ref().unwrap();
        let right_child = node.right.as_ref().unwrap();

        let mid_point = tree_width / 2;

        if target_index < mid_point {
            siblings.push(ProofElement {
                hash: right_child.hash,
                is_right_sibling: true,
            });
            self.collect_siblings(left_child, target_index, mid_point, _level + 1, siblings);
        } else {
            siblings.push(ProofElement {
                hash: left_child.hash,
                is_right_sibling: false,
            });
            self.collect_siblings(
                right_child,
                target_index - mid_point,
                tree_width - mid_point,
                _level + 1,
                siblings,
            );
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct SerializedTrie {
            root_hash: Vec<u8>,
            ordered_leaves: Vec<Vec<u8>>,
        }

        let serialized = SerializedTrie {
            root_hash: self.get_root_hash().to_vec(),
            ordered_leaves: self.ordered_leaves.clone(),
        };

        serde_json::to_vec(&serialized).map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct SerializedTrie {
            root_hash: Vec<u8>,
            ordered_leaves: Vec<Vec<u8>>,
        }

        let serialized: SerializedTrie = serde_json::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))?;

        let mut trie = MerkleTrie::new();
        trie.ordered_leaves = serialized.ordered_leaves;

        // Rebuild index map
        for (idx, leaf_data) in trie.ordered_leaves.iter().enumerate() {
            trie.leaf_index_map.insert(leaf_data.clone(), idx);
        }

        // Rebuild tree
        trie.build_tree()?;

        Ok(trie)
    }

    pub fn get_leaf_count(&self) -> usize {
        self.ordered_leaves.len()
    }
}

impl Default for MerkleTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_keccak256() {
        let data = b"hello world";
        let hash = keccak256(data);
        let expected = hex::decode("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad").unwrap();
        assert_eq!(hash.to_vec(), expected);
    }

    #[test]
    fn test_merkle_trie_basic() {
        let mut trie = MerkleTrie::new();
        let mut data = std::collections::HashMap::new();

        let addr1 = Address::from_str("0x742C4d97C86bCF0176776C16e073b8c6f9Db4021").unwrap();
        let addr2 = Address::from_str("0x8ba1f109551bD432803012645Ac136c5a2B51A").unwrap();

        data.insert(addr1, U256::from(1000000000000000000u128));
        data.insert(addr2, U256::from(500000000000000000u128));

        trie.update_eligibility_data(data).unwrap();

        let root_hash = trie.get_root_hash();
        assert_ne!(root_hash, B256::ZERO);

        let proof = trie.compute_merkle_proof(&addr1).unwrap();
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_serialization() {
        let mut trie = MerkleTrie::new();
        let mut data = std::collections::HashMap::new();

        let addr = Address::from_str("0x742C4d97C86bCF0176776C16e073b8c6f9Db4021").unwrap();
        data.insert(addr, U256::from(1000000000000000000u128));

        trie.update_eligibility_data(data).unwrap();

        let serialized = trie.serialize().unwrap();
        let deserialized = MerkleTrie::deserialize(&serialized).unwrap();

        assert_eq!(trie.get_root_hash(), deserialized.get_root_hash());
    }
}
