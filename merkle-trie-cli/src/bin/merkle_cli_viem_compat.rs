use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use clap::Parser;
use anyhow::{Result, Context};
use csv::ReaderBuilder;
use serde::{Serialize, Deserialize};
use keccak_hasher::KeccakHasher;
use hash_db::Hasher as HashDbHasher;

#[derive(Parser, Debug)]
#[command(name = "merkle-viem-compat")]
#[command(about = "Generate Merkle tree compatible with viem/TypeScript implementation", long_about = None)]
struct Args {
    /// Input CSV file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output JSON file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print verbose output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Pretty print JSON output
    #[arg(short, long, default_value_t = false)]
    pretty: bool,

    /// Show raw leaves
    #[arg(long, default_value_t = false)]
    show_leaves: bool,

    /// Show tree structure
    #[arg(long, default_value_t = false)]
    show_tree: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AllocationProof {
    allocation: String,
    proof: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputData {
    root_hash: String,
    allocations: BTreeMap<String, AllocationProof>,
}

#[derive(Debug, Clone)]
struct CsvRow {
    address: String,
    allocation: String,
}

/// Keccak256 hash using keccak-hasher
fn keccak256(data: &[u8]) -> [u8; 32] {
    KeccakHasher::hash(data)
}

/// Convert bytes to hex string with 0x prefix
fn bytes_to_hex(data: &[u8]) -> String {
    format!("0x{}", hex::encode(data))
}

/// Convert hex string to bytes
fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(cleaned).context("Failed to decode hex string")
}

/// Convert address to checksum format (EIP-55)
fn to_checksum_address(address: &str) -> Result<String> {
    let cleaned = address.strip_prefix("0x").unwrap_or(address).to_lowercase();

    if cleaned.len() != 40 {
        anyhow::bail!("Invalid address length: expected 40 hex characters");
    }

    // Verify it's valid hex
    hex::decode(&cleaned).context("Invalid hex address")?;

    // Hash the lowercase address
    let hash = keccak256(cleaned.as_bytes());
    let hash_hex = hex::encode(hash);

    let mut checksum_addr = String::from("0x");

    for (i, ch) in cleaned.chars().enumerate() {
        if ch.is_ascii_digit() {
            checksum_addr.push(ch);
        } else {
            // Get the corresponding nibble from the hash
            let hash_char = hash_hex.chars().nth(i).unwrap();
            let hash_value = u32::from_str_radix(&hash_char.to_string(), 16).unwrap();

            if hash_value >= 8 {
                checksum_addr.push(ch.to_ascii_uppercase());
            } else {
                checksum_addr.push(ch.to_ascii_lowercase());
            }
        }
    }

    Ok(checksum_addr)
}

/// Generate leaf hash from address and amount
/// Equivalent to: keccak256(encodePacked(["address", "uint256"], [address, amount]))
fn leaf_hash(address: &str, amount: u128) -> Result<[u8; 32]> {
    // Get checksum address
    let checksum_addr = to_checksum_address(address)?;

    // Convert address to bytes (20 bytes)
    let addr_bytes = hex::decode(checksum_addr.strip_prefix("0x").unwrap_or(&checksum_addr))
        .context("Failed to decode address")?;

    if addr_bytes.len() != 20 {
        anyhow::bail!("Address must be 20 bytes");
    }

    // Convert amount to 32-byte big-endian
    let amount_bytes = amount.to_be_bytes();
    let mut amount_32 = [0u8; 32];
    amount_32[16..32].copy_from_slice(&amount_bytes);

    // Concatenate: address (20 bytes) + amount (32 bytes) = 52 bytes
    let mut packed = Vec::with_capacity(52);
    packed.extend_from_slice(&addr_bytes);
    packed.extend_from_slice(&amount_32);

    // Hash the packed data
    Ok(keccak256(&packed))
}

/// Hash a pair of nodes with sorting (lexicographic order)
/// Equivalent to TypeScript: if (left >= right) { [left, right] = [right, left] }
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let (first, second) = if left >= right {
        (right, left)
    } else {
        (left, right)
    };

    // Concatenate and hash: bytes32 + bytes32 = 64 bytes
    let mut packed = Vec::with_capacity(64);
    packed.extend_from_slice(first);
    packed.extend_from_slice(second);

    keccak256(&packed)
}

/// Build Merkle tree from leaves
fn build_merkle_tree(leaves: Vec<[u8; 32]>) -> Result<(Vec<Vec<[u8; 32]>>, [u8; 32])> {
    if leaves.is_empty() {
        anyhow::bail!("Cannot build tree from empty leaves");
    }

    let mut levels: Vec<Vec<[u8; 32]>> = Vec::new();
    levels.push(leaves.clone());

    while levels.last().unwrap().len() > 1 {
        let current_level = levels.last().unwrap();
        let mut next_level = Vec::new();

        let mut i = 0;
        while i < current_level.len() {
            let left = current_level[i];

            // If odd number, pair with itself
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                left
            };

            let parent = hash_pair(&left, &right);
            next_level.push(parent);

            i += 2;
        }

        levels.push(next_level);
    }

    let root = levels.last().unwrap()[0];
    Ok((levels, root))
}

/// Generate Merkle proof for a leaf at given index
fn get_merkle_proof(leaf_index: usize, levels: &[Vec<[u8; 32]>]) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let mut index = leaf_index;

    // Iterate through all levels except the root
    for level in levels.iter().take(levels.len() - 1) {
        let sibling_index = if index % 2 == 0 {
            index + 1
        } else {
            index - 1
        };

        let sibling = if sibling_index < level.len() {
            level[sibling_index]
        } else {
            level[index] // Duplicate for odd number
        };

        proof.push(sibling);
        index /= 2;
    }

    proof
}

/// Verify Merkle proof
fn verify_merkle_proof(leaf: &[u8; 32], proof: &[[u8; 32]], root: &[u8; 32]) -> bool {
    let mut current = *leaf;

    for sibling in proof {
        current = hash_pair(&current, sibling);
    }

    &current == root
}

/// Read CSV data
fn read_csv_data(file_path: &PathBuf) -> Result<Vec<CsvRow>> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {:?}", file_path))?;

    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let mut data = Vec::new();
    let mut row_count = 0;

    for result in csv_reader.records() {
        let record = result.with_context(|| format!("Failed to read CSV record at row {}", row_count + 1))?;

        let address = record.get(0)
            .ok_or_else(|| anyhow::anyhow!("Missing address at row {}", row_count + 1))?
            .trim()
            .to_string();

        // Support both 'allocation' and 'amount' column names
        let allocation = record.get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing allocation/amount at row {}", row_count + 1))?
            .trim()
            .to_string();

        data.push(CsvRow { address, allocation });
        row_count += 1;
    }

    Ok(data)
}

/// Generate output JSON
fn generate_output(
    data: &[CsvRow],
    leaves: &[[u8; 32]],
    levels: &[Vec<[u8; 32]>],
    root: &[u8; 32],
) -> OutputData {
    let mut allocations = BTreeMap::new();

    for (i, row) in data.iter().enumerate() {
        let proof = get_merkle_proof(i, levels);
        let proof_hex: Vec<String> = proof.iter().map(|p| bytes_to_hex(p)).collect();

        let checksum_addr = to_checksum_address(&row.address)
            .unwrap_or_else(|_| row.address.clone());

        allocations.insert(
            checksum_addr,
            AllocationProof {
                allocation: row.allocation.clone(),
                proof: proof_hex,
            },
        );
    }

    OutputData {
        root_hash: bytes_to_hex(root),
        allocations,
    }
}

/// Write output to file or stdout
fn write_output(output_path: Option<&PathBuf>, data: &OutputData, pretty: bool) -> Result<()> {
    let json_string = if pretty {
        serde_json::to_string_pretty(data)?
    } else {
        serde_json::to_string(data)?
    };

    if let Some(path) = output_path {
        let mut file = File::create(path)
            .with_context(|| format!("Failed to create output file: {:?}", path))?;
        file.write_all(json_string.as_bytes())
            .with_context(|| format!("Failed to write to output file: {:?}", path))?;
    } else {
        println!("{}", json_string);
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        println!("Merkle Tree Generator (viem-compatible)");
        println!("========================================");
        println!("Input file: {:?}", args.input);
        if let Some(ref output) = args.output {
            println!("Output file: {:?}", output);
        }
        println!();
    }

    // Read CSV data
    if args.verbose {
        println!("Reading CSV data...");
    }
    let data = read_csv_data(&args.input)?;

    if args.verbose {
        println!("Loaded {} entries", data.len());
        println!();
    }

    // Generate leaf hashes
    if args.verbose {
        println!("Generating leaf hashes...");
    }

    let mut leaves = Vec::new();
    for row in &data {
        let amount = row.allocation.parse::<u128>()
            .with_context(|| format!("Failed to parse allocation amount: {}", row.allocation))?;
        let leaf = leaf_hash(&row.address, amount)?;
        leaves.push(leaf);
    }

    if args.show_leaves || args.verbose {
        println!("Raw leaves:");
        for (i, leaf) in leaves.iter().enumerate() {
            println!("  [{}] {}", i, bytes_to_hex(leaf));
        }
        println!();
    }

    // Manual tree construction for comparison (matching TypeScript implementation)
    if args.verbose && leaves.len() >= 3 {
        println!("Manual tree construction (TypeScript example):");
        let aa = hash_pair(&leaves[0], &leaves[1]);
        println!("  aa = hashPair(leaves[0], leaves[1])");
        println!("     = {}", bytes_to_hex(&aa));

        let bb = hash_pair(&leaves[2], &leaves[2]);
        println!("  bb = hashPair(leaves[2], leaves[2])");
        println!("     = {}", bytes_to_hex(&bb));

        let cc = hash_pair(&aa, &bb);
        println!("  Merkle root (manual) = hashPair(aa, bb)");
        println!("                       = {}", bytes_to_hex(&cc));
        println!();
    }

    // Build complete Merkle tree
    if args.verbose {
        println!("Building complete Merkle tree...");
    }

    let (levels, root) = build_merkle_tree(leaves.clone())?;

    if args.verbose {
        println!("Merkle root: {}", bytes_to_hex(&root));
        println!("Tree depth: {}", levels.len() - 1);
        println!();
    }

    // Show tree structure
    if args.show_tree {
        println!("Tree structure:");
        for (level_idx, level) in levels.iter().enumerate() {
            println!("  Level {}: {} nodes", level_idx, level.len());
            for node in level {
                println!("    {}", bytes_to_hex(node));
            }
        }
        println!();
    }

    // Verify all proofs
    if args.verbose {
        println!("Verifying proofs...");
        let mut all_valid = true;
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = get_merkle_proof(i, &levels);
            let is_valid = verify_merkle_proof(leaf, &proof, &root);
            if !is_valid {
                println!("  ✗ Leaf [{}] proof verification FAILED", i);
                all_valid = false;
            }
        }
        if all_valid {
            println!("  ✓ All {} proofs verified successfully", leaves.len());
        }
        println!();
    }

    // Generate JSON output
    let output_data = generate_output(&data, &leaves, &levels, &root);

    // Write output
    write_output(args.output.as_ref(), &output_data, args.pretty)?;

    if args.verbose {
        if args.output.is_some() {
            println!("✓ Output written successfully");
        }
        println!("\n✓ Successfully generated Merkle tree data!");
        println!("  Root Hash: {}", bytes_to_hex(&root));
        println!("  Allocations: {}", data.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let data = b"hello world";
        let hash = keccak256(data);
        let expected = hex::decode("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad").unwrap();
        assert_eq!(hash.to_vec(), expected);
    }

    #[test]
    fn test_checksum_address() {
        let addr = "0x742c4d97c86bcf0176776c16e073b8c6f9db4021";
        let checksum = to_checksum_address(addr).unwrap();
        assert_eq!(checksum, "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021");
    }

    #[test]
    fn test_leaf_hash() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128; // 1 ETH

        let leaf = leaf_hash(address, amount).unwrap();

        // Verify it's 32 bytes
        assert_eq!(leaf.len(), 32);

        // Should be deterministic
        let leaf2 = leaf_hash(address, amount).unwrap();
        assert_eq!(leaf, leaf2);
    }

    #[test]
    fn test_hash_pair_sorting() {
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];

        let hash1 = hash_pair(&leaf1, &leaf2);
        let hash2 = hash_pair(&leaf2, &leaf1);

        // Should be identical due to sorting
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_merkle_proof_verification() {
        // Create simple tree with 4 leaves
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];

        let (levels, root) = build_merkle_tree(leaves.clone()).unwrap();

        // Verify all proofs
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = get_merkle_proof(i, &levels);
            assert!(verify_merkle_proof(leaf, &proof, &root));
        }
    }

    #[test]
    fn test_single_leaf() {
        let leaves = vec![[1u8; 32]];
        let (levels, root) = build_merkle_tree(leaves.clone()).unwrap();

        assert_eq!(root, leaves[0]);
        assert_eq!(levels.len(), 1);
    }

    #[test]
    fn test_two_leaves() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
        ];

        let (levels, root) = build_merkle_tree(leaves.clone()).unwrap();

        // Root should be hash of the two leaves
        let expected_root = hash_pair(&leaves[0], &leaves[1]);
        assert_eq!(root, expected_root);

        // Should have 2 levels (leaves + root)
        assert_eq!(levels.len(), 2);
    }

    #[test]
    fn test_odd_number_leaves() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
        ];

        let (levels, root) = build_merkle_tree(leaves.clone()).unwrap();

        // Should handle odd number by duplicating last leaf
        assert!(root != [0u8; 32]);

        // All proofs should verify
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = get_merkle_proof(i, &levels);
            assert!(verify_merkle_proof(leaf, &proof, &root));
        }
    }

    #[test]
    fn test_bytes_to_hex() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "0xdeadbeef");
    }

    #[test]
    fn test_hex_to_bytes() {
        let hex = "0xdeadbeef";
        let bytes = hex_to_bytes(hex).unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);

        // Test without 0x prefix
        let hex2 = "deadbeef";
        let bytes2 = hex_to_bytes(hex2).unwrap();
        assert_eq!(bytes2, vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
