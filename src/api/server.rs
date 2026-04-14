use std::net::SocketAddr;

use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

use crate::{api::handlers, error::Result};

pub async fn run(pool: PgPool, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/contracts", get(handlers::list_contracts))
        .route("/contracts", post(handlers::register_contract))
        .route("/events", get(handlers::list_events))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    
    Ok(())
}