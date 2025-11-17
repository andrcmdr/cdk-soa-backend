# ABI2SOL - ABI to Solidity Interface Converter

A command-line tool that converts Ethereum ABI JSON files into Solidity interface declarations compatible with Alloy's `sol!()` macro.

## Overview

`abi2sol` is a Rust-based utility that parses Ethereum contract ABI JSON and generates clean, readable Solidity interface code. This is particularly useful when working with Alloy framework, as it produces interface definitions ready to be used with the `sol!` macro for type-safe contract interactions.

## Features

- **Complete ABI Support**: Handles functions, events, errors, constructors, fallback, and receive functions
- **Struct Extraction**: Automatically extracts and generates struct definitions from tuple types
- **Function Categorization**: Optionally categorize functions by type (pure, view, payable, state-changing)
- **Flexible Input**: Read from files or stdin
- **Customizable Output**: Control what to include (events, errors, types) and output format
- **Compact Mode**: Generate minimal output without comments

## Installation

### Prerequisites

- Rust 1.91.0 or later
- Cargo package manager

### Building from Source

```bash
# Clone or navigate to the project directory
cd abi2sol

# Build the release binary
cargo build --release

# The binary will be available at:
# target/release/abi2sol
```

### Installation

```bash
# Install globally
cargo install --path .

# Or run directly with cargo
cargo run -- [OPTIONS] [FILE]
```

## Usage

### Basic Usage

```bash
# Convert ABI from a file
abi2sol contract.abi.json

# Read from stdin
cat contract.abi.json | abi2sol -

# Or using process substitution
abi2sol - < contract.abi.json
```

### Common Examples

```bash
# Generate interface with custom name
abi2sol --interface-name IMyToken token.abi.json

# Categorize functions by type
abi2sol --categorize contract.abi.json

# Compact output without comments
abi2sol --compact contract.abi.json

# Exclude events and errors
abi2sol --events false --errors false contract.abi.json

# Only show functions (no events, errors, or types)
abi2sol -e false -r false -t false contract.abi.json
```

## CLI Reference

### Synopsis

```bash
abi2sol [OPTIONS] [FILE]
```

### Arguments

#### `[FILE]`

Path to the ABI JSON file to convert. Use `-` to read from stdin.

- **Type**: Path to file or `-` for stdin
- **Required**: No (defaults to stdin if omitted)
- **Examples**:
  - `contract.abi.json`
  - `-` (read from stdin)

### Options

#### `-i, --interface-name <NAME>`

Specify the name of the generated Solidity interface.

- **Type**: String
- **Default**: `IContract`
- **Example**: `--interface-name IUniswapV2Router`

```bash
abi2sol -i IMyContract contract.abi.json
# Output: interface IMyContract { ... }
```

#### `-e, --events <BOOL>`

Control whether to include event definitions in the output.

- **Type**: Boolean (`true` or `false`)
- **Default**: `true`
- **Example**: `--events false` or `-e false`

```bash
# Exclude events
abi2sol --events false contract.abi.json

# Explicitly include events (default behavior)
abi2sol --events true contract.abi.json
```

#### `-r, --errors <BOOL>`

Control whether to include error definitions in the output.

- **Type**: Boolean (`true` or `false`)
- **Default**: `true`
- **Example**: `--errors false` or `-r false`

```bash
# Exclude errors
abi2sol --errors false contract.abi.json

# Explicitly include errors (default behavior)
abi2sol -r true contract.abi.json
```

**Note**: The short flag `-r` is used (instead of `-e`) to avoid conflict with the `--events` option.

#### `-t, --types <BOOL>`

Control whether to include struct/type definitions extracted from tuple parameters in the output.

- **Type**: Boolean (`true` or `false`)
- **Default**: `true`
- **Example**: `--types false` or `-t false`

```bash
# Exclude struct definitions
abi2sol --types false contract.abi.json

# Explicitly include types (default behavior)
abi2sol -t true contract.abi.json
```

#### `--categorize`

Enable categorization of functions by their type. When enabled, functions are grouped into sections:
- Constructor
- Pure functions (no state read or write)
- View functions (read-only)
- Payable functions (can receive ETH)
- State-changing functions (non-payable, modifies state)
- Fallback function
- Receive function

- **Type**: Flag (boolean)
- **Default**: `false`
- **Example**: `--categorize`

```bash
# Generate categorized output
abi2sol --categorize contract.abi.json

# Default behavior (no categorization)
abi2sol contract.abi.json
```

**Output Example with `--categorize`**:
```solidity
interface IContract {
    // Constructor
    constructor(address _owner);

    // View functions (read-only)
    function balanceOf(address owner) external view returns (uint256);
    function totalSupply() external view returns (uint256);

    // State-changing functions
    function transfer(address to, uint256 amount) external returns (bool);

    // Payable functions (can receive ETH)
    function deposit() external payable;
}
```

#### `-c, --compact`

Generate compact output without comments. Removes all descriptive comments from the output.

- **Type**: Flag (boolean)
- **Default**: `false`
- **Example**: `--compact` or `-c`

```bash
# Compact output
abi2sol --compact contract.abi.json

# Default output with comments
abi2sol contract.abi.json
```

**Comparison**:

Default output:
```solidity
// Generated with abi2sol
// Usage: sol! { ... }

interface IContract {
    // Functions
    function transfer(address to, uint256 amount) external returns (bool);

    // Events
    event Transfer(address indexed from, address indexed to, uint256 amount);
}
```

Compact output (`--compact`):
```solidity
interface IContract {
    function transfer(address to, uint256 amount) external returns (bool);
    event Transfer(address indexed from, address indexed to, uint256 amount);
}
```

#### `-h, --help`

Display help information about the tool and its options.

```bash
abi2sol --help
```

#### `-V, --version`

Display the version information.

```bash
abi2sol --version
```

## Advanced Usage Examples

### Combining Multiple Options

```bash
# Generate a compact, categorized interface with custom name
abi2sol --interface-name IERC20 --categorize --compact token.abi.json

# Only functions and events, no errors or types, with categorization
abi2sol -r false -t false --categorize contract.abi.json

# Minimal output: only functions
abi2sol -e false -r false -t false -c contract.abi.json
```

### Using with Pipelines

```bash
# Fetch ABI from Etherscan and convert
curl -s "https://api.etherscan.io/api?module=contract&action=getabi&address=0x..." \
  | jq -r .result \
  | abi2sol --interface-name IFetchedContract

# Process multiple ABIs
for file in contracts/*.abi.json; do
  abi2sol "$file" > "interfaces/$(basename "$file" .abi.json).sol"
done

# Convert and save to file
abi2sol contract.abi.json > IContract.sol
```

### Integration with Alloy

The generated interface can be directly used with Alloy's `sol!` macro:

```rust
use alloy::sol;

// Paste the generated interface here
sol! {
    interface IERC20 {
        function balanceOf(address owner) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);

        event Transfer(address indexed from, address indexed to, uint256 amount);

        error InsufficientBalance(uint256 available, uint256 required);
    }
}

// Use the generated types
let balance = IERC20::balanceOfCall { owner: address };
```

## Output Format

The tool generates Solidity interface code with the following structure:

1. **Header Comments** (unless `--compact` is used)
2. **Interface Declaration** with the specified name
3. **Constructor** (if present in ABI)
4. **Struct Definitions** (if `--types` is enabled)
5. **Functions** (categorized if `--categorize` is enabled)
6. **Events** (if `--events` is enabled)
7. **Errors** (if `--errors` is enabled)

### Function Categorization

When using `--categorize`, functions are organized by their state mutability:

- **Pure**: Functions that don't read or modify state
- **View**: Functions that read state but don't modify it
- **Payable**: Functions that can receive ETH
- **State-changing**: Non-payable functions that modify state
- **Special functions**: Constructor, fallback, and receive

### Type Handling

The tool intelligently handles:

- **Primitive types**: `uint256`, `address`, `bool`, `bytes`, etc.
- **Arrays**: Fixed and dynamic arrays (`uint256[]`, `address[5]`)
- **Tuples/Structs**: Automatically extracted and named based on parameter names or internal types
- **Mappings**: Preserved from internal type information when available
- **Custom types**: Uses internal type information for accurate type names

## Error Handling

The tool provides clear error messages for common issues:

- **Invalid JSON**: If the ABI JSON is malformed
- **File not found**: If the specified file doesn't exist
- **Parse errors**: If the JSON doesn't match the expected ABI schema

Example error output:
```bash
$ abi2sol invalid.json
Error: Failed to read file: invalid.json

Caused by:
    No such file or directory (os error 2)
```

## Technical Details

### Dependencies

- **alloy**: Ethereum ABI parsing and types (v1.0)
- **clap**: Command-line argument parsing (v4.5)
- **serde/serde_json**: JSON serialization (v1.0)
- **anyhow**: Error handling (v1.0)

### Performance

The release build is optimized for size and performance:
- LTO (Link Time Optimization) enabled
- Strip symbols for smaller binary
- Panic abort for reduced code size
- Optimization level `z` for minimal binary size

## Limitations and Known Issues

1. **Struct Naming**: When structs don't have internal type information, they're named based on parameter names (converted to PascalCase) or use generic names
2. **Fallback Function Parameters**: Currently, fallback functions are always generated as `fallback() external [payable]` without parameters
3. **Overloaded Functions**: Functions with the same name but different signatures are all included without special handling
4. **Library Functions**: Library-specific modifiers are not preserved

## Contributing

When contributing to this project, please ensure:

1. Code follows Rust best practices and style guidelines
2. All new features include appropriate documentation
3. CLI options maintain consistency with existing patterns
4. Error messages are clear and helpful

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

This project is part of a larger codebase. Refer to the root project for license information as well.

## Version History

- **v0.1.0**: Initial release with core functionality
  - ABI to Solidity conversion
  - Support for functions, events, errors, and constructors
  - Function categorization
  - Struct extraction
  - Flexible input/output options
