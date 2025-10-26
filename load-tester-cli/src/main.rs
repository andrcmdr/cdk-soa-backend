//! Load Tester CLI - Blockchain benchmarking and stress testing tool

mod cli;
mod config;
mod runner;
mod scenarios;
mod stats;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::Cli;
use config::LoadTestConfig;
use runner::TestRunner;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Print banner
    print_banner();

    // Load configuration
    let config = LoadTestConfig::from_cli(&cli)?;

    // Create and run test runner
    let mut runner = TestRunner::new(config).await?;
    runner.run().await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn print_banner() {
    println!("{}", "╔══════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║      🔥 BLOCKCHAIN LOAD TESTER 🔥                    ║".bright_cyan());
    println!("{}", "║      Network Benchmarking & Stress Testing           ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}
