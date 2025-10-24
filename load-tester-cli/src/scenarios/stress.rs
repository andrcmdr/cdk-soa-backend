//! Stress test scenario with ramping

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

pub struct StressScenario {
    ramp_up: u64,
    peak: u64,
    ramp_down: u64,
    target_tps: u64,
}

impl StressScenario {
    pub fn new(ramp_up: u64, peak: u64, ramp_down: u64, target_tps: u64) -> Self {
        Self { ramp_up, peak, ramp_down, target_tps }
    }

    fn calculate_current_tps(&self, elapsed: u64) -> u64 {
        if elapsed < self.ramp_up {
            // Ramping up
            (self.target_tps * elapsed) / self.ramp_up
        } else if elapsed < self.ramp_up + self.peak {
            // Peak load
            self.target_tps
        } else if elapsed < self.ramp_up + self.peak + self.ramp_down {
            // Ramping down
            let time_in_ramp_down = elapsed - self.ramp_up - self.peak;
            self.target_tps - ((self.target_tps * time_in_ramp_down) / self.ramp_down)
        } else {
            0
        }
    }
}

#[async_trait]
impl TestScenario for StressScenario {
    fn name(&self) -> &str {
        "Stress Test"
    }

    fn total_operations(&self) -> usize {
        let total_time = self.ramp_up + self.peak + self.ramp_down;
        (self.target_tps * total_time / 2) as usize
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting stress test: ramp_up={}s, peak={}s, ramp_down={}s, target={}tps",
              self.ramp_up, self.peak, self.ramp_down, self.target_tps);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();
        let total_duration = self.ramp_up + self.peak + self.ramp_down;

        let mut last_second = 0;
        let mut current_second_count = 0;

        while start_time.elapsed().as_secs() < total_duration {
            let elapsed = start_time.elapsed().as_secs();
            let target_tps = self.calculate_current_tps(elapsed);

            // Reset counter for new second
            if elapsed != last_second {
                last_second = elapsed;
                current_second_count = 0;
            }

            if current_second_count >= target_tps {
                sleep(Duration::from_millis(100)).await;
                continue;
            }

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
                    &[DynSolValue::Uint(alloy_primitives::U256::from(50).into(), 256)],
                ).await {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, 50000, timestamp);
                        progress.set_message(format!("Target: {} TPS, Current: {:.2} TPS",
                                                    target_tps, stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Transaction failed: {}", e);
                    }
                }

                progress.inc(1);
            });

            current_second_count += 1;
            sleep(Duration::from_millis(1000 / target_tps.max(1))).await;
        }

        info!("Stress test completed");
        Ok(())
    }
}
