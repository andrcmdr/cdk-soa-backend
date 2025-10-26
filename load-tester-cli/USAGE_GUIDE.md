# Load Tester CLI - Comprehensive Usage Guide

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Concepts](#basic-concepts)
3. [Test Scenarios Explained](#test-scenarios-explained)
4. [Interpreting Results](#interpreting-results)
5. [Best Practices](#best-practices)
6. [Real-World Examples](#real-world-examples)
7. [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

1. **LoadTester Contract Deployed**
   - Deploy the LoadTester contract to your target network
   - Note the contract address

2. **RPC Access**
   - HTTP or WebSocket RPC endpoint
   - Sufficient rate limits for your tests

3. **Private Key**
   - Account with sufficient native tokens for gas
   - Keep private keys secure

4. **ABI File**
   - Save the LoadTester ABI as JSON
   - Default location: `abi/LoadTester.json`

### Installation

```bash
# Clone repository
git clone https://github.com/andrcmdr/cdk-soa-backend
cd cdk-soa-backend/load-tester-cli

# Build
cargo build --release

# Install (optional)
cargo install --path .
```

## Basic Concepts

### Workers

Workers are concurrent execution threads. More workers = higher potential throughput.

- **Low workers (1-5)**: Sequential testing, precise timing
- **Medium workers (10-50)**: Moderate load, good for most tests
- **High workers (50-200)**: Stress testing, maximum throughput

### Rate Limiting

Controls transactions per second (TPS).

- **No limit (0)**: Maximum throughput
- **Limited (e.g., 100)**: Controlled, sustainable load
- **Very limited (e.g., 10)**: Precise timing, consistency testing

### Duration

- **Short (< 60s)**: Quick tests, experimentation
- **Medium (60-600s)**: Standard benchmarks
- **Long (> 600s)**: Endurance, stability testing

## Test Scenarios Explained

### 1. Basic Load Test

**Purpose**: Measure pure transaction throughput

**When to use**:
- Initial network benchmarking
- Comparing different configurations
- Establishing baseline performance

**Parameters**:
```bash
-n, --count       # Total transactions
-i, --iterations  # Work per transaction (affects gas)
```

**Example**:
```bash
# Light load
load-tester-cli basic -n 1000 -i 50 -w 10

# Heavy load
load-tester-cli basic -n 10000 -i 500 -w 50
```

### 2. Storage Stress Test

**Purpose**: Test storage operations (SLOAD/SSTORE)

**When to use**:
- Testing state-heavy applications
- Measuring storage performance
- Comparing storage costs

**Parameters**:
```bash
-w, --writes  # Storage writes per tx
-r, --reads   # Storage reads per tx
-n, --count   # Total transactions
```

**Example**:
```bash
# Read-heavy
load-tester-cli storage -w 5 -r 50 -n 1000

# Write-heavy
load-tester-cli storage -w 50 -r 5 -n 1000

# Balanced
load-tester-cli storage -w 20 -r 20 -n 1000
```

### 3. Calldata Size Test

**Purpose**: Measure calldata costs and limits

**When to use**:
- Testing large data uploads
- Measuring calldata pricing
- Finding size limits

**Parameters**:
```bash
--min-size     # Starting size
--max-size     # Maximum size
--increment    # Size increase per step
```

**Example**:
```bash
# Test from 1KB to 100KB
load-tester-cli calldata --min-size 1000 --max-size 100000 --increment 10000
```

### 4. Batch Minting

**Purpose**: Test ERC token batch operations

**When to use**:
- NFT launch preparation
- Token distribution planning
- Batch operation optimization

**Parameters**:
```bash
-t, --token-type   # erc20, erc721, erc1155
-b, --batch-size   # Recipients per batch
-n, --batches      # Number of batches
```

**Example**:
```bash
# ERC-20 airdrop
load-tester-cli batch-mint -t erc20 -b 500 -n 20  # 10,000 recipients

# ERC-721 NFT launch
load-tester-cli batch-mint -t erc721 -b 100 -n 100  # 10,000 NFTs
```

### 5. External Call Test

**Purpose**: Measure external call overhead

**When to use**:
- Testing contract interactions
- Measuring call gas costs
- Validating integration patterns

**Example**:
```bash
# Standard calls
load-tester-cli external-call -t call -g 100000 -n 1000

# Delegate calls
load-tester-cli external-call -t delegatecall -g 200000 -n 500
```

### 6. Cryptography Test

**Purpose**: Test cryptographic operations

**When to use**:
- Merkle tree verification testing
- Signature verification benchmarks
- Cryptographic gas cost analysis

**Example**:
```bash
# Merkle proof verification
load-tester-cli crypto -t merkle -n 5000

# Signature verification
load-tester-cli crypto -t signature -n 5000
```

### 7. Mixed Workload

**Purpose**: Simulate realistic application usage

**When to use**:
- Production readiness testing
- Realistic load simulation
- Long-term testing

**Profiles**:
- `balanced`: Equal mix of operations
- `storage-heavy`: More storage operations (60%)
- `compute-heavy`: More computation (60%)
- `calldata-heavy`: More calldata (60%)

**Example**:
```bash
# 10-minute balanced test
load-tester-cli mixed -d 600 -p balanced -w 20

# Storage-heavy application
load-tester-cli mixed -d 1800 -p storage-heavy -w 30
```

### 8. Stress Test

**Purpose**: Find network limits and breaking points

**Phases**:
1. **Ramp up**: Gradually increase load
2. **Peak**: Maintain maximum load
3. **Ramp down**: Gradually decrease load

**When to use**:
- Capacity planning
- Finding bottlenecks
- Performance limits

**Example**:
```bash
# Find breaking points
load-tester-cli stress \
  --ramp-up 120 \
  --peak 300 \
  --ramp-down 120 \
  --target-tps 2000
```

### 9. Endurance Test

**Purpose**: Long-term stability and consistency

**When to use**:
- Stability validation
- Memory leak detection
- Long-term reliability

**Example**:
```bash
# 24-hour test
load-tester-cli endurance -d 24 -t 100

# 1-week test
load-tester-cli endurance -d 168 -t 50
```

## Interpreting Results

### Key Metrics

#### 1. Transactions Per Second (TPS)

```
Average TPS: 156.32
```

**Interpretation**:
- **< 10 TPS**: Low throughput, potential issues
- **10-100 TPS**: Moderate throughput, typical for complex operations
- **100-1000 TPS**: High throughput, good performance
- **> 1000 TPS**: Very high throughput, excellent performance

#### 2. Success Rate

```
Successful: 998 (99.8%)
Failed: 2
```

**Interpretation**:
- **> 99%**: Excellent reliability
- **95-99%**: Good, investigate failures
- **< 95%**: Poor, significant issues

#### 3. Latency Percentiles

```
Average Latency: 45.23ms
P50 Latency: 42.10ms
P95 Latency: 68.45ms
P99 Latency: 89.12ms
Max Latency: 125.67ms
```

**Interpretation**:
- **P50 (Median)**: Typical user experience
- **P95**: Experience for 95% of users
- **P99**: Worst case for most users
- **Max**: Absolute worst case

**Thresholds**:
- **< 100ms**: Excellent
- **100-500ms**: Good
- **500-1000ms**: Acceptable
- **> 1000ms**: Poor

#### 4. Gas Usage

```
Total Gas: 100,000,000
Average Gas per TX: 100,200.40
```

**Use for**:
- Cost estimation
- Optimization opportunities
- Capacity planning

## Best Practices

### 1. Start Small

```bash
# First run: small test
load-tester-cli basic -n 100 -i 50 -w 5

# If successful: scale up
load-tester-cli basic -n 1000 -i 50 -w 10

# Then: full test
load-tester-cli basic -n 10000 -i 50 -w 50
```

### 2. Use Rate Limiting

```bash
# Don't overwhelm the network
load-tester-cli --rate-limit 100 basic -n 10000
```

### 3. Monitor Resources

```bash
# In another terminal
watch -n 1 'ps aux | grep load-tester-cli'
htop
```

### 4. Save Results

```bash
# Always save for later analysis
load-tester-cli --save-results results_$(date +%Y%m%d_%H%M%S).json basic -n 1000
```

### 5. Incremental Testing

```bash
#!/bin/bash
# Test at different loads
for workers in 5 10 20 50 100; do
  echo "Testing with $workers workers..."
  load-tester-cli -w $workers basic -n 1000 \
    --save-results "results_w${workers}.json"
done
```

## Real-World Examples

### Example 1: NFT Launch Preparation

**Goal**: Verify network can handle 10,000 NFT mints in 1 hour

```bash
# Test configuration
load-tester-cli batch-mint \
  -t erc721 \
  -b 100 \
  -n 100 \
  -w 20 \
  --rate-limit 30 \
  --save-results nft_launch_test.json

# Expected: ~3 TPS, complete in ~30 minutes
# If successful: Network can handle launch
```

### Example 2: DEX Deployment Testing

**Goal**: Simulate DEX usage patterns

```bash
# Mixed workload simulating:
# - Storage (liquidity updates)
# - Compute (price calculations)
# - External calls (token transfers)

load-tester-cli mixed \
  -d 600 \
  -p balanced \
  -w 50 \
  --save-results dex_simulation.json
```

### Example 3: Network Capacity Planning

**Goal**: Find maximum sustainable TPS

```bash
# Progressive stress test
load-tester-cli stress \
  --ramp-up 300 \
  --peak 600 \
  --ramp-down 300 \
  --target-tps 5000 \
  -w 100 \
  --save-results capacity_test.json

# Analyze results to find sustainable TPS
```

### Example 4: Contract Optimization

**Goal**: Compare gas costs before/after optimization

```bash
# Before optimization
load-tester-cli basic -n 1000 \
  --contract 0xOLD_CONTRACT \
  --save-results before.json

# After optimization
load-tester-cli basic -n 1000 \
  --contract 0xNEW_CONTRACT \
  --save-results after.json

# Compare average gas usage
```

## Troubleshooting

### Problem: Low TPS

**Symptoms**: TPS much lower than expected

**Solutions**:
1. Increase workers: `-w 50`
2. Remove rate limiting: `--rate-limit 0`
3. Check RPC performance
4. Verify network conditions

### Problem: High Failure Rate

**Symptoms**: Many failed transactions

**Solutions**:
1. Reduce workers: `-w 5`
2. Add rate limiting: `--rate-limit 50`
3. Check gas prices
4. Verify contract configuration

### Problem: Timeouts

**Symptoms**: Transactions timing out

**Solutions**:
1. Increase RPC timeout in config
2. Reduce concurrent load
3. Check network latency
4. Verify RPC endpoint health

### Problem: Nonce Errors

**Symptoms**: "Nonce too low" errors

**Solutions**:
1. Reduce workers
2. Add rate limiting
3. Ensure single process using key
4. Wait between test runs

### Problem: Out of Gas

**Symptoms**: Transactions reverting

**Solutions**:
1. Increase gas limits in contract
2. Reduce operation complexity
3. Check gas estimation
4. Verify sufficient funds

## Advanced Topics

### Custom Test Development

See source code in `src/scenarios/` for examples of implementing custom test scenarios.

### Automated Testing Pipelines

Integrate with CI/CD:

```yaml
name: Load Test
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run load test
        run: |
          ./load-tester-cli basic -n 1000 --save-results results.json
```

### Result Analysis

Use scripts to analyze JSON results:

```python
import json
import matplotlib.pyplot as plt

with open('results.json') as f:
    data = json.load(f)

# Plot latency distribution
plt.hist(data['latencies_ms'], bins=50)
plt.xlabel('Latency (ms)')
plt.ylabel('Count')
plt.savefig('latency_distribution.png')
```
