use std::collections::HashMap;

use crate::error::{IndexerError, Result};

#[derive(Debug)]
pub struct Config {
    pub rpc_url: String,
    pub database_url: String,
    pub start_block: u64,
    pub confirmation_depth: u64,
    pub api_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            rpc_url: require("RPC_URL")?,
            database_url: require("DATABASE_URL")?,
            start_block: parse(&std::env::vars().collect(), "START_BLOCK", "0")?,
            confirmation_depth: 12,
            api_port: parse(&std::env::vars().collect(), "API_PORT", "3000")?,
        })
    }
}

fn require(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| IndexerError::Config(format!("{key} is not set")))
}

fn parse<T>(env: &HashMap<String, String>, key: &str, default: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = env.get(key).map(String::as_str).unwrap_or(default);
    raw.parse::<T>()
        .map_err(|e| IndexerError::Config(format!("{key} is invalid: {e}")))
}
