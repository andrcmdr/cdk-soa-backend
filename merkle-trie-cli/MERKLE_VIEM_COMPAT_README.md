## Merkle Trie generator and comparison CLI tool (TypeScript's Viem compatible)

## Usage:

### 1. Basic usage:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --verbose \
  --pretty
```

### 2. Show all details:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --verbose \
  --show-leaves \
  --show-tree \
  --pretty
```

### 3. Output to stdout:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --verbose \
  --pretty
```

### 4. Run tests:
```bash
# Run all tests
cargo test

# Run viem compatibility tests
cargo test viem_compatibility_tests

# Run with output
cargo test viem_compatibility_tests -- --nocapture
```

## Key Features:

1. ✅ **Exact viem compatibility**: Matches TypeScript `encodePacked` behavior
2. ✅ **EIP-55 checksum**: Proper Ethereum address checksumming
3. ✅ **Sorted pair hashing**: Lexicographic ordering like TypeScript
4. ✅ **Keccak256**: Using `keccak-hasher` for correct hashing
5. ✅ **Manual tree construction**: Reproduces TypeScript implementation exactly
6. ✅ **Comprehensive tests**: Verifies compatibility with Viem/TypeScript
7. ✅ **CLI interface**: Similar to Python version
8. ✅ **JSON output**: Compatible with reference format

## Comparison Table:

| Feature | TypeScript (viem) | Python | Rust (this) |
|---------|------------------|---------|-------------|
| Checksum | `checksumAddress()` | `to_checksum_address()` | `to_checksum_address()` |
| Encoding | `encodePacked()` | Manual concatenation | Manual concatenation |
| Keccak256 | `keccak256()` | `eth_utils.keccak()` | `KeccakHasher::hash()` |
| Pair sort | `if (left >= right)` | Same | Same |
| Output | JSON | JSON | JSON |

All three implementations produce **identical results**!
