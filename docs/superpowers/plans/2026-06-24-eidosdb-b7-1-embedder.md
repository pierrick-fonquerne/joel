# B7.1 Embedder (candle + multilingual-e5-small) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Doter Joel d'un port `Embedder` (domaine) et de son adaptateur `CandleEmbedder` (modele `intfloat/multilingual-e5-small`, 384d, pur Rust) qui transforme du texte en vecteur normalise, prerequis du `KnowledgeStore` (B7.2).

**Architecture:** Le trait `Embedder` et `EmbedError` vivent dans `crates/domain` (zero dependance technique). L'adaptateur concret vit dans un nouveau crate `crates/knowledge`, qui isole les dependances ML lourdes (`candle`, `tokenizers`, `hf-hub`) hors de `domain`, `persistence` et `api`. La logique pure (prefixe e5, normalisation L2) est testee unitairement ; l'inference du modele est couverte par un test golden `#[ignore]` (semantique : paire proche plus proche qu'une paire eloignee).

**Tech Stack:** Rust edition 2024, toolchain stable. `candle-core` / `candle-nn` / `candle-transformers` (modele `xlm_roberta`), `tokenizers`, `hf-hub` (telechargement modele), `async-trait`, `thiserror`.

## Global Constraints

- Edition 2024, toolchain stable (channel `stable`, voir `rust-toolchain.toml`).
- Lints workspace : `unsafe_code = deny`, `missing_docs = warn`, `clippy::all = deny`, `clippy::pedantic = warn`, `unwrap_used = deny`, `expect_used = deny`. Doc comment sur chaque `pub`. Aucun `unwrap`/`expect` hors tests ; dans les modules de test, ajouter `#![allow(clippy::unwrap_used)]`.
- Jamais `==` sur `f32` (comparer via une tolerance ou `total_cmp`).
- Pas de cast `as` non-elargissant : utiliser `try_from`.
- Ordre pre-commit : `cargo fmt` PUIS `cargo clippy` PUIS `cargo test`. Tester `--all-features`.
- Git TBD : travailler sur la branche `feature/eidosdb-b7-1-embedder` depuis `main`. Jamais de commit sur `main`. Verifier `git rev-parse --abbrev-ref HEAD` avant tout commit (clone potentiellement partage). Sur Windows, lancer cargo et git via le Bash tool avec `dangerouslyDisableSandbox: true`.
- Aucune mention d'IA ni de concurrents dans les commits/commentaires.
- Repo : `C:\Users\pierr\Documents\Developpements\Perso\joel`, workspace Cargo sous `backend/`.

---

### Task 0: Branche de travail

**Files:** aucun (operation git).

- [ ] **Step 1: Verifier la branche et partir de main a jour**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel" && git rev-parse --abbrev-ref HEAD && git fetch origin main && git checkout main && git pull --ff-only && git checkout -b feature/eidosdb-b7-1-embedder
```
Expected: bascule sur `feature/eidosdb-b7-1-embedder`.

---

### Task 1: Port `Embedder` + `EmbedError` (domaine)

**Files:**
- Create: `backend/crates/domain/src/knowledge/mod.rs`
- Modify: `backend/crates/domain/src/lib.rs` (ajouter `pub mod knowledge;`)
- Test: dans `backend/crates/domain/src/knowledge/mod.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: rien (depend uniquement de `async_trait`, `thiserror`, deja presents dans `domain`).
- Produces:
  - `trait Embedder: Send + Sync` avec `fn dimension(&self) -> usize`, `async fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbedError>`, `async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError>`.
  - `enum EmbedError` (variantes `ModelLoad(String)`, `Inference(String)`, `Tokenize(String)`), `derive(Debug, thiserror::Error)`.

- [ ] **Step 1: Ecrire le test qui echoue**

Creer `backend/crates/domain/src/knowledge/mod.rs` :

```rust
//! Knowledge ports: text embedding contract consumed by the knowledge store.

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
        assert_eq!(embedder.embed_passage("hello").await.unwrap().len(), 3);
        assert_eq!(embedder.embed_query("hello").await.unwrap().len(), 3);
    }
}
```

Modifier `backend/crates/domain/src/lib.rs` pour ajouter sous `pub mod auth;` :

```rust
pub mod knowledge;
```

- [ ] **Step 2: Lancer le test pour confirmer l'echec (compilation)**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo test -p domain knowledge:: 2>&1 | tail -20
```
Expected: si le module n'etait pas encore declare, erreur de compilation ; une fois `mod.rs` + `lib.rs` ecrits, le test compile et passe. (Ce port est trivial : le rouge est l'absence du module.)

- [ ] **Step 3: Verifier fmt + clippy**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo fmt && cargo clippy -p domain --all-features -- -D warnings 2>&1 | tail -20
```
Expected: aucun warning.

- [ ] **Step 4: Lancer le test pour confirmer le vert**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo test -p domain knowledge:: 2>&1 | tail -10
```
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Commit**

```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel" && git add -A && git commit -m "feat(domain): add Embedder port and EmbedError"
```

---

### Task 2: Helpers purs du crate `knowledge` (prefixe e5 + normalisation L2)

**Files:**
- Create: `backend/crates/knowledge/Cargo.toml`
- Create: `backend/crates/knowledge/src/lib.rs`
- Create: `backend/crates/knowledge/src/text.rs` (prefixe e5 + normalisation, logique pure)
- Modify: `backend/Cargo.toml` (ajouter `crates/knowledge` aux membres)
- Test: dans `backend/crates/knowledge/src/text.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: rien.
- Produces:
  - `enum E5Prefix { Query, Passage }` avec `fn apply(self, text: &str) -> String` -> `"query: {text}"` / `"passage: {text}"`.
  - `fn normalize_l2(vector: &[f32]) -> Vec<f32>` : renvoie le vecteur divise par sa norme L2 ; si la norme est ~0, renvoie une copie inchangee.

- [ ] **Step 1: Declarer le crate dans le workspace**

Modifier `backend/Cargo.toml`, membre `members` :

```toml
members = ["crates/domain", "crates/persistence", "crates/knowledge", "bins/api", "bins/runner"]
```

Creer `backend/crates/knowledge/Cargo.toml` :

```toml
[package]
name = "knowledge"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../domain" }
async-trait = "0.1"
thiserror = "2"
candle-core = "0.9"
candle-nn = "0.9"
candle-transformers = "0.9"
tokenizers = "0.21"
hf-hub = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Creer `backend/crates/knowledge/src/lib.rs` :

```rust
//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small.

mod text;

pub use text::{normalize_l2, E5Prefix};
```

- [ ] **Step 2: Ecrire les tests qui echouent**

Creer `backend/crates/knowledge/src/text.rs` avec les tests AVANT l'implementation :

```rust
//! Pure text helpers for the e5 embedding family.

/// The query/passage prefix prepended before embedding, as required by e5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E5Prefix {
    /// Prefix for search queries.
    Query,
    /// Prefix for stored documents.
    Passage,
}

impl E5Prefix {
    /// Prepends the e5 prefix to `text`.
    #[must_use]
    pub fn apply(self, text: &str) -> String {
        unimplemented!()
    }
}

/// Divides `vector` by its L2 norm; returns it unchanged if the norm is ~0.
#[must_use]
pub fn normalize_l2(vector: &[f32]) -> Vec<f32> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn query_prefix_is_prepended() {
        assert_eq!(E5Prefix::Query.apply("chat"), "query: chat");
    }

    #[test]
    fn passage_prefix_is_prepended() {
        assert_eq!(E5Prefix::Passage.apply("chat"), "passage: chat");
    }

    #[test]
    fn normalized_vector_has_unit_norm() {
        let out = normalize_l2(&[3.0, 4.0]);
        let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm was {norm}");
    }

    #[test]
    fn zero_vector_is_returned_unchanged() {
        let out = normalize_l2(&[0.0, 0.0, 0.0]);
        assert_eq!(out, vec![0.0, 0.0, 0.0]);
    }
}
```

- [ ] **Step 3: Lancer les tests pour confirmer l'echec**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo test -p knowledge text:: 2>&1 | tail -20
```
Expected: panique `not implemented` sur les 4 tests (rouge).

- [ ] **Step 4: Implementer le minimum**

Remplacer les deux corps `unimplemented!()` :

```rust
    #[must_use]
    pub fn apply(self, text: &str) -> String {
        match self {
            E5Prefix::Query => format!("query: {text}"),
            E5Prefix::Passage => format!("passage: {text}"),
        }
    }
```

```rust
#[must_use]
pub fn normalize_l2(vector: &[f32]) -> Vec<f32> {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return vector.to_vec();
    }
    vector.iter().map(|x| x / norm).collect()
}
```

- [ ] **Step 5: fmt + clippy + test au vert**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo fmt && cargo clippy -p knowledge --all-features -- -D warnings 2>&1 | tail -20 && cargo test -p knowledge text:: 2>&1 | tail -10
```
Expected: clippy sans warning, `test result: ok. 4 passed`.

- [ ] **Step 6: Commit**

```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel" && git add -A && git commit -m "feat(knowledge): add e5 prefix and L2 normalization helpers"
```

---

### Task 3: Adaptateur `CandleEmbedder` (multilingual-e5-small)

**Files:**
- Create: `backend/crates/knowledge/src/candle_embedder.rs`
- Modify: `backend/crates/knowledge/src/lib.rs` (declarer + re-exporter `CandleEmbedder`)
- Test: dans `backend/crates/knowledge/src/candle_embedder.rs` (test golden `#[ignore]`)

**Interfaces:**
- Consumes: `domain::knowledge::{Embedder, EmbedError}`, `crate::text::{E5Prefix, normalize_l2}`.
- Produces:
  - `struct CandleEmbedder` avec `fn load() -> Result<Self, EmbedError>` (telecharge/charge `intfloat/multilingual-e5-small`) et l'impl du trait `Embedder` (dimension = 384).

**Note d'implementation (modele) :** `multilingual-e5-small` est une architecture **XLM-RoBERTa**. Utiliser `candle_transformers::models::xlm_roberta::{XLMRobertaModel, Config}`. Pipeline d'embedding : appliquer le prefixe e5 -> tokenizer -> tenseurs `input_ids`, `attention_mask`, `token_type_ids` (zeros) -> `model.forward(...)` -> dernier hidden state `[1, seq, 384]` -> mean pooling masque par l'attention -> `normalize_l2`. S'inspirer de l'exemple officiel `candle-examples/examples/xlm-roberta` (ou `bert`) pour le forward et le pooling. Inference CPU (`Device::Cpu`).

- [ ] **Step 1: Ecrire le test golden (ignore) qui echoue**

Creer `backend/crates/knowledge/src/candle_embedder.rs` :

```rust
//! Candle-backed embedder using the multilingual-e5-small model.

use async_trait::async_trait;
use domain::knowledge::{EmbedError, Embedder};

/// Embedder backed by `intfloat/multilingual-e5-small` (384 dimensions).
pub struct CandleEmbedder {
    // champs remplis a l'implementation (modele, tokenizer, device)
}

impl CandleEmbedder {
    /// Loads the model and tokenizer from the Hugging Face hub (cached locally).
    ///
    /// # Errors
    /// Returns [`EmbedError::ModelLoad`] if the model, config or tokenizer
    /// cannot be downloaded or parsed.
    pub fn load() -> Result<Self, EmbedError> {
        unimplemented!()
    }
}

#[async_trait]
impl Embedder for CandleEmbedder {
    fn dimension(&self) -> usize {
        384
    }
    async fn embed_passage(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        unimplemented!()
    }
    async fn embed_query(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        unimplemented!()
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
        assert!((norm - 1.0).abs() < 1e-3, "vecteur non normalise: {norm}");
        assert!(
            cosine(&a, &similar) > cosine(&a, &unrelated),
            "la paire proche devrait avoir une similarite superieure"
        );
    }
}
```

Modifier `backend/crates/knowledge/src/lib.rs` :

```rust
//! Knowledge adapters: text embedding backed by candle + multilingual-e5-small.

mod candle_embedder;
mod text;

pub use candle_embedder::CandleEmbedder;
pub use text::{normalize_l2, E5Prefix};
```

- [ ] **Step 2: Confirmer que le test ignore ne s'execute pas et que ca compile**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo test -p knowledge 2>&1 | tail -15
```
Expected: compile, le test golden apparait `ignored`. (Rouge = `unimplemented!()` non encore code ; on l'implemente a l'etape suivante.)

- [ ] **Step 3: Implementer `CandleEmbedder`**

Remplir la struct et les corps. Squelette de reference (ajuster les noms exacts a l'API `candle-transformers` 0.9 si besoin ; un build cassant indiquera precisement quoi corriger) :

```rust
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

use crate::text::{normalize_l2, E5Prefix};

pub struct CandleEmbedder {
    model: XLMRobertaModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl CandleEmbedder {
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

        let config: Config = serde_json::from_slice(
            &std::fs::read(config_path).map_err(|e| EmbedError::ModelLoad(e.to_string()))?,
        )
        .map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let tokenizer =
            Tokenizer::from_file(tokenizer_path).map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)
                .map_err(|e| EmbedError::ModelLoad(e.to_string()))?
        };
        let model =
            XLMRobertaModel::new(&config, vb).map_err(|e| EmbedError::ModelLoad(e.to_string()))?;
        Ok(Self { model, tokenizer, device })
    }

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
        let attention_mask = Tensor::from_vec(mask.clone(), (1, seq), &self.device)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        let token_type_ids = input_ids
            .zeros_like()
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        let hidden = self
            .model
            .forward(&input_ids, &attention_mask, &token_type_ids, None, None, None)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        // mean pooling masque -> vecteur 384d, puis normalisation L2
        let pooled = masked_mean_pool(&hidden, &attention_mask)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        Ok(normalize_l2(&pooled))
    }
}

fn masked_mean_pool(hidden: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Vec<f32>> {
    // hidden: [1, seq, dim] ; mask: [1, seq]
    let mask = attention_mask.to_dtype(candle_core::DType::F32)?.unsqueeze(2)?; // [1, seq, 1]
    let summed = hidden.broadcast_mul(&mask)?.sum(1)?; // [1, dim]
    let counts = mask.sum(1)?; // [1, 1]
    let mean = summed.broadcast_div(&counts)?; // [1, dim]
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
```

Ajouter `serde_json = "1"` aux dependances de `knowledge/Cargo.toml` (utilise par `load`).

- [ ] **Step 4: Verifier le build + clippy (sans lancer le test ignore)**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo fmt && cargo clippy -p knowledge --all-features -- -D warnings 2>&1 | tail -30
```
Expected: compile et clippy propre. Si un nom d'API `candle-transformers` differe (signature `forward`, nom `Config`/`XLMRobertaModel`), corriger selon le message du compilateur en s'alignant sur l'exemple officiel `xlm-roberta`/`bert`.

- [ ] **Step 5: Lancer le test golden manuellement (telecharge le modele)**

Run:
```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend" && cargo test -p knowledge --all-features -- --ignored embeds_with_expected_dimension_and_semantics --nocapture 2>&1 | tail -20
```
Expected: `test result: ok. 1 passed` (dimension 384, vecteur normalise, paire proche plus similaire que la paire eloignee). Premiere execution = telechargement (~470 Mo) puis cache.

- [ ] **Step 6: Commit**

```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel" && git add -A && git commit -m "feat(knowledge): add CandleEmbedder backed by multilingual-e5-small"
```

---

## Self-Review

- **Couverture spec** : B7.1 = port `Embedder` (Task 1) + `CandleEmbedder` e5-small 384d, prefixes query/passage, normalisation L2 (Tasks 2-3). Conforme a la ligne B7.1 de la spec. Le wiring runner/serveur EidosDB et le port `KnowledgeStore` sont hors B7.1 (lot B7.2).
- **Placeholders** : les `unimplemented!()` sont des etapes Red explicites de TDD, remplacees dans la meme tache ; aucun TODO residuel.
- **Coherence des types** : `EmbedError`/`Embedder` definis Task 1, consommes Tasks 2-3 ; `E5Prefix`/`normalize_l2` definis Task 2, consommes Task 3. Signatures stables.
- **Risque connu** : versions exactes de l'API `candle-transformers` 0.9 (noms `XLMRobertaModel`/`Config`, signature `forward`) a confirmer au build (Step 4 de Task 3) ; le compilateur guide la correction. Fiabilite : cargo, jamais le LSP.

## Prochain lot

B7.2 (port `KnowledgeStore` + `EidosKnowledgeStore`, serveur EidosDB in-process, dependance git eidosdb) : plan separe a ecrire apres validation de B7.1.
