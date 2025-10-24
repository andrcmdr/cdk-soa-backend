//! Cryptography test scenario

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

pub struct CryptoScenario {
    test_type: String,
    count: usize,
}

impl CryptoScenario {
    pub fn new(test_type: String, count: usize) -> Self {
        Self { test_type, count }
    }

    fn generate_merkle_proof() -> Vec<alloy_primitives::B256> {
        // Generate a simple Merkle proof
        (0..5).map(|_| alloy_primitives::B256::random()).collect()
    }
}

#[async_trait]
impl TestScenario for CryptoScenario {
    fn name(&self) -> &str {
        "Cryptography Test"
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
        info!("Starting crypto test: {} type, {} verifications",
              self.test_type, self.count);

        let semaphore = Arc::new(Semaphore::new(workers));
        let start_time = Instant::now();

        let mut tasks = Vec::new();

        for i in 0..self.count {
            let contract = contract.clone();
            let stats = stats.clone();
            let progress = progress.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let test_type = self.test_type.clone();

            let task = tokio::spawn(async move {
                let _permit = permit;

                let tx_start = Instant::now();
                let timestamp = start_time.elapsed().as_secs_f64();

                let result = match test_type.as_str() {
                    "Merkle" => {
                        let leaf = alloy_primitives::B256::random();
                        let proof = Self::generate_merkle_proof();

                        let proof_dyn: Vec<DynSolValue> = proof
                            .into_iter()
                            .map(|p| DynSolValue::FixedBytes(p, 32))
                            .collect();

                        contract.call_function(
                            "verifyProof",
                            &[
                                DynSolValue::FixedBytes(leaf, 32),
                                DynSolValue::Array(proof_dyn),
                            ],
                        ).await.map(|_| alloy_primitives::B256::ZERO)
                    }
                    "Signature" => {
                        // For view functions, we use call_function instead of send_transaction
                        let expected = alloy_primitives::Address::random();
                        let hash = alloy_primitives::B256::random();
                        let sig = vec![0u8; 65]; // Dummy signature

                        contract.call_function(
                            "verifySig",
                            &[
                                DynSolValue::Address(expected),
                                DynSolValue::FixedBytes(hash, 32),
                                DynSolValue::Bytes(sig),
                            ],
                        ).await.map(|_| alloy_primitives::B256::ZERO)
                    }
                    _ => {
                        Err(tx_producer::TxProducerError::InvalidInput("Invalid test type".to_string()))
                    }
                };

                match result {
                    Ok(_) => {
                        let latency = tx_start.elapsed();
                        let mut stats = stats.write().await;
                        stats.record_success(latency, 50000, timestamp);
                        progress.set_message(format!("Ops/s: {:.2}", stats.tps(start_time.elapsed())));
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.record_failure(e.to_string(), timestamp);
                        warn!("Verification {} failed: {}", i, e);
                    }
                }

                progress.inc(1);
            });

            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        info!("Crypto test completed");
        Ok(())
    }
}
