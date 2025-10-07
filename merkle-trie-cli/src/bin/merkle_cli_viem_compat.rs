use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process;
use clap::Parser;
use anyhow::{Result, Context};
use csv::ReaderBuilder;
use serde::{Serialize, Deserialize};
use keccak_hasher::KeccakHasher;
use hash_db::Hasher as HashDbHasher;

// Exit codes
const EXIT_SUCCESS: i32 = 0;
const EXIT_ROOT_MISMATCH_CLI: i32 = 1;
const EXIT_ROOT_MISMATCH_JSON: i32 = 2;
const EXIT_PROOFS_MISMATCH_JSON: i32 = 3;

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

    /// Keep 0x prefix in leaf data for hashing (don't strip it)
    #[arg(long, default_value_t = false)]
    keep_prefix: bool,

    /// Expected root hash to compare against (with 0x prefix)
    #[arg(long)]
    compare_root: Option<String>,

    /// Reference JSON file to compare output against
    #[arg(long)]
    compare_json: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AllocationProof {
    allocation: String,
    proof: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct OutputData {
    root_hash: String,
    allocations: BTreeMap<String, AllocationProof>,
}

#[derive(Debug, Clone)]
struct CsvRow {
    address: String,
    allocation: String,
}

#[derive(Debug)]
struct ComparisonResult {
    root_hash_match: bool,
    proofs_match: bool,
    missing_addresses: Vec<String>,
    extra_addresses: Vec<String>,
    mismatched_allocations: Vec<String>,
    mismatched_proofs: Vec<String>,
}

impl ComparisonResult {
    fn is_success(&self) -> bool {
        self.root_hash_match
            && self.proofs_match
            && self.missing_addresses.is_empty()
            && self.extra_addresses.is_empty()
            && self.mismatched_allocations.is_empty()
            && self.mismatched_proofs.is_empty()
    }

    fn print_report(&self) {
        println!("\n=== Comparison Report ===");

        if self.root_hash_match {
            println!("✓ Root hash matches");
        } else {
            println!("✗ Root hash DOES NOT match");
        }

        if self.proofs_match && self.missing_addresses.is_empty()
            && self.extra_addresses.is_empty()
            && self.mismatched_allocations.is_empty()
            && self.mismatched_proofs.is_empty() {
            println!("✓ All proofs match");
        } else {
            println!("✗ Proofs have differences");

            if !self.missing_addresses.is_empty() {
                println!("\n  Missing addresses (in reference but not in output):");
                for addr in &self.missing_addresses {
                    println!("    - {}", addr);
                }
            }

            if !self.extra_addresses.is_empty() {
                println!("\n  Extra addresses (in output but not in reference):");
                for addr in &self.extra_addresses {
                    println!("    - {}", addr);
                }
            }

            if !self.mismatched_allocations.is_empty() {
                println!("\n  Mismatched allocations:");
                for addr in &self.mismatched_allocations {
                    println!("    - {}", addr);
                }
            }

            if !self.mismatched_proofs.is_empty() {
                println!("\n  Mismatched proofs:");
                for addr in &self.mismatched_proofs {
                    println!("    - {}", addr);
                }
            }
        }

        println!("\n=========================");
    }
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

/// Normalize hex string for comparison
fn normalize_hex(hex_str: &str) -> String {
    hex_str.strip_prefix("0x").unwrap_or(hex_str).to_lowercase()
}

/// Compare two root hashes (case-insensitive, prefix-insensitive)
fn compare_root_hashes(hash1: &str, hash2: &str) -> bool {
    normalize_hex(hash1) == normalize_hex(hash2)
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
fn leaf_hash(address: &str, amount: u128, keep_prefix: bool) -> Result<[u8; 32]> {
    let mut packed = Vec::new();

    if keep_prefix && address.starts_with("0x") {
        // Keep 0x prefix as bytes in the leaf data
        packed.extend_from_slice(address.as_bytes());
    } else {
        // Get checksum address and decode to bytes
        let checksum_addr = to_checksum_address(address)?;
        let addr_bytes = hex::decode(checksum_addr.strip_prefix("0x").unwrap_or(&checksum_addr))
            .context("Failed to decode address")?;

        if addr_bytes.len() != 20 {
            anyhow::bail!("Address must be 20 bytes");
        }

        packed.extend_from_slice(&addr_bytes);
    }

    // Convert amount to 32-byte big-endian
    let amount_bytes = amount.to_be_bytes();
    let mut amount_32 = [0u8; 32];
    amount_32[16..32].copy_from_slice(&amount_bytes);

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

/// Load reference JSON file
fn load_reference_json(path: &PathBuf) -> Result<OutputData> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open reference JSON file: {:?}", path))?;

    let reader = BufReader::new(file);
    let data: OutputData = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse reference JSON file: {:?}", path))?;

    Ok(data)
}

/// Compare output data with reference data
fn compare_output_data(actual: &OutputData, reference: &OutputData) -> ComparisonResult {
    let mut result = ComparisonResult {
        root_hash_match: compare_root_hashes(&actual.root_hash, &reference.root_hash),
        proofs_match: true,
        missing_addresses: Vec::new(),
        extra_addresses: Vec::new(),
        mismatched_allocations: Vec::new(),
        mismatched_proofs: Vec::new(),
    };

    // Normalize addresses for comparison
    let actual_addrs: BTreeMap<String, &AllocationProof> = actual.allocations
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();

    let reference_addrs: BTreeMap<String, &AllocationProof> = reference.allocations
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();

    // Check for missing addresses (in reference but not in actual)
    for addr in reference_addrs.keys() {
        if !actual_addrs.contains_key(addr) {
            result.missing_addresses.push(addr.clone());
            result.proofs_match = false;
        }
    }

    // Check for extra addresses (in actual but not in reference)
    for addr in actual_addrs.keys() {
        if !reference_addrs.contains_key(addr) {
            result.extra_addresses.push(addr.clone());
            result.proofs_match = false;
        }
    }

    // Check for mismatched allocations and proofs
    for (addr, actual_proof) in &actual_addrs {
        if let Some(reference_proof) = reference_addrs.get(addr) {
            // Compare allocations
            if actual_proof.allocation != reference_proof.allocation {
                result.mismatched_allocations.push(addr.clone());
                result.proofs_match = false;
            }

            // Compare proofs (normalize hex for comparison)
            if actual_proof.proof.len() != reference_proof.proof.len() {
                result.mismatched_proofs.push(addr.clone());
                result.proofs_match = false;
            } else {
                for (actual_hash, ref_hash) in actual_proof.proof.iter().zip(reference_proof.proof.iter()) {
                    if !compare_root_hashes(actual_hash, ref_hash) {
                        result.mismatched_proofs.push(addr.clone());
                        result.proofs_match = false;
                        break;
                    }
                }
            }
        }
    }

    result
}

/// Generate output JSON
fn generate_output(
    data: &[CsvRow],
    leaves: &[[u8; 32]],
    levels: &[Vec<[u8; 32]>],
    root: &[u8; 32],
) -> Result<OutputData> {
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

    Ok(OutputData {
        root_hash: bytes_to_hex(root),
        allocations,
    })
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
        println!("Keep 0x prefix in leaf data: {}", args.keep_prefix);
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
        let leaf = leaf_hash(&row.address, amount, args.keep_prefix)?;
        leaves.push(leaf);
    }

    if args.show_leaves || args.verbose {
        println!("Raw leaves:");
        for (i, leaf) in leaves.iter().enumerate() {
            println!("  [{}] {}", i, bytes_to_hex(leaf));
        }
        println!();
    }

    // Manual tree construction for comparison (matching TypeScript example)
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

    // Compare root hash if provided
    let mut root_hash_cli_matches = true;
    if let Some(expected_root) = &args.compare_root {
        root_hash_cli_matches = compare_root_hashes(&bytes_to_hex(&root), expected_root);
        println!("\n=== Root Hash Comparison (CLI) ===");
        println!("Expected: {}", expected_root);
        println!("Actual:   {}", bytes_to_hex(&root));
        if root_hash_cli_matches {
            println!("✓ Root hash matches");
        } else {
            println!("✗ Root hash DOES NOT match");
        }
        println!("===================================");
    }

    // Generate JSON output
    let output_data = generate_output(&data, &leaves, &levels, &root)?;

    // Compare with reference JSON if provided
    let mut json_comparison: Option<ComparisonResult> = None;
    if let Some(ref_json_path) = &args.compare_json {
        println!("\nLoading reference JSON from {:?}...", ref_json_path);
        let reference_data = load_reference_json(ref_json_path)?;

        println!("Comparing output with reference data...");
        let result = compare_output_data(&output_data, &reference_data);
        result.print_report();
        json_comparison = Some(result);
    }

    // Write output
    write_output(args.output.as_ref(), &output_data, args.pretty)?;

    if args.verbose {
        if args.output.is_some() {
            println!("\n✓ Output written successfully");
        }
        println!("\n✓ Successfully generated Merkle tree data!");
        println!("  Root Hash: {}", bytes_to_hex(&root));
        println!("  Allocations: {}", data.len());

        if args.keep_prefix {
            println!("\n  Note: 0x prefix was kept in leaf data for hashing.");
        }
    }

    // Determine exit code
    let exit_code = if let Some(comparison) = json_comparison {
        if !comparison.root_hash_match {
            eprintln!("\n✗ ERROR: Root hash in reference JSON does not match!");
            eprintln!("  Exit code: {}", EXIT_ROOT_MISMATCH_JSON);
            EXIT_ROOT_MISMATCH_JSON
        } else if !comparison.proofs_match || !comparison.missing_addresses.is_empty()
            || !comparison.extra_addresses.is_empty()
            || !comparison.mismatched_allocations.is_empty()
            || !comparison.mismatched_proofs.is_empty() {
            eprintln!("\n✗ ERROR: Proofs in reference JSON do not match!");
            eprintln!("  Exit code: {}", EXIT_PROOFS_MISMATCH_JSON);
            EXIT_PROOFS_MISMATCH_JSON
        } else {
            EXIT_SUCCESS
        }
    } else if args.compare_root.is_some() && !root_hash_cli_matches {
        eprintln!("\n✗ ERROR: Root hash provided via CLI does not match!");
        eprintln!("  Exit code: {}", EXIT_ROOT_MISMATCH_CLI);
        EXIT_ROOT_MISMATCH_CLI
    } else {
        EXIT_SUCCESS
    };

    if exit_code != EXIT_SUCCESS {
        process::exit(exit_code);
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
    fn test_normalize_hex() {
        assert_eq!(normalize_hex("0xABCD"), "abcd");
        assert_eq!(normalize_hex("ABCD"), "abcd");
        assert_eq!(normalize_hex("0xabcd"), "abcd");
    }

    #[test]
    fn test_compare_root_hashes() {
        assert!(compare_root_hashes("0xABCD", "0xabcd"));
        assert!(compare_root_hashes("ABCD", "0xabcd"));
        assert!(compare_root_hashes("0xABCD", "abcd"));
        assert!(!compare_root_hashes("0xABCD", "0x1234"));
    }

    #[test]
    fn test_leaf_hash_without_prefix() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128;

        let leaf = leaf_hash(address, amount, false).unwrap();

        // Verify it's 32 bytes
        assert_eq!(leaf.len(), 32);

        // Should be deterministic
        let leaf2 = leaf_hash(address, amount, false).unwrap();
        assert_eq!(leaf, leaf2);
    }

    #[test]
    fn test_leaf_hash_with_prefix() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = 1000000000000000000u128;

        let leaf_with = leaf_hash(address, amount, true).unwrap();
        let leaf_without = leaf_hash(address, amount, false).unwrap();

        // Should produce different hashes
        assert_ne!(leaf_with, leaf_without);
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
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];

        let (levels, root) = build_merkle_tree(leaves.clone()).unwrap();

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = get_merkle_proof(i, &levels);
            assert!(verify_merkle_proof(leaf, &proof, &root));
        }
    }

    #[test]
    fn test_comparison_result_success() {
        let result = ComparisonResult {
            root_hash_match: true,
            proofs_match: true,
            missing_addresses: Vec::new(),
            extra_addresses: Vec::new(),
            mismatched_allocations: Vec::new(),
            mismatched_proofs: Vec::new(),
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_comparison_result_failure() {
        let result = ComparisonResult {
            root_hash_match: false,
            proofs_match: true,
            missing_addresses: Vec::new(),
            extra_addresses: Vec::new(),
            mismatched_allocations: Vec::new(),
            mismatched_proofs: Vec::new(),
        };
        assert!(!result.is_success());
    }
}
