//! Joel HTTP API: router assembly and HTTP adapters.

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

/// Health status payload returned by the health endpoint.
#[derive(Serialize)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

/// Builds the application router with all HTTP routes.
pub fn build_router() -> Router {
    Router::new().route("/api/healthz", get(healthz))
}

async fn healthz() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
