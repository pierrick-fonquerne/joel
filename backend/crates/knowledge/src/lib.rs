//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small.

mod candle_embedder;
mod text;

pub use candle_embedder::CandleEmbedder;
pub use text::{E5Prefix, normalize_l2};
