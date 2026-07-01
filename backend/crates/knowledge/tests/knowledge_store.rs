//! Integration tests for the `EidosKnowledgeStore` adapter.
//!
//! Each test spins up a real in-process `EidosDB` server via `eidos-embedded`
//! and exercises the `KnowledgeStore` port through a `StubEmbedder` that
//! produces deterministic, content-sensitive 384-dim vectors without
//! downloading any model.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use domain::knowledge::{
    Corpus, EmbedError, Embedder, KnowledgeStore, MemoryId, RecallFilter, Remembrance,
};
use eidos_embedded::EmbeddedEidos;
use eidosdb_client::EidosClient;
use knowledge::EidosKnowledgeStore;
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// StubEmbedder: deterministic 384-dim embedder for tests; no model download.
// ---------------------------------------------------------------------------

/// Deterministic 384-dim embedder for tests: no model download.
struct StubEmbedder;

#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize {
        384
    }

    async fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(deterministic_vector(text))
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(deterministic_vector(text))
    }
}

fn deterministic_vector(text: &str) -> Vec<f32> {
    let mut values = vec![0.0_f32; 384];
    for (index, byte) in text.bytes().enumerate() {
        values[index % 384] += f32::from(byte);
    }
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        values[0] = 1.0;
        return values;
    }
    values.iter().map(|v| v / norm).collect()
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

async fn store() -> (EidosKnowledgeStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (endpoint, handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr)
        .await
        .expect("start");
    std::mem::forget(handle); // keep the server alive for the test duration
    let client = EidosClient::connect(endpoint).await.expect("connect");
    let store = EidosKnowledgeStore::new(client, Arc::new(StubEmbedder));
    store.ensure_collections().await.expect("ensure");
    (store, dir)
}

fn item(text: &str, theme: &str) -> Remembrance {
    Remembrance {
        id: MemoryId::new(),
        text: text.into(),
        payload: serde_json::json!({ "theme": theme, "title": text }),
        occurred_at: OffsetDateTime::from_unix_timestamp(1_000).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn remember_then_recall_returns_the_memory() {
    let (store, _dir) = store().await;
    store
        .remember(Corpus::Press, item("nuclear policy", "energy"))
        .await
        .expect("remember");
    let hits = store
        .recall(Corpus::Press, "nuclear policy", 5, &RecallFilter::default())
        .await
        .expect("recall");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].payload["theme"], "energy");
}

#[tokio::test]
async fn remember_is_idempotent_on_memory_id() {
    let (store, _dir) = store().await;
    let mut first = item("same text", "news");
    let id = first.id;
    store
        .remember(Corpus::Press, first.clone())
        .await
        .expect("first");
    first.id = id;
    store.remember(Corpus::Press, first).await.expect("second");
    let hits = store
        .recall(Corpus::Press, "same text", 10, &RecallFilter::default())
        .await
        .expect("recall");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id, "RecallHit.id must match the original MemoryId");
}

#[tokio::test]
async fn forget_removes_the_memory() {
    let (store, _dir) = store().await;
    let it = item("ephemeral", "news");
    let id = it.id;
    store.remember(Corpus::Press, it).await.expect("remember");
    assert!(store.forget(Corpus::Press, id).await.expect("forget"));
    let hits = store
        .recall(Corpus::Press, "ephemeral", 5, &RecallFilter::default())
        .await
        .expect("recall");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn recall_respects_the_theme_filter() {
    let (store, _dir) = store().await;
    store
        .remember(Corpus::Press, item("a", "energy"))
        .await
        .expect("a");
    store
        .remember(Corpus::Press, item("b", "sport"))
        .await
        .expect("b");
    let filter = RecallFilter {
        theme: Some("sport".into()),
        ..RecallFilter::default()
    };
    let hits = store
        .recall(Corpus::Press, "a", 10, &filter)
        .await
        .expect("recall");
    assert!(hits.iter().all(|h| h.payload["theme"] == "sport"));
}
