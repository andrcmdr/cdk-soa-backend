//! External call test scenario

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn};

use tx_producer::prelude::*;
use crate::scenarios::TestScenario;
use crate::stats::TestStatistics;

pub struct ExternalCallScenario {
    call_type: String,
    gas_limit: u64,
    count: usize,
}

impl ExternalCallScenario {
    pub fn new(call_type: String, gas_limit: u64, count: usize) -> Self {
        Self { call_type, gas_limit, count }
    }
}

#[async_trait]
impl TestScenario for ExternalCallScenario {
    fn name(&self) -> &str {
        "External Call Test"
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
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting external call test: {} type, {} gas limit, {} calls",
              self.call_type, self.gas_limit, self.count);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();

        let mut tasks = Vec::new();

        for i in 0..self.count {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let call_type = self.call_type.clone();
            let gas_limit = self.gas_limit;

            let task = tokio::spawn(async move {
                let _permit = permit;

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                let data = vec![0u8; 32]; // Dummy call data

                let function_name = match call_type.as_str() {
                    "Call" => "callDummy",
                    "Delegatecall" => "delegateWork",
                    _ => "callDummy",
                };

                match contract.send_transaction(
                    function_name,
                    &[
                        DynSolValue::Bytes(data),
                        DynSolValue::Uint(alloy_primitives::U256::from(gas_limit).into(), 256),
                    ],
                ).await {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, gas_limit, timestamp);
                        progress.set_message(format!("TPS: {:.2}", stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Call {} failed: {}", i, e);
                    }
                }

                progress.inc(1);
            });

            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        info!("External call test completed");
        Ok(())
    }
}
