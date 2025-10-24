//! Basic load test scenario

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use tx_producer::prelude::*;
use crate::scenarios::TestScenario;
use crate::stats::TestStatistics;

pub struct BasicScenario {
    count: usize,
    iterations: u64,
}

impl BasicScenario {
    pub fn new(count: usize, iterations: u64) -> Self {
        Self { count, iterations }
    }
}

#[async_trait]
impl TestScenario for BasicScenario {
    fn name(&self) -> &str {
        "Basic Load Test"
    }

    fn total_operations(&self) -> usize {
        self.count
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        rate_limit: u64,
    ) -> Result<()> {
        info!("Starting basic load test: {} transactions", self.count);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();

        let mut tasks = Vec::new();

        for i in 0..self.count {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let iterations = self.iterations;

            let task = tokio::spawn(async move {
                let _permit = permit;

                // Rate limiting
                if rate_limit > 0 {
                    let delay = Duration::from_secs_f64(1.0 / rate_limit as f64);
                    sleep(delay).await;
                }

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                match contract.send_transaction(
                    "consumeGas",
                    &[DynSolValue::Uint(alloy_primitives::U256::from(iterations).into(), 256)],
                ).await {
                    Ok(tx_hash) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, 100000, timestamp); // Approximate gas
                        progress.set_message(format!("TPS: {:.2}", stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Transaction {} failed: {}", i, e);
                    }
                }

                progress.inc(1);
            });

            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        info!("Basic load test completed");
        Ok(())
    }
}
