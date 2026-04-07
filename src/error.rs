use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("RPC error (code {code}): {message}")]
    Rpc { code: i64, message: String },

    /// The node returned a response with neither result nor error.
    /// This violates the JSON-RPC spec but happens with some nodes.
    #[error("RPC call '{0}' returned no result and no error")]
    RpcMissingResult(String),

    /// HTTP transport failure — connection refused, timeout, DNS failure.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Reorg detected at block {block_number}: expected parent {expected}, got {actual}")]
    ReorgDetected {
        block_number: u64,
        expected: String,
        actual: String,
    },

    #[error("ABI parse error: {0}")]
    AbiParse(String)
}

pub type Result<T> = std::result::Result<T, IndexerError>;