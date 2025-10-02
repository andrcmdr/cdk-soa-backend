use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
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

/// Normalize Ethereum address to lowercase without 0x prefix
fn normalize_address(address: &str) -> String {
    address
        .trim()
        .to_lowercase()
        .strip_prefix("0x")
        .unwrap_or(address.trim())
        .to_string()
}

/// Encode address and amount as leaf data
fn encode_leaf_data(address: &str, amount: &str) -> Result<Vec<u8>> {
    let mut leaf_data = Vec::new();

    // Parse address (remove 0x prefix if present)
    let addr_str = address.strip_prefix("0x").unwrap_or(address);
    let addr_bytes = hex::decode(addr_str)
        .with_context(|| format!("Failed to decode address: {}", address))?;

    if addr_bytes.len() != 20 {
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
fn process_csv_file(input_path: &PathBuf) -> Result<(MerkleTrie, BTreeMap<String, String>)> {
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
        let normalized_address = normalize_address(address);

        // Encode leaf data
        let leaf_data = encode_leaf_data(&normalized_address, amount)
            .with_context(|| format!("Failed to encode leaf data at row {}", row_count + 1))?;

        // Add to trie (BTreeMap in trie will handle sorting)
        trie.add_leaf(leaf_data);

        // Store address-amount mapping (BTreeMap keeps it sorted)
        address_amount_map.insert(normalized_address, amount.to_string());

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
fn generate_output(trie: &MerkleTrie, address_amount_map: BTreeMap<String, String>) -> Result<OutputData> {
    let root_hash = trie.get_root_hash_hex()
        .ok_or_else(|| anyhow::anyhow!("Failed to get root hash"))?;

    let mut allocations = BTreeMap::new();

    for (address, amount) in address_amount_map.iter() {
        // Encode leaf data for this address
        let leaf_data = encode_leaf_data(address, amount)?;

        // Generate proof
        let proof = trie.generate_proof(&leaf_data)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate proof for address: 0x{}", address))?;

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

        // Add to output (with 0x prefix)
        allocations.insert(format!("0x{}", address), allocation_proof);
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

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Merkle Trie CLI Tool");
    println!("===================");
    println!("Input file: {:?}", args.input);
    println!("Output file: {:?}", args.output);
    println!();

    // Process CSV file
    println!("Processing CSV file...");
    let (trie, address_amount_map) = process_csv_file(&args.input)?;

    // Get root hash
    let root_hash = trie.get_root_hash_hex()
        .ok_or_else(|| anyhow::anyhow!("Failed to get root hash"))?;

    if args.verbose {
        println!("\nRoot Hash: {}", root_hash);
        println!("Total addresses: {}", address_amount_map.len());
    }

    // Generate output with proofs
    println!("\nGenerating Merkle proofs...");
    let output_data = generate_output(&trie, address_amount_map)?;

    // Write to output file
    println!("Writing output to {:?}...", args.output);
    write_output(&args.output, &output_data, args.pretty)?;

    println!("\n✓ Successfully generated Merkle Trie data!");
    println!("  Root Hash: {}", output_data.root_hash);
    println!("  Allocations: {}", output_data.allocations.len());
    println!("\n  Note: Data is automatically sorted by leaf data (address + amount)");
    println!("        for deterministic output. The same data will always produce");
    println!("        the same root hash regardless of CSV row order.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_address() {
        assert_eq!(normalize_address("0xABCDEF"), "abcdef");
        assert_eq!(normalize_address("ABCDEF"), "abcdef");
        assert_eq!(normalize_address("0xabcdef"), "abcdef");
    }

    #[test]
    fn test_encode_leaf_data() {
        let address = "742C4d97C86bCF0176776C16e073b8c6f9Db4021";
        let amount = "1000000000000000000";

        let result = encode_leaf_data(address, amount);
        assert!(result.is_ok());

        let leaf_data = result.unwrap();
        assert_eq!(leaf_data.len(), 52); // 20 bytes (address) + 32 bytes (amount)
    }

    #[test]
    fn test_encode_leaf_data_invalid_address() {
        let address = "invalid";
        let amount = "1000";

        let result = encode_leaf_data(address, amount);
        assert!(result.is_err());
    }
}
