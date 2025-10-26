//! Mixed workload scenario

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use tx_producer::prelude::*;
use crate::scenarios::TestScenario;
use crate::stats::TestStatistics;

pub struct MixedScenario {
    duration: u64,
    profile: String,
}

impl MixedScenario {
    pub fn new(duration: u64, profile: String) -> Self {
        Self { duration, profile }
    }

    async fn execute_operation(
        &self,
        contract: &ContractClient,
        op_type: &str,
    ) -> Result<alloy_primitives::B256> {
        match op_type {
            "storage" => {
                contract.send_transaction(
                    "touchStorage",
                    &[
                        DynSolValue::Uint(alloy_primitives::U256::from(5).into(), 256),
                        DynSolValue::Uint(alloy_primitives::U256::from(5).into(), 256),
                        DynSolValue::FixedBytes(alloy_primitives::B256::random(), 32),
                    ],
                ).await
            }
            "compute" => {
                contract.send_transaction(
                    "consumeGas",
                    &[DynSolValue::Uint(alloy_primitives::U256::from(100).into(), 256)],
                ).await
            }
            "calldata" => {
                let data = vec![0u8; 1000];
                contract.send_transaction(
                    "bigCalldataEcho",
                    &[DynSolValue::Bytes(data)],
                ).await
            }
            _ => {
                contract.send_transaction(
                    "consumeGas",
                    &[DynSolValue::Uint(alloy_primitives::U256::from(50).into(), 256)],
                ).await
            }
        }
    }

    fn get_operation_mix(&self) -> Vec<(&str, f64)> {
        match self.profile.as_str() {
            "StorageHeavy" => vec![
                ("storage", 0.6),
                ("compute", 0.2),
                ("calldata", 0.2),
            ],
            "ComputeHeavy" => vec![
                ("storage", 0.2),
                ("compute", 0.6),
                ("calldata", 0.2),
            ],
            "CalldataHeavy" => vec![
                ("storage", 0.2),
                ("compute", 0.2),
                ("calldata", 0.6),
            ],
            "Balanced" | _ => vec![
                ("storage", 0.33),
                ("compute", 0.33),
                ("calldata", 0.34),
            ],
        }
    }
}

#[async_trait]
impl TestScenario for MixedScenario {
    fn name(&self) -> &str {
        "Mixed Workload"
    }

    fn total_operations(&self) -> usize {
        (self.duration * 10) as usize // Estimate
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting mixed workload test: {} seconds, {} profile",
              self.duration, self.profile);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_secs(self.duration);

        let op_mix = self.get_operation_mix();
        let mut rng = rand::thread_rng();

        let mut task_count = 0;
        while Instant::now() < end_time {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            // Select operation based on mix
            let rand_val: f64 = rng.gen();
            let mut cumulative = 0.0;
            let mut selected_op = "compute";

            for (op, weight) in &op_mix {
                cumulative += weight;
                if rand_val <= cumulative {
                    selected_op = op;
                    break;
                }
            }

            let op_type = selected_op.to_string();
            let profile = self.profile.clone();

            tokio::spawn(async move {
                let _permit = permit;

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                let scenario = MixedScenario::new(0, profile);
                match scenario.execute_operation(&contract, &op_type).await {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, 100000, timestamp);
                        progress.set_message(format!("TPS: {:.2}", stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Operation {} failed: {}", op_type, e);
                    }
                }

                progress.inc(1);
            });

            task_count += 1;
            sleep(Duration::from_millis(100)).await;
        }

        info!("Mixed workload test completed: {} operations", task_count);
        Ok(())
    }
}
