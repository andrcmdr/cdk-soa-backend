#[cfg(test)]
mod integration_tests {
    use merkle_trie_cli::merkle_trie::MerkleTrie;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_preserve_order_vs_sorted() {
        // Create test data in specific order
        let ordered_data = vec![
            b"zebra".to_vec(),
            b"apple".to_vec(),
            b"mango".to_vec(),
            b"banana".to_vec(),
        ];

        // Build trie with insertion order
        let trie_ordered = MerkleTrie::from_data(ordered_data.clone());

        // Build trie with sorted data
        let mut sorted_data = ordered_data.clone();
        sorted_data.sort();
        let trie_sorted = MerkleTrie::from_data(sorted_data);

        // Root hashes should be different
        assert_ne!(
            trie_ordered.get_root_hash(),
            trie_sorted.get_root_hash(),
            "Different insertion orders should produce different root hashes"
        );

        // But all proofs should still verify for their respective tries
        for data in &ordered_data {
            let proof_ordered = trie_ordered.generate_proof(data).unwrap();
            let proof_sorted = trie_sorted.generate_proof(data).unwrap();

            assert!(trie_ordered.verify_proof(&proof_ordered));
            assert!(trie_sorted.verify_proof(&proof_sorted));

            // Proofs should be different
            assert_ne!(proof_ordered.siblings, proof_sorted.siblings);
        }
    }

    #[test]
    fn test_leaf_insertion_order_preserved() {
        let data = vec![
            b"third".to_vec(),
            b"first".to_vec(),
            b"second".to_vec(),
        ];

        let trie = MerkleTrie::from_data(data.clone());

        // Verify insertion order is preserved
        for (i, expected_data) in data.iter().enumerate() {
            let actual_data = trie.get_leaf_at_index(i).unwrap();
            assert_eq!(actual_data, expected_data);
        }
    }

    #[test]
    fn test_proof_verification_comprehensive() {
        // Test with 8 leaves for 3-level tree
        let data: Vec<Vec<u8>> = (0..8)
            .map(|i| format!("data{}", i).into_bytes())
            .collect();

        let trie = MerkleTrie::from_data(data);

        // Verify all proofs
        for i in 0..8 {
            let proof = trie.generate_proof_by_index(i).unwrap();

            // 8 leaves = 3 levels, so 3 siblings
            assert_eq!(proof.siblings.len(), 3);

            // Verify proof
            assert!(trie.verify_proof(&proof));

            // Verify against root hash
            let root_hash = trie.get_root_hash().unwrap();
            assert!(MerkleTrie::verify_proof_against_root(&proof, &root_hash));
        }
    }

    #[test]
    fn test_address_amount_encoding() {
        // Simulate real airdrop data encoding
        let address = "742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128; // 1 ETH in wei

        let addr_bytes = hex::decode(address).unwrap();
        assert_eq!(addr_bytes.len(), 20);

        let amount_bytes = amount.to_be_bytes();
        let mut amount_32 = [0u8; 32];
        amount_32[16..32].copy_from_slice(&amount_bytes);

        let mut leaf_data = Vec::new();
        leaf_data.extend_from_slice(&addr_bytes);
        leaf_data.extend_from_slice(&amount_32);

        assert_eq!(leaf_data.len(), 52); // 20 + 32

        // Add to trie and verify
        let mut trie = MerkleTrie::new();
        trie.add_leaf(leaf_data.clone());
        trie.build_tree();

        let proof = trie.generate_proof(&leaf_data).unwrap();
        assert!(trie.verify_proof(&proof));
    }
}
