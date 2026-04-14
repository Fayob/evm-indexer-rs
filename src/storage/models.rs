use serde::Serialize;

/// A contract registered for indexing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Contract {
    pub address: String,
    pub name: String,
    /// Raw ABI JSON — parsed by the decoder layer, not here.
    pub abi: serde_json::Value,
}


#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DecodedEventRow {
    pub contract_address: String,
    pub contract_name: String,
    pub event_name: String,
    pub block_number: i64,
    pub transaction_hash: String,
    pub log_index: i64,
    pub parameters: serde_json::Value,
}