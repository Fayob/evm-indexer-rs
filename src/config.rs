use crate::error::{IndexerError, Result};

#[derive(Debug)]
pub struct Config {
    pub rpc_url: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            rpc_url: require("RPC_URL")?,
            database_url: require("DATABASE_URL")?,
        })
    }
}

fn require(key: &str) -> Result<String> {
    std::env::var(key)
        .map_err(|_| IndexerError::Config(format!("{key} is not set")))
}
