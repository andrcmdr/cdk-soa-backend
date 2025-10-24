#!/bin/bash
# Comprehensive benchmark suite

set -e

# Configuration
RPC_URL="${RPC_URL:-http://localhost:8545}"
CONTRACT="${CONTRACT:-0x1234567890123456789012345678901234567890}"
PRIVATE_KEY="${PRIVATE_KEY:-}"
RESULTS_DIR="results/$(date +%Y%m%d_%H%M%S)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}  Blockchain Load Testing Benchmark Suite${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"

# Check prerequisites
if [ -z "$PRIVATE_KEY" ]; then
    echo -e "${RED}Error: PRIVATE_KEY environment variable not set${NC}"
    exit 1
fi

echo -e "${YELLOW}Configuration:${NC}"
echo "  RPC URL: $RPC_URL"
echo "  Contract: $CONTRACT"
echo "  Results: $RESULTS_DIR"
echo ""

# Function to run a test
run_test() {
    local name=$1
    shift
    local args=$@

    echo -e "${YELLOW}Running: $name${NC}"
    load-tester \
        --rpc-url "$RPC_URL" \
        --contract "$CONTRACT" \
        --private-key "$PRIVATE_KEY" \
        --save-results "$RESULTS_DIR/${name}.json" \
        $args

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ $name completed${NC}"
    else
        echo -e "${RED}✗ $name failed${NC}"
    fi
    echo ""
}

# 1. Basic throughput test
run_test "basic_light" basic -n 1000 -i 50 -w 10

# 2. Basic heavy test
run_test "basic_heavy" basic -n 5000 -i 200 -w 20

# 3. Storage read-heavy
run_test "storage_read" storage -w 5 -r 50 -n 1000

# 4. Storage write-heavy
run_test "storage_write" storage -w 50 -r 5 -n 1000

# 5. Calldata test
run_test "calldata" calldata --min-size 100 --max-size 10000 --increment 1000

# 6. Mixed workload
run_test "mixed_balanced" mixed -d 300 -p balanced -w 30

# 7. Stress test
run_test "stress" stress --ramp-up 60 --peak 180 --ramp-down 60 --target-tps 500

echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}  Benchmark Suite Complete${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "Summary:"
for result in "$RESULTS_DIR"/*.json; do
    name=$(basename "$result" .json)
    tps=$(jq -r '.tps // "N/A"' "$result" 2>/dev/null || echo "N/A")
    success=$(jq -r '.success_rate // "N/A"' "$result" 2>/dev/null || echo "N/A")
    echo "  $name: ${tps} TPS, ${success}% success"
done
