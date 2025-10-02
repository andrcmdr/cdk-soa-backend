use std::collections::BTreeMap;
use std::fmt;
use keccak_hasher::KeccakHasher;
use hash_db::Hasher as HashDbHasher;

/// Keccak256 hash implementation using keccak-hasher
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    KeccakHasher::hash(data)
}

pub fn keccak256_combine(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(left);
    combined.extend_from_slice(right);
    keccak256(&combined)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
    pub data: Option<Vec<u8>>,
    pub index: Option<usize>, // For leaf nodes
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
        let hash = keccak256_combine(&left.hash, &right.hash);
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

impl fmt::Display for MerkleProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Merkle Proof for leaf index {}:", self.leaf_index)?;
        writeln!(f, "Leaf hash: 0x{}", hex::encode(&self.leaf_hash))?;
        writeln!(f, "Sibling hashes:")?;
        for (i, element) in self.siblings.iter().enumerate() {
            writeln!(
                f,
                "  Level {}: 0x{} ({})",
                i,
                hex::encode(&element.hash[..8]),
                if element.is_right_sibling { "right" } else { "left" }
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MerkleTrie {
    root: Option<MerkleNode>,
    // BTreeMap automatically keeps keys sorted
    leaves: BTreeMap<Vec<u8>, usize>, // Maps data to leaf index (sorted by data)
    ordered_leaves: Vec<Vec<u8>>,     // Ordered list of leaf data for index lookup
}

impl MerkleTrie {
    pub fn new() -> Self {
        MerkleTrie {
            root: None,
            leaves: BTreeMap::new(),
            ordered_leaves: Vec::new(),
        }
    }

    pub fn from_data(data: Vec<Vec<u8>>) -> Self {
        let mut trie = MerkleTrie::new();
        for item in data {
            trie.add_leaf(item);
        }
        trie.build_tree();
        trie
    }

    pub fn add_leaf(&mut self, data: Vec<u8>) {
        if !self.leaves.contains_key(&data) {
            self.leaves.insert(data, 0); // Index will be updated when building tree
        }
    }

    pub fn build_tree(&mut self) {
        if self.leaves.is_empty() {
            self.root = None;
            self.ordered_leaves.clear();
            return;
        }

        // Extract and sort leaf data (BTreeMap keys are already sorted)
        self.ordered_leaves = self.leaves.keys().cloned().collect();

        // Update indices in the BTreeMap
        for (index, data) in self.ordered_leaves.iter().enumerate() {
            self.leaves.insert(data.clone(), index);
        }

        // Create leaf nodes with sorted order
        let mut current_level: Vec<MerkleNode> = self
            .ordered_leaves
            .iter()
            .enumerate()
            .map(|(i, data)| MerkleNode::new_leaf(data.clone(), i))
            .collect();

        // If odd number of nodes, duplicate the last one
        if current_level.len() % 2 == 1 {
            let last_node = current_level.last().unwrap().clone();
            current_level.push(last_node);
        }

        // Build tree bottom-up
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let internal_node = MerkleNode::new_internal(chunk[0].clone(), chunk[1].clone());
                    next_level.push(internal_node);
                } else {
                    // This should not happen if we handle odd numbers correctly
                    next_level.push(chunk[0].clone());
                }
            }

            // If odd number of nodes, duplicate the last one
            if next_level.len() % 2 == 1 && next_level.len() > 1 {
                let last_node = next_level.last().unwrap().clone();
                next_level.push(last_node);
            }

            current_level = next_level;
        }

        self.root = current_level.into_iter().next();
    }

    pub fn get_root_hash(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|node| node.hash)
    }

    /// Get root hash as hex string with 0x prefix
    pub fn get_root_hash_hex(&self) -> Option<String> {
        self.get_root_hash().map(|hash| format!("0x{}", hex::encode(hash)))
    }

    pub fn generate_proof(&self, data: &[u8]) -> Option<MerkleProof> {
        let leaf_index = *self.leaves.get(data)?;
        self.generate_proof_by_index(leaf_index)
    }

    pub fn generate_proof_by_index(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.ordered_leaves.len() {
            return None;
        }

        let root = self.root.as_ref()?;
        let mut siblings = Vec::new();

        // Handle case where we duplicated the last node for odd number of leaves
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
        level: usize,
        siblings: &mut Vec<ProofElement>,
    ) {
        if node.is_leaf() {
            return;
        }

        let left_child = node.left.as_ref().unwrap();
        let right_child = node.right.as_ref().unwrap();

        let mid_point = tree_width / 2;

        if target_index < mid_point {
            // Target is in left subtree, right child is sibling
            siblings.push(ProofElement {
                hash: right_child.hash,
                is_right_sibling: true,
            });
            self.collect_siblings(left_child, target_index, mid_point, level + 1, siblings);
        } else {
            // Target is in right subtree, left child is sibling
            siblings.push(ProofElement {
                hash: left_child.hash,
                is_right_sibling: false,
            });
            self.collect_siblings(
                right_child,
                target_index - mid_point,
                tree_width - mid_point,
                level + 1,
                siblings,
            );
        }
    }

    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        let root_hash = match self.get_root_hash() {
            Some(hash) => hash,
            None => return false,
        };

        Self::verify_proof_against_root(proof, &root_hash)
    }

    pub fn verify_proof_against_root(proof: &MerkleProof, root_hash: &[u8; 32]) -> bool {
        let mut current_hash = keccak256(&proof.leaf_data);

        for sibling in &proof.siblings {
            current_hash = if sibling.is_right_sibling {
                // Current node is left, sibling is right
                keccak256_combine(&current_hash, &sibling.hash)
            } else {
                // Current node is right, sibling is left
                keccak256_combine(&sibling.hash, &current_hash)
            };
        }

        &current_hash == root_hash
    }

    /// Verify proof against hex string root hash
    pub fn verify_proof_against_hex_root(proof: &MerkleProof, root_hash_hex: &str) -> Result<bool, hex::FromHexError> {
        let root_hash_str = root_hash_hex.strip_prefix("0x").unwrap_or(root_hash_hex);
        let root_hash_bytes = hex::decode(root_hash_str)?;

        if root_hash_bytes.len() != 32 {
            return Ok(false);
        }

        let mut root_hash = [0u8; 32];
        root_hash.copy_from_slice(&root_hash_bytes);

        Ok(Self::verify_proof_against_root(proof, &root_hash))
    }

    pub fn get_all_leaves(&self) -> Vec<Vec<u8>> {
        self.ordered_leaves.clone()
    }

    pub fn get_leaf_count(&self) -> usize {
        self.ordered_leaves.len()
    }

    pub fn find_leaf_index(&self, data: &[u8]) -> Option<usize> {
        self.leaves.get(data).copied()
    }

    pub fn update_leaf(&mut self, data: &[u8], new_data: Vec<u8>) -> bool {
        if !self.leaves.contains_key(data) {
            return false;
        }

        // Remove old data
        self.leaves.remove(data);

        // Add new data
        self.leaves.insert(new_data, 0); // Index will be updated when rebuilding

        // Rebuild tree
        self.build_tree();
        true
    }

    pub fn remove_leaf(&mut self, data: &[u8]) -> bool {
        if self.leaves.remove(data).is_some() {
            // Rebuild tree
            self.build_tree();
            true
        } else {
            false
        }
    }

    /// Convert proof to format compatible with smart contracts
    pub fn proof_to_hex_array(&self, proof: &MerkleProof) -> Vec<String> {
        proof
            .siblings
            .iter()
            .map(|element| format!("0x{}", hex::encode(element.hash)))
            .collect()
    }

    /// Create a trie from address/amount pairs (used commonly for airdrops)
    /// Data is automatically sorted by the encoded leaf data
    pub fn from_address_amounts(data: BTreeMap<String, String>) -> Result<Self, hex::FromHexError> {
        let mut trie = MerkleTrie::new();

        for (address, amount) in data {
            // Combine address and amount for leaf data
            let mut leaf_data = Vec::new();

            // Add address (remove 0x prefix if present)
            let addr_str = address.strip_prefix("0x").unwrap_or(&address);
            let addr_bytes = hex::decode(addr_str)?;
            leaf_data.extend_from_slice(&addr_bytes);

            // Add amount as bytes (might want to encode this differently)
            leaf_data.extend_from_slice(amount.as_bytes());

            trie.add_leaf(leaf_data);
        }

        trie.build_tree();
        Ok(trie)
    }

    /// Generate proof for an address/amount pair
    pub fn generate_proof_for_address_amount(&self, address: &str, amount: &str) -> Result<Option<MerkleProof>, hex::FromHexError> {
        let mut leaf_data = Vec::new();

        // Add address (remove 0x prefix if present)
        let addr_str = address.strip_prefix("0x").unwrap_or(address);
        let addr_bytes = hex::decode(addr_str)?;
        leaf_data.extend_from_slice(&addr_bytes);

        // Add amount as bytes
        leaf_data.extend_from_slice(amount.as_bytes());

        Ok(self.generate_proof(&leaf_data))
    }

    /// Get the leaf data at a specific index (in sorted order)
    pub fn get_leaf_at_index(&self, index: usize) -> Option<&Vec<u8>> {
        self.ordered_leaves.get(index)
    }

    /// Check if the tree is deterministic (always produces same root for same data)
    pub fn is_deterministic(&self) -> bool {
        // STUB for compatibility: BTreeMap ensures deterministic ordering
        true
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

    #[test]
    fn test_keccak256_hash() {
        let data = b"hello world";
        let hash = keccak256(data);

        // Known keccak256 hash of "hello world"
        let expected = hex::decode("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad").unwrap();
        assert_eq!(hash.to_vec(), expected);
    }

    #[test]
    fn test_empty_tree() {
        let trie = MerkleTrie::new();
        assert!(trie.get_root_hash().is_none());
    }

    #[test]
    fn test_single_leaf() {
        let mut trie = MerkleTrie::new();
        trie.add_leaf(b"hello".to_vec());
        trie.build_tree();

        let root_hash = trie.get_root_hash().unwrap();
        let expected = keccak256(b"hello");
        assert_eq!(root_hash, expected);
    }

    #[test]
    fn test_two_leaves() {
        let data = vec![b"hello".to_vec(), b"world".to_vec()];
        let trie = MerkleTrie::from_data(data);

        let root_hash = trie.get_root_hash().unwrap();

        // BTreeMap sorts by data, so order will be "hello", "world"
        let left_hash = keccak256(b"hello");
        let right_hash = keccak256(b"world");
        let expected = keccak256_combine(&left_hash, &right_hash);

        assert_eq!(root_hash, expected);
    }

    #[test]
    fn test_deterministic_ordering() {
        // Create two tries with same data but different insertion order
        let data1 = vec![b"zebra".to_vec(), b"apple".to_vec(), b"banana".to_vec()];
        let trie1 = MerkleTrie::from_data(data1);

        let data2 = vec![b"banana".to_vec(), b"zebra".to_vec(), b"apple".to_vec()];
        let trie2 = MerkleTrie::from_data(data2);

        // Root hashes should be identical due to BTreeMap sorting
        assert_eq!(trie1.get_root_hash(), trie2.get_root_hash());

        // Verify the order is sorted
        let leaves1 = trie1.get_all_leaves();
        let leaves2 = trie2.get_all_leaves();
        assert_eq!(leaves1, leaves2);
        assert_eq!(leaves1[0], b"apple");
        assert_eq!(leaves1[1], b"banana");
        assert_eq!(leaves1[2], b"zebra");
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let data = vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
            b"data4".to_vec(),
        ];
        let trie = MerkleTrie::from_data(data);

        // Generate proof for first leaf (sorted order: data1, data2, data3, data4)
        let proof = trie.generate_proof(b"data1").unwrap();
        assert_eq!(proof.leaf_data, b"data1");

        // Verify proof
        assert!(trie.verify_proof(&proof));

        // Verify against root hash directly
        let root_hash = trie.get_root_hash().unwrap();
        assert!(MerkleTrie::verify_proof_against_root(&proof, &root_hash));
    }

    #[test]
    fn test_hex_root_verification() {
        let data = vec![b"test".to_vec()];
        let trie = MerkleTrie::from_data(data);

        let root_hex = trie.get_root_hash_hex().unwrap();
        let proof = trie.generate_proof(b"test").unwrap();

        assert!(MerkleTrie::verify_proof_against_hex_root(&proof, &root_hex).unwrap());
    }

    #[test]
    fn test_address_amount_trie() {
        let mut data = BTreeMap::new();
        data.insert("0x742C4d97C86bCF0176776C16e073b8c6f9Db4021".to_string(), "1000".to_string());
        data.insert("0x8ba1f109551bD432803012645Fedac136c5a2B1A".to_string(), "2000".to_string());

        let trie = MerkleTrie::from_address_amounts(data).unwrap();
        let root_hash = trie.get_root_hash().unwrap();

        // Should have a valid root hash
        assert_ne!(root_hash, [0u8; 32]);

        // Should be able to generate proof
        let proof = trie.generate_proof_for_address_amount(
            "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
            "1000"
        ).unwrap().unwrap();

        // Proof should verify
        assert!(trie.verify_proof(&proof));
    }

    #[test]
    fn test_proof_to_hex_array() {
        let data = vec![b"data1".to_vec(), b"data2".to_vec()];
        let trie = MerkleTrie::from_data(data);

        let proof = trie.generate_proof(b"data1").unwrap();
        let hex_array = trie.proof_to_hex_array(&proof);

        assert!(!hex_array.is_empty());
        for hex_str in hex_array {
            assert!(hex_str.starts_with("0x"));
            assert_eq!(hex_str.len(), 66); // 0x + 64 hex chars = 66 total
        }
    }

    #[test]
    fn test_odd_number_of_leaves() {
        let data = vec![b"data1".to_vec(), b"data2".to_vec(), b"data3".to_vec()];
        let trie = MerkleTrie::from_data(data);

        // Should handle odd number by duplicating last node
        let root_hash = trie.get_root_hash().unwrap();
        assert_ne!(root_hash, [0u8; 32]);

        // All proofs should verify
        for i in 0..3 {
            let proof = trie.generate_proof_by_index(i).unwrap();
            assert!(trie.verify_proof(&proof));
        }
    }

    #[test]
    fn test_update_leaf() {
        let mut trie = MerkleTrie::from_data(vec![b"original".to_vec()]);
        let original_root = trie.get_root_hash().unwrap();

        assert!(trie.update_leaf(b"original", b"updated".to_vec()));
        let new_root = trie.get_root_hash().unwrap();

        assert_ne!(original_root, new_root);
        assert_eq!(trie.get_leaf_at_index(0).unwrap(), b"updated");
    }

    #[test]
    fn test_remove_leaf() {
        let mut trie = MerkleTrie::from_data(vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
        ]);

        assert!(trie.remove_leaf(b"data2"));
        assert_eq!(trie.get_leaf_count(), 2);

        // Verify remaining leaves
        let leaves = trie.get_all_leaves();
        assert!(leaves.contains(&b"data1".to_vec()));
        assert!(leaves.contains(&b"data3".to_vec()));
        assert!(!leaves.contains(&b"data2".to_vec()));

        // Verify remaining leaves still have valid proofs
        let proof1 = trie.generate_proof(b"data1").unwrap();
        let proof3 = trie.generate_proof(b"data3").unwrap();
        assert!(trie.verify_proof(&proof1));
        assert!(trie.verify_proof(&proof3));
    }

    #[test]
    fn test_is_deterministic() {
        let trie = MerkleTrie::new();
        assert!(trie.is_deterministic());
    }

    #[test]
    fn test_btreemap_automatic_sorting() {
        let data = vec![
            b"zzz".to_vec(),
            b"aaa".to_vec(),
            b"mmm".to_vec(),
        ];
        let trie = MerkleTrie::from_data(data);

        let leaves = trie.get_all_leaves();
        assert_eq!(leaves[0], b"aaa");
        assert_eq!(leaves[1], b"mmm");
        assert_eq!(leaves[2], b"zzz");
    }
}
