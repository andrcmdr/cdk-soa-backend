#!/bin/bash

set -e

echo "====================================="
echo "Testing Merkle Tree CLI Exit Codes"
echo "====================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run test
run_test() {
    local test_name="$1"
    local expected_exit_code=$2
    shift 2
    local command="$@"

    echo -n "Test: ${test_name}... "

    set +e
    eval "$command" > /dev/null 2>&1
    actual_exit_code=$?
    set -e

    if [ $actual_exit_code -eq $expected_exit_code ]; then
        echo -e "${GREEN}PASS${NC} (exit code: $actual_exit_code)"
        ((TESTS_PASSED+=1))
    else
        echo -e "${RED}FAIL${NC} (expected: $expected_exit_code, got: $actual_exit_code)"
        ((TESTS_FAILED+=1))
    fi
}

# Build the project
echo "Building project..."
cargo build --release --bin merkle-cli-viem-compat
echo ""

# Create test data
cat > test_data.csv << EOF
address,allocation
0x742C4d97C86bCF0176776C16e073b8c6f9Db4021,1000000000000000000
0x8ba1f109551bD432803012645Ac136c5a2B51Abc,500000000000000000
0x06a37c563d88894a98438e3b2fe17f365f1d3530,990000000000000000
EOF

echo "Test data created: test_data.csv"
echo ""

# Generate reference output
echo "Generating reference output..."
./target/release/merkle-cli-viem-compat \
  --input test_data.csv \
  --output reference.json \
  --pretty > /dev/null 2>&1

echo "Reference output created: reference.json"
echo ""

# Extract root hash from reference
CORRECT_ROOT=$(grep '"root_hash"' reference.json | cut -d '"' -f 4)
echo "Reference root hash: $CORRECT_ROOT"
echo ""

WRONG_ROOT="0x0000000000000000000000000000000000000000000000000000000000000000"

# Create invalid reference JSON (wrong root hash)
cat > invalid_root.json << EOF
{
  "root_hash": "$WRONG_ROOT",
  "allocations": {}
}
EOF

# Create invalid reference JSON (wrong proofs)
cat > invalid_proofs.json << EOF
{
  "root_hash": "$CORRECT_ROOT",
  "allocations": {
    "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021": {
      "allocation": "1000000000000000000",
      "proof": ["0x0000000000000000000000000000000000000000000000000000000000000000"]
    }
  }
}
EOF

echo "====================================="
echo "Running Tests"
echo "====================================="
echo ""

# Test 1: Success case (no comparisons)
run_test "Success - No comparisons" 0 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output1.json"

# Test 2: Success case (correct root hash via CLI)
run_test "Success - Correct root hash via CLI" 0 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output2.json --compare-root '$CORRECT_ROOT'"

# Test 3: Failure case (wrong root hash via CLI) - Exit code 1
run_test "Failure - Wrong root hash via CLI" 1 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output3.json --compare-root '$WRONG_ROOT'"

# Test 4: Success case (correct JSON comparison)
run_test "Success - Correct JSON comparison" 0 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output4.json --compare-json reference.json"

# Test 5: Failure case (wrong root in JSON) - Exit code 2
run_test "Failure - Wrong root hash in JSON" 2 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output5.json --compare-json invalid_root.json"

# Test 6: Failure case (wrong proofs in JSON) - Exit code 3
run_test "Failure - Wrong proofs in JSON" 3 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output6.json --compare-json invalid_proofs.json"

# Test 7: CLI root check takes precedence over JSON
run_test "CLI root check (wrong) before JSON check" 1 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output7.json --compare-root '$WRONG_ROOT' --compare-json reference.json"

# Test 8: Keep prefix mode
run_test "Success - Keep prefix mode" 0 \
    "./target/release/merkle-cli-viem-compat --input test_data.csv --output output8.json --keep-prefix"

echo ""
echo "====================================="
echo "Test Summary"
echo "====================================="
echo -e "Passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Failed: ${RED}${TESTS_FAILED}${NC}"
echo "====================================="

# Cleanup
rm -f test_data.csv reference.json invalid_root.json invalid_proofs.json
rm -f output*.json

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi
