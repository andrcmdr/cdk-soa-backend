//! Error types for the transaction producer library

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, TxProducerError>;

/// Main error type for the library
#[derive(Debug, Error)]
pub enum TxProducerError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Provider error
    #[error("Provider error: {0}")]
    Provider(String),

    /// ABI loading error
    #[error("ABI load error: {0}")]
    AbiLoad(String),

    /// Contract call error
    #[error("Contract call error: {0}")]
    ContractCall(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Decoding error
    #[error("Decoding error: {0}")]
    Decoding(String),

    /// Signature error
    #[error("Signature error: {0}")]
    Signature(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
