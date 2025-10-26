//! Calldata size test scenario

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::info;

use tx_producer::prelude::*;
use crate::scenarios::TestScenario;
use crate::stats::TestStatistics;

pub struct CalldataScenario {
    min_size: usize,
    max_size: usize,
    increment: usize,
}

impl CalldataScenario {
    pub fn new(min_size: usize, max_size: usize, increment: usize) -> Self {
        Self { min_size, max_size, increment }
    }
}

#[async_trait]
impl TestScenario for CalldataScenario {
    fn name(&self) -> &str {
        "Calldata Size Test"
    }

    fn total_operations(&self) -> usize {
        ((self.max_size - self.min_size) / self.increment) + 1
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        _workers: usize,
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting calldata test: {} to {} bytes", self.min_size, self.max_size);

        let start_time = Instant::now();
        let mut rng = rand::thread_rng();

        let mut size = self.min_size;
        while size <= self.max_size {
            // Generate random data of specified size
            let data: Vec<u8> = (0..size).map(|_| rng.gen()).collect();

            let tx_start = Instant::now();
            let timestamp = start_time.elapsed().as_secs_f64();

            progress.set_message(format!("Testing {} bytes", size));

            match contract.send_transaction(
                "bigCalldataEcho",
                &[DynSolValue::Bytes(data.clone())],
            ).await {
                Ok(_tx_hash) => {
                    let latency = tx_start.elapsed();
                    let gas_estimate = 21000 + (size as u64 * 16); // Approximate calldata gas
                    let mut stats = stats.write().await;
                    stats.record_success(latency, gas_estimate, timestamp);
                }
                Err(e) => {
                    let mut stats = stats.write().await;
                    stats.record_failure(format!("Size {}: {}", size, e), timestamp);
                }
            }

            progress.inc(1);
            size += self.increment;
        }

        info!("Calldata test completed");
        Ok(())
    }
}
