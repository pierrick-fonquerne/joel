//! Integration test: the recall sub-router returns hits stored in an embedded `EidosDB`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use domain::knowledge::{Corpus, EmbedError, Embedder, KnowledgeStore, MemoryId, Remembrance};
use eidos_embedded::EmbeddedEidos;
use eidosdb_client::EidosClient;
use http_body_util::BodyExt;
use knowledge::EidosKnowledgeStore;
use time::OffsetDateTime;
use tower::ServiceExt;

/// Stub embedder that returns a unit vector in the first dimension.
struct StubEmbedder;

#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize {
        384
    }

    async fn embed_passage(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(unit_vec())
    }

    async fn embed_query(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(unit_vec())
    }
}

fn unit_vec() -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[0] = 1.0;
    v
}

#[tokio::test]
async fn recall_returns_seeded_hit() {
    let dir = tempfile::tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (endpoint, handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr)
        .await
        .expect("embedded start");

    let client = EidosClient::connect(endpoint)
        .await
        .expect("client connect");
    let store = EidosKnowledgeStore::new(client, Arc::new(StubEmbedder));
    store
        .ensure_collections()
        .await
        .expect("ensure collections");

    let store_arc: Arc<dyn KnowledgeStore> = Arc::new(store);

    store_arc
        .remember(
            Corpus::Press,
            Remembrance {
                id: MemoryId::new(),
                text: "hello world".to_owned(),
                payload: serde_json::json!({ "theme": "test" }),
                occurred_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
            },
        )
        .await
        .expect("remember");

    let app = api::knowledge_routes::router(Arc::clone(&store_arc));

    let response = app
        .oneshot(
            Request::get("/api/recall?corpus=press&q=hello&k=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(
        json.as_array().is_some_and(|arr| !arr.is_empty()),
        "expected at least one hit, got: {json}"
    );

    let hit = &json[0];
    assert!(hit["id"].is_string());
    assert!(hit["score"].is_number());
    assert_eq!(hit["payload"]["theme"], "test");

    drop(store_arc);
    handle.shutdown();
}

#[tokio::test]
async fn recall_bad_corpus_returns_400() {
    let dir = tempfile::tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (endpoint, handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr)
        .await
        .expect("embedded start");

    let client = EidosClient::connect(endpoint)
        .await
        .expect("client connect");
    let store = EidosKnowledgeStore::new(client, Arc::new(StubEmbedder));
    store
        .ensure_collections()
        .await
        .expect("ensure collections");

    let store_arc: Arc<dyn KnowledgeStore> = Arc::new(store);
    let app = api::knowledge_routes::router(store_arc);

    let response = app
        .oneshot(
            Request::get("/api/recall?corpus=unknown&q=hello&k=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    handle.shutdown();
}
