//! HTTP routes for semantic recall over the knowledge store.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use domain::knowledge::{Corpus, KnowledgeStore, RecallFilter};
use serde::Deserialize;

#[derive(Deserialize)]
struct RecallParams {
    corpus: String,
    q: String,
    #[serde(default = "default_k")]
    k: usize,
}

fn default_k() -> usize {
    10
}

/// Builds the semantic recall sub-router, isolated from the main `AppState`.
///
/// The sub-router owns `store` as its sole state, so it can be merged into any
/// `axum::Router` without disturbing other state types.
///
/// Route: `GET /api/recall?corpus=<press|memory>&q=<text>&k=<n>`
///
/// Returns a JSON array of `{ id, score, payload }` ordered by descending
/// relevance.
pub fn router(store: Arc<dyn KnowledgeStore>) -> Router {
    Router::new()
        .route("/api/recall", get(recall))
        .with_state(store)
}

async fn recall(
    State(store): State<Arc<dyn KnowledgeStore>>,
    Query(params): Query<RecallParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let corpus = match params.corpus.as_str() {
        "press" => Corpus::Press,
        "memory" => Corpus::AgentMemory,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let hits = store
        .recall(corpus, &params.q, params.k, &RecallFilter::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let body = hits
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id.to_string(),
                "score": h.score.value(),
                "payload": h.payload,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::Value::Array(body)))
}
