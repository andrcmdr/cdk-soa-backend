## Installation and Usage:

### 1. Install dependencies:
```bash
# Install Python 3, PIP and VEnv
sudo apt-get install python3 python3-pip python3-venv
# Create virtual environment
python3 -m venv ./.venv/
# Activate virtual environment
# For Bash
source ./.venv/bin/activate
# For Fish
source ./.venv/bin/activate.fish
# Install dependencies
pip3 install -r requirements.txt
```

### 2. Run basic script:
```bash
python3 merkle-cli.py example.csv
```

### 3. Generate JSON output:
```bash
# Basic usage
python3 merkle-cli-json.py -i example.csv -o output.json -v -p

# Without file output (print to stdout)
python3 merkle-cli-json.py -i example.csv -v

# Minimal output
python3 merkle-cli-json.py -i example.csv -o output.json
```

### 4. Run compatibility tests:
```bash
python3 test_compatibility.py
```

## Key Features:

1. ✅ **Exact TypeScript equivalence**: Uses same hashing algorithm (keccak256)
2. ✅ **Checksum addresses**: Properly handles EIP-55 checksum addresses
3. ✅ **Sorted pair hashing**: Implements same sorting logic as TypeScript
4. ✅ **Packed encoding**: Matches viem's `encodePacked` behavior
5. ✅ **JSON output**: Compatible with Rust CLI output format
6. ✅ **CLI interface**: Similar flags to Rust implementation
7. ✅ **Comprehensive tests**: Verifies compatibility with TypeScript/Rust

## Comparison with TypeScript:

| Feature | TypeScript (viem) | Python (this implementation) |
|---------|------------------|------------------------------|
| Keccak256 | `keccak256()` | `eth_utils.keccak()` |
| Address checksum | `checksumAddress()` | `to_checksum_address()` |
| Packed encoding | `encodePacked()` | Manual bytes concatenation |
| Pair sorting | Manual `if (left >= right)` | Same logic |
| CSV reading | `csvtojson` | `csv.DictReader` |

The Python implementation produces **identical hashes** as the TypeScript version!
