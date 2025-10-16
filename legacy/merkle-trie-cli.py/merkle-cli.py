#!/usr/bin/env python3
"""
Python implementation of Merkle tree generation from CSV file.
Equivalent to the TypeScript viem-based implementation.
"""

import csv
import sys
from typing import List, Tuple, Dict
from eth_utils import to_checksum_address, keccak
from eth_abi import encode


def read_csv_data(file_path: str) -> List[Dict[str, str]]:
    """
    Read CSV file and return list of dictionaries.

    Args:
        file_path: Path to CSV file

    Returns:
        List of dictionaries with 'address' and 'allocation' keys
    """
    data = []
    try:
        with open(file_path, 'r', encoding='utf-8') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                data.append({
                    'address': row['address'],
                    'allocation': row['allocation']
                })
        return data
    except FileNotFoundError:
        print(f"Error: File '{file_path}' not found")
        sys.exit(1)
    except KeyError as e:
        print(f"Error: Missing required column {e}")
        sys.exit(1)


def leaf_hash(address: str, amount: int) -> bytes:
    """
    Generate leaf hash from address and amount.
    Equivalent to viem's keccak256(encodePacked(["address", "uint256"], [address, amount]))

    Args:
        address: Ethereum address (with or without 0x prefix)
        amount: Amount as integer (in wei)

    Returns:
        32-byte hash as bytes
    """
    # Ensure address has checksum
    checksum_addr = to_checksum_address(address)

    # Encode packed: address (20 bytes) + uint256 (32 bytes)
    # Remove '0x' prefix from address for encoding
    addr_bytes = bytes.fromhex(checksum_addr[2:])
    amount_bytes = amount.to_bytes(32, byteorder='big')

    # Concatenate and hash
    packed = addr_bytes + amount_bytes
    hash_result = keccak(packed)

    return hash_result


def hash_pair(left: bytes, right: bytes) -> bytes:
    """
    Hash a pair of nodes, sorting them first.
    Equivalent to viem's hashPair with sorting.

    Args:
        left: Left hash (32 bytes)
        right: Right hash (32 bytes)

    Returns:
        32-byte hash as bytes
    """
    # Sort the pair (lexicographic order)
    if left >= right:
        left, right = right, left

    # Concatenate and hash
    packed = left + right
    hash_result = keccak(packed)

    return hash_result


def bytes_to_hex(data: bytes) -> str:
    """Convert bytes to hex string with 0x prefix."""
    return '0x' + data.hex()


def build_merkle_tree(leaves: List[bytes]) -> Tuple[bytes, List[List[bytes]]]:
    """
    Build a complete Merkle tree from leaves.

    Args:
        leaves: List of leaf hashes

    Returns:
        Tuple of (root_hash, tree_levels)
    """
    if not leaves:
        raise ValueError("Cannot build tree from empty leaves")

    levels = [leaves[:]]  # Copy leaves

    while len(levels[-1]) > 1:
        current_level = levels[-1]
        next_level = []

        # Process pairs
        for i in range(0, len(current_level), 2):
            left = current_level[i]

            # If odd number, pair with itself
            if i + 1 < len(current_level):
                right = current_level[i + 1]
            else:
                right = left

            parent = hash_pair(left, right)
            next_level.append(parent)

        levels.append(next_level)

    root = levels[-1][0]
    return root, levels


def get_merkle_proof(leaf_index: int, levels: List[List[bytes]]) -> List[Tuple[bytes, bool]]:
    """
    Generate Merkle proof for a leaf at given index.

    Args:
        leaf_index: Index of the leaf in the tree
        levels: All levels of the tree

    Returns:
        List of (sibling_hash, is_right) tuples
    """
    proof = []
    index = leaf_index

    for level in levels[:-1]:  # Exclude root level
        # Determine if we need left or right sibling
        if index % 2 == 0:
            # Current node is left, sibling is right
            sibling_index = index + 1
            is_right = True
        else:
            # Current node is right, sibling is left
            sibling_index = index - 1
            is_right = False

        # Get sibling (or duplicate if odd)
        if sibling_index < len(level):
            sibling = level[sibling_index]
        else:
            sibling = level[index]  # Duplicate for odd number

        proof.append((sibling, is_right))

        # Move to parent level
        index = index // 2

    return proof


def verify_merkle_proof(leaf: bytes, proof: List[Tuple[bytes, bool]], root: bytes) -> bool:
    """
    Verify a Merkle proof.

    Args:
        leaf: Leaf hash to verify
        proof: List of (sibling_hash, is_right) tuples
        root: Expected root hash

    Returns:
        True if proof is valid
    """
    current = leaf

    for sibling, is_right in proof:
        if is_right:
            # Sibling is on the right
            current = hash_pair(current, sibling)
        else:
            # Sibling is on the left
            current = hash_pair(sibling, current)

    return current == root


def main():
    """Main function - equivalent to the TypeScript main()."""

    # Read CSV data
    print("Reading CSV data...")
    data = read_csv_data("example.csv")
    print(f"Loaded {len(data)} entries\n")

    # Generate leaf hashes
    print("Generating leaf hashes...")
    leaves = []
    for row in data:
        address = row['address']
        allocation = int(row['allocation'])
        leaf = leaf_hash(address, allocation)
        leaves.append(leaf)

    print("Raw leaves:")
    for i, leaf in enumerate(leaves):
        print(f"  [{i}] {bytes_to_hex(leaf)}")
    print()

    # Manual tree construction (matching TypeScript example)
    if len(leaves) >= 3:
        print("Manual tree construction (TypeScript example):")
        aa = hash_pair(leaves[0], leaves[1])
        print(f"  aa = hashPair(leaves[0], leaves[1]) = {bytes_to_hex(aa)}")

        bb = hash_pair(leaves[2], leaves[2])
        print(f"  bb = hashPair(leaves[2], leaves[2]) = {bytes_to_hex(bb)}")

        cc = hash_pair(aa, bb)
        print(f"  Merkle root (manual): {bytes_to_hex(cc)}")
        print()

    # Build complete Merkle tree
    print("Building complete Merkle tree...")
    root, levels = build_merkle_tree(leaves)

    print(f"Merkle root: {bytes_to_hex(root)}")
    print(f"Tree depth: {len(levels) - 1}")
    print()

    # Display tree structure
    print("Tree structure:")
    for level_idx, level in enumerate(levels):
        print(f"  Level {level_idx}: {len(level)} nodes")
        for node in level:
            print(f"    {bytes_to_hex(node)}")
    print()

    # Generate and verify proofs for all leaves
    print("Generating and verifying proofs...")
    for i, leaf in enumerate(leaves):
        proof = get_merkle_proof(i, levels)
        is_valid = verify_merkle_proof(leaf, proof, root)

        print(f"  Leaf [{i}] proof:")
        print(f"    Leaf: {bytes_to_hex(leaf)}")
        print(f"    Proof length: {len(proof)} siblings")
        for j, (sibling, is_right) in enumerate(proof):
            side = "right" if is_right else "left"
            print(f"      [{j}] {bytes_to_hex(sibling)} ({side})")
        print(f"    Valid: {is_valid}")
        print()


if __name__ == "__main__":
    main()
