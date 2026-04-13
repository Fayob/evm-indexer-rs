use axum::{Json, extract::State, response::IntoResponse};
use reqwest::StatusCode;
use serde::Serialize;
use sqlx::PgPool;

use crate::{error::IndexerError, storage::db};

struct ApiError(IndexerError);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
        .into_response()
    }
}

impl From<IndexerError> for ApiError {
    fn from(err: IndexerError) -> Self {
        Self(err)
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

//  Contracts
#[derive(Serialize)]
pub struct ContractResponse {
    pub address: String,
    pub name: String,
}

/// List all registered contracts.
pub async fn list_contracts(State(pool): State<PgPool>) -> ApiResult<impl IntoResponse> {
    let contracts = db::load_contracts(&pool).await?;

    let response: Vec<ContractResponse> = contracts
        .into_iter()
        .map(|c| ContractResponse {
            address: c.address,
            name: c.name
        })
        .collect();

        Ok(Json(response))
}

