//! Test scenario implementations

mod basic;
mod storage;
mod calldata;
mod batch_mint;
mod external_call;
mod crypto;
mod mixed;
mod stress;
mod endurance;

pub use basic::BasicScenario;
pub use storage::StorageScenario;
pub use calldata::CalldataScenario;
pub use batch_mint::BatchMintScenario;
pub use external_call::ExternalCallScenario;
pub use crypto::CryptoScenario;
pub use mixed::MixedScenario;
pub use stress::StressScenario;
pub use endurance::EnduranceScenario;

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::RwLock;

use tx_producer::prelude::*;
use crate::stats::TestStatistics;

#[async_trait]
pub trait TestScenario: Send + Sync {
    /// Get scenario name
    fn name(&self) -> &str;

    /// Get total number of operations
    fn total_operations(&self) -> usize;

    /// Execute the scenario
    async fn execute(
        &self,
        contract: Arc<ContractClient>,
        stats: Arc<RwLock<TestStatistics>>,
        progress: ProgressBar,
        workers: usize,
        rate_limit: u64,
    ) -> Result<()>;
}
