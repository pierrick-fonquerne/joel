//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small,
//! and a gRPC `KnowledgeStore` adapter over `EidosDB`.

mod candle_embedder;
mod eidos_knowledge_store;
mod mapping;
mod text;

pub use candle_embedder::CandleEmbedder;
pub use eidos_knowledge_store::EidosKnowledgeStore;
pub use mapping::{build_payload, payload_to_json, recall_filter_to_filter};
pub use text::{E5Prefix, normalize_l2};
