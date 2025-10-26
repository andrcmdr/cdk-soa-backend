# Load Tester CLI

A comprehensive CLI tool for blockchain load testing and benchmarking using the LoadTester smart contract.

## Features

- 🔥 **Multiple Test Scenarios** - Basic, Storage, Calldata, Batch, External Calls, Crypto, Mixed, Stress, Endurance
- 📊 **Detailed Statistics** - TPS, latency percentiles, gas usage, success rates
- ⚡ **Concurrent Execution** - Configurable worker threads
- 🎯 **Rate Limiting** - Control transaction rate
- 📈 **Progress Tracking** - Real-time progress bars
- 💾 **Results Export** - JSON, CSV, text formats
- 🔧 **Flexible Configuration** - CLI arguments or JSON config files

## Installation

```bash
cd load-tester-cli
cargo build --release
```

The binary will be available at `target/release/load-tester-cli`.

## Quick Start

### 1. Basic Load Test

```bash
load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x1234567890123456789012345678901234567890 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  basic -n 1000 -i 100
```

### 2. Storage Stress Test

```bash
load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x1234567890123456789012345678901234567890 \
  --private-key 0x... \
  storage -w 20 -r 20 -n 5000
```

### 3. Mixed Workload

```bash
load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x1234567890123456789012345678901234567890 \
  --private-key 0x... \
  mixed -d 300 -p balanced
```

### 4. Stress Test with Ramping

```bash
load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x1234567890123456789012345678901234567890 \
  --private-key 0x... \
  stress --ramp-up 60 --peak 300 --ramp-down 60 --target-tps 1000
```

## Test Scenarios

### Basic Load Test

Tests basic transaction throughput by calling `consumeGas()`.

```bash
load-tester-cli basic [OPTIONS]

Options:
  -n, --count <COUNT>          Number of transactions [default: 1000]
  -i, --iterations <ITERATIONS> Gas loop iterations [default: 100]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  basic -n 5000 -i 200
```

### Storage Stress Test

Tests storage read/write operations.

```bash
load-tester-cli storage [OPTIONS]

Options:
  -w, --writes <WRITES>  Number of storage writes [default: 10]
  -r, --reads <READS>    Number of storage reads [default: 10]
  -n, --count <COUNT>    Number of transactions [default: 1000]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  storage -w 50 -r 50 -n 2000
```

### Calldata Size Test

Tests varying calldata sizes.

```bash
load-tester-cli calldata [OPTIONS]

Options:
  --min-size <MIN_SIZE>      Minimum size in bytes [default: 100]
  --max-size <MAX_SIZE>      Maximum size in bytes [default: 10000]
  --increment <INCREMENT>    Size increment [default: 1000]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  calldata --min-size 100 --max-size 50000 --increment 5000
```

### Batch Token Minting

Tests batch minting operations for ERC-20, ERC-721, or ERC-1155.

```bash
load-tester-cli batch-mint [OPTIONS]

Options:
  -t, --token-type <TYPE>      Token type [possible: erc20, erc721, erc1155]
  -b, --batch-size <SIZE>      Batch size [default: 10]
  -n, --batches <BATCHES>      Number of batches [default: 100]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  batch-mint -t erc20 -b 50 -n 200
```

### External Call Test

Tests external contract interactions.

```bash
load-tester-cli external-call [OPTIONS]

Options:
  -t, --call-type <TYPE>     Call type [possible: call, delegatecall, staticcall]
  -g, --gas-limit <LIMIT>    Gas limit per call [default: 100000]
  -n, --count <COUNT>        Number of calls [default: 1000]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  external-call -t call -g 200000 -n 3000
```

### Cryptography Test

Tests signature verification or Merkle proof verification.

```bash
load-testerc-cli crypto [OPTIONS]

Options:
  -t, --test-type <TYPE>     Test type [possible: signature, merkle]
  -n, --count <COUNT>        Number of verifications [default: 1000]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  crypto -t merkle -n 5000
```

### Mixed Workload

Simulates realistic mixed operations.

```bash
load-tester-cli mixed [OPTIONS]

Options:
  -d, --duration <DURATION>  Duration in seconds [default: 300]
  -p, --profile <PROFILE>    Workload profile [possible: balanced, storage-heavy, compute-heavy, calldata-heavy]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  mixed -d 600 -p storage-heavy
```

### Stress Test

Gradually increases load to find breaking points.

```bash
load-tester-cli stress [OPTIONS]

Options:
  --ramp-up <SECONDS>        Ramp up duration [default: 60]
  --peak <SECONDS>           Peak duration [default: 300]
  --ramp-down <SECONDS>      Ramp down duration [default: 60]
  --target-tps <TPS>         Target TPS at peak [default: 1000]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  stress --ramp-up 120 --peak 600 --ramp-down 120 --target-tps 2000
```

### Endurance Test

Sustained load over extended periods.

```bash
load-tester-cli endurance [OPTIONS]

Options:
  -d, --hours <HOURS>  Test duration in hours [default: 24]
  -t, --tps <TPS>      Target TPS [default: 100]
```

**Example:**
```bash
load-tester-cli --rpc-url http://localhost:8545 --contract 0x... --private-key 0x... \
  endurance -d 48 -t 200
```

## Global Options

```bash
Options:
  -r, --rpc-url <URL>              RPC endpoint URL [env: RPC_URL]
  -c, --contract <ADDRESS>         Contract address [env: CONTRACT_ADDRESS]
  -k, --private-key <KEY>          Private key [env: PRIVATE_KEY]
      --chain-id <ID>              Chain ID [default: 1] [env: CHAIN_ID]
  -a, --abi <PATH>                 Path to ABI file [default: abi/LoadTester.json]
  -w, --workers <COUNT>            Number of workers [default: 10]
  -d, --duration <SECONDS>         Test duration [default: 60]
  -r, --rate-limit <TPS>           Rate limit (0 = unlimited) [default: 0]
  -o, --output <FORMAT>            Output format [possible: text, json, csv]
      --save-results <PATH>        Save results to file
  -v, --verbose                    Verbose output
  -h, --help                       Print help
  -V, --version                    Print version
```

## Using Configuration Files

Create a JSON config file and use it with the `custom` scenario:

```bash
load-tester-cli custom -f config.examples/basic-load.json
```

**Config file structure:**
```json
{
  "rpc_url": "http://localhost:8545",
  "contract_address": "0x1234...",
  "private_key": "0xac09...",
  "chain_id": 1,
  "abi_path": "abi/LoadTester.json",
  "workers": 10,
  "duration": 60,
  "rate_limit": 0,
  "scenario": {
    "type": "Basic",
    "count": 1000,
    "iterations": 100
  }
}
```

## Output Examples

### Text Output (Default)

```
╔══════════════════════════════════════════════════════╗
║      🔥 BLOCKCHAIN LOAD TESTER 🔥                   ║
║      Network Benchmarking & Stress Testing          ║
╚══════════════════════════════════════════════════════╝

Configuration:
  RPC URL: http://localhost:8545
  Contract: 0x1234567890123456789012345678901234567890
  Workers: 10
  Duration: 60s

Starting load test...
█████████████████████████████████████████ 1000/1000 (100%) TPS: 156.32

═══════════════════════════════════════
Test Results
═══════════════════════════════════════

Overall Statistics:
  Total Duration: 6.41s
  Total Transactions: 1000
  Successful: 998 (99.8%)
  Failed: 2

Performance:
  Average TPS: 156.32
  Average Latency: 45.23ms
  P50 Latency: 42.10ms
  P95 Latency: 68.45ms
  P99 Latency: 89.12ms
  Max Latency: 125.67ms

Gas Usage:
  Total Gas: 100000000
  Average Gas per TX: 100200.40

═══════════════════════════════════════
```

### JSON Output

```bash
load-tester-cli --output json --save-results results.json basic -n 1000
```

### CSV Export

```bash
load-tester-cli --output csv --save-results results.csv basic -n 1000
```

## Environment Variables

Set environment variables to avoid repeating common parameters:

```bash
export RPC_URL=http://localhost:8545
export CONTRACT_ADDRESS=0x1234567890123456789012345678901234567890
export PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
export CHAIN_ID=1

# Now you can run tests without those flags
load-tester-cli basic -n 1000
```

## Performance Tips

### 1. Optimize Workers

- Start with 10 workers
- Increase gradually to find optimal concurrency
- Too many workers can overwhelm the RPC endpoint

### 2. Rate Limiting

Use rate limiting to avoid overwhelming the network:

```bash
load-tester-cli --rate-limit 100 basic -n 10000
```

### 3. Batch Operations

For high throughput, use batch scenarios:

```bash
load-tester-cli batch-mint -t erc20 -b 100 -n 1000
```

### 4. Connection Tuning

- Use local RPC nodes when possible
- Consider WebSocket connections for better performance
- Ensure sufficient network bandwidth

## Troubleshooting

### Connection Issues

```bash
# Test connection first
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  http://localhost:8545
```

### Gas Limit Errors

Increase gas limits in contract configuration:

```bash
load-tester-cli external-call -g 500000  # Increase from default
```

### Nonce Issues

- Reduce workers if seeing nonce errors
- Use rate limiting
- Ensure single private key isn't used by multiple processes

### Memory Issues

For long-running tests:
- Use endurance test with periodic reporting
- Export results incrementally
- Monitor system resources

## Advanced Usage

### Custom Scenarios

Create complex test scenarios with JSON configs:

```json
{
  "scenario": {
    "type": "Mixed",
    "duration": 3600,
    "profile": "Custom",
    "operations": [
      { "type": "storage", "weight": 0.4 },
      { "type": "compute", "weight": 0.3 },
      { "type": "calldata", "weight": 0.3 }
    ]
  }
}
```

### Scripted Testing

```bash
#!/bin/bash
# Progressive load test

for tps in 100 200 500 1000 2000; do
  echo "Testing at ${tps} TPS..."
  load-tester-cli --rate-limit $tps basic -n 10000 \
    --save-results "results_${tps}tps.json"
  sleep 60
done
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Run Load Test
  run: |
    load-tester-cli \
      --rpc-url ${{ secrets.RPC_URL }} \
      --contract ${{ secrets.CONTRACT }} \
      --private-key ${{ secrets.PRIVATE_KEY }} \
      basic -n 1000 \
      --save-results results.json

- name: Upload Results
  uses: actions/upload-artifact@v2
  with:
    name: load-test-results
    path: results.json
```

## License

Licensed under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Support

For issues and questions:
- GitHub Issues: https://github.com/andrcmdr/cdk-soa-backend/issues
- Documentation: https://github.com/andrcmdr/cdk-soa-backend/tree/main/load-tester-cli/USAGE_GUIDE.md

## Contributing

Contributions are welcomed! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
