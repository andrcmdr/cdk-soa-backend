## Merkle Trie generator and comparison CLI tool

## Usage Examples:

### Basic usage:

Build and run the CLI tool:

```bash
# Build the project
cargo build --release

# Run the CLI tool
cargo run --bin merkle-cli -- \
  --input example.csv \
  --output output.json \
  --verbose \
  --pretty

# Or use the compiled binary
./target/release/merkle-cli \
  --input example.csv \
  --output output.json \
  --verbose \
  --pretty
```

### Keep 0x prefix in leaf data:
```bash
cargo run --bin merkle-cli -- \
  --input example.csv \
  --output output.json \
  --keep-prefix \
  --verbose \
  --pretty
```

### Compare root hash:
```bash
cargo run --bin merkle-cli -- \
  --input example.csv \
  --output output.json \
  --compare-root "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" \
  --verbose \
  --pretty

# Check exit code
echo $?  # Returns 0 if match, 1 if mismatch
```

### Compare with reference JSON:
```bash
cargo run --bin merkle-cli -- \
  --input example.csv \
  --output output.json \
  --compare-json example_reference.json \
  --verbose \
  --pretty

# Check exit code
echo $?  # Returns 0 if all match, 1 if any mismatch
```

### Complete comparison (root hash + JSON with proofs):
```bash
cargo run --bin merkle-cli -- \
  --input example.csv \
  --output output.json \
  --compare-root "0xabcdef123456..." \
  --compare-json reference.json \
  --keep-prefix \
  --verbose \
  --pretty

# Check exit code
echo $?  # Returns 0 if all match, 1 if there are any mismatch
```

## Exit Codes:

- **0**: Success (all comparisons passed or no comparisons requested)
- **1**: Failure (root hash mismatch or/and proof differences detected)

## Comparison CLI Output Example:

```
=== Root Hash Comparison ===
Expected: 0x1234...abcd
Actual:   0x5678...efgh
✗ Root hash DOES NOT match
============================

=== Comparison Report ===
✗ Root hash DOES NOT match
✗ Proofs have differences

  Missing addresses (in reference but not in output):
    - 0x1234567890123456789012345678901234567890

  Mismatched proofs:
    - 0xabcdef1234567890abcdef1234567890abcdef12

=========================

✗ ERROR: Output comparison with reference JSON failed!
```

This CLI tool provides comprehensive comparison capabilities and proper exit codes for CI/CD integration.

## New Features:

1. ✅ **`--keep-prefix`**: Keeps `0x` prefix in leaf data for hashing
2. ✅ **`--compare-root <HASH>`**: Compares computed root hash with expected value
3. ✅ **`--compare-json <PATH>`**: Compares output with reference JSON
4. ✅ **Exit codes**: Returns shell error status (1) if comparison fails
5. ✅ **Detailed reports**: Shows exactly what differs (missing addresses, mismatched proofs, etc.)

## CLI Options:

- `-i, --input <PATH>`: Input CSV file path (required)
- `-o, --output <PATH>`: Output JSON file path (required)
- `-v, --verbose`: Print root hash and statistics to stdout
- `-p, --pretty`: Pretty print JSON output

- `--keep-prefix`: Keeps `0x` prefix in leaf data for hashing
- `--compare-root <HASH>`: Compares computed root hash with expected value
- `--compare-json <PATH>`: Compares output with reference JSON

## Expected Output JSON File Format:

The tool will generate a JSON output file in exactly specified format:

```json
{
  "root_hash": "0x1234567890abcdef...",
  "allocations": {
    "0x06a37c563d88894a98438e3b2fe17f365f1d3530": {
      "allocation": "990000000000000000",
      "proof": [
        "0x5fa272eb5be1047ecbd6f02c97bc29f552c2cb081d793f10ed7f9c9c9e229ec6",
        "0x36003a3a59da38caf1f58e57a89c0e62957cbc78699bc9aa1d59c65dd5ca4b88",
        "0xfb85a4a2bd4a7cb643681d468ae32d7f36716abefbcc540771d005d96474ea0d"
      ]
    },
    "0x742c4d97c86bcf0176776c16e073b8c6f9db4021": {
      "allocation": "1000000000000000000",
      "proof": [
        "0x7fc3ecd9577a0cf7d414b1cc9e0c94e006cf073f99b63c2046a30a5dccfca9e7",
        "0x8849588141eaee743b7b2ebd93d78afbe099e40b65a4aa708580a72e0918e375"
      ]
    }
  }
}
```

## Key Features:

1. **CSV Processing**: Reads CSV files with `address` and `amount` columns
2. **Leaf Encoding**: Concatenates address (20 bytes) + amount (32 bytes big-endian) and hashes with keccak256
3. **Address Normalization**: Converts addresses to lowercase and handles 0x prefix
4. **Merkle Proof Generation**: Generates complete Merkle proofs for each address
5. **JSON Output**: Outputs in the exact format specified with root hash and allocations
6. **Error Handling**: Comprehensive error messages for invalid CSV data
7. **Validation**: Validates addresses and amounts during processing

The tool ensures that the leaf data encoding matches Ethereum standards (20-byte addresses and 32-byte amounts in big-endian format), making the proofs compatible with smart contracts.

## Data ordering

The order of data rows significantly affects the proofs, hashes, and root hash in a Merkle tree.

## Why Order Matters

In the current implementation, the Merkle tree is built as a **binary tree** where:

1. **Leaf Position**: Each data row becomes a leaf at a specific position (index 0, 1, 2, 3...)
2. **Tree Structure**: The tree is built bottom-up by pairing adjacent leaves
3. **Hash Computation**: Parent hashes are computed as `keccak256(left_child || right_child)`

## Example:

Let's say we have 4 addresses:

### Order 1: [A, B, C, D]
```
       Root
      /    \
    H1      H2
   /  \    /  \
  A    B  C    D
```
- Root = keccak256(H1 || H2)
- H1 = keccak256(A || B)
- H2 = keccak256(C || D)

### Order 2: [D, C, B, A]
```
       Root'
      /    \
    H1'     H2'
   /  \    /  \
  D    C  B    A
```
- Root' = keccak256(H1' || H2')
- H1' = keccak256(D || C)
- H2' = keccak256(B || A)

**Result**: Root ≠ Root' (completely different!)

## Impact on Proofs

The proof for address A in both cases will be different:
- **Order 1**: Proof includes [B, H2] to reconstruct the path to Root
- **Order 2**: Proof includes [H2', C] to reconstruct the path to Root'

## Solution: Deterministic Ordering

To ensure consistent results regardless of input order, data should be sorted before building the tree.
An updated version of the CLI tool is made/built with deterministic ordering of data internally by the usage of `BTreeMap` data sctructure, which always ensures data ordered by key.
The tree will automatically sort leaves by their encoded data (address + amount), ensuring deterministic output regardless of CSV row order.

## Key changes of the current version:

1. **BTreeMap Usage**:
   - Changed `HashMap` to `BTreeMap` in the `MerkleTrie` struct
   - `leaves: BTreeMap<Vec<u8>, usize>` - automatically keeps leaf data sorted
   - `ordered_leaves: Vec<Vec<u8>>` - maintains the sorted order for quick index access

2. **keccak-hasher Integration**:
   - Using `keccak_hasher::KeccakHasher` which implements the `hash_db::Hasher` trait
   - `KeccakHasher::hash()` provides the keccak256 implementation
   - Compatible with Ethereum's keccak256

3. **Automatic Sorting**:
   - No manual sorting needed - BTreeMap handles it automatically
   - Removed `--sort` flag from CLI since it's always deterministic
   - Keys in BTreeMap are always in sorted order

4. **Enhanced Features**:
   - Added `leaf_hash` to `MerkleProof` for easier verification
   - Added `get_leaf_count()` method
   - Added `get_leaf_at_index()` method
   - Added `is_deterministic()` method that always returns `true`

5. **CLI Output Uses BTreeMap**:
   - `OutputData.allocations` is now `BTreeMap<String, AllocationProof>`
   - JSON output will have addresses in sorted order

## Benefits:

✅ **Deterministic**: Same data always produces same root hash
✅ **Sorted by Default**: No need for manual sorting
✅ **Efficient**: BTreeMap provides O(log n) operations
✅ **Compatible**: Uses standard Ethereum keccak256 hashing
✅ **Verifiable**: Anyone can reproduce the same tree and root hash
