//! Test runner and execution engine

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn, error};

use tx_producer::prelude::*;

use crate::config::{LoadTestConfig, ScenarioConfig};
use crate::scenarios::{self, TestScenario as ScenarioTrait};
use crate::stats::TestStatistics;

pub struct TestRunner {
    config: LoadTestConfig,
    contract: Arc<ContractClient>,
    stats: Arc<tokio::sync::RwLock<TestStatistics>>,
}

impl TestRunner {
    pub async fn new(config: LoadTestConfig) -> Result<Self> {
        info!("Initializing test runner...");

        // Configure provider
        let provider_config = ProviderConfig {
            rpc_url: config.rpc_url.clone(),
            chain_id: config.chain_id,
            timeout_seconds: 60,
        };

        // Create provider with signer
        let provider_manager = ProviderManager::new(provider_config)
            .context("Failed to create provider")?
            .with_signer(&config.private_key)
            .context("Failed to add signer")?;

        info!("Provider initialized, checking connection...");
        let block_number = provider_manager.check_connection().await?;
        info!("Connected to network at block {}", block_number);

        // Configure contract
        let contract_address: alloy_primitives::Address = config.contract_address
            .parse()
            .context("Invalid contract address")?;

        let contract_config = ContractConfig {
            address: contract_address,
            abi_path: config.abi_path.clone(),
        };

        // Create contract client
        let contract = ContractClient::new(
            contract_config,
            Arc::new(provider_manager),
        )
        .await
        .context("Failed to create contract client")?;

        info!("Contract client initialized: {}", contract_address);

        let stats = Arc::new(tokio::sync::RwLock::new(TestStatistics::new()));

        Ok(Self {
            config,
            contract: Arc::new(contract),
            stats,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("{}", "Starting load test...".bright_green().bold());
        println!();

        self.print_config();

        // Create scenario
        let scenario = self.create_scenario()?;

        // Run test
        let start_time = Instant::now();
        self.execute_scenario(scenario).await?;
        let total_duration = start_time.elapsed();

        // Print results
        println!();
        self.print_results(total_duration).await?;

        Ok(())
    }

    fn create_scenario(&self) -> Result<Box<dyn ScenarioTrait>> {
        match &self.config.scenario {
            ScenarioConfig::Basic { count, iterations } => {
                Ok(Box::new(scenarios::BasicScenario::new(*count, *iterations)))
            }
            ScenarioConfig::Storage { writes, reads, count } => {
                Ok(Box::new(scenarios::StorageScenario::new(*writes, *reads, *count)))
            }
            ScenarioConfig::Calldata { min_size, max_size, increment } => {
                Ok(Box::new(scenarios::CalldataScenario::new(*min_size, *max_size, *increment)))
            }
            ScenarioConfig::BatchMint { token_type, batch_size, batches } => {
                Ok(Box::new(scenarios::BatchMintScenario::new(
                    token_type.clone(),
                    *batch_size,
                    *batches,
                )))
            }
            ScenarioConfig::ExternalCall { call_type, gas_limit, count } => {
                Ok(Box::new(scenarios::ExternalCallScenario::new(
                    call_type.clone(),
                    *gas_limit,
                    *count,
                )))
            }
            ScenarioConfig::Crypto { test_type, count } => {
                Ok(Box::new(scenarios::CryptoScenario::new(test_type.clone(), *count)))
            }
            ScenarioConfig::Mixed { duration, profile } => {
                Ok(Box::new(scenarios::MixedScenario::new(*duration, profile.clone())))
            }
            ScenarioConfig::Stress { ramp_up, peak, ramp_down, target_tps } => {
                Ok(Box::new(scenarios::StressScenario::new(
                    *ramp_up,
                    *peak,
                    *ramp_down,
                    *target_tps,
                )))
            }
            ScenarioConfig::Endurance { hours, tps } => {
                Ok(Box::new(scenarios::EnduranceScenario::new(*hours, *tps)))
            }
        }
    }

    async fn execute_scenario(&self, scenario: Box<dyn ScenarioTrait>) -> Result<()> {
        let progress = ProgressBar::new(scenario.total_operations() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("█▓▒░"),
        );

        // Execute scenario
        scenario.execute(
            self.contract.clone(),
            self.stats.clone(),
            progress.clone(),
            self.config.workers,
            self.config.rate_limit,
        ).await?;

        progress.finish_with_message("Complete");
        Ok(())
    }

    fn print_config(&self) {
        println!("{}", "Configuration:".bright_yellow().bold());
        println!("  RPC URL: {}", self.config.rpc_url);
        println!("  Contract: {}", self.config.contract_address);
        println!("  Workers: {}", self.config.workers);
        println!("  Duration: {}s", self.config.duration);
        if self.config.rate_limit > 0 {
            println!("  Rate Limit: {} TPS", self.config.rate_limit);
        }
        println!();
    }

    async fn print_results(&self, duration: Duration) -> Result<()> {
        let stats = self.stats.read().await;

        println!("{}", "═══════════════════════════════════════".bright_cyan());
        println!("{}", "Test Results".bright_green().bold());
        println!("{}", "═══════════════════════════════════════".bright_cyan());
        println!();

        println!("{}", "Overall Statistics:".bright_yellow());
        println!("  Total Duration: {:.2}s", duration.as_secs_f64());
        println!("  Total Transactions: {}", stats.total_transactions);
        println!("  Successful: {} ({:.1}%)",
            stats.successful_transactions,
            stats.success_rate() * 100.0
        );
        println!("  Failed: {}", stats.failed_transactions);
        println!();

        println!("{}", "Performance:".bright_yellow());
        println!("  Average TPS: {:.2}", stats.tps(duration));
        println!("  Average Latency: {:.2}ms", stats.avg_latency_ms());
        println!("  P50 Latency: {:.2}ms", stats.p50_latency_ms());
        println!("  P95 Latency: {:.2}ms", stats.p95_latency_ms());
        println!("  P99 Latency: {:.2}ms", stats.p99_latency_ms());
        println!("  Max Latency: {:.2}ms", stats.max_latency_ms());
        println!();

        println!("{}", "Gas Usage:".bright_yellow());
        println!("  Total Gas: {}", stats.total_gas_used);
        println!("  Average Gas per TX: {:.2}", stats.avg_gas_per_tx());
        println!();

        if stats.failed_transactions > 0 {
            println!("{}", "Failed Transactions:".bright_red());
            for (i, error) in stats.errors.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, error);
            }
            if stats.errors.len() > 10 {
                println!("  ... and {} more", stats.errors.len() - 10);
            }
            println!();
        }

        println!("{}", "═══════════════════════════════════════".bright_cyan());

        Ok(())
    }
}
