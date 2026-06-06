//! Joel HTTP API: router assembly and HTTP adapters.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;

/// Health status payload returned by the health endpoint.
#[derive(Serialize)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Database connectivity indicator.
    pub db: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

/// Builds the application router with all HTTP routes.
pub fn build_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/healthz", get(healthz))
        .with_state(pool)
}

async fn healthz(State(pool): State<PgPool>) -> Json<Health> {
    let db = if persistence::ping(&pool).await.is_ok() {
        "up"
    } else {
        "down"
    };
    Json(Health {
        status: "ok",
        db,
        version: env!("CARGO_PKG_VERSION"),
    })
}
