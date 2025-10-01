## Merkle Trie generator and comparison CLI tool

## Usage:

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

## CLI Options:

- `-i, --input <PATH>`: Input CSV file path (required)
- `-o, --output <PATH>`: Output JSON file path (required)
- `-v, --verbose`: Print root hash and statistics to stdout
- `-p, --pretty`: Pretty print JSON output

## Expected Output Format:

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
