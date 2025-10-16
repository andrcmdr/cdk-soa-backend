#!/usr/bin/env python3
"""
Test script to verify Python implementation matches TypeScript/Rust behavior.
"""

import sys
from eth_utils import to_checksum_address, keccak


def leaf_hash(address: str, amount: int) -> bytes:
    """Generate leaf hash."""
    checksum_addr = to_checksum_address(address)
    addr_bytes = bytes.fromhex(checksum_addr[2:])
    amount_bytes = amount.to_bytes(32, byteorder='big')
    packed = addr_bytes + amount_bytes
    return keccak(packed)


def hash_pair(left: bytes, right: bytes) -> bytes:
    """Hash pair with sorting."""
    if left >= right:
        left, right = right, left
    packed = left + right
    return keccak(packed)


def bytes_to_hex(data: bytes) -> str:
    """Convert to hex."""
    return '0x' + data.hex()


def test_basic_hashing():
    """Test basic leaf hashing."""
    print("Test 1: Basic leaf hashing")
    print("=" * 60)

    address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021"
    amount = 1000000000000000000  # 1 ETH

    leaf = leaf_hash(address, amount)
    print(f"Address: {address}")
    print(f"Amount: {amount}")
    print(f"Leaf hash: {bytes_to_hex(leaf)}")
    print()


def test_pair_hashing():
    """Test pair hashing with sorting."""
    print("Test 2: Pair hashing")
    print("=" * 60)

    # Create two test leaves
    addr1 = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021"
    addr2 = "0x8ba1f109551bD432803012645Ac136c5a2B51Abc"

    leaf1 = leaf_hash(addr1, 1000000000000000000)
    leaf2 = leaf_hash(addr2, 500000000000000000)

    print(f"Leaf 1: {bytes_to_hex(leaf1)}")
    print(f"Leaf 2: {bytes_to_hex(leaf2)}")

    # Hash pair (should sort automatically)
    parent = hash_pair(leaf1, leaf2)
    print(f"Parent (sorted): {bytes_to_hex(parent)}")

    # Verify sorting works both ways
    parent2 = hash_pair(leaf2, leaf1)
    print(f"Parent (reversed input): {bytes_to_hex(parent2)}")
    print(f"Hashes match: {parent == parent2}")
    print()


def test_tree_construction():
    """Test full tree construction."""
    print("Test 3: Tree construction")
    print("=" * 60)

    # Sample data
    data = [
        ("0x742C4d97C86bCF0176776C16e073b8c6f9Db4021", 1000000000000000000),
        ("0x8ba1f109551bD432803012645Ac136c5a2B51Abc", 500000000000000000),
        ("0x06a37c563d88894a98438e3b2fe17f365f1d3530", 990000000000000000),
    ]

    # Generate leaves
    leaves = [leaf_hash(addr, amt) for addr, amt in data]

    print("Leaves:")
    for i, leaf in enumerate(leaves):
        print(f"  [{i}] {bytes_to_hex(leaf)}")

    # Build tree manually (matching TypeScript example)
    aa = hash_pair(leaves[0], leaves[1])
    bb = hash_pair(leaves[2], leaves[2])  # Duplicate for odd number
    root = hash_pair(aa, bb)

    print(f"\nIntermediate nodes:")
    print(f"  aa = hashPair(leaves[0], leaves[1])")
    print(f"     = {bytes_to_hex(aa)}")
    print(f"  bb = hashPair(leaves[2], leaves[2])")
    print(f"     = {bytes_to_hex(bb)}")
    print(f"\nRoot = hashPair(aa, bb)")
    print(f"     = {bytes_to_hex(root)}")
    print()


def test_checksum_address():
    """Test checksum address conversion."""
    print("Test 4: Checksum addresses")
    print("=" * 60)

    test_addresses = [
        "0x742c4d97c86bcf0176776c16e073b8c6f9db4021",  # lowercase
        "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",  # mixed case
        "0X742C4D97C86BCF0176776C16E073B8C6F9DB4021",  # uppercase
    ]

    for addr in test_addresses:
        checksum = to_checksum_address(addr)
        print(f"Input:    {addr}")
        print(f"Checksum: {checksum}")
        print()


def main():
    """Run all tests."""
    print("\n" + "=" * 60)
    print("Python Implementation Compatibility Tests")
    print("=" * 60 + "\n")

    test_basic_hashing()
    test_pair_hashing()
    test_tree_construction()
    test_checksum_address()

    print("=" * 60)
    print("All tests completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
