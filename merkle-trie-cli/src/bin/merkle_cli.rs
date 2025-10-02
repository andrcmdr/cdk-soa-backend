use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process;
use clap::Parser;
use anyhow::{Result, Context};
use csv::ReaderBuilder;
use serde::{Serialize, Deserialize};

// Import the merkle trie implementation
use merkle_trie_cli::merkle_trie::{MerkleTrie, keccak256};

#[derive(Parser, Debug)]
#[command(name = "merkle-cli")]
#[command(about = "Generate Merkle Trie from CSV file with address and amount columns", long_about = None)]
struct Args {
    /// Input CSV file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output JSON file path
    #[arg(short, long)]
    output: PathBuf,

    /// Print root hash to stdout
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Pretty print JSON output
    #[arg(short, long, default_value_t = false)]
    pretty: bool,

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

/// Normalize Ethereum address to lowercase
fn normalize_address(address: &str, keep_prefix: bool) -> String {
    let addr = address.trim().to_lowercase();

    if keep_prefix {
        // Keep the 0x prefix as-is
        addr
    } else {
        // Strip 0x prefix for processing
        addr.strip_prefix("0x").unwrap_or(&addr).to_string()
    }
}

/// Ensure address has 0x prefix for output
fn ensure_prefix(address: &str) -> String {
    if address.starts_with("0x") {
        address.to_string()
    } else {
        format!("0x{}", address)
    }
}

/// Encode address and amount as leaf data
fn encode_leaf_data(address: &str, amount: &str, keep_prefix: bool) -> Result<Vec<u8>> {
    let mut leaf_data = Vec::new();

    // Parse address
    let addr_str = if keep_prefix {
        // If keeping prefix, include it in the data
        address
    } else {
        // Strip prefix for processing
        address.strip_prefix("0x").unwrap_or(address)
    };

    let addr_bytes = if keep_prefix && addr_str.starts_with("0x") {
        // Include "0x" as bytes
        addr_str.as_bytes().to_vec()
    } else {
        // Decode as hex
        hex::decode(addr_str)
            .with_context(|| format!("Failed to decode address: {}", address))?
    };

    if !keep_prefix && addr_bytes.len() != 20 {
        anyhow::bail!("Invalid address length: {} (expected 20 bytes)", addr_bytes.len());
    }

    // Parse amount as U256 and convert to 32-byte big-endian
    let amount_trimmed = amount.trim();
    let amount_u256 = amount_trimmed.parse::<u128>()
        .with_context(|| format!("Failed to parse amount: {}", amount))?;

    // Convert amount to 32-byte big-endian representation
    let amount_bytes = amount_u256.to_be_bytes();
    let mut amount_32 = [0u8; 32];
    amount_32[16..32].copy_from_slice(&amount_bytes);

    // Concatenate: address (20 bytes) + amount (32 bytes)
    leaf_data.extend_from_slice(&addr_bytes);
    leaf_data.extend_from_slice(&amount_32);

    Ok(leaf_data)
}

/// Process CSV file and build Merkle Trie
/// BTreeMap automatically handles sorting, so no manual sorting needed
fn process_csv_file(input_path: &PathBuf, keep_prefix: bool) -> Result<(MerkleTrie, BTreeMap<String, String>)> {
    let file = File::open(input_path)
        .with_context(|| format!("Failed to open file: {:?}", input_path))?;

    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(reader);

    let mut trie = MerkleTrie::new();
    let mut address_amount_map: BTreeMap<String, String> = BTreeMap::new();
    let mut row_count = 0;

    // First, collect all entries
    for result in csv_reader.records() {
        let record = result.with_context(|| format!("Failed to read CSV record at row {}", row_count + 1))?;

        if record.len() < 2 {
            anyhow::bail!("Invalid CSV format at row {}: expected 2 columns, found {}", row_count + 1, record.len());
        }

        let address = record.get(0)
            .ok_or_else(|| anyhow::anyhow!("Missing address at row {}", row_count + 1))?
            .trim();

        let amount = record.get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing amount at row {}", row_count + 1))?
            .trim();

        if address.is_empty() {
            anyhow::bail!("Empty address at row {}", row_count + 1);
        }

        if amount.is_empty() {
            anyhow::bail!("Empty amount at row {}", row_count + 1);
        }

        // Normalize address
        let normalized_address = normalize_address(address, keep_prefix);

        // Encode leaf data
        let leaf_data = encode_leaf_data(&normalized_address, amount, keep_prefix)
            .with_context(|| format!("Failed to encode leaf data at row {}", row_count + 1))?;

        // Add to trie (BTreeMap in trie will handle sorting)
        trie.add_leaf(leaf_data);

        // Store address-amount mapping (ensure prefix for output)
        let output_address = ensure_prefix(&normalized_address);
        address_amount_map.insert(output_address, amount.to_string());

        row_count += 1;
    }

    if row_count == 0 {
        anyhow::bail!("CSV file is empty or contains no valid records");
    }

    println!("Processed {} records from CSV", row_count);

    // Build the tree (automatically sorted by BTreeMap)
    println!("Building Merkle tree (automatically sorted by leaf data)...");
    trie.build_tree();

    Ok((trie, address_amount_map))
}

/// Generate output JSON with proofs for all addresses
fn generate_output(trie: &MerkleTrie, address_amount_map: BTreeMap<String, String>, keep_prefix: bool) -> Result<OutputData> {
    let root_hash = trie.get_root_hash_hex()
        .ok_or_else(|| anyhow::anyhow!("Failed to get root hash"))?;

    let mut allocations = BTreeMap::new();

    for (address, amount) in address_amount_map.iter() {
        // Remove prefix for encoding if needed
        let addr_for_encoding = if keep_prefix {
            address.as_str()
        } else {
            address.strip_prefix("0x").unwrap_or(address)
        };

        // Encode leaf data for this address
        let leaf_data = encode_leaf_data(addr_for_encoding, amount, keep_prefix)?;

        // Generate proof
        let proof = trie.generate_proof(&leaf_data)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate proof for address: {}", address))?;

        // Convert proof siblings to hex array
        let proof_hashes: Vec<String> = proof.siblings
            .iter()
            .map(|element| format!("0x{}", hex::encode(element.hash)))
            .collect();

        // Create allocation proof
        let allocation_proof = AllocationProof {
            allocation: amount.clone(),
            proof: proof_hashes,
        };

        // Add to output (ensure 0x prefix)
        let output_address = ensure_prefix(address);
        allocations.insert(output_address, allocation_proof);
    }

    Ok(OutputData {
        root_hash,
        allocations,
    })
}

/// Write output to JSON file
fn write_output(output_path: &PathBuf, data: &OutputData, pretty: bool) -> Result<()> {
    let mut file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {:?}", output_path))?;

    let json_string = if pretty {
        serde_json::to_string_pretty(data)?
    } else {
        serde_json::to_string(data)?
    };

    file.write_all(json_string.as_bytes())
        .with_context(|| format!("Failed to write to output file: {:?}", output_path))?;

    Ok(())
}

/// Compare root hash with expected value
fn compare_root_hash(actual: &str, expected: &str) -> bool {
    let actual_normalized = actual.to_lowercase();
    let expected_normalized = expected.to_lowercase();

    actual_normalized == expected_normalized
}

/// Load reference JSON data
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
        root_hash_match: compare_root_hash(&actual.root_hash, &reference.root_hash),
        proofs_match: true,
        missing_addresses: Vec::new(),
        extra_addresses: Vec::new(),
        mismatched_allocations: Vec::new(),
        mismatched_proofs: Vec::new(),
    };

    // Check for missing addresses (in reference but not in actual)
    for addr in reference.allocations.keys() {
        if !actual.allocations.contains_key(addr) {
            result.missing_addresses.push(addr.clone());
            result.proofs_match = false;
        }
    }

    // Check for extra addresses (in actual but not in reference)
    for addr in actual.allocations.keys() {
        if !reference.allocations.contains_key(addr) {
            result.extra_addresses.push(addr.clone());
            result.proofs_match = false;
        }
    }

    // Check for mismatched allocations and proofs
    for (addr, actual_proof) in &actual.allocations {
        if let Some(reference_proof) = reference.allocations.get(addr) {
            // Compare allocations
            if actual_proof.allocation != reference_proof.allocation {
                result.mismatched_allocations.push(addr.clone());
                result.proofs_match = false;
            }

            // Compare proofs
            if actual_proof.proof != reference_proof.proof {
                result.mismatched_proofs.push(addr.clone());
                result.proofs_match = false;
            }
        }
    }

    result
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Merkle Trie CLI Tool");
    println!("===================");
    println!("Input file: {:?}", args.input);
    println!("Output file: {:?}", args.output);
    println!("Keep 0x prefix in leaf data: {}", args.keep_prefix);
    println!();

    // Process CSV file
    println!("Processing CSV file...");
    let (trie, address_amount_map) = process_csv_file(&args.input, args.keep_prefix)?;

    // Get root hash
    let root_hash = trie.get_root_hash_hex()
        .ok_or_else(|| anyhow::anyhow!("Failed to get root hash"))?;

    if args.verbose {
        println!("\nRoot Hash: {}", root_hash);
        println!("Total addresses: {}", address_amount_map.len());
    }

    // Compare root hash if provided
    let mut root_hash_matches = true;
    if let Some(expected_root) = &args.compare_root {
        root_hash_matches = compare_root_hash(&root_hash, expected_root);
        println!("\n=== Root Hash Comparison ===");
        println!("Expected: {}", expected_root);
        println!("Actual:   {}", root_hash);
        if root_hash_matches {
            println!("✓ Root hash matches");
        } else {
            println!("✗ Root hash DOES NOT match");
        }
        println!("============================");
    }

    // Generate output with proofs
    println!("\nGenerating Merkle proofs...");
    let output_data = generate_output(&trie, address_amount_map, args.keep_prefix)?;

    // Compare with reference JSON if provided
    let mut comparison_result: Option<ComparisonResult> = None;
    if let Some(ref_json_path) = &args.compare_json {
        println!("\nLoading reference JSON from {:?}...", ref_json_path);
        let reference_data = load_reference_json(ref_json_path)?;

        println!("Comparing output with reference data...");
        let result = compare_output_data(&output_data, &reference_data);
        result.print_report();
        comparison_result = Some(result);
    }

    // Write to output file
    println!("\nWriting output to {:?}...", args.output);
    write_output(&args.output, &output_data, args.pretty)?;

    println!("\n✓ Successfully generated Merkle Trie data!");
    println!("  Root Hash: {}", output_data.root_hash);
    println!("  Allocations: {}", output_data.allocations.len());

    if args.keep_prefix {
        println!("\n  Note: 0x prefix was kept in leaf data for hashing.");
    } else {
        println!("\n  Note: Data is automatically sorted by leaf data (address + amount)");
        println!("        for deterministic output. The same data will always produce");
        println!("        the same root hash regardless of CSV row order.");
    }

    // Determine exit code
    let mut exit_code = 0;

    // Check root hash comparison
    if args.compare_root.is_some() && !root_hash_matches {
        eprintln!("\n✗ ERROR: Root hash comparison failed!");
        exit_code = 1;
    }

    // Check JSON comparison
    if let Some(result) = comparison_result {
        if !result.is_success() {
            eprintln!("\n✗ ERROR: Output comparison with reference JSON failed!");
            exit_code = 1;
        }
    }

    // Exit with appropriate code
    if exit_code != 0 {
        process::exit(exit_code);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_address_strip_prefix() {
        assert_eq!(normalize_address("0xABCDEF", false), "abcdef");
        assert_eq!(normalize_address("ABCDEF", false), "abcdef");
        assert_eq!(normalize_address("0xabcdef", false), "abcdef");
    }

    #[test]
    fn test_normalize_address_keep_prefix() {
        assert_eq!(normalize_address("0xABCDEF", true), "0xabcdef");
        assert_eq!(normalize_address("ABCDEF", true), "abcdef");
        assert_eq!(normalize_address("0xabcdef", true), "0xabcdef");
    }

    #[test]
    fn test_ensure_prefix() {
        assert_eq!(ensure_prefix("abcdef"), "0xabcdef");
        assert_eq!(ensure_prefix("0xabcdef"), "0xabcdef");
    }

    #[test]
    fn test_compare_root_hash() {
        assert!(compare_root_hash(
            "0xABCDEF123456",
            "0xabcdef123456"
        ));
        assert!(compare_root_hash(
            "ABCDEF123456",
            "0xabcdef123456"
        ));
        assert!(!compare_root_hash(
            "0xABCDEF",
            "0x123456"
        ));
    }

    #[test]
    fn test_encode_leaf_data_without_prefix() {
        let address = "742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = "1000000000000000000";

        let result = encode_leaf_data(address, amount, false);
        assert!(result.is_ok());

        let leaf_data = result.unwrap();
        assert_eq!(leaf_data.len(), 52); // 20 bytes (address) + 32 bytes (amount)
    }

    #[test]
    fn test_encode_leaf_data_with_prefix() {
        let address = "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = "1000000000000000000";

        let result = encode_leaf_data(address, amount, true);
        assert!(result.is_ok());

        let leaf_data = result.unwrap();
        // "0x" (2 bytes) + 40 hex chars = 42 bytes for address + 32 bytes for amount = 74 bytes
        assert_eq!(leaf_data.len(), 74);
    }

    #[test]
    fn test_encode_leaf_data_invalid_address() {
        let address = "invalid";
        let amount = "1000";

        let result = encode_leaf_data(address, amount, false);
        assert!(result.is_err());
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
