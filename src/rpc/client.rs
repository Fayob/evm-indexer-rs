use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    error::{IndexerError, Result},
    rpc::types::{Log, LogFilter},
};

#[derive(Debug, Serialize)]
struct RpcRequest<'a, P: Serialize> {
    jsonrpc: &'a str,
    method: &'a str,
    params: P,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug)]
pub struct RpcClient {
    http: reqwest::Client,
    url: String,
    /// Monotonically increasing request id.
    /// Not wrapped in a Mutex because we don't need strict
    /// uniqueness — just reasonable differentiation for debugging.
    id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create a new RPC client pointing at the given URL.
    pub fn new(url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            url,
            id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    pub async fn call<P: Serialize>(&self, method: &str, params: P) -> Result<Value> {
        let id = self.id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let request = RpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id,
        };

        let response = self
            .http
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json::<RpcResponse>()
            .await?;

        if let Some(err) = response.error {
            return Err(IndexerError::Rpc {
                code: err.code,
                message: err.message,
            });
        }

        response
            .result
            .ok_or_else(|| IndexerError::RpcMissingResult(method.to_string()))
    }

    /// Fetch the current block number from the node.
    pub async fn get_block_number(&self) -> Result<u64> {
        let value = self.call("eth_blockNumber", serde_json::json!([])).await?;
        let hex = value
            .as_str()
            .ok_or_else(|| IndexerError::RpcMissingResult("eth_blockNumber".into()))?;
        let stripped = hex.strip_prefix("0x").unwrap_or(hex);
        u64::from_str_radix(stripped, 16)
            .map_err(|e| IndexerError::RpcMissingResult(format!("block number parse: {e}")))
    }

    /// Fetch a full block by number, including full transaction objects.
    ///
    /// Returns None if the block doesn't exist yet — this happens
    /// when we request a block ahead of the chain tip.
    pub async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<Option<crate::rpc::types::Block>> {
        let hex = format!("0x{block_number:x}");
        // Second param `true` means return full tx objects, not just hashes.
        let value = self
            .call("eth_getBlockByNumber", serde_json::json!([hex, true]))
            .await?;

        if value.is_null() {
            return Ok(None);
        }

        let block = serde_json::from_value(value)
            .map_err(|e| IndexerError::RpcMissingResult(format!("block deserialize: {e}")))?;

        Ok(Some(block))
    }

    /// Fetch all logs matching the given filter.
    ///
    /// Most RPC providers
    /// cap the response at 10,000 logs per call — if we hit that
    /// limit we need to reduce the block range. We handle that
    /// in the fetcher layer.
    pub async fn get_logs(&self, filter: &LogFilter) -> Result<Vec<Log>> {
        let value = self.call("eth_getLogs", json!([filter])).await?;

        let logs = serde_json::from_value(value)
            .map_err(|e| IndexerError::RpcMissingResult(format!("logs deserialize: {e}")))?;

        Ok(logs)
    }
}
