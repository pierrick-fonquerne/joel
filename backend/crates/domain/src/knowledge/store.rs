//! The `KnowledgeStore` port: text-in, semantic recall out.

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::knowledge::{Corpus, EmbedError, MemoryId, Score};

/// A fragment to memorize. Its text is embedded by the store.
#[derive(Debug, Clone)]
pub struct Remembrance {
    /// Caller-provided stable id (used for idempotent upsert and forget).
    pub id: MemoryId,
    /// The text to embed and index.
    pub text: String,
    /// Opaque business metadata (title, url, theme, source...).
    pub payload: serde_json::Value,
    /// When the underlying event occurred.
    pub occurred_at: OffsetDateTime,
}

/// One recall result, ordered by descending score.
#[derive(Debug, Clone)]
pub struct RecallHit {
    /// The matched memory id.
    pub id: MemoryId,
    /// Relevance score.
    pub score: Score,
    /// The stored business metadata.
    pub payload: serde_json::Value,
}

/// Optional recall constraints over reserved metadata.
#[derive(Debug, Clone, Default)]
pub struct RecallFilter {
    /// Restrict to a theme.
    pub theme: Option<String>,
    /// Lower time bound (inclusive).
    pub since: Option<OffsetDateTime>,
    /// Upper time bound (inclusive).
    pub until: Option<OffsetDateTime>,
}

/// Failure modes of the knowledge store.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    /// Embedding failed.
    #[error("embedding failed: {0}")]
    Embed(#[from] EmbedError),
    /// The storage backend rejected the request.
    #[error("backend error: {0}")]
    Backend(String),
    /// The requested entity was absent.
    #[error("not found")]
    NotFound,
    /// Invalid configuration (e.g. dimension mismatch).
    #[error("configuration error: {0}")]
    Config(String),
}

/// Text-in, semantic-recall-out port.
///
/// Implementations are responsible for embedding text and persisting
/// vectors together with business metadata in an `EidosDB` collection.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Embeds `remembrance.text` and upserts the vector + payload.
    async fn remember(
        &self,
        corpus: Corpus,
        remembrance: Remembrance,
    ) -> Result<(), KnowledgeError>;

    /// Embeds `query` and returns the top-`k` nearest memories.
    async fn recall(
        &self,
        corpus: Corpus,
        query: &str,
        k: usize,
        filter: &RecallFilter,
    ) -> Result<Vec<RecallHit>, KnowledgeError>;

    /// Forgets the memory with id `id`, returning whether it existed.
    async fn forget(&self, corpus: Corpus, id: MemoryId) -> Result<bool, KnowledgeError>;
}
