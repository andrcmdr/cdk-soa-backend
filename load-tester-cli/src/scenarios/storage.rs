//! Storage stress test scenario

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

pub struct StorageScenario {
    writes: u64,
    reads: u64,
    count: usize,
}

impl StorageScenario {
    pub fn new(writes: u64, reads: u64, count: usize) -> Self {
        Self { writes, reads, count }
    }
}

#[async_trait]
impl TestScenario for StorageScenario {
    fn name(&self) -> &str {
        "Storage Stress Test"
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
        info!("Starting storage stress test: {} writes, {} reads, {} transactions",
              self.writes, self.reads, self.count);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();

        let mut tasks = Vec::new();

        for i in 0..self.count {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let writes = self.writes;
            let reads = self.reads;

            let task = tokio::spawn(async move {
                let _permit = permit;

                if rate_limit > 0 {
                    let delay = Duration::from_secs_f64(1.0 / rate_limit as f64);
                    sleep(delay).await;
                }

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                // Generate random tag for this transaction
                let tag = alloy_primitives::B256::random();

                match contract.send_transaction(
                    "touchStorage",
                    &[
                        DynSolValue::Uint(alloy_primitives::U256::from(writes).into(), 256),
                        DynSolValue::Uint(alloy_primitives::U256::from(reads).into(), 256),
                        DynSolValue::FixedBytes(tag, 32),
                    ],
                ).await {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let gas_estimate = 20000 + (writes * 20000) + (reads * 2100);
                        let mut stats = stats.write().await;
                        stats.record_success(latency, gas_estimate, timestamp);
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

        for task in tasks {
            let _ = task.await;
        }

        info!("Storage stress test completed");
        Ok(())
    }
}
