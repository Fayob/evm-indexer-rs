use crate::decoder::abi::{AbiEvent, AbiParam, compute_selector, parse_abi_events};
use crate::error::{IndexerError, Result};
use crate::rpc::types::Log;
use crate::storage::models::Contract;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// A decoded event — the structured result of decoding a raw log.
#[derive(Debug, Clone)]
pub struct DecodedEvent {
    pub contract_address: String,
    pub contract_name: String,
    pub event_name: String,
    pub transaction_hash: String,
    pub block_number: u64,
    pub log_index: u64,
    /// The decoded parameters as a JSON object.
    /// Keys are parameter names, values are decoded values.
    pub parameters: Value,
}

/// A registry mapping event selectors to their ABI definitions.
///
/// Built once at startup from registered contracts, rebuilt
/// whenever the contract list is reloaded.
pub struct EventRegistry {
    /// selector → (contract_address, contract_name, event)
    entries: HashMap<String, (String, String, AbiEvent)>,
}

impl EventRegistry {
    /// Build an EventRegistry from a list of registered contracts.
    pub fn from_contracts(contracts: &[Contract]) -> Result<Self> {
        let mut entries = HashMap::new();

        for contract in contracts {
            let events = parse_abi_events(&contract.abi)?;

            for event in events {
                let selector = compute_selector(&event);
                entries.insert(
                    selector,
                    (contract.address.clone(), contract.name.clone(), event),
                );
            }
        }

        Ok(Self { entries })
    }

    /// Attempt to decode a raw log using the registry.
    ///
    /// Returns None if the log's selector is not in the registry —
    /// this is not an error, it just means we don't care about
    /// this event.
    pub fn decode_log(&self, log: &Log) -> Result<Option<DecodedEvent>> {
        // topics[0] is the event selector.
        let selector = match log.topics.first() {
            Some(s) => s,
            None => return Ok(None),
        };

        let (contract_address, contract_name, event) = match self.entries.get(selector) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // Only decode logs from the contract we registered.
        // A different contract could emit an event with the
        // same selector — we filter by address to be precise.
        if log.address.to_lowercase() != contract_address.to_lowercase() {
            return Ok(None);
        }

        let parameters = decode_params(log, event)?;

        Ok(Some(DecodedEvent {
            contract_address: contract_address.clone(),
            contract_name: contract_name.clone(),
            event_name: event.name.clone(),
            transaction_hash: log.transaction_hash.clone(),
            block_number: log.block_number,
            log_index: log.log_index,
            parameters,
        }))
    }
}

/// Decode the parameters of a log into a JSON object.
fn decode_params(log: &Log, event: &AbiEvent) -> Result<Value> {
    let mut map = Map::new();

    // Indexed params come from topics[1..].
    let indexed: Vec<&AbiParam> = event.inputs.iter().filter(|p| p.indexed).collect();

    // Non-indexed params come from data.
    let non_indexed: Vec<&AbiParam> = event.inputs.iter().filter(|p| !p.indexed).collect();

    // Decode indexed parameters from topics.
    for (i, param) in indexed.iter().enumerate() {
        // topics[0] is the selector, params start at topics[1].
        let topic = log.topics.get(i + 1).ok_or_else(|| {
            IndexerError::LogDecode(format!("missing topic {} for param {}", i + 1, param.name))
        })?;

        let value = decode_topic(topic, &param.r#type)?;
        map.insert(param.name.clone(), value);
    }

    // Decode non-indexed parameters from data.
    let data = strip_hex(&log.data);
    let data_bytes = hex::decode(&data)
        .map_err(|e| IndexerError::LogDecode(format!("invalid data hex: {e}")))?;

    let mut offset = 0;
    for param in non_indexed.iter() {
        let (value, consumed) = decode_data_param(&data_bytes, offset, &param.r#type)?;
        map.insert(param.name.clone(), value);
        offset += consumed;
    }

    Ok(Value::Object(map))
}

/// Decode a single 32-byte topic into a JSON value.
fn decode_topic(topic: &str, kind: &str) -> Result<Value> {
    let hex = strip_hex(topic);
    let bytes = hex::decode(&hex)
        .map_err(|e| IndexerError::LogDecode(format!("invalid topic hex: {e}")))?;

    match kind {
        "address" => {
            // Address is right-aligned in 32 bytes.
            // Take the last 20 bytes.
            let addr = hex::encode(&bytes[12..]);
            Ok(Value::String(format!("0x{addr}")))
        }
        k if k.starts_with("uint") || k.starts_with("int") => {
            // Integers are left-aligned in 32 bytes.
            // Parse as u128 — covers up to uint128.
            // uint256 would need a big integer library.
            // We note this as a known limitation.
            let value = u128::from_be_bytes(
                bytes[16..32]
                    .try_into()
                    .map_err(|_| IndexerError::LogDecode("uint conversion failed".into()))?,
            );
            Ok(Value::String(value.to_string()))
        }
        "bool" => {
            let value = bytes[31] != 0;
            Ok(Value::Bool(value))
        }
        "bytes32" => Ok(Value::String(format!("0x{}", hex::encode(&bytes)))),
        other => Err(IndexerError::LogDecode(format!(
            "unsupported indexed type: {other}"
        ))),
    }
}

/// Decode a single parameter from the ABI-encoded data blob.
/// Returns the decoded value and the number of bytes consumed.
///
/// This handles fixed-size types only for now.
/// Dynamic types (string, bytes, arrays) use offset pointers
/// in ABI encoding — we add those when a registered ABI needs them.
fn decode_data_param(data: &[u8], offset: usize, kind: &str) -> Result<(Value, usize)> {
    // Every ABI-encoded slot is 32 bytes.
    let slot = data.get(offset..offset + 32).ok_or_else(|| {
        IndexerError::LogDecode(format!("data too short at offset {offset} for type {kind}"))
    })?;

    match kind {
        "address" => {
            let addr = hex::encode(&slot[12..]);
            Ok((Value::String(format!("0x{addr}")), 32))
        }
        k if k.starts_with("uint") || k.starts_with("int") => {
            let value = u128::from_be_bytes(
                slot[16..32]
                    .try_into()
                    .map_err(|_| IndexerError::LogDecode("uint conversion failed".into()))?,
            );
            Ok((Value::String(value.to_string()), 32))
        }
        "bool" => Ok((Value::Bool(slot[31] != 0), 32)),
        "bytes32" => Ok((Value::String(format!("0x{}", hex::encode(slot))), 32)),
        other => Err(IndexerError::LogDecode(format!(
            "unsupported data type: {other}"
        ))),
    }
}

/// Strip the "0x" prefix from a hex string.
fn strip_hex(s: &str) -> String {
    s.strip_prefix("0x").unwrap_or(s).to_string()
}
