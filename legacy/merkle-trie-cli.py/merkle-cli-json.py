#!/usr/bin/env python3
"""
Generate Merkle tree with JSON output (matching Rust CLI format).
"""

import csv
import json
import sys
import argparse
from typing import List, Dict
from eth_utils import to_checksum_address, keccak


def read_csv_data(file_path: str) -> List[Dict[str, str]]:
    """Read CSV file and return list of dictionaries."""
    data = []
    try:
        with open(file_path, 'r', encoding='utf-8') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                # Support both 'allocation' and 'amount' column names
                amount = row.get('allocation') or row.get('amount')
                data.append({
                    'address': row['address'],
                    'allocation': amount
                })
        return data
    except FileNotFoundError:
        print(f"Error: File '{file_path}' not found", file=sys.stderr)
        sys.exit(1)
    except KeyError as e:
        print(f"Error: Missing required column {e}", file=sys.stderr)
        sys.exit(1)


def leaf_hash(address: str, amount: int) -> bytes:
    """Generate leaf hash from address and amount."""
    checksum_addr = to_checksum_address(address)
    addr_bytes = bytes.fromhex(checksum_addr[2:])
    amount_bytes = amount.to_bytes(32, byteorder='big')
    packed = addr_bytes + amount_bytes
    return keccak(packed)


def hash_pair(left: bytes, right: bytes) -> bytes:
    """Hash a pair of nodes with sorting."""
    if left >= right:
        left, right = right, left
    packed = left + right
    return keccak(packed)


def bytes_to_hex(data: bytes) -> str:
    """Convert bytes to hex string with 0x prefix."""
    return '0x' + data.hex()


def build_merkle_tree(leaves: List[bytes]) -> tuple:
    """Build Merkle tree and return root and levels."""
    if not leaves:
        raise ValueError("Cannot build tree from empty leaves")

    levels = [leaves[:]]

    while len(levels[-1]) > 1:
        current_level = levels[-1]
        next_level = []

        for i in range(0, len(current_level), 2):
            left = current_level[i]
            right = current_level[i + 1] if i + 1 < len(current_level) else left
            parent = hash_pair(left, right)
            next_level.append(parent)

        levels.append(next_level)

    return levels[-1][0], levels


def get_merkle_proof(leaf_index: int, levels: List[List[bytes]]) -> List[bytes]:
    """Generate Merkle proof for a leaf."""
    proof = []
    index = leaf_index

    for level in levels[:-1]:
        if index % 2 == 0:
            sibling_index = index + 1
        else:
            sibling_index = index - 1

        if sibling_index < len(level):
            sibling = level[sibling_index]
        else:
            sibling = level[index]

        proof.append(sibling)
        index = index // 2

    return proof


def generate_json_output(data: List[Dict], leaves: List[bytes], root: bytes, levels: List[List[bytes]]) -> Dict:
    """Generate JSON output matching Rust CLI format."""
    allocations = {}

    for i, row in enumerate(data):
        address = to_checksum_address(row['address'])
        proof = get_merkle_proof(i, levels)
        proof_hex = [bytes_to_hex(p) for p in proof]

        allocations[address] = {
            "allocation": row['allocation'],
            "proof": proof_hex
        }

    return {
        "root_hash": bytes_to_hex(root),
        "allocations": allocations
    }


def main():
    """Main function with CLI arguments."""
    parser = argparse.ArgumentParser(
        description='Generate Merkle tree from CSV file (Python implementation)'
    )
    parser.add_argument('-i', '--input', required=True, help='Input CSV file path')
    parser.add_argument('-o', '--output', help='Output JSON file path')
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose output')
    parser.add_argument('-p', '--pretty', action='store_true', help='Pretty print JSON')

    args = parser.parse_args()

    # Read CSV data
    if args.verbose:
        print(f"Reading CSV from {args.input}...")

    data = read_csv_data(args.input)

    if args.verbose:
        print(f"Loaded {len(data)} entries")

    # Generate leaf hashes
    leaves = []
    for row in data:
        address = row['address']
        allocation = int(row['allocation'])
        leaf = leaf_hash(address, allocation)
        leaves.append(leaf)

    if args.verbose:
        print("\nRaw leaves:")
        for i, leaf in enumerate(leaves):
            print(f"  [{i}] {bytes_to_hex(leaf)}")

    # Build Merkle tree
    root, levels = build_merkle_tree(leaves)

    if args.verbose:
        print(f"\nMerkle root: {bytes_to_hex(root)}")
        print(f"Tree depth: {len(levels) - 1}")

    # Generate JSON output
    output_data = generate_json_output(data, leaves, root, levels)

    # Write to file or stdout
    json_str = json.dumps(output_data, indent=2 if args.pretty else None)

    if args.output:
        with open(args.output, 'w', encoding='utf-8') as f:
            f.write(json_str)
        if args.verbose:
            print(f"\n✓ Output written to {args.output}")
    else:
        print(json_str)

    if args.verbose:
        print(f"\n✓ Successfully generated Merkle tree data!")
        print(f"  Root Hash: {bytes_to_hex(root)}")
        print(f"  Allocations: {len(data)}")


if __name__ == "__main__":
    main()
