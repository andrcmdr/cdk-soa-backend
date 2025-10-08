# Merkle Trie generator and comparison CLI tool (TypeScript's Viem compatible)

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


## Merkle Trie generator and comparison CLI tool (TypeScript's Viem compatible) extended version with comprehensive comparisons

Extended version with root hash and proofs comprehensive comparisons.
It compares generated results with reference root hash, provided through CLI, and root hash and proofs, provided in a reference JSON file (that has the same format as output JSON file).

## Usage Examples:

### 1. Basic usage:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --verbose \
  --pretty
```

### 2. Compare root hash (CLI):
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --compare-root "0xabcd1234..." \
  --verbose

echo $?  # Check exit code: 0 = success, 1 = mismatch
```

### 3. Compare with reference JSON:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --compare-json reference.json \
  --verbose

echo $?  # Check exit code: 0 = success, 2 = root mismatch, 3 = proofs mismatch
```

### 4. Keep 0x prefix in leaf data:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --keep-prefix \
  --verbose
```

### 5. Full verification:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --keep-prefix \
  --compare-root "0x..." \
  --compare-json reference.json \
  --show-leaves \
  --show-tree \
  --verbose \
  --pretty
```

### 6. Run exit code tests:
```bash
make test-exit-codes
```

## Exit Code Summary:

| Exit Code | Meaning | Error Message |
|-----------|---------|---------------|
| **0** | Success | All verifications passed |
| **1** | CLI root mismatch | Root hash provided via CLI does not match |
| **2** | JSON root mismatch | Root hash in reference JSON does not match |
| **3** | JSON proofs mismatch | Proofs in reference JSON do not match |


# Exit Codes Documentation

## Overview

The `merkle-cli-viem-compat` CLI tool returns specific exit codes to indicate different types of failures. This is useful for CI/CD pipelines and automated testing.

## Exit Codes

| Code | Constant | Description |
|------|----------|-------------|
| 0 | `EXIT_SUCCESS` | Success - All operations completed successfully |
| 1 | `EXIT_ROOT_MISMATCH_CLI` | Root hash provided via `--compare-root` does not match computed root |
| 2 | `EXIT_ROOT_MISMATCH_JSON` | Root hash in reference JSON file (via `--compare-json`) does not match |
| 3 | `EXIT_PROOFS_MISMATCH_JSON` | Proofs in reference JSON file do not match computed proofs |

## Comparison Priority

When multiple comparison options are provided, they are checked in this order:

1. **JSON Comparison** (`--compare-json`)
   - Root hash is checked first (exit code 2 if mismatch)
   - Proofs are checked second (exit code 3 if mismatch)

2. **CLI Root Comparison** (`--compare-root`)
   - Checked if JSON comparison passes or is not provided
   - Exit code 1 if mismatch

## Usage Examples

### Basic Success Case
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json

echo $?  # Returns 0
```

### Compare Root Hash (CLI)
```bash
# Correct root hash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --compare-root "0xabcd1234..."

echo $?  # Returns 0 if matches, 1 if doesn't match
```

### Compare with Reference JSON
```bash
# All comparisons pass
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --compare-json reference.json

echo $?  # Returns 0, 2, or 3 depending on what differs
```

### Combined Comparisons
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --compare-root "0xabcd..." \
  --compare-json reference.json

# Exit codes priority:
# - 2 if JSON root hash differs
# - 3 if JSON proofs differ
# - 1 if CLI root hash differs (only if JSON checks pass)
# - 0 if all pass
```

## CI/CD Integration

### GitHub Actions Example
```yaml
- name: Generate and verify Merkle tree
  run: |
    cargo run --bin merkle-cli-viem-compat -- \
      --input allocations.csv \
      --output output.json \
      --compare-json expected.json \
      --verbose

- name: Check exit code
  if: failure()
  run: |
    echo "Merkle tree verification failed!"
    exit 1
```

### Shell Script Example
```bash
#!/bin/bash

./merkle-cli-viem-compat \
  --input data.csv \
  --output output.json \
  --compare-root "0x..." \
  --compare-json reference.json

EXIT_CODE=$?

case $EXIT_CODE in
  0)
    echo "✓ All verifications passed"
    ;;
  1)
    echo "✗ Root hash mismatch (CLI)"
    exit 1
    ;;
  2)
    echo "✗ Root hash mismatch (JSON)"
    exit 1
    ;;
  3)
    echo "✗ Proofs mismatch (JSON)"
    exit 1
    ;;
  *)
    echo "✗ Unknown error"
    exit 1
    ;;
esac
```

## Testing Exit Codes

Run the test script to verify all exit codes work correctly:

```bash
chmod +x test_exit_codes.sh
./test_exit_codes.sh
```

## Error Messages

Each exit code is accompanied by a descriptive error message:

- **Exit 1**: `✗ ERROR: Root hash provided via CLI does not match!`
- **Exit 2**: `✗ ERROR: Root hash in reference JSON does not match!`
- **Exit 3**: `✗ ERROR: Proofs in reference JSON do not match!`

## Verbose Mode

Use `--verbose` flag to see detailed comparison reports:

```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --compare-json reference.json \
  --verbose
```

This will show:
- Which addresses are missing or extra
- Which allocations don't match
- Which proofs differ
- Exact comparison details


## Merkle Trie generator and comparison CLI tool (TypeScript's Viem compatible) extended version with comprehensive comparisons and show leaves content option

## Leaf Content Display Documentation

## Overview

The `--show-leaf-content` flag provides detailed information about how each leaf in the Merkle tree is constructed from the input data.

## Usage

```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --show-leaf-content
```

## Output Format

For each leaf, the following information is displayed:

### Without `--keep-prefix`

```
Leaf [0]:
  Address:      0x742C4d97C86bCF0176776C16e073b8c6f9Db4021
  Amount:       1000000000000000000 wei
  Amount (ETH): 1.0000 ETH
  Packed data:  52 bytes total
    - Address:           20 bytes
      0x742c4d97c86bcf0176776c16e073b8c6f9db4021
    - Amount (uint256):  32 bytes
      0x0000000000000000000000000000000000000000000000000de0b6b3a7640000
  Leaf hash:    0x8f7a9d0b3c2e1a4f5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8
```

### With `--keep-prefix`

```
Leaf [0]:
  Address:      0x742C4d97C86bCF0176776C16e073b8c6f9Db4021
  Amount:       1000000000000000000 wei
  Amount (ETH): 1.0000 ETH
  Packed data:  74 bytes total
    - Address (with 0x): 42 bytes
      Raw: 0x742C4d97C86bCF0176776C16e073b8c6f9Db4021
      Hex: 0x307837343243346439374338366243463031373637373643313665303733623863366639446234303231
    - Amount (uint256):  32 bytes
      0x0000000000000000000000000000000000000000000000000de0b6b3a7640000
  Leaf hash:    0x1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b
```

## Fields Explained

| Field | Description |
|-------|-------------|
| **Index** | Position of the leaf in the tree (0-based) |
| **Address** | Ethereum address from CSV |
| **Amount (wei)** | Raw amount in smallest unit |
| **Amount (ETH)** | Human-readable amount in ETH |
| **Packed data** | Total bytes of concatenated address + amount |
| **Address bytes** | Hexadecimal representation of address (20 bytes without prefix, or raw ASCII with prefix) |
| **Amount bytes** | 32-byte big-endian representation of the amount |
| **Leaf hash** | keccak256 hash of the packed data |

## Packed Data Structure

### Standard Mode (default)

```
| Address (20 bytes) | Amount (32 bytes) |
|--------------------+-------------------|
| 0x742c4d...        | 0x000000...640000 |
```

Total: 52 bytes → keccak256 → 32-byte leaf hash

### Keep Prefix Mode (`--keep-prefix`)

```
| Address with "0x" (42 ASCII bytes) | Amount (32 bytes) |
|------------------------------------+-------------------|
| "0x742C4d..."                      | 0x000000...640000 |
```

Total: 74 bytes → keccak256 → 32-byte leaf hash

## Combining with Other Options

### Show Everything
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --show-leaf-content \
  --show-leaves \
  --show-tree \
  --verbose \
  --pretty
```

### Show Leaf Content and Verify
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --output output.json \
  --show-leaf-content \
  --compare-json reference.json \
  --verbose
```

### Debug Specific Encoding
```bash
# See how keeping the prefix affects leaf hashes
cargo run --bin merkle-cli-viem-compat -- \
  --input data.csv \
  --show-leaf-content \
  --keep-prefix
```

## Use Cases

1. **Debugging**: Verify that address and amount encoding is correct
2. **Auditing**: Review exact byte-level construction of leaves
3. **Documentation**: Generate detailed reports of tree construction
4. **Troubleshooting**: Compare packed data when root hashes don't match
5. **Learning**: Understand how viem's `encodePacked` works

## Example Output Interpretation

Given this input:
```csv
address,allocation
0x742C4d97C86bCF0176776C16e073b8c6f9Db4021,1000000000000000000
```

The tool shows:
- Address is converted to checksum format
- Amount 1000000000000000000 wei = 1 ETH
- Packed data combines address (20 bytes) + amount (32 bytes)
- Final leaf hash is keccak256 of those 52 bytes

This matches viem's:
```typescript
keccak256(encodePacked(["address", "uint256"], [address, amount]))
```

## Complete Usage Examples:

### 1. Basic leaf content display:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --show-leaf-content
```

### 2. Detailed breakdown with prefix:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --show-leaf-content \
  --keep-prefix \
  --verbose
```

### 3. Complete analysis:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --output output.json \
  --show-leaf-content \
  --show-leaves \
  --show-tree \
  --compare-json reference.json \
  --verbose \
  --pretty
```

### 4. Quick leaf inspection:
```bash
cargo run --bin merkle-cli-viem-compat -- \
  --input example.csv \
  --show-leaf-content | grep -A 10 "Leaf \[0\]"
```

The `--show-leaf-content` option now provides comprehensive information about:
- Original address and amount
- Human-readable ETH conversion
- Byte-level breakdown of packed data
- Separate display for address and amount components
- Final leaf hash

This is extremely useful for debugging, auditing, and understanding exactly how the Merkle tree is constructed!

This is the complete implementation with all features including:
- ✅ `--show-leaf-content` for detailed leaf inspection
- ✅ `--keep-prefix` for including 0x in leaf data
- ✅ `--compare-root` for CLI root hash comparison
- ✅ `--compare-json` for full JSON comparison
- ✅ Exit codes: 0 (success), 1 (CLI root mismatch), 2 (JSON root mismatch), 3 (JSON proofs mismatch)
- ✅ Full viem/TypeScript compatibility
- ✅ EIP-55 checksum address support
- ✅ Comprehensive test suite
