//! CLI argument parsing

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "load-tester")]
#[command(about = "Blockchain load testing and benchmarking tool", long_about = None)]
#[command(version)]
pub struct Cli {
    /// RPC endpoint URL
    #[arg(short, long, env = "RPC_URL", default_value = "http://localhost:8545")]
    pub rpc_url: String,

    /// Contract address
    #[arg(short, long, env = "CONTRACT_ADDRESS")]
    pub contract: String,

    /// Private key for signing transactions
    #[arg(short = 'k', long, env = "PRIVATE_KEY")]
    pub private_key: String,

    /// Chain ID
    #[arg(long, env = "CHAIN_ID", default_value = "1")]
    pub chain_id: u64,

    /// Path to ABI file
    #[arg(short, long, default_value = "abi/LoadTester.json")]
    pub abi: PathBuf,

    /// Test scenario to run
    #[command(subcommand)]
    pub scenario: TestScenario,

    /// Number of concurrent workers
    #[arg(short = 'w', long, default_value = "10")]
    pub workers: usize,

    /// Test duration in seconds (0 = run once)
    #[arg(short = 'd', long, default_value = "60")]
    pub duration: u64,

    /// Rate limit (transactions per second, 0 = unlimited)
    #[arg(short = 'r', long, default_value = "0")]
    pub rate_limit: u64,

    /// Output format
    #[arg(short = 'o', long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Save results to file
    #[arg(long)]
    pub save_results: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

#[derive(Debug, Subcommand)]
pub enum TestScenario {
    /// Basic load test - simple function calls
    Basic {
        /// Number of transactions
        #[arg(short = 'n', long, default_value = "1000")]
        count: usize,

        /// Gas loop iterations per transaction
        #[arg(short = 'i', long, default_value = "100")]
        iterations: u64,
    },

    /// Storage stress test
    Storage {
        /// Number of storage writes
        #[arg(short = 'w', long, default_value = "10")]
        writes: u64,

        /// Number of storage reads
        #[arg(short = 'r', long, default_value = "10")]
        reads: u64,

        /// Number of transactions
        #[arg(short = 'n', long, default_value = "1000")]
        count: usize,
    },

    /// Calldata test - varying sizes
    Calldata {
        /// Minimum calldata size in bytes
        #[arg(long, default_value = "100")]
        min_size: usize,

        /// Maximum calldata size in bytes
        #[arg(long, default_value = "10000")]
        max_size: usize,

        /// Size increment
        #[arg(long, default_value = "1000")]
        increment: usize,
    },

    /// Batch token minting test
    BatchMint {
        /// Token type (erc20, erc721, erc1155)
        #[arg(short = 't', long, value_enum)]
        token_type: TokenType,

        /// Batch size
        #[arg(short = 'b', long, default_value = "10")]
        batch_size: usize,

        /// Number of batches
        #[arg(short = 'n', long, default_value = "100")]
        batches: usize,
    },

    /// External call test
    ExternalCall {
        /// Call type (call, delegatecall, staticcall)
        #[arg(short = 't', long, value_enum)]
        call_type: CallType,

        /// Gas limit per call
        #[arg(short = 'g', long, default_value = "100000")]
        gas_limit: u64,

        /// Number of calls
        #[arg(short = 'n', long, default_value = "1000")]
        count: usize,
    },

    /// Cryptography test (signatures, Merkle proofs)
    Crypto {
        /// Test type (signature, merkle)
        #[arg(short = 't', long, value_enum)]
        test_type: CryptoTestType,

        /// Number of verifications
        #[arg(short = 'n', long, default_value = "1000")]
        count: usize,
    },

    /// Mixed workload test
    Mixed {
        /// Duration in seconds
        #[arg(short = 'd', long, default_value = "300")]
        duration: u64,

        /// Workload profile
        #[arg(short = 'p', long, value_enum, default_value = "balanced")]
        profile: WorkloadProfile,
    },

    /// Stress test - push limits
    Stress {
        /// Ramp up duration in seconds
        #[arg(long, default_value = "60")]
        ramp_up: u64,

        /// Peak duration in seconds
        #[arg(long, default_value = "300")]
        peak: u64,

        /// Ramp down duration in seconds
        #[arg(long, default_value = "60")]
        ramp_down: u64,

        /// Target TPS at peak
        #[arg(long, default_value = "1000")]
        target_tps: u64,
    },

    /// Endurance test - sustained load
    Endurance {
        /// Test duration in hours
        #[arg(short = 'd', long, default_value = "24")]
        hours: u64,

        /// Target TPS
        #[arg(short = 't', long, default_value = "100")]
        tps: u64,
    },

    /// Custom scenario from config file
    Custom {
        /// Path to scenario config file
        #[arg(short = 'f', long)]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum TokenType {
    Erc20,
    Erc721,
    Erc1155,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CallType {
    Call,
    Delegatecall,
    Staticcall,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CryptoTestType {
    Signature,
    Merkle,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum WorkloadProfile {
    Balanced,
    StorageHeavy,
    ComputeHeavy,
    CalldataHeavy,
    Mixed,
}
