//! Statistics collection and reporting

use std::time::Duration;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStatistics {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub total_gas_used: u64,
    pub errors: Vec<String>,

    #[serde(skip)]
    latency_histogram: Histogram<u64>,

    latencies_ms: Vec<f64>,
    start_times: Vec<f64>,
}

impl TestStatistics {
    pub fn new() -> Self {
        Self {
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            total_gas_used: 0,
            errors: Vec::new(),
            latency_histogram: Histogram::<u64>::new(3).unwrap(),
            latencies_ms: Vec::new(),
            start_times: Vec::new(),
        }
    }

    pub fn record_success(&mut self, latency: Duration, gas_used: u64, timestamp: f64) {
        self.total_transactions += 1;
        self.successful_transactions += 1;
        self.total_gas_used += gas_used;

        let latency_ms = latency.as_millis() as u64;
        let _ = self.latency_histogram.record(latency_ms);

        self.latencies_ms.push(latency.as_secs_f64() * 1000.0);
        self.start_times.push(timestamp);
    }

    pub fn record_failure(&mut self, error: String, timestamp: f64) {
        self.total_transactions += 1;
        self.failed_transactions += 1;

        if self.errors.len() < 100 {
            self.errors.push(error);
        }

        self.start_times.push(timestamp);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            return 0.0;
        }
        self.successful_transactions as f64 / self.total_transactions as f64
    }

    pub fn tps(&self, duration: Duration) -> f64 {
        if duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.total_transactions as f64 / duration.as_secs_f64()
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        self.latencies_ms.iter().sum::<f64>() / self.latencies_ms.len() as f64
    }

    pub fn p50_latency_ms(&self) -> f64 {
        self.latency_histogram.value_at_percentile(50.0) as f64
    }

    pub fn p95_latency_ms(&self) -> f64 {
        self.latency_histogram.value_at_percentile(95.0) as f64
    }

    pub fn p99_latency_ms(&self) -> f64 {
        self.latency_histogram.value_at_percentile(99.0) as f64
    }

    pub fn max_latency_ms(&self) -> f64 {
        self.latency_histogram.max() as f64
    }

    pub fn avg_gas_per_tx(&self) -> f64 {
        if self.successful_transactions == 0 {
            return 0.0;
        }
        self.total_gas_used as f64 / self.successful_transactions as f64
    }

    pub fn merge(&mut self, other: &TestStatistics) {
        self.total_transactions += other.total_transactions;
        self.successful_transactions += other.successful_transactions;
        self.failed_transactions += other.failed_transactions;
        self.total_gas_used += other.total_gas_used;
        self.errors.extend(other.errors.clone());
        self.latencies_ms.extend(other.latencies_ms.clone());
        self.start_times.extend(other.start_times.clone());

        for &latency in &other.latencies_ms {
            let _ = self.latency_histogram.record(latency as u64);
        }
    }

    pub fn export_csv(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        writeln!(file, "timestamp,latency_ms,success")?;

        for (i, (&time, &latency)) in self.start_times.iter().zip(&self.latencies_ms).enumerate() {
            let success = i < self.successful_transactions as usize;
            writeln!(file, "{},{},{}", time, latency, success)?;
        }

        Ok(())
    }
}

impl Default for TestStatistics {
    fn default() -> Self {
        Self::new()
    }
}
