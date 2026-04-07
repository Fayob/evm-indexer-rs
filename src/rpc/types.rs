use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a hex-encoded string like "0x1b4" into a u64.
///
/// The node encodes all integers as hex strings per the
/// JSON-RPC spec. We decode at the boundary so the rest
/// of the codebase never sees raw hex.
fn deserialize_hex_u64<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where 
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    u64::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)
}

/// Deserialize an optional hex-encoded u64.
/// Some fields like `to` on a contract creation tx are null.
fn deserialize_hex_u64_opt<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => {
            let stripped = s.strip_prefix("0x").unwrap_or(&s);
            u64::from_str_radix(stripped, 16)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// The block hash. Used for reorg detection.
    pub hash: String,

    /// The parent block's hash.
    /// If this doesn't match our stored hash of height-1,
    /// we have a reorg.
    pub parent_hash: String,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub number: u64,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub timestamp: u64,

    /// Full transaction objects, not just hashes.
    /// We request full objects via eth_getBlockByNumber
    /// with the second param set to true.
    pub transactions: Vec<Transaction>,
}

/// An Ethereum transaction as embedded in a block.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub hash: String,

    /// The sender address.
    pub from: String,

    /// The recipient address.
    /// None for contract creation transactions.
    pub to: Option<String>,

    /// Transfer value in wei, hex-encoded.
    /// Stored as String — wei values exceed u64 max.
    /// A proper implementation uses U256. We revisit
    /// this when we add the storage layer.
    pub value: String,

    /// Encoded contract call or deployment bytecode.
    pub input: String,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub transaction_index: u64,
}

/// A single event log emitted during transaction execution.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    /// The contract that emitted this log.
    pub address: String,

    /// topics[0] is always keccak256(EventSignature).
    /// topics[1..] are the indexed event parameters.
    pub topics: Vec<String>,

    /// ABI-encoded non-indexed event parameters.
    pub data: String,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub block_number: u64,

    pub block_hash: String,
    pub transaction_hash: String,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub transaction_index: u64,

    #[serde(deserialize_with = "deserialize_hex_u64")]
    pub log_index: u64,
}

/// Filter parameter for eth_getLogs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    pub from_block: String,
    pub to_block: String,
    /// List of contract addresses to filter by.
    /// Empty means all contracts — we always provide this.
    pub address: Vec<String>,
}
