//! Joel HTTP API: router assembly and HTTP adapters.

pub mod auth_routes;
pub mod knowledge_routes;
pub mod state;
pub mod webauthn_routes;

pub use state::Config;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};

use crate::state::AppState;

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

/// Builds the application router from a pool and explicit configuration.
///
/// # Errors
/// Propagates [`domain::auth::model::AuthError`] when crypto material is invalid.
pub fn build_router_with(
    pool: PgPool,
    config: &Config,
) -> Result<Router, domain::auth::model::AuthError> {
    let state = AppState::build(pool, config)?;
    Ok(Router::new()
        .route("/api/healthz", get(healthz))
        .route(
            "/api/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        .merge(auth_routes::router())
        .merge(webauthn_routes::router())
        .with_state(state))
}

/// Builds the full application router, including the semantic recall route.
///
/// Calls [`build_router_with`] for the core routes and, when the environment
/// variable `JOEL_EIDOS_ENDPOINT` is set, connects to the `EidosDB` gRPC
/// server, loads the `CandleEmbedder`, and merges the knowledge sub-router.
///
/// When `JOEL_EIDOS_ENDPOINT` is absent the recall route is simply not
/// mounted; the rest of the API works as normal.
///
/// # Errors
///
/// Propagates [`domain::auth::model::AuthError`] from [`build_router_with`],
/// and any error produced by the embedder load or the gRPC client connect.
pub async fn build_app(
    pool: PgPool,
    config: &Config,
) -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    let mut app = build_router_with(pool, config)?;

    if let Ok(endpoint) = std::env::var("JOEL_EIDOS_ENDPOINT") {
        let embedder = knowledge::CandleEmbedder::load()?;
        let client = eidosdb_client::EidosClient::connect(endpoint)
            .await
            .map_err(|e| format!("{e}"))?;
        let store = knowledge::EidosKnowledgeStore::new(client, Arc::new(embedder));
        store.ensure_collections().await?;
        let store: Arc<dyn domain::knowledge::KnowledgeStore> = Arc::new(store);
        app = app.merge(knowledge_routes::router(store));
    }

    Ok(app)
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
async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let db_up = persistence::ping(&state.pool).await.is_ok();
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
