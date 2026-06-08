//! Joel HTTP API: router assembly and HTTP adapters.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};

/// Health status payload returned by the health endpoint.
#[derive(Serialize, ToSchema)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Database connectivity indicator.
    pub db: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

#[derive(OpenApi)]
#[openapi(info(title = "Joel API"), paths(healthz), components(schemas(Health)))]
struct ApiDoc;

/// Builds the application router with all HTTP routes.
#[must_use = "the router must be passed to an Axum server"]
pub fn build_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/healthz", get(healthz))
        .route(
            "/api/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        .with_state(pool)
}

/// Reports liveness of the api process and its database connection.
#[utoipa::path(
    get,
    path = "/api/healthz",
    responses(
        (status = 200, body = Health, description = "Service healthy"),
        (status = 503, body = Health, description = "Service degraded")
    )
)]
async fn healthz(State(pool): State<PgPool>) -> impl IntoResponse {
    let db_up = persistence::ping(&pool).await.is_ok();
    let status_code = if db_up {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let payload = Health {
        status: if db_up { "ok" } else { "degraded" },
        db: if db_up { "up" } else { "down" },
        version: env!("CARGO_PKG_VERSION"),
    };
    (status_code, Json(payload))
}
