use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{IndexerError, Result};

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
        let id = self
            .id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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
}