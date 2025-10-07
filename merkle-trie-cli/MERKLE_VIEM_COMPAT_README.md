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

