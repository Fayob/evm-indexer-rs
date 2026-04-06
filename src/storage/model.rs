/// A contract registered for indexing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Contract {
    pub address: String,
    pub name: String,
    /// Raw ABI JSON — parsed by the decoder layer, not here.
    pub abi: serde_json::Value,
}
