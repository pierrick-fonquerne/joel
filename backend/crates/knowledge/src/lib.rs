//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small.

mod text;

pub use text::{E5Prefix, normalize_l2};
