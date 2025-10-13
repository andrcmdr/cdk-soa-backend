use anyhow::Result;
use csv::{ReaderBuilder, WriterBuilder};
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize, Serialize)]
pub struct EligibilityRow {
    pub address: String,
    pub amount: String,
}

pub struct CsvProcessor;

impl CsvProcessor {
    pub fn process_csv_bytes(data: &[u8]) -> AppResult<HashMap<Address, U256>> {
        let cursor = Cursor::new(data);
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(cursor);

        let mut eligibility_data = HashMap::new();

        for result in reader.deserialize() {
            let record: EligibilityRow = result
                .map_err(|e| AppError::CsvProcessing(e))?;

            let address: Address = record.address.parse()
                .map_err(|e| AppError::InvalidInput(format!("Invalid address '{}': {}", record.address, e)))?;

            let amount: U256 = record.amount.parse()
                .map_err(|e| AppError::InvalidInput(format!("Invalid amount '{}': {}", record.amount, e)))?;

            eligibility_data.insert(address, amount);
        }

        Ok(eligibility_data)
    }

    pub fn generate_csv_bytes(eligibility_data: &HashMap<Address, U256>) -> AppResult<Vec<u8>> {
        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_writer(Vec::new());

        // Write header
        writer.write_record(&["address", "amount"])
            .map_err(|e| AppError::CsvProcessing(e))?;

        // Write data
        for (address, amount) in eligibility_data {
            let record = EligibilityRow {
                address: format!("0x{}", hex::encode(address)),
                amount: amount.to_string(),
            };
            writer.serialize(&record)
                .map_err(|e| AppError::CsvProcessing(e))?;
        }

        // Directly map the error to a CsvProcessing error without trying to use CsvError::IntoInner
        let csv_data = writer.into_inner()
            .map_err(|e| AppError::CsvProcessing(csv::Error::from(e.into_error())))?;

        Ok(csv_data)
    }

    pub fn validate_csv_data(data: &HashMap<Address, U256>) -> AppResult<()> {
        if data.is_empty() {
            return Err(AppError::InvalidInput("CSV data is empty".to_string()));
        }

        for (address, amount) in data {
            if address == &Address::ZERO {
                return Err(AppError::InvalidInput("Invalid zero address found".to_string()));
            }

            if amount == &U256::ZERO {
                return Err(AppError::InvalidInput(format!("Invalid zero amount found for address: {}", address)));
            }
        }

        Ok(())
    }
}
