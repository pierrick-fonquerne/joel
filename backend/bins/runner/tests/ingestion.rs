//! Integration test: bootstrap the runner and verify one round-trip of remember/recall.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use domain::knowledge::{
    Corpus, EmbedError, Embedder, KnowledgeStore, MemoryId, RecallFilter, Remembrance,
};
use runner::bootstrap;
use time::OffsetDateTime;

struct StubEmbedder;

#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize {
        384
    }

    async fn embed_passage(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(unit())
    }

    async fn embed_query(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(unit())
    }
}

fn unit() -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[0] = 1.0;
    v
}

#[tokio::test]
async fn bootstrap_serves_and_ingests() {
    let dir = tempfile::tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (store, handle) = bootstrap(dir.path().to_path_buf(), addr, Arc::new(StubEmbedder))
        .await
        .expect("bootstrap");
    store
        .remember(
            Corpus::Press,
            Remembrance {
                id: MemoryId::new(),
                text: "hello".into(),
                payload: serde_json::json!({ "theme": "x" }),
                occurred_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
            },
        )
        .await
        .expect("remember");
    let hits = store
        .recall(Corpus::Press, "hello", 5, &RecallFilter::default())
        .await
        .expect("recall");
    assert_eq!(hits.len(), 1);
    handle.shutdown();
}
