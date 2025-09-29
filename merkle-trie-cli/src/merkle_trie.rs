use std::collections::HashMap;
use std::fmt;

// Simple SHA-256 implementation (simplified for demonstration)
// In a real implementation, we'd want to use a proper crypto library
pub fn hash_data(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();

    // Convert to 32-byte array (simplified)
    let mut result = [0u8; 32];
    result[0..8].copy_from_slice(&hash.to_be_bytes());

    // For demonstration, we'll use a simple hash function
    // In production, we'd use a proper cryptographic hash like SHA-256
    for i in 8..32 {
        result[i] = (hash.wrapping_mul(i as u64 + 1) >> (i % 8)) as u8;
    }

    result
}

pub fn hash_combine(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(left);
    combined.extend_from_slice(right);
    hash_data(&combined)
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
        let hash = hash_data(&data);
        MerkleNode {
            hash,
            left: None,
            right: None,
            data: Some(data),
            index: Some(index),
        }
    }

    pub fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let hash = hash_combine(&left.hash, &right.hash);
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
        writeln!(f, "Leaf data: {:?}", self.leaf_data)?;
        writeln!(f, "Sibling hashes:")?;
        for (i, element) in self.siblings.iter().enumerate() {
            writeln!(
                f,
                "  Level {}: {:02x?} ({})",
                i,
                &element.hash[..8],
                if element.is_right_sibling { "right" } else { "left" }
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MerkleTrie {
    root: Option<MerkleNode>,
    leaves: Vec<Vec<u8>>,
    leaf_map: HashMap<Vec<u8>, usize>, // Maps data to leaf index
}

impl MerkleTrie {
    pub fn new() -> Self {
        MerkleTrie {
            root: None,
            leaves: Vec::new(),
            leaf_map: HashMap::new(),
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
        if !self.leaf_map.contains_key(&data) {
            let index = self.leaves.len();
            self.leaf_map.insert(data.clone(), index);
            self.leaves.push(data);
        }
    }

    pub fn build_tree(&mut self) {
        if self.leaves.is_empty() {
            self.root = None;
            return;
        }

        // Create leaf nodes
        let mut current_level: Vec<MerkleNode> = self
            .leaves
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

    pub fn generate_proof(&self, data: &[u8]) -> Option<MerkleProof> {
        let leaf_index = *self.leaf_map.get(data)?;
        self.generate_proof_by_index(leaf_index)
    }

    pub fn generate_proof_by_index(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let root = self.root.as_ref()?;
        let mut siblings = Vec::new();
        let mut current_index = leaf_index;

        // Handle case where we duplicated the last node for odd number of leaves
        let mut actual_tree_size = self.leaves.len();
        if actual_tree_size % 2 == 1 {
            actual_tree_size += 1;
        }

        self.collect_siblings(root, current_index, actual_tree_size, 0, &mut siblings);

        Some(MerkleProof {
            leaf_index,
            leaf_data: self.leaves[leaf_index].clone(),
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
        let mut current_hash = hash_data(&proof.leaf_data);

        for sibling in &proof.siblings {
            current_hash = if sibling.is_right_sibling {
                // Current node is left, sibling is right
                hash_combine(&current_hash, &sibling.hash)
            } else {
                // Current node is right, sibling is left
                hash_combine(&sibling.hash, &current_hash)
            };
        }

        &current_hash == root_hash
    }

    pub fn get_all_leaves(&self) -> &Vec<Vec<u8>> {
        &self.leaves
    }

    pub fn find_leaf_index(&self, data: &[u8]) -> Option<usize> {
        self.leaf_map.get(data).copied()
    }

    pub fn update_leaf(&mut self, index: usize, new_data: Vec<u8>) -> bool {
        if index >= self.leaves.len() {
            return false;
        }

        // Remove old mapping
        let old_data = &self.leaves[index];
        self.leaf_map.remove(old_data);

        // Add new mapping
        self.leaf_map.insert(new_data.clone(), index);
        self.leaves[index] = new_data;

        // Rebuild tree
        self.build_tree();
        true
    }

    pub fn remove_leaf(&mut self, data: &[u8]) -> bool {
        if let Some(&index) = self.leaf_map.get(data) {
            self.leaf_map.remove(data);
            self.leaves.remove(index);

            // Update indices in the map
            let mut new_map = HashMap::new();
            for (leaf_data, &old_index) in &self.leaf_map {
                let new_index = if old_index > index {
                    old_index - 1
                } else {
                    old_index
                };
                new_map.insert(leaf_data.clone(), new_index);
            }
            self.leaf_map = new_map;

            // Rebuild tree
            self.build_tree();
            true
        } else {
            false
        }
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
        let expected = hash_data(b"hello");
        assert_eq!(root_hash, expected);
    }

    #[test]
    fn test_two_leaves() {
        let data = vec![b"hello".to_vec(), b"world".to_vec()];
        let trie = MerkleTrie::from_data(data);

        let root_hash = trie.get_root_hash().unwrap();

        let left_hash = hash_data(b"hello");
        let right_hash = hash_data(b"world");
        let expected = hash_combine(&left_hash, &right_hash);

        assert_eq!(root_hash, expected);
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

        // Generate proof for first leaf
        let proof = trie.generate_proof(b"data1").unwrap();
        assert_eq!(proof.leaf_index, 0);
        assert_eq!(proof.leaf_data, b"data1");

        // Verify proof
        assert!(trie.verify_proof(&proof));

        // Verify against root hash directly
        let root_hash = trie.get_root_hash().unwrap();
        assert!(MerkleTrie::verify_proof_against_root(&proof, &root_hash));
    }

    #[test]
    fn test_odd_number_of_leaves() {
        let data = vec![b"data1".to_vec(), b"data2".to_vec(), b"data3".to_vec()];
        let trie = MerkleTrie::from_data(data);

        // Should handle odd number by duplicating last node
        let root_hash = trie.get_root_hash().unwrap();
        assert!(root_hash != [0u8; 32]);

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

        assert!(trie.update_leaf(0, b"updated".to_vec()));
        let new_root = trie.get_root_hash().unwrap();

        assert_ne!(original_root, new_root);
        assert_eq!(trie.leaves[0], b"updated");
    }

    #[test]
    fn test_remove_leaf() {
        let mut trie = MerkleTrie::from_data(vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
        ]);

        assert!(trie.remove_leaf(b"data2"));
        assert_eq!(trie.leaves.len(), 2);
        assert_eq!(trie.leaves[0], b"data1");
        assert_eq!(trie.leaves[1], b"data3");

        // Verify remaining leaves still have valid proofs
        let proof1 = trie.generate_proof(b"data1").unwrap();
        let proof3 = trie.generate_proof(b"data3").unwrap();
        assert!(trie.verify_proof(&proof1));
        assert!(trie.verify_proof(&proof3));
    }
}
