//! Candle-backed embedder using the `intfloat/multilingual-e5-small` model.

use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use domain::knowledge::{EmbedError, Embedder};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

use crate::text::{E5Prefix, normalize_l2};

/// Embedder backed by `intfloat/multilingual-e5-small` (384 dimensions, XLM-RoBERTa architecture).
///
/// Downloads and caches the model weights from the Hugging Face hub on first use.
pub struct CandleEmbedder {
    model: XLMRobertaModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl CandleEmbedder {
    /// Downloads and loads `intfloat/multilingual-e5-small` from the Hugging Face hub.
    ///
    /// On the first call the model files (~470 MB) are downloaded and cached locally.
    /// Subsequent calls reuse the local cache.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::ModelLoad`] if any model artifact (config, tokenizer,
    /// weights) cannot be downloaded or parsed.
    pub fn load() -> Result<Self, EmbedError> {
        let device = Device::Cpu;
        let api = Api::new().map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let repo = api.model("intfloat/multilingual-e5-small".to_string());

        let config_path = repo
            .get("config.json")
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let weights_path = repo
            .get("model.safetensors")
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;

        let config_bytes =
            std::fs::read(&config_path).map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let config: Config = serde_json::from_slice(&config_bytes)
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;

        // Use the buffered (non-mmap) loader to avoid `unsafe` code.
        // The weights file is ~117 MB for multilingual-e5-small.
        let weights_bytes =
            std::fs::read(&weights_path).map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let vb = VarBuilder::from_buffered_safetensors(weights_bytes, DType::F32, &device)
            .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;

        let model =
            XLMRobertaModel::new(&config, vb).map_err(|e| EmbedError::ModelLoad(e.to_string()))?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Runs the full embedding pipeline for `text` with `prefix`.
    ///
    /// Steps: prefix -> tokenize -> forward -> masked mean pool -> L2 normalize.
    fn embed(&self, prefix: E5Prefix, text: &str) -> Result<Vec<f32>, EmbedError> {
        let input = prefix.apply(text);

        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| EmbedError::Tokenize(e.to_string()))?;

        let ids: Vec<u32> = encoding.get_ids().to_vec();
        let mask: Vec<u32> = encoding.get_attention_mask().to_vec();
        let seq = ids.len();

        let input_ids = Tensor::from_vec(ids, (1, seq), &self.device)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        let attention_mask = Tensor::from_vec(mask, (1, seq), &self.device)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        let token_type_ids = input_ids
            .zeros_like()
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        // Shape: [1, seq, hidden_size]
        let hidden = self
            .model
            .forward(
                &input_ids,
                &attention_mask,
                &token_type_ids,
                None,
                None,
                None,
            )
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        let pooled = masked_mean_pool(&hidden, &attention_mask)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        Ok(normalize_l2(&pooled))
    }
}

/// Computes the attention-masked mean of `hidden_states` over the sequence dimension.
///
/// `hidden_states` has shape `[batch, seq, hidden]`.
/// `attention_mask` has shape `[batch, seq]` with values 0 or 1 (u32).
///
/// Returns a flat `Vec<f32>` of length `hidden` (assumes batch size 1).
fn masked_mean_pool(
    hidden_states: &Tensor,
    attention_mask: &Tensor,
) -> candle_core::Result<Vec<f32>> {
    // Cast the integer mask to f32 and expand to [1, seq, 1] for broadcasting.
    let mask_f32 = attention_mask.to_dtype(DType::F32)?.unsqueeze(2)?; // [1, seq, 1]

    // Weighted sum over sequence tokens: broadcast mask [1, seq, 1] -> [1, seq, hidden], then sum.
    let weighted_sum = hidden_states.broadcast_mul(&mask_f32)?.sum(1)?; // [1, hidden]

    // Sum of mask weights (number of non-padding tokens): [1, 1]
    let mask_sum = mask_f32.sum(1)?; // [1, 1]

    // Mean: [1, hidden]
    let mean = weighted_sum.broadcast_div(&mask_sum)?;

    // Flatten to Vec<f32> (batch size is 1)
    mean.squeeze(0)?.to_vec1::<f32>()
}

#[async_trait]
impl Embedder for CandleEmbedder {
    fn dimension(&self) -> usize {
        384
    }

    async fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(E5Prefix::Passage, text)
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(E5Prefix::Query, text)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[tokio::test]
    #[ignore = "telecharge multilingual-e5-small (~470 Mo) ; lancer manuellement"]
    async fn embeds_with_expected_dimension_and_semantics() {
        let embedder = CandleEmbedder::load().unwrap();
        assert_eq!(embedder.dimension(), 384);

        let a = embedder
            .embed_passage("Le chat dort sur le canape")
            .await
            .unwrap();
        let similar = embedder
            .embed_query("Un felin se repose sur le sofa")
            .await
            .unwrap();
        let unrelated = embedder
            .embed_query("Le cours de la bourse a chute aujourd'hui")
            .await
            .unwrap();

        assert_eq!(a.len(), 384);

        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0_f32).abs() < 1e-3,
            "vecteur non normalise: {norm}"
        );
        assert!(
            cosine(&a, &similar) > cosine(&a, &unrelated),
            "la paire proche devrait avoir une similarite superieure"
        );
    }
}
