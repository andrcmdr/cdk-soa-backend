# v0.1.0

## Summary

First version of a complete, battle-testing-ready load testing CLI tool with:

### ✅ Features Implemented:

1. **9 Test Scenarios**:
   - Basic load test
   - Storage stress test
   - Calldata size test
   - Batch token minting
   - External call test
   - Cryptography test
   - Mixed workload
   - Stress test with ramping
   - Endurance test

2. **Comprehensive Statistics**:
   - TPS tracking
   - Latency percentiles (P50, P95, P99)
   - Gas usage
   - Success rates
   - HDR histogram for accurate percentiles

3. **Advanced Features**:
   - Concurrent execution with configurable workers
   - Rate limiting
   - Progress bars
   - Multiple output formats (text, JSON, CSV)
   - Configuration files
   - Environment variable support

4. **Documentation**:
   - Complete README
   - Usage guide
   - Example configurations
   - Benchmark script

### 📁 Project Structure:
```
load-tester-cli/
├── Cargo.toml
├── README.md
├── USAGE_GUIDE.md
├── CONTRACT_INTERFACE.md
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── runner.rs
│   ├── stats.rs
│   └── scenarios/
│       ├── mod.rs
│       ├── basic.rs
│       ├── storage.rs
│       ├── calldata.rs
│       ├── batch_mint.rs
│       ├── external_call.rs
│       ├── crypto.rs
│       ├── mixed.rs
│       ├── stress.rs
│       └── endurance.rs
├── config.examples/
│   ├── basic-load.json
│   ├── storage-stress.json
│   └── mixed-workload.json
└── scripts/
    └── run-benchmarks.sh
```

### 🚀 Usage:

```bash
# Build
cd load-tester-cli
cargo build --release

# Run basic test
./target/release/load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x... \
  --private-key 0x... \
  basic -n 1000 -i 100

# Run stress test
./target/release/load-tester-cli \
  --rpc-url http://localhost:8545 \
  --contract 0x... \
  --private-key 0x... \
  stress --ramp-up 60 --peak 300 --ramp-down 60 --target-tps 1000

# Run full benchmark suite
./scripts/run-benchmarks.sh
```

This tool is battle-testing-ready and can be used for comprehensive blockchain network benchmarking and stress testing.
