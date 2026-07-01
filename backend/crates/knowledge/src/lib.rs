//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small.

mod candle_embedder;
mod mapping;
mod text;

pub use candle_embedder::CandleEmbedder;
pub use mapping::{build_payload, payload_to_json, recall_filter_to_filter};
pub use text::{E5Prefix, normalize_l2};
