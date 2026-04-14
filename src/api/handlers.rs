use axum::{Json, extract::{Query, State}, response::IntoResponse};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{error::IndexerError, storage::{db, models::{Contract}}};

pub struct ApiError(IndexerError);

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

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self(IndexerError::from(err))
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

/// Request body for registering a new contract.
#[derive(Deserialize)]
pub struct RegisterContractRequest {
    pub address: String,
    pub name: String,
    pub abi: serde_json::Value,
}

/// Register a new contract for indexing.
pub async fn register_contract(
    State(pool): State<PgPool>,
    Json(body): Json<RegisterContractRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract = Contract {
        address: body.address.to_lowercase(),
        name: body.name,
        abi: body.abi,
    };

    db::save_contract(&pool, &contract).await?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "status": "registered" }))))
}

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub contract: Option<String>,
    pub event: Option<String>,
    pub limit: Option<i64>,
}

/// List decoded events with optional filtering.
pub async fn list_events(
    State(pool): State<PgPool>,
    Query(params): Query<EventsQuery>,
) -> ApiResult<impl IntoResponse> {
    let limit = params.limit.unwrap_or(50).min(500); // Max limit of 500

    let rows = db::get_decoded_events(
        &pool, 
        params.contract.as_deref(), 
        params.event.as_deref(), 
        limit
    )
    .await?;

    Ok(Json(rows))
}
