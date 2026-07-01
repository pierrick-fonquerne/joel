//! Knowledge ports: text embedding contract consumed by the knowledge store.

mod corpus;
mod memory_id;
mod score;
mod store;

pub use corpus::Corpus;
pub use memory_id::MemoryId;
pub use score::Score;
pub use store::{KnowledgeError, KnowledgeStore, RecallFilter, RecallHit, Remembrance};

use async_trait::async_trait;

/// Errors raised while turning text into a vector.
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    /// The embedding model could not be loaded.
    #[error("model load failed: {0}")]
    ModelLoad(String),
    /// The forward pass failed.
    #[error("inference failed: {0}")]
    Inference(String),
    /// Tokenization of the input text failed.
    #[error("tokenize failed: {0}")]
    Tokenize(String),
}

/// Turns text into a fixed-dimension, L2-normalized embedding.
///
/// Implementations encapsulate any model-specific prompt prefixing
/// (for instance the `query:` / `passage:` prefixes of the e5 family),
/// so callers only ever pass raw text.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Dimension of every vector this embedder produces.
    fn dimension(&self) -> usize;
    /// Embeds a document/passage to be stored.
    async fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
    /// Embeds a query used to search.
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    struct ConstEmbedder;

    #[async_trait]
    impl Embedder for ConstEmbedder {
        fn dimension(&self) -> usize {
            3
        }
        async fn embed_passage(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
            Ok(vec![1.0, 0.0, 0.0])
        }
        async fn embed_query(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
            Ok(vec![0.0, 1.0, 0.0])
        }
    }

    #[tokio::test]
    async fn const_embedder_satisfies_port() {
        let embedder = ConstEmbedder;
        assert_eq!(embedder.dimension(), 3);
        assert_eq!(
            embedder.embed_passage("hello").await.unwrap(),
            vec![1.0, 0.0, 0.0]
        );
        assert_eq!(
            embedder.embed_query("hello").await.unwrap(),
            vec![0.0, 1.0, 0.0]
        );
    }
}
