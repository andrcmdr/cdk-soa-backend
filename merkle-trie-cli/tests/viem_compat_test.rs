#[cfg(test)]
mod viem_compatibility_tests {
    use keccak_hasher::KeccakHasher;
    use hash_db::Hasher as HashDbHasher;

    fn keccak256(data: &[u8]) -> [u8; 32] {
        KeccakHasher::hash(data)
    }

    fn to_checksum_address(address: &str) -> Result<String, String> {
        let cleaned = address.strip_prefix("0x").unwrap_or(address).to_lowercase();

        if cleaned.len() != 40 {
            return Err("Invalid address length".to_string());
        }

        let hash = keccak256(cleaned.as_bytes());
        let hash_hex = hex::encode(hash);

        let mut checksum_addr = String::from("0x");

        for (i, ch) in cleaned.chars().enumerate() {
            if ch.is_ascii_digit() {
                checksum_addr.push(ch);
            } else {
                let hash_char = hash_hex.chars().nth(i).unwrap();
                let hash_value = u32::from_str_radix(&hash_char.to_string(), 16).unwrap();

                if hash_value >= 8 {
                    checksum_addr.push(ch.to_ascii_uppercase());
                } else {
                    checksum_addr.push(ch.to_ascii_lowercase());
                }
            }
        }

        Ok(checksum_addr)
    }

    fn leaf_hash(address: &str, amount: u128) -> Result<[u8; 32], String> {
        let checksum_addr = to_checksum_address(address)?;
        let addr_bytes = hex::decode(checksum_addr.strip_prefix("0x").unwrap_or(&checksum_addr))
            .map_err(|e| e.to_string())?;

        if addr_bytes.len() != 20 {
            return Err("Address must be 20 bytes".to_string());
        }

        let amount_bytes = amount.to_be_bytes();
        let mut amount_32 = [0u8; 32];
        amount_32[16..32].copy_from_slice(&amount_bytes);

        let mut packed = Vec::with_capacity(52);
        packed.extend_from_slice(&addr_bytes);
        packed.extend_from_slice(&amount_32);

        Ok(keccak256(&packed))
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
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

    #[test]
    fn test_keccak256_compatibility() {
        // Test vector from Ethereum
        let data = b"";
        let hash = keccak256(data);
        let expected = hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470").unwrap();
        assert_eq!(hash.to_vec(), expected, "Empty string hash");

        let data2 = b"hello world";
        let hash2 = keccak256(data2);
        let expected2 = hex::decode("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad").unwrap();
        assert_eq!(hash2.to_vec(), expected2, "Hello world hash");
    }

    #[test]
    fn test_checksum_address_compatibility() {
        // Test vectors from EIP-55
        let test_cases = vec![
            ("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"),
            ("0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359", "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359"),
            ("0xdbf03b407c01e7cd3cbea99509d93f8dddc8c6fb", "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB"),
            ("0xd1220a0cf47c7b9be7a2e6ba89f429762e7b9adb", "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb"),
        ];

        for (input, expected) in test_cases {
            let result = to_checksum_address(input).unwrap();
            assert_eq!(result, expected, "Checksum mismatch for {}", input);
        }
    }

    #[test]
    fn test_leaf_hash_encoding() {
        // Test that leaf hash matches Viem's encodePacked behavior
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128; // 1 ETH in wei

        let leaf = leaf_hash(address, amount).unwrap();

        // Verify the encoding manually
        let addr_bytes = hex::decode("742C4d97C86bCF0176776C16e073b8c6f9Db4021").unwrap();
        assert_eq!(addr_bytes.len(), 20);

        let mut amount_bytes = [0u8; 32];
        amount_bytes[16..32].copy_from_slice(&amount.to_be_bytes());

        let mut manual_packed = Vec::new();
        manual_packed.extend_from_slice(&addr_bytes);
        manual_packed.extend_from_slice(&amount_bytes);

        let manual_hash = keccak256(&manual_packed);
        assert_eq!(leaf, manual_hash, "Leaf hash encoding mismatch");
    }

    #[test]
    fn test_hash_pair_sorting() {
        let leaf1 = [0x11u8; 32];
        let leaf2 = [0x22u8; 32];

        let hash_12 = hash_pair(&leaf1, &leaf2);
        let hash_21 = hash_pair(&leaf2, &leaf1);

        assert_eq!(hash_12, hash_21, "Hash pair should be commutative");

        // Verify the sorting happens correctly
        let mut packed_sorted = Vec::new();
        packed_sorted.extend_from_slice(&leaf1); // leaf1 < leaf2
        packed_sorted.extend_from_slice(&leaf2);

        let expected = keccak256(&packed_sorted);
        assert_eq!(hash_12, expected, "Hash pair sorting verification");
    }

    #[test]
    fn test_typescript_impl_reproduction() {
        // Reproduce the exact TypeScript Viem implementation
        let addresses = vec![
            "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
            "0x8ba1f109551bD432803012645Ac136c5a2B51Abc",
            "0x06a37c563d88894a98438e3b2fe17f365f1d3530",
        ];

        let amounts = vec![
            1000000000000000000u128, // 1 ETH
            500000000000000000u128,  // 0.5 ETH
            990000000000000000u128,  // 0.99 ETH
        ];

        // Generate leaves
        let mut leaves = Vec::new();
        for (addr, amt) in addresses.iter().zip(amounts.iter()) {
            let leaf = leaf_hash(addr, *amt).unwrap();
            leaves.push(leaf);
        }

        assert_eq!(leaves.len(), 3);

        // Reproduce TypeScript tree construction:
        // const aa = hashPair(leaves[0], leaves[1])
        // const bb = hashPair(leaves[2], leaves[2])
        // const cc = hashPair(aa, bb)

        let aa = hash_pair(&leaves[0], &leaves[1]);
        let bb = hash_pair(&leaves[2], &leaves[2]); // Duplicate for odd number
        let cc = hash_pair(&aa, &bb);

        // Root should be non-zero
        assert_ne!(cc, [0u8; 32]);

        println!("TypeScript implementation reproduction:");
        println!("  leaves[0] = 0x{}", hex::encode(leaves[0]));
        println!("  leaves[1] = 0x{}", hex::encode(leaves[1]));
        println!("  leaves[2] = 0x{}", hex::encode(leaves[2]));
        println!("  aa = 0x{}", hex::encode(aa));
        println!("  bb = 0x{}", hex::encode(bb));
        println!("  root (cc) = 0x{}", hex::encode(cc));
    }

    #[test]
    fn test_amount_encoding_edge_cases() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";

        // Test zero amount
        let leaf_zero = leaf_hash(address, 0).unwrap();
        assert_ne!(leaf_zero, [0u8; 32]);

        // Test max u128
        let leaf_max = leaf_hash(address, u128::MAX).unwrap();
        assert_ne!(leaf_max, [0u8; 32]);
        assert_ne!(leaf_zero, leaf_max);

        // Test small amount
        let leaf_small = leaf_hash(address, 1).unwrap();
        assert_ne!(leaf_small, leaf_zero);
        assert_ne!(leaf_small, leaf_max);
    }

    #[test]
    fn test_different_addresses_produce_different_hashes() {
        let addresses = vec![
            "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
            "0x8ba1f109551bD432803012645Ac136c5a2B51Abc",
            "0x06a37c563d88894a98438e3b2fe17f365f1d3530",
        ];

        let amount = 1000000000000000000u128;

        let mut hashes = Vec::new();
        for addr in addresses {
            let hash = leaf_hash(addr, amount).unwrap();
            hashes.push(hash);
        }

        // All hashes should be unique
        for i in 0..hashes.len() {
            for j in i+1..hashes.len() {
                assert_ne!(hashes[i], hashes[j],
                    "Addresses {} and {} produced same hash", i, j);
            }
        }
    }

    #[test]
    fn test_case_insensitive_address_input() {
        let amount = 1000000000000000000u128;

        let lowercase = "0x742c4d97c86bcf0176776c16e073b8c6f9db4021";
        let uppercase = "0x742C4D97C86BCF0176776C16E073B8C6F9DB4021";
        let mixed = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";

        let hash1 = leaf_hash(lowercase, amount).unwrap();
        let hash2 = leaf_hash(uppercase, amount).unwrap();
        let hash3 = leaf_hash(mixed, amount).unwrap();

        // All should produce the same hash (checksum normalization)
        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_packed_encoding_size() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128;

        // Manually create packed data
        let addr_bytes = hex::decode("742C4d97C86bCF0176776C16e073b8c6f9Db4021").unwrap();
        let amount_bytes = amount.to_be_bytes();
        let mut amount_32 = [0u8; 32];
        amount_32[16..32].copy_from_slice(&amount_bytes);

        let mut packed = Vec::new();
        packed.extend_from_slice(&addr_bytes);
        packed.extend_from_slice(&amount_32);

        // Should be exactly 52 bytes (20 + 32)
        assert_eq!(packed.len(), 52);

        // First 20 bytes should be address
        assert_eq!(&packed[0..20], &addr_bytes[..]);

        // Next 32 bytes should be amount
        assert_eq!(&packed[20..52], &amount_32[..]);
    }
}
