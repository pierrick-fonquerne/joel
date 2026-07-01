//! gRPC adapter implementing the `KnowledgeStore` port over `EidosDB`.

use std::sync::Arc;

use async_trait::async_trait;
use domain::knowledge::{
    Corpus, Embedder, KnowledgeError, KnowledgeStore, MemoryId, RecallFilter, RecallHit,
    Remembrance, Score,
};
use eidosdb_client::{CollectionSpec, EidosClient};
use eidosdb_core::{Dimension, Embedding, Metric, VectorId};
use eidosdb_hnsw::HnswConfig;
use eidosdb_lexical::Document;
use eidosdb_proto::convert::IndexTypeChoice;
use eidosdb_query::{FieldValue, Filter, HybridQuery, SearchHit, SearchQuery, Value};
use uuid::Uuid;

use crate::mapping::{KEY_MEMORY_ID, build_payload, payload_to_json, recall_filter_to_filter};

/// RRF dampening constant for hybrid search.
const RRF_K: f64 = 60.0;
/// Per-channel over-fetch multiplier for hybrid search.
const OVERFETCH_FACTOR: usize = 3;

/// `KnowledgeStore` backed by an `EidosDB` gRPC server.
pub struct EidosKnowledgeStore {
    client: EidosClient,
    embedder: Arc<dyn Embedder>,
}

impl EidosKnowledgeStore {
    /// Builds the adapter from a connected client and an embedder.
    #[must_use]
    pub fn new(client: EidosClient, embedder: Arc<dyn Embedder>) -> Self {
        Self { client, embedder }
    }

    /// Idempotently creates the `press` and `memory` collections.
    ///
    /// Calls `describe_collection` first; creates the collection only if it
    /// is absent (any error on `describe_collection` is treated as absence).
    ///
    /// # Errors
    ///
    /// Returns [`KnowledgeError::Backend`] if `create_collection` fails.
    pub async fn ensure_collections(&self) -> Result<(), KnowledgeError> {
        for corpus in [Corpus::Press, Corpus::AgentMemory] {
            let mut client = self.client.clone();
            let name = corpus.collection();
            if client.describe_collection(name).await.is_ok() {
                continue;
            }
            client
                .create_collection(CollectionSpec {
                    name: name.to_string(),
                    metric: Metric::Cosine,
                    dimension: Dimension(self.embedder.dimension()),
                    index_type: IndexTypeChoice::Hnsw,
                    hnsw: Some(HnswConfig::default()),
                })
                .await
                .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    /// Extracts the `theme` string from the business payload, if present.
    #[must_use]
    fn theme_of(payload: &serde_json::Value) -> Option<&str> {
        payload.get("theme").and_then(serde_json::Value::as_str)
    }

    /// Converts an `EidosDB` [`SearchHit`] into a domain [`RecallHit`].
    ///
    /// The `memory_id` is recovered from the reserved payload key; if it is
    /// absent or cannot be parsed, a fresh [`MemoryId`] is generated.
    #[must_use]
    fn to_hit(hit: &SearchHit) -> RecallHit {
        let payload = hit
            .payload
            .as_ref()
            .map_or(serde_json::Value::Null, payload_to_json);
        let id = hit
            .payload
            .as_ref()
            .and_then(|p| p.get(KEY_MEMORY_ID))
            .and_then(|f| match f {
                FieldValue::Scalar(Value::Text(text)) => Uuid::parse_str(text).ok(),
                _ => None,
            })
            .map_or_else(MemoryId::new, MemoryId::from_uuid);
        RecallHit {
            id,
            score: Score::new(hit.score.0),
            payload,
        }
    }
}

#[async_trait]
impl KnowledgeStore for EidosKnowledgeStore {
    async fn remember(&self, corpus: Corpus, item: Remembrance) -> Result<(), KnowledgeError> {
        let vector = self.embedder.embed_passage(&item.text).await?;
        let embedding =
            Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let document =
            Document::new(item.text).map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        let theme = Self::theme_of(&item.payload).map(str::to_string);
        let payload = build_payload(item.id, item.occurred_at, theme.as_deref(), &item.payload)?;
        let mut client = self.client.clone();
        let name = corpus.collection();
        client
            .delete_by_filter(
                name,
                Filter::Eq(KEY_MEMORY_ID.into(), Value::Text(item.id.to_string())),
            )
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        client
            .upsert(
                name,
                VectorId::new(),
                embedding,
                Some(document),
                Some(payload),
            )
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn remember_batch(
        &self,
        corpus: Corpus,
        items: Vec<Remembrance>,
    ) -> Result<(), KnowledgeError> {
        for item in items {
            self.remember(corpus, item).await?;
        }
        Ok(())
    }

    async fn recall(
        &self,
        corpus: Corpus,
        query: &str,
        k: usize,
        filter: &RecallFilter,
    ) -> Result<Vec<RecallHit>, KnowledgeError> {
        let vector = self.embedder.embed_query(query).await?;
        let embedding =
            Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let mut client = self.client.clone();
        let hits = client
            .search(
                corpus.collection(),
                SearchQuery {
                    embedding,
                    k,
                    metric: None,
                    filter: recall_filter_to_filter(filter),
                },
            )
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(hits.iter().map(Self::to_hit).collect())
    }

    async fn recall_hybrid(
        &self,
        corpus: Corpus,
        query: &str,
        k: usize,
        filter: &RecallFilter,
    ) -> Result<Vec<RecallHit>, KnowledgeError> {
        let vector = self.embedder.embed_query(query).await?;
        let embedding =
            Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let mut client = self.client.clone();
        let hits = client
            .search_hybrid(
                corpus.collection(),
                HybridQuery {
                    vector: Some(embedding),
                    text: Some(query.to_string()),
                    k,
                    filter: recall_filter_to_filter(filter),
                    metric: None,
                    rrf_k: RRF_K,
                    overfetch_factor: OVERFETCH_FACTOR,
                },
            )
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(hits.iter().map(Self::to_hit).collect())
    }

    async fn forget(&self, corpus: Corpus, id: MemoryId) -> Result<bool, KnowledgeError> {
        let mut client = self.client.clone();
        let deleted = client
            .delete_by_filter(
                corpus.collection(),
                Filter::Eq(KEY_MEMORY_ID.into(), Value::Text(id.to_string())),
            )
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(deleted > 0)
    }
}
