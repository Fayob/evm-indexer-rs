use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;