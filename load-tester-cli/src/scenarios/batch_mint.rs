//! Batch token minting scenario

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn};

use tx_producer::prelude::*;
use crate::scenarios::TestScenario;
use crate::stats::TestStatistics;

pub struct BatchMintScenario {
    token_type: String,
    batch_size: usize,
    batches: usize,
}

impl BatchMintScenario {
    pub fn new(token_type: String, batch_size: usize, batches: usize) -> Self {
        Self { token_type, batch_size, batches }
    }

    fn generate_addresses(&self) -> Vec<alloy_primitives::Address> {
        let mut rng = rand::thread_rng();
        (0..self.batch_size)
            .map(|_| {
                let bytes: [u8; 20] = rng.gen();
                alloy_primitives::Address::from(bytes)
            })
            .collect()
    }
}

#[async_trait]
impl TestScenario for BatchMintScenario {
    fn name(&self) -> &str {
        "Batch Token Minting"
    }

    fn total_operations(&self) -> usize {
        self.batches
    }

    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        _rate_limit: u64,
    ) -> Result<()> {
        info!("Starting batch mint test: {} type, {} per batch, {} batches",
              self.token_type, self.batch_size, self.batches);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();

        let mut tasks = Vec::new();

        for i in 0..self.batches {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let token_type = self.token_type.clone();
            let batch_size = self.batch_size;

            let task = tokio::spawn(async move {
                let _permit = permit;

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                let result = match token_type.as_str() {
                    "Erc20" => {
                        let addresses: Vec<alloy_primitives::Address> = (0..batch_size)
                            .map(|_| alloy_primitives::Address::random())
                            .collect();
                        let amounts: Vec<alloy_primitives::U256> = (0..batch_size)
                            .map(|_| alloy_primitives::U256::from(1000000))
                            .collect();

                        let addresses_dyn: Vec<DynSolValue> = addresses
                            .into_iter()
                            .map(|a| DynSolValue::Address(a))
                            .collect();
                        let amounts_dyn: Vec<DynSolValue> = amounts
                            .into_iter()
                            .map(|a| DynSolValue::Uint(a.into(), 256))
                            .collect();

                        contract.send_transaction(
                            "batchMintERC20",
                            &[
                                DynSolValue::Array(addresses_dyn),
                                DynSolValue::Array(amounts_dyn),
                            ],
                        ).await
                    }
                    "Erc721" => {
                        let addresses: Vec<alloy_primitives::Address> = (0..batch_size)
                            .map(|_| alloy_primitives::Address::random())
                            .collect();

                        let addresses_dyn: Vec<DynSolValue> = addresses
                            .into_iter()
                            .map(|a| DynSolValue::Address(a))
                            .collect();

                        contract.send_transaction(
                            "batchMintERC721",
                            &[DynSolValue::Array(addresses_dyn)],
                        ).await
                    }
                    "Erc1155" => {
                        let addresses: Vec<alloy_primitives::Address> = (0..batch_size)
                            .map(|_| alloy_primitives::Address::random())
                            .collect();
                        let amounts: Vec<alloy_primitives::U256> = (0..batch_size)
                            .map(|_| alloy_primitives::U256::from(100))
                            .collect();

                        let addresses_dyn: Vec<DynSolValue> = addresses
                            .into_iter()
                            .map(|a| DynSolValue::Address(a))
                            .collect();
                        let amounts_dyn: Vec<DynSolValue> = amounts
                            .into_iter()
                            .map(|a| DynSolValue::Uint(a.into(), 256))
                            .collect();

                        contract.send_transaction(
                            "batchMintERC1155",
                            &[
                                DynSolValue::Array(addresses_dyn),
                                DynSolValue::Uint(alloy_primitives::U256::from(1).into(), 256),
                                DynSolValue::Array(amounts_dyn),
                                DynSolValue::Bytes(vec![]),
                            ],
                        ).await
                    }
                    _ => {
                        return;
                    }
                };

                match result {
                    Ok(_tx_hash) => {
                        let latency = tx_start.elapsed();
                        let gas_estimate = 50000 + (batch_size as u64 * 50000);
                        let mut stats = stats.write().await;
                        stats.record_success(latency, gas_estimate, timestamp);
                        progress.set_message(format!("TPS: {:.2}", stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Batch {} failed: {}", i, e);
                    }
                }

                progress.inc(1);
            });

            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        info!("Batch mint test completed");
        Ok(())
    }
}
