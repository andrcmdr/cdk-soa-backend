//! Integration tests for batch transaction functionality

#[cfg(test)]
mod tests {
    use tx_producer::prelude::*;

    #[test]
    fn test_batch_transaction_creation() {
        let batch_tx = BatchTransaction {
            id: "test1".to_string(),
            contract_address: None,
            function_name: "setValue".to_string(),
            args: vec![serde_json::json!("100")],
            gas_limit: Some(100000),
            gas_price: None,
            value: None,
        };

        assert_eq!(batch_tx.id, "test1");
        assert_eq!(batch_tx.function_name, "setValue");
        assert_eq!(batch_tx.gas_limit, Some(100000));
    }

    #[test]
    fn test_batch_result_success_rate() {
        let result = BatchResult {
            total: 10,
            successful: 8,
            failed: 2,
            results: vec![],
            total_gas_used: 1000000,
        };

        assert_eq!(result.total, 10);
        assert_eq!(result.successful, 8);
        assert_eq!(result.failed, 2);
        assert!(!result.all_succeeded());
    }

    #[test]
    fn test_batch_execution_strategy() {
        let strategy = BatchExecutionStrategy::ParallelRateLimited { max_concurrent: 5 };

        match strategy {
            BatchExecutionStrategy::ParallelRateLimited { max_concurrent } => {
                assert_eq!(max_concurrent, 5);
            }
            _ => panic!("Wrong strategy type"),
        }
    }

    #[test]
    fn test_batch_result_filtering() {
        use alloy_primitives::B256;

        let hash1 = B256::default();
        let hash2 = B256::from([1u8; 32]);

        let result = BatchResult {
            total: 3,
            successful: 2,
            failed: 1,
            results: vec![
                BatchTransactionResult {
                    id: "tx1".to_string(),
                    success: true,
                    tx_hash: Some(hash1),
                    error: None,
                    gas_used: Some(21000),
                },
                BatchTransactionResult {
                    id: "tx2".to_string(),
                    success: false,
                    tx_hash: None,
                    error: Some("Out of gas".to_string()),
                    gas_used: None,
                },
                BatchTransactionResult {
                    id: "tx3".to_string(),
                    success: true,
                    tx_hash: Some(hash2),
                    error: None,
                    gas_used: Some(22000),
                },
            ],
            total_gas_used: 43000,
        };

        let successful_hashes = result.successful_hashes();
        assert_eq!(successful_hashes.len(), 2);

        let failed_ids = result.failed_ids();
        assert_eq!(failed_ids, vec!["tx2"]);
    }
}
