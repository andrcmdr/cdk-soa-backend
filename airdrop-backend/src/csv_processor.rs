use anyhow::Result;
use csv::ReaderBuilder;
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct EligibilityRow {
    pub address: String,
    pub amount: String,
}

pub struct CsvProcessor;

impl CsvProcessor {
    pub fn process_csv<P: AsRef<Path>>(path: P) -> Result<HashMap<Address, U256>> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)?;

        let mut eligibility_data = HashMap::new();

        for result in reader.deserialize() {
            let record: EligibilityRow = result?;

            let address: Address = record.address.parse()?;
            let amount: U256 = record.amount.parse()?;

            eligibility_data.insert(address, amount);
        }

        Ok(eligibility_data)
    }

    pub fn validate_csv_data(data: &HashMap<Address, U256>) -> Result<()> {
        if data.is_empty() {
            return Err(anyhow::anyhow!("CSV data is empty"));
        }

        for (address, amount) in data {
            if address == &Address::ZERO {
                return Err(anyhow::anyhow!("Invalid zero address found"));
            }

            if amount == &U256::ZERO {
                return Err(anyhow::anyhow!("Invalid zero amount found for address: {}", address));
            }
        }

        Ok(())
    }
}
