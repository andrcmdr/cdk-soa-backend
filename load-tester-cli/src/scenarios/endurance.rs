//! Endurance test scenario

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

pub struct EnduranceScenario {
    hours: u64,
    tps: u64,
}

impl EnduranceScenario {
    pub fn new(hours: u64, tps: u64) -> Self {
        Self { hours, tps }
    }
}

#[async_trait]
impl TestScenario for EnduranceScenario {
    fn name(&self) -> &str {
        "Endurance Test"
    }

    fn total_operations(&self) -> usize {
        (self.hours * 3600 * self.tps) as usize
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting endurance test: {} hours at {} TPS", self.hours, self.tps);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();
        let duration = Duration::from_secs(self.hours * 3600);
        let end_time = start_time + duration;

        let delay_per_tx = Duration::from_secs_f64(1.0 / self.tps as f64);

        let mut last_report = Instant::now();
        let report_interval = Duration::from_secs(300); // Report every 5 minutes

        while Instant::now() < end_time {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                let _permit = permit;

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                match contract.send_transaction(
                    "consumeGas",
                    &[DynSolValue::Uint(alloy_primitives::U256::from(100).into(), 256)],
                ).await {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, 100000, timestamp);
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Transaction failed: {}", e);
                    }
                }

                progress.inc(1);
            });

            // Periodic reporting
            if last_report.elapsed() >= report_interval {
                let stats = stats.read().await;
                let elapsed = start_time.elapsed();
                let remaining = duration.saturating_sub(elapsed);

                info!("Endurance test progress: {:.1}% complete, {} transactions, {:.2} TPS, remaining: {}h {}m",
                      (elapsed.as_secs_f64() / duration.as_secs_f64()) * 100.0,
                      stats.total_transactions,
                      stats.tps(elapsed),
                      remaining.as_secs() / 3600,
                      (remaining.as_secs() % 3600) / 60
                );

                progress.set_message(format!(
                    "TPS: {:.2}, Success: {:.1}%, Remaining: {}h {}m",
                    stats.tps(elapsed),
                    stats.success_rate() * 100.0,
                    remaining.as_secs() / 3600,
                    (remaining.as_secs() % 3600) / 60
                ));

                last_report = Instant::now();
            }

            sleep(delay_per_tx).await;
        }

        info!("Endurance test completed");
        Ok(())
    }
}
