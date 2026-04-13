use axum::{Router, routing::get};
use sqlx::PgPool;

use crate::{api::handlers, error::Result};

pub async fn run(pool: PgPool, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/contracts", get(handlers::list_contracts));

    Ok(())
}