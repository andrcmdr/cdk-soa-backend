//! Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::cli::{Cli, TestScenario};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    pub rpc_url: String,
    pub contract_address: String,
    pub private_key: String,
    pub chain_id: u64,
    pub abi_path: String,
    pub workers: usize,
    pub duration: u64,
    pub rate_limit: u64,
    pub scenario: ScenarioConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScenarioConfig {
    Basic {
        count: usize,
        iterations: u64,
    },
    Storage {
        writes: u64,
        reads: u64,
        count: usize,
    },
    Calldata {
        min_size: usize,
        max_size: usize,
        increment: usize,
    },
    BatchMint {
        token_type: String,
        batch_size: usize,
        batches: usize,
    },
    ExternalCall {
        call_type: String,
        gas_limit: u64,
        count: usize,
    },
    Crypto {
        test_type: String,
        count: usize,
    },
    Mixed {
        duration: u64,
        profile: String,
    },
    Stress {
        ramp_up: u64,
        peak: u64,
        ramp_down: u64,
        target_tps: u64,
    },
    Endurance {
        hours: u64,
        tps: u64,
    },
}

impl LoadTestConfig {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let scenario = match &cli.scenario {
            TestScenario::Basic { count, iterations } => ScenarioConfig::Basic {
                count: *count,
                iterations: *iterations,
            },
            TestScenario::Storage { writes, reads, count } => ScenarioConfig::Storage {
                writes: *writes,
                reads: *reads,
                count: *count,
            },
            TestScenario::Calldata { min_size, max_size, increment } => ScenarioConfig::Calldata {
                min_size: *min_size,
                max_size: *max_size,
                increment: *increment,
            },
            TestScenario::BatchMint { token_type, batch_size, batches } => ScenarioConfig::BatchMint {
                token_type: format!("{:?}", token_type),
                batch_size: *batch_size,
                batches: *batches,
            },
            TestScenario::ExternalCall { call_type, gas_limit, count } => ScenarioConfig::ExternalCall {
                call_type: format!("{:?}", call_type),
                gas_limit: *gas_limit,
                count: *count,
            },
            TestScenario::Crypto { test_type, count } => ScenarioConfig::Crypto {
                test_type: format!("{:?}", test_type),
                count: *count,
            },
            TestScenario::Mixed { duration, profile } => ScenarioConfig::Mixed {
                duration: *duration,
                profile: format!("{:?}", profile),
            },
            TestScenario::Stress { ramp_up, peak, ramp_down, target_tps } => ScenarioConfig::Stress {
                ramp_up: *ramp_up,
                peak: *peak,
                ramp_down: *ramp_down,
                target_tps: *target_tps,
            },
            TestScenario::Endurance { hours, tps } => ScenarioConfig::Endurance {
                hours: *hours,
                tps: *tps,
            },
            TestScenario::Custom { config } => {
                return Self::from_file(config);
            }
        };

        Ok(Self {
            rpc_url: cli.rpc_url.clone(),
            contract_address: cli.contract.clone(),
            private_key: cli.private_key.clone(),
            chain_id: cli.chain_id,
            abi_path: cli.abi.to_string_lossy().to_string(),
            workers: cli.workers,
            duration: cli.duration,
            rate_limit: cli.rate_limit,
            scenario,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        let config: Self = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
