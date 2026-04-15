use serde::Deserialize;
use tiny_keccak::{Hasher, Keccak};

use crate::error::{IndexerError, Result};

/// A single parameter in an ABI event definition.
#[derive(Debug, Clone, Deserialize)]
pub struct AbiParam {
    pub name: String,
    /// The Solidity type — "address", "uint256", "bytes32", etc.
    // #[serde(rename = "type")]
    pub r#type: String,
    /// Whether this parameter is indexed (lives in topics).
    #[serde(default)]
    pub indexed: bool,
}

/// A single event definition from a contract ABI.
#[derive(Debug, Clone, Deserialize)]
pub struct AbiEvent {
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<AbiParam>,
}

/// A parsed ABI — just the events we care about.
/// We ignore functions, constructors, and errors for now.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AbiEntry {
    Event(AbiEvent),
    #[serde(other)]
    Other,
}

/// Compute the keccak256 selector for an event.
///
/// The selector is keccak256 of the canonical event signature.
/// Canonical means: no spaces, no `indexed` keyword, just
/// "EventName(type1,type2,type3)".
///
/// Example: Transfer(address,address,uint256)
/// → 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef
pub fn compute_selector(event: &AbiEvent) -> String {
    let signature = event_signature(event);
    let hash = keccak256(signature.as_bytes());
    format!("0x{}", hex::encode(hash))
}

/// Build the canonical event signature string.
fn event_signature(event: &AbiEvent) -> String {
    let params = event
        .inputs
        .iter()
        .map(|p| p.r#type.clone())
        .collect::<Vec<_>>()
        .join(",");

    format!("{}({})", event.name, params)
}

/// Compute keccak256 of arbitrary bytes.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}

/// Parse a JSON ABI value into a list of ABI events.
pub fn parse_abi_events(abi: &serde_json::Value) -> Result<Vec<AbiEvent>> {
    let entries: Vec<AbiEntry> =
        serde_json::from_value(abi.clone()).map_err(|e| IndexerError::AbiParse(e.to_string()))?;

    let events = entries
        .into_iter()
        .filter_map(|entry| match entry {
            AbiEntry::Event(e) => Some(e),
            AbiEntry::Other => None,
        })
        .collect();

    Ok(events)
}
