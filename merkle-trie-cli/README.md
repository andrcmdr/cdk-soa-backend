## Merkle Trie Generator

## Changelog:

## v0.1.0:

## This implementation provides:

## Key Features

1. **Pure Rust Implementation**: No external cryptographic dependencies - uses a simplified hash function for demonstration (in production, we'd replace it with proper hashing/digest function, like SHA-256)

2. **Complete Merkle Tree**: Binary tree structure with internal nodes and leaf nodes

3. **Root Hash**: Computed from bottom-up tree construction

4. **Merkle Proofs**: Complete proofs with all sibling hashes needed to verify a leaf's inclusion

5. **Proof Verification**: Both instance-based and static verification methods

## Core Components

- **MerkleNode**: Represents both leaf and internal nodes
- **MerkleProof**: Contains leaf data and sibling hashes with position information
- **MerkleTrie**: Main structure managing the tree and operations

## Key Methods

- `add_leaf()`: Add data to the tree
- `build_tree()`: Construct the tree from leaves
- `get_root_hash()`: Get the tree's root hash
- `generate_proof()`: Create merkle proof for any leaf
- `verify_proof()`: Verify a proof against the tree
- `update_leaf()`: Modify existing leaf data
- `remove_leaf()`: Remove leaf and rebuild tree

## Usage Example

```rust
let mut trie = MerkleTrie::new();
trie.add_leaf(b"Hello".to_vec());
trie.add_leaf(b"World".to_vec());
trie.build_tree();

let root_hash = trie.get_root_hash().unwrap();
let proof = trie.generate_proof(b"Hello").unwrap();
assert!(trie.verify_proof(&proof));
```

The implementation handles edge cases like odd numbers of leaves (by duplication) and provides comprehensive testing. For production use, we'd replace the simplified hash function with a proper cryptographic hash/digest, like SHA-256.

## v0.2.0:

## Key Changes Made:

1. **Keccak256 Implementation**: Replaced the simple hash function with proper keccak256 using `tiny-keccak`
2. **Hex Support**: Added hex encoding/decoding support for better compatibility with Ethereum tooling
3. **Enhanced API**: Added methods for:
   - `get_root_hash_hex()`: Get root hash as hex string
   - `verify_proof_against_hex_root()`: Verify proof against hex root hash
   - `from_address_amounts()`: Create trie from address/amount pairs
   - `generate_proof_for_address_amount()`: Generate proof for address/amount pairs
   - `proof_to_hex_array()`: Convert proof to hex array format for smart contracts

4. **Better Display**: Enhanced the `Display` implementation for `MerkleProof` to show hex values
5. **Production Ready**: Uses cryptographically secure keccak256 hash function

## Usage Examples:

```rust
// Create trie from address/amount pairs
let mut data = HashMap::new();
data.insert("0x742C4d97C86bCF0176776C16e073b8c6f9Db4021".to_string(), "1000000000000000000".to_string());
data.insert("0x8ba1f109551bD432803012645Hac136c5a2B1A".to_string(), "500000000000000000".to_string());

let trie = MerkleTrie::from_address_amounts(data)?;
let root_hash = trie.get_root_hash_hex().unwrap(); // Returns "0x..."

// Generate proof for specific address/amount
let proof = trie.generate_proof_for_address_amount(
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
    "1000000000000000000"
)?.unwrap();

// Get proof in hex format for smart contracts
let hex_proof = trie.proof_to_hex_array(&proof);

// Verify proof
assert!(trie.verify_proof(&proof));
```

This implementation is now compatible with Ethereum's `keccak256` hashing and provides features needed for production Merkle trie system.
