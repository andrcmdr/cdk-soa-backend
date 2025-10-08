#!/bin/bash

# Example script showing how to use --show-leaf-content option

echo "Creating test data..."
cat > example_leaves.csv << EOF
address,allocation
0x742C4d97C86bCF0176776C16e073b8c6f9Db4021,1000000000000000000
0x8ba1f109551bD432803012645Ac136c5a2B51Abc,500000000000000000
0x06a37c563d88894a98438e3b2fe17f365f1d3530,2500000000000000000
EOF

echo ""
echo "========================================"
echo "Example 1: Show leaf content (without 0x prefix in leaf data)"
echo "========================================"
cargo run --bin merkle-cli-viem-compat -- \
  --input example_leaves.csv \
  --output output1.json \
  --show-leaf-content \
  --pretty

echo ""
echo "========================================"
echo "Example 2: Show leaf content (with 0x prefix in leaf data)"
echo "========================================"
cargo run --bin merkle-cli-viem-compat -- \
  --input example_leaves.csv \
  --output output2.json \
  --show-leaf-content \
  --keep-prefix \
  --pretty

echo ""
echo "========================================"
echo "Example 3: Show all details"
echo "========================================"
cargo run --bin merkle-cli-viem-compat -- \
  --input example_leaves.csv \
  --output output3.json \
  --show-leaf-content \
  --show-tree \
  --verbose \
  --pretty

echo ""
echo "Cleaning up..."
rm -f example_leaves.csv output*.json

echo "Done!"
