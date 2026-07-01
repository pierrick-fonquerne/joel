# EidosDB B7.2 KnowledgeStore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Joel's memory layer on EidosDB: a `KnowledgeStore` domain port and its gRPC adapter over an embedded EidosDB server, replacing pgvector as the product ceiling.

**Architecture:** Two sequential phases dictated by a pinned git dependency. Phase A adds a general `delete_by_filter` capability across EidosDB (`proto -> query -> server -> client`) plus `Clone` on `EidosClient`, on a dedicated branch whose revision we pin. Phase B consumes that revision in Joel: domain port, embedded-server crate, adapter, then `runner`/`api` wiring.

**Tech Stack:** Rust edition 2024, tonic gRPC, candle (e5-small, delivered in B7.1), EidosDB crates (`-client/-core/-query/-hnsw/-lexical/-server/-proto`), redb storage, async-trait, time, uuid, serde_json.

## Global Constraints

- Toolchain: `edition = "2024"`, `rust-version = "1.88"` (both repos).
- Lints (Joel workspace, inherited by every crate): `clippy::all = deny`, `clippy::pedantic = warn`, `unwrap_used = deny`, `expect_used = deny`, `unsafe_code = deny`, `missing_docs = warn`. EidosDB repo: clippy strict, MIT OR Apache-2.0.
- No `unwrap`/`expect` outside `#[cfg(test)]`. Run `cargo fmt` BEFORE `cargo clippy`.
- Identifiers in English, no abbreviations except standards (id, url, http). Doc comments in English. No em-dash anywhere (use `:`, `,`, `()`, `->`).
- Never download the embedding model in the standard CI gate: integration tests use a deterministic `StubEmbedder`; the real `CandleEmbedder` golden test stays `#[ignore]`.
- TDD strict: failing test first (Red), minimal code (Green), refactor. Frequent commits. Never commit on `main` (TBD).
- Joel branch: `feature/eidosdb-b7-2-knowledgestore` (already created from `origin/main`). EidosDB branch: `feature/delete-by-filter` (created in Task A1).
- EidosDB repo path: `C:\Users\pierr\Documents\Developpements\Encelade\Nubster\eidosdb`. Joel repo path: `C:\Users\pierr\Documents\Developpements\Perso\joel` (backend workspace under `backend/`).

---

# PHASE A : EidosDB sub-deliverable (repo `Nubster/eidosdb`, branch `feature/delete-by-filter`)

### Task A1: `Collection::delete_by_filter` (query crate)

**Files:**
- Create branch: `feature/delete-by-filter`
- Modify: `crates/eidosdb-query/src/collection.rs` (add method after `delete`, ~line 99)
- Test: `crates/eidosdb-query/src/collection.rs` (tests module)

**Interfaces:**
- Consumes: `Filter::compile() -> CompiledFilter`, `PayloadStore::matching_ids(&CompiledFilter) -> Result<HashSet<VectorId>, PayloadError>`, `Collection::delete(&VectorId) -> Result<bool, QueryError>` (all existing).
- Produces: `Collection::delete_by_filter(&mut self, filter: &Filter) -> Result<u64, QueryError>`.

- [ ] **Step 1: Create the EidosDB feature branch**

```bash
cd "C:/Users/pierr/Documents/Developpements/Encelade/Nubster/eidosdb"
git fetch origin && git checkout -b feature/delete-by-filter origin/main
git rev-parse --abbrev-ref HEAD
```
Expected: `feature/delete-by-filter`

- [ ] **Step 2: Write the failing test** (in `collection.rs` tests module, mirror `filter_excludes_non_matching_payloads`)

```rust
#[test]
fn delete_by_filter_removes_only_matching_points() {
    let mut c = collection();
    let wiki = VectorId::new();
    let blog = VectorId::new();
    c.upsert(wiki, embedding(&[1.0, 0.0]), None, Some(payload("wiki"))).expect("wiki");
    c.upsert(blog, embedding(&[1.0, 0.0]), None, Some(payload("blog"))).expect("blog");
    let deleted = c
        .delete_by_filter(&Filter::Eq("source".into(), Value::Text("wiki".into())))
        .expect("delete_by_filter");
    assert_eq!(deleted, 1);
    let hits = c
        .search(&SearchQuery { embedding: embedding(&[1.0, 0.0]), k: 10, metric: None, filter: None })
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, blog);
}
```
Note: reuse the existing test helpers `collection()`, `embedding()`, `payload()` already present in that tests module.

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p eidosdb-query delete_by_filter_removes_only_matching_points
```
Expected: FAIL ("no method named `delete_by_filter`").

- [ ] **Step 4: Write minimal implementation** (after `delete`, ~line 99)

```rust
/// Deletes every vector whose payload matches `filter`, returning the count removed.
///
/// # Errors
///
/// Propagates [`QueryError`] from the payload store or the underlying delete.
pub fn delete_by_filter(&mut self, filter: &Filter) -> Result<u64, QueryError> {
    let compiled = filter.compile();
    let matching = self.payloads.matching_ids(&compiled)?;
    let mut count = 0u64;
    for id in matching {
        if self.delete(&id)? {
            count += 1;
        }
    }
    Ok(count)
}
```
If `HashSet`/`Filter`/`Value` are not in scope for the impl block, add the `use` lines the surrounding code already relies on.

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p eidosdb-query delete_by_filter_removes_only_matching_points
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/eidosdb-query/src/collection.rs
git commit -m "feat(query): add Collection::delete_by_filter"
```

---

### Task A2: proto messages, RPC and conversions

**Files:**
- Modify: `crates/eidosdb-proto/proto/eidosdb.proto` (add 2 messages + 1 rpc)
- Create: `crates/eidosdb-proto/src/convert/delete.rs`
- Modify: `crates/eidosdb-proto/src/convert/mod.rs` (register module)
- Test: `crates/eidosdb-proto/src/convert/delete.rs` (tests module)

**Interfaces:**
- Consumes: `filter_to_pb(&Filter) -> pb::Filter`, `filter_from_pb(pb::Filter) -> Result<Filter, ConversionError>`, `ConversionError::MissingField(&'static str)` (existing).
- Produces: `pb::DeleteByFilterRequest { collection: String, filter: Option<pb::Filter> }`, `pb::DeleteByFilterResponse { deleted: u64 }`, `delete_by_filter_to_pb(&str, &Filter) -> pb::DeleteByFilterRequest`, `delete_by_filter_from_pb(pb::DeleteByFilterRequest) -> Result<(String, Filter), ConversionError>`, and the generated `pb::eidos_db_client::EidosDbClient::delete_by_filter` / server trait method `delete_by_filter`.

- [ ] **Step 1: Extend the .proto** (add messages near the Delete messages ~line 124, add rpc in the `service EidosDb` block ~line 137)

```protobuf
message DeleteByFilterRequest { string collection = 1; Filter filter = 2; }
message DeleteByFilterResponse { uint64 deleted = 1; }
```
Inside `service EidosDb { ... }`, after `rpc Delete(...)`:
```protobuf
  rpc DeleteByFilter(DeleteByFilterRequest) returns (DeleteByFilterResponse);
```

- [ ] **Step 2: Write the failing conversion test** (`crates/eidosdb-proto/src/convert/delete.rs`)

```rust
//! Conversions for the delete-by-filter request.

use crate::convert::{filter_from_pb, filter_to_pb};
use crate::error::ConversionError;
use crate::pb;
use eidosdb_query::{Filter, Value};

/// Builds the wire request for a delete-by-filter call.
#[must_use]
pub fn delete_by_filter_to_pb(collection: &str, filter: &Filter) -> pb::DeleteByFilterRequest {
    pb::DeleteByFilterRequest {
        collection: collection.to_string(),
        filter: Some(filter_to_pb(filter)),
    }
}

/// Parses a wire delete-by-filter request into the collection name and domain filter.
///
/// # Errors
///
/// Returns [`ConversionError::MissingField`] if the filter is absent.
pub fn delete_by_filter_from_pb(
    request: pb::DeleteByFilterRequest,
) -> Result<(String, Filter), ConversionError> {
    let filter = request
        .filter
        .ok_or(ConversionError::MissingField("delete_by_filter.filter"))?;
    Ok((request.collection, filter_from_pb(filter)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_collection_and_filter() {
        let filter = Filter::Eq("theme".into(), Value::Text("press".into()));
        let wire = delete_by_filter_to_pb("press", &filter);
        let (collection, parsed) = delete_by_filter_from_pb(wire).expect("round trip");
        assert_eq!(collection, "press");
        assert_eq!(parsed, filter);
    }

    #[test]
    fn rejects_missing_filter() {
        let wire = pb::DeleteByFilterRequest { collection: "press".into(), filter: None };
        assert!(delete_by_filter_from_pb(wire).is_err());
    }
}
```
Register the module in `crates/eidosdb-proto/src/convert/mod.rs`:
```rust
pub mod delete;
pub use delete::*;
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p eidosdb-proto round_trips_collection_and_filter
```
Expected: FAIL to compile (`pb::DeleteByFilterRequest` unknown) until the .proto regenerates. Running the build triggers `tonic_build`; the failure confirms the generated types/methods do not yet exist or the convert fns are missing.

- [ ] **Step 4: Build to regenerate proto and confirm green**

The messages exist once `build.rs` regenerates from the edited `.proto`. The convert code in Step 2 IS the implementation, so:
```bash
cargo test -p eidosdb-proto round_trips_collection_and_filter rejects_missing_filter
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/eidosdb-proto/proto/eidosdb.proto crates/eidosdb-proto/src/convert/
git commit -m "feat(proto): add DeleteByFilter message, rpc and conversions"
```

---

### Task A3: server handler + client method (wire vertical)

**Files:**
- Modify: `crates/eidosdb-server/src/collection_kind.rs` (dispatch, after `delete` ~line 222)
- Modify: `crates/eidosdb-server/src/service.rs` (implement `delete_by_filter` trait method)
- Modify: `crates/eidosdb-client/src/lib.rs` (add `delete_by_filter` method)
- Test: `crates/eidosdb-client/tests/client.rs` (in-process integration test)

**Interfaces:**
- Consumes: `Collection::delete_by_filter` (Task A1), `delete_by_filter_from_pb`/`delete_by_filter_to_pb` (Task A2), `run_blocking`, `not_found`, `query_error_to_status`, `conversion_error_to_status`, `Registry::get` (existing), `eidosdb_query::{Filter, Value, Payload, FieldValue}` (existing).
- Produces: `CollectionKind::delete_by_filter(&mut self, &Filter) -> Result<u64, QueryError>`, server `EidosDbService::delete_by_filter`, `EidosClient::delete_by_filter(&mut self, &str, Filter) -> Result<u64, ClientError>`.

- [ ] **Step 1: Write the failing integration test** (`crates/eidosdb-client/tests/client.rs`, mirror existing `spawn_server`/`create` helpers; import `BTreeMap`, `Payload`, `FieldValue`, `Value`, `Filter` as the file already does for other tests)

```rust
#[tokio::test]
async fn delete_by_filter_removes_matching_points() {
    let (endpoint, _dir) = spawn_server().await;
    let mut client = EidosClient::connect(endpoint).await.expect("connect");
    create(&mut client, "notes", 3).await;

    let mut source_field = |source: &str| {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("source".to_string(), FieldValue::Scalar(Value::Text(source.into())));
        Payload::new(fields).expect("payload")
    };

    let wiki = VectorId::new();
    let blog = VectorId::new();
    client.upsert("notes", wiki, Embedding::new(vec![1.0, 0.0, 0.0]).unwrap(), None, Some(source_field("wiki"))).await.expect("wiki");
    client.upsert("notes", blog, Embedding::new(vec![0.0, 1.0, 0.0]).unwrap(), None, Some(source_field("blog"))).await.expect("blog");

    let deleted = client
        .delete_by_filter("notes", Filter::Eq("source".into(), Value::Text("wiki".into())))
        .await
        .expect("delete_by_filter");
    assert_eq!(deleted, 1);
    assert_eq!(client.describe_collection("notes").await.expect("describe").count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p eidosdb-client delete_by_filter_removes_matching_points
```
Expected: FAIL to compile (`EidosClient` has no `delete_by_filter`).

- [ ] **Step 3: Implement the dispatch** (`crates/eidosdb-server/src/collection_kind.rs`, after `delete`)

```rust
/// Deletes all vectors matching `filter`, returning the count removed.
///
/// # Errors
///
/// Propagates [`QueryError`] from the inner collection.
pub fn delete_by_filter(&mut self, filter: &Filter) -> Result<u64, QueryError> {
    match self {
        Self::Flat(c) => c.delete_by_filter(filter),
        Self::Hnsw(c) => c.delete_by_filter(filter),
    }
}
```
Add `use eidosdb_query::Filter;` if not already imported in that file.

- [ ] **Step 4: Implement the server handler** (`crates/eidosdb-server/src/service.rs`, after the `delete` impl)

```rust
async fn delete_by_filter(
    &self,
    request: Request<pb::DeleteByFilterRequest>,
) -> Result<Response<pb::DeleteByFilterResponse>, Status> {
    let (collection, filter) =
        delete_by_filter_from_pb(request.into_inner()).map_err(|e| conversion_error_to_status(&e))?;
    let handle = self.registry.get(&collection).ok_or_else(|| not_found(&collection))?;
    let deleted = run_blocking(handle, move |kind| {
        kind.delete_by_filter(&filter).map_err(|e| query_error_to_status(&e))
    })
    .await?;
    Ok(Response::new(pb::DeleteByFilterResponse { deleted }))
}
```
Add `delete_by_filter_from_pb` to the `use` list pulling the other `*_from_pb` converters.

- [ ] **Step 5: Implement the client method** (`crates/eidosdb-client/src/lib.rs`, after `delete`)

```rust
/// Deletes all points matching `filter`, returning the number deleted.
///
/// # Errors
///
/// Returns [`ClientError::Status`] if the server rejects the request,
/// or [`ClientError::Conversion`] if the filter cannot be encoded.
pub async fn delete_by_filter(
    &mut self,
    collection: &str,
    filter: Filter,
) -> Result<u64, ClientError> {
    let response = self
        .inner
        .delete_by_filter(delete_by_filter_to_pb(collection, &filter))
        .await?;
    Ok(response.into_inner().deleted)
}
```
Add `Filter` and `delete_by_filter_to_pb` to the client's `use` list (alongside the existing `search_query_to_pb`, `SearchQuery`, etc.).

- [ ] **Step 6: Run test to verify it passes**

```bash
cargo test -p eidosdb-client delete_by_filter_removes_matching_points
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/eidosdb-server/src crates/eidosdb-client/src crates/eidosdb-client/tests
git commit -m "feat(server,client): wire DeleteByFilter end to end"
```

---

### Task A4: `Clone` on `EidosClient` + pin the revision

**Files:**
- Modify: `crates/eidosdb-client/src/lib.rs` (derive Clone on `EidosClient`)
- Test: `crates/eidosdb-client/tests/client.rs`

**Interfaces:**
- Consumes: `pb::eidos_db_client::EidosDbClient<Channel>` (tonic `Channel` is `Clone`).
- Produces: `EidosClient: Clone`.

- [ ] **Step 1: Write the failing test** (`tests/client.rs`)

```rust
#[tokio::test]
async fn client_clone_shares_connection() {
    let (endpoint, _dir) = spawn_server().await;
    let client = EidosClient::connect(endpoint).await.expect("connect");
    let mut writer = client.clone();
    let mut reader = client.clone();
    create(&mut writer, "notes", 3).await;
    assert_eq!(reader.describe_collection("notes").await.expect("describe").count, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p eidosdb-client client_clone_shares_connection
```
Expected: FAIL to compile (`EidosClient` is not `Clone`).

- [ ] **Step 3: Derive Clone** (`crates/eidosdb-client/src/lib.rs`, above `pub struct EidosClient`)

```rust
#[derive(Clone)]
pub struct EidosClient {
    inner: pb::eidos_db_client::EidosDbClient<Channel>,
}
```

- [ ] **Step 4: Run test to verify it passes, then the full client suite**

```bash
cargo test -p eidosdb-client client_clone_shares_connection
cargo test -p eidosdb-client
```
Expected: PASS.

- [ ] **Step 5: Full gate, commit, push, capture the revision**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
git add -A && git commit -m "feat(client): derive Clone on EidosClient"
git push -u origin feature/delete-by-filter
git rev-parse HEAD
```
Record the printed 40-char SHA. This is `EIDOSDB_REV`, pinned by every Joel crate in Phase B. Expected: all gates green.

---

# PHASE B : Joel (repo `Perso/joel`, branch `feature/eidosdb-b7-2-knowledgestore`)

> All Joel commands run from `C:/Users/pierr/Documents/Developpements/Perso/joel/backend`. Replace `EIDOSDB_REV` with the SHA captured in Task A4. The EidosDB git URL is `https://github.com/nubster-opensources/eidosdb`.

### Task B1: domain port `knowledge`

**Files:**
- Create: `backend/crates/domain/src/knowledge/corpus.rs`
- Create: `backend/crates/domain/src/knowledge/memory_id.rs`
- Create: `backend/crates/domain/src/knowledge/score.rs`
- Create: `backend/crates/domain/src/knowledge/store.rs`
- Modify: `backend/crates/domain/src/knowledge/mod.rs` (declare + re-export; keeps existing `Embedder`/`EmbedError`)
- Modify: `backend/crates/domain/Cargo.toml` (ensure `uuid`, `time`, `serde_json`, `thiserror`, `async-trait` present)

**Interfaces:**
- Consumes: existing `domain::knowledge::EmbedError`.
- Produces: `Corpus { Press, AgentMemory }` + `Corpus::collection(self) -> &'static str`; `MemoryId(Uuid)` with `new()`, `as_uuid()`, `Display`; `Score(f32)` with `value()`; `Remembrance { id, text, payload, occurred_at }`; `RecallHit { id, score, payload }`; `RecallFilter { theme, since, until }`; `trait KnowledgeStore`; `enum KnowledgeError`.

- [ ] **Step 1: Write the failing tests** (`backend/crates/domain/src/knowledge/corpus.rs`)

```rust
//! The logical corpus a memory belongs to.

/// A logical collection of memories backing one EidosDB collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corpus {
    /// Press review documents.
    Press,
    /// Agent memories.
    AgentMemory,
}

impl Corpus {
    /// The EidosDB collection name backing this corpus.
    #[must_use]
    pub fn collection(self) -> &'static str {
        match self {
            Self::Press => "press",
            Self::AgentMemory => "memory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_corpus_to_its_collection() {
        assert_eq!(Corpus::Press.collection(), "press");
        assert_eq!(Corpus::AgentMemory.collection(), "memory");
    }
}
```

- [ ] **Step 2: Add the newtypes** (`memory_id.rs` and `score.rs`)

`memory_id.rs`:
```rust
//! Stable identifier for a memory, provided by the caller.

use uuid::Uuid;

/// Caller-provided stable identifier for a memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryId(Uuid);

impl MemoryId {
    /// Generates a fresh time-ordered identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing UUID.
    #[must_use]
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// The wrapped UUID.
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_uuid() {
        let id = MemoryId::new();
        assert_eq!(MemoryId::from_uuid(id.as_uuid()), id);
    }
}
```
`score.rs`:
```rust
//! Relevance score newtype. Never compared by equality.

/// A relevance score; larger means closer. Never compared with `==`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score(f32);

impl Score {
    /// Wraps a raw score.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// The raw score value.
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_its_value() {
        assert!((Score::new(0.5).value() - 0.5).abs() < f32::EPSILON);
    }
}
```
Requires `uuid` with feature `v7` in `domain/Cargo.toml` (Joel currently pins `v4`; add `v7`).

- [ ] **Step 3: Add the port and error** (`store.rs`)

```rust
//! The KnowledgeStore port: text-in, semantic recall out.

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

/// Stores and recalls memories by semantic similarity.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Idempotently stores one memory (replaces any with the same id).
    async fn remember(&self, corpus: Corpus, item: Remembrance) -> Result<(), KnowledgeError>;
    /// Idempotently stores a batch of memories.
    async fn remember_batch(&self, corpus: Corpus, items: Vec<Remembrance>) -> Result<(), KnowledgeError>;
    /// Vector recall of the top `k` memories matching `query` under `filter`.
    async fn recall(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError>;
    /// Hybrid (vector + lexical) recall of the top `k` memories.
    async fn recall_hybrid(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError>;
    /// Forgets the memory with id `id`, returning whether it existed.
    async fn forget(&self, corpus: Corpus, id: MemoryId) -> Result<bool, KnowledgeError>;
}
```

- [ ] **Step 4: Wire the module** (`knowledge/mod.rs`, keep existing `Embedder`/`EmbedError`)

```rust
mod corpus;
mod memory_id;
mod score;
mod store;

pub use corpus::Corpus;
pub use memory_id::MemoryId;
pub use score::Score;
pub use store::{KnowledgeError, KnowledgeStore, RecallFilter, RecallHit, Remembrance};
```
(Leave the existing `Embedder` / `EmbedError` definitions in `mod.rs`, or split them into `embedder.rs`; if split, re-export them too.)

- [ ] **Step 5: Run tests**

```bash
cargo test -p domain knowledge
```
Expected: PASS (3 unit tests).

- [ ] **Step 6: Commit**

```bash
git add backend/crates/domain
git commit -m "feat(domain): add KnowledgeStore port and types"
```

---

### Task B2: `eidos-embedded` crate (embedded server + first git dependency)

**Files:**
- Create: `backend/crates/eidos-embedded/Cargo.toml`
- Create: `backend/crates/eidos-embedded/src/lib.rs`
- Create: `backend/crates/eidos-embedded/tests/embedded.rs`
- Modify: `backend/Cargo.toml` (add member)
- Modify: `backend/deny.toml` (allow the EidosDB git source)

**Interfaces:**
- Consumes: `eidosdb_server::{registry::Registry, service::EidosDbService}`, `eidosdb_proto::pb::eidos_db_server::EidosDbServer` (git, `EIDOSDB_REV`).
- Produces: `EmbeddedEidos::start(data_dir: PathBuf, addr: SocketAddr) -> Result<(String, ShutdownHandle), EmbeddedError>`; `ShutdownHandle` (aborts the server task on drop or via `shutdown()`).

- [ ] **Step 1: Allow the git source in `deny.toml`** (append)

```toml
[sources]
unknown-git = "deny"
allow-git = ["https://github.com/nubster-opensources/eidosdb"]
```

- [ ] **Step 2: Create the crate manifest** (`crates/eidos-embedded/Cargo.toml`)

```toml
[package]
name = "eidos-embedded"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
eidosdb-server = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-proto = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
tonic = "0.12"
tokio = { version = "1", features = ["rt", "net"] }
tokio-stream = { version = "0.1", features = ["net"] }
thiserror = "2"

[dev-dependencies]
eidosdb-client = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-core = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
tempfile = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }
```
Note: confirm the `tonic` version against EidosDB's `tonic` (read `eidosdb-proto/Cargo.toml`); match it exactly to avoid a duplicate-version clash.
Add the member to `backend/Cargo.toml`:
```toml
members = ["crates/domain", "crates/persistence", "crates/knowledge", "crates/eidos-embedded", "bins/api", "bins/runner"]
```

- [ ] **Step 3: Write the failing integration test** (`crates/eidos-embedded/tests/embedded.rs`)

```rust
use std::net::SocketAddr;

use eidos_embedded::EmbeddedEidos;
use eidosdb_client::{CollectionSpec, EidosClient};
use eidosdb_core::{Dimension, Metric};
use eidosdb_proto::convert::enums::IndexTypeChoice;

#[tokio::test]
async fn starts_and_serves_a_reachable_server() {
    let dir = tempfile::tempdir().expect("tempdir");
    let addr: SocketAddr = "127.0.0.1:0".parse().expect("addr");
    let (endpoint, _handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr)
        .await
        .expect("start");

    let mut client = EidosClient::connect(endpoint).await.expect("connect");
    client
        .create_collection(CollectionSpec {
            name: "press".into(),
            metric: Metric::Cosine,
            dimension: Dimension(3),
            index_type: IndexTypeChoice::Hnsw,
            hnsw: Some(Default::default()),
        })
        .await
        .expect("create");
    assert_eq!(client.list_collections().await.expect("list").len(), 1);
}
```
(Confirm the exact import paths for `CollectionSpec`/`IndexTypeChoice`/`Metric`/`Dimension` against `eidosdb-client`'s re-exports; adjust `use` lines to match.)

- [ ] **Step 4: Run test to verify it fails**

```bash
cargo test -p eidos-embedded starts_and_serves_a_reachable_server
```
Expected: FAIL to compile (`eidos_embedded::EmbeddedEidos` missing).

- [ ] **Step 5: Implement** (`crates/eidos-embedded/src/lib.rs`)

```rust
//! Embedded EidosDB server: open a data dir and serve gRPC in-process.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use eidosdb_proto::pb::eidos_db_server::EidosDbServer;
use eidosdb_server::registry::Registry;
use eidosdb_server::service::EidosDbService;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

/// Failure modes while starting the embedded server.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddedError {
    /// Opening the data directory failed.
    #[error("registry open failed: {0}")]
    Registry(String),
    /// Binding the listen address failed.
    #[error("bind failed: {0}")]
    Bind(String),
}

/// Handle that aborts the server task when dropped.
pub struct ShutdownHandle(JoinHandle<()>);

impl ShutdownHandle {
    /// Aborts the embedded server task.
    pub fn shutdown(self) {
        self.0.abort();
    }
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Starts an embedded EidosDB server.
pub struct EmbeddedEidos;

impl EmbeddedEidos {
    /// Opens `data_dir` and serves gRPC on `addr`. Pass port `0` for an ephemeral port.
    ///
    /// Returns the reachable endpoint (`http://host:port`) and a shutdown handle.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddedError`] if the data dir cannot be opened or the address cannot be bound.
    pub async fn start(
        data_dir: PathBuf,
        addr: SocketAddr,
    ) -> Result<(String, ShutdownHandle), EmbeddedError> {
        let registry = Registry::open(data_dir).map_err(|e| EmbeddedError::Registry(e.to_string()))?;
        let registry = Arc::new(registry);
        let listener = TcpListener::bind(addr).await.map_err(|e| EmbeddedError::Bind(e.to_string()))?;
        let local = listener.local_addr().map_err(|e| EmbeddedError::Bind(e.to_string()))?;
        let service = EidosDbServer::new(EidosDbService::new(registry));
        let task = tokio::spawn(async move {
            let _ = tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await;
        });
        Ok((format!("http://{local}"), ShutdownHandle(task)))
    }
}
```
(If `Registry::open` returns a non-`Display` error, map via `format!("{e:?}")`.)

- [ ] **Step 6: Run test, then commit**

```bash
cargo test -p eidos-embedded starts_and_serves_a_reachable_server
cargo fmt -p eidos-embedded && cargo clippy -p eidos-embedded --all-targets -- -D warnings
git add backend/crates/eidos-embedded backend/Cargo.toml backend/deny.toml backend/Cargo.lock
git commit -m "feat(eidos-embedded): embedded EidosDB server over pinned git rev"
```
Expected: PASS, green clippy.

---

### Task B3: adapter mapping helpers (pure)

**Files:**
- Modify: `backend/crates/knowledge/Cargo.toml` (add EidosDB client/core/query/hnsw/lexical git deps)
- Create: `backend/crates/knowledge/src/mapping.rs`
- Modify: `backend/crates/knowledge/src/lib.rs` (declare `mod mapping;` + re-export helpers)

**Interfaces:**
- Consumes: `domain::knowledge::RecallFilter`, `eidosdb_query::{Filter, Value, Payload, FieldValue}`.
- Produces: `recall_filter_to_filter(&RecallFilter) -> Option<Filter>`; `build_payload(memory_id: MemoryId, occurred_at: OffsetDateTime, theme: Option<&str>, business: &serde_json::Value) -> Result<Payload, KnowledgeError>`; `payload_to_json(&Payload) -> serde_json::Value`. Reserved keys: `memory_id` (Text), `occurred_at` (Integer epoch seconds), `theme` (Text), `payload_json` (Text = serialized business metadata).

- [ ] **Step 1: Add the git dependencies** (`crates/knowledge/Cargo.toml`, append to `[dependencies]`)

```toml
eidosdb-client = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-core = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-query = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-hnsw = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
eidosdb-lexical = { git = "https://github.com/nubster-opensources/eidosdb", rev = "EIDOSDB_REV" }
time = { version = "0.3", features = ["serde"] }
```

- [ ] **Step 2: Write the failing tests** (`crates/knowledge/src/mapping.rs`)

```rust
//! Pure mappings between domain recall types and EidosDB query types.

use domain::knowledge::{KnowledgeError, MemoryId, RecallFilter};
use eidosdb_query::{FieldValue, Filter, Payload, Value};
use time::OffsetDateTime;

/// Reserved payload keys.
pub const KEY_MEMORY_ID: &str = "memory_id";
pub const KEY_OCCURRED_AT: &str = "occurred_at";
pub const KEY_THEME: &str = "theme";
pub const KEY_PAYLOAD_JSON: &str = "payload_json";

/// Translates a [`RecallFilter`] into an EidosDB [`Filter`], or `None` if empty.
#[must_use]
pub fn recall_filter_to_filter(filter: &RecallFilter) -> Option<Filter> {
    let mut clauses = Vec::new();
    if let Some(theme) = &filter.theme {
        clauses.push(Filter::Eq(KEY_THEME.into(), Value::Text(theme.clone())));
    }
    if let Some(since) = filter.since {
        clauses.push(Filter::Gte(KEY_OCCURRED_AT.into(), Value::Integer(since.unix_timestamp())));
    }
    if let Some(until) = filter.until {
        clauses.push(Filter::Lte(KEY_OCCURRED_AT.into(), Value::Integer(until.unix_timestamp())));
    }
    match clauses.len() {
        0 => None,
        1 => clauses.into_iter().next(),
        _ => Some(Filter::And(clauses)),
    }
}

/// Builds the reserved-key payload for an upsert.
///
/// # Errors
///
/// Returns [`KnowledgeError::Config`] if the payload cannot be constructed.
pub fn build_payload(
    memory_id: MemoryId,
    occurred_at: OffsetDateTime,
    theme: Option<&str>,
    business: &serde_json::Value,
) -> Result<Payload, KnowledgeError> {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(KEY_MEMORY_ID.to_string(), FieldValue::Scalar(Value::Text(memory_id.to_string())));
    fields.insert(KEY_OCCURRED_AT.to_string(), FieldValue::Scalar(Value::Integer(occurred_at.unix_timestamp())));
    if let Some(theme) = theme {
        fields.insert(KEY_THEME.to_string(), FieldValue::Scalar(Value::Text(theme.to_string())));
    }
    let json = serde_json::to_string(business).map_err(|e| KnowledgeError::Config(e.to_string()))?;
    fields.insert(KEY_PAYLOAD_JSON.to_string(), FieldValue::Scalar(Value::Text(json)));
    Payload::new(fields).map_err(|e| KnowledgeError::Config(format!("{e:?}")))
}

/// Recovers the business metadata stored under `payload_json`.
#[must_use]
pub fn payload_to_json(payload: &Payload) -> serde_json::Value {
    payload
        .get(KEY_PAYLOAD_JSON)
        .and_then(|field| match field {
            FieldValue::Scalar(Value::Text(text)) => serde_json::from_str(text).ok(),
            _ => None,
        })
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_maps_to_none() {
        assert!(recall_filter_to_filter(&RecallFilter::default()).is_none());
    }

    #[test]
    fn theme_and_window_combine_with_and() {
        let filter = RecallFilter {
            theme: Some("press".into()),
            since: Some(OffsetDateTime::from_unix_timestamp(100).unwrap()),
            until: Some(OffsetDateTime::from_unix_timestamp(200).unwrap()),
        };
        match recall_filter_to_filter(&filter) {
            Some(Filter::And(clauses)) => assert_eq!(clauses.len(), 3),
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn business_payload_round_trips_through_json() {
        let id = MemoryId::new();
        let business = serde_json::json!({ "title": "Hello", "url": "https://x" });
        let payload = build_payload(id, OffsetDateTime::from_unix_timestamp(10).unwrap(), Some("press"), &business)
            .expect("payload");
        assert_eq!(payload_to_json(&payload), business);
    }
}
```
Confirm `Payload::get` exists and returns `Option<&FieldValue>`; if the accessor differs, adjust `payload_to_json` to the real API (read `eidosdb-query/src/payload.rs`).

- [ ] **Step 3: Wire and run**

In `crates/knowledge/src/lib.rs` add:
```rust
mod mapping;
pub use mapping::{build_payload, payload_to_json, recall_filter_to_filter};
```
```bash
cargo test -p knowledge mapping
```
Expected: PASS (3 tests). FAIL first if run before the impl exists.

- [ ] **Step 4: Commit**

```bash
git add backend/crates/knowledge backend/Cargo.lock
git commit -m "feat(knowledge): pure mappings for recall filter and payload"
```

---

### Task B4: `EidosKnowledgeStore` adapter + integration tests

**Files:**
- Create: `backend/crates/knowledge/src/eidos_knowledge_store.rs`
- Create: `backend/crates/knowledge/tests/support/stub_embedder.rs` (or inline in the test file)
- Create: `backend/crates/knowledge/tests/knowledge_store.rs`
- Modify: `backend/crates/knowledge/src/lib.rs` (re-export `EidosKnowledgeStore`)
- Modify: `backend/crates/knowledge/Cargo.toml` (dev-deps: `eidos-embedded`, `tempfile`, `tokio` macros, `async-trait`)

**Interfaces:**
- Consumes: `EidosClient` (clone-per-call), `Arc<dyn Embedder>`, mapping helpers (B3), `CollectionSpec`, `SearchQuery`, `HybridQuery`, `Embedding`, `VectorId`, `Document`, `HnswConfig`, `Dimension`, `Metric`, `IndexTypeChoice`.
- Produces: `EidosKnowledgeStore::new(client: EidosClient, embedder: Arc<dyn Embedder>) -> Self`; `EidosKnowledgeStore::ensure_collections(&self) -> Result<(), KnowledgeError>`; `impl KnowledgeStore for EidosKnowledgeStore`.

- [ ] **Step 1: Write the StubEmbedder support** (`tests/support/stub_embedder.rs`)

```rust
use async_trait::async_trait;
use domain::knowledge::{EmbedError, Embedder};

/// Deterministic 384-dim embedder for tests: no model download.
pub struct StubEmbedder;

#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize {
        384
    }
    async fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(deterministic_vector(text))
    }
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(deterministic_vector(text))
    }
}

fn deterministic_vector(text: &str) -> Vec<f32> {
    let mut values = vec![0.0f32; 384];
    for (index, byte) in text.bytes().enumerate() {
        values[index % 384] += f32::from(byte);
    }
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        values[0] = 1.0;
        return values;
    }
    values.iter().map(|v| v / norm).collect()
}
```

- [ ] **Step 2: Write the failing integration test** (`tests/knowledge_store.rs`)

```rust
mod support { pub mod stub_embedder; }

use std::net::SocketAddr;
use std::sync::Arc;

use domain::knowledge::{Corpus, KnowledgeStore, MemoryId, RecallFilter, Remembrance};
use eidos_embedded::EmbeddedEidos;
use eidosdb_client::EidosClient;
use knowledge::EidosKnowledgeStore;
use support::stub_embedder::StubEmbedder;
use time::OffsetDateTime;

async fn store() -> (EidosKnowledgeStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (endpoint, handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr).await.expect("start");
    std::mem::forget(handle); // keep the server alive for the test duration
    let client = EidosClient::connect(endpoint).await.expect("connect");
    let store = EidosKnowledgeStore::new(client, Arc::new(StubEmbedder));
    store.ensure_collections().await.expect("ensure");
    (store, dir)
}

fn item(text: &str, theme: &str) -> Remembrance {
    Remembrance {
        id: MemoryId::new(),
        text: text.into(),
        payload: serde_json::json!({ "theme": theme, "title": text }),
        occurred_at: OffsetDateTime::from_unix_timestamp(1_000).unwrap(),
    }
}

#[tokio::test]
async fn remember_then_recall_returns_the_memory() {
    let (store, _dir) = store().await;
    store.remember(Corpus::Press, item("nuclear policy", "energy")).await.expect("remember");
    let hits = store.recall(Corpus::Press, "nuclear policy", 5, &RecallFilter::default()).await.expect("recall");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].payload["theme"], "energy");
}

#[tokio::test]
async fn remember_is_idempotent_on_memory_id() {
    let (store, _dir) = store().await;
    let mut first = item("same text", "news");
    let id = first.id;
    store.remember(Corpus::Press, first.clone()).await.expect("first");
    first.id = id;
    store.remember(Corpus::Press, first).await.expect("second");
    let hits = store.recall(Corpus::Press, "same text", 10, &RecallFilter::default()).await.expect("recall");
    assert_eq!(hits.len(), 1);
}

#[tokio::test]
async fn forget_removes_the_memory() {
    let (store, _dir) = store().await;
    let it = item("ephemeral", "news");
    let id = it.id;
    store.remember(Corpus::Press, it).await.expect("remember");
    assert!(store.forget(Corpus::Press, id).await.expect("forget"));
    let hits = store.recall(Corpus::Press, "ephemeral", 5, &RecallFilter::default()).await.expect("recall");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn recall_respects_the_theme_filter() {
    let (store, _dir) = store().await;
    store.remember(Corpus::Press, item("a", "energy")).await.expect("a");
    store.remember(Corpus::Press, item("b", "sport")).await.expect("b");
    let filter = RecallFilter { theme: Some("sport".into()), ..RecallFilter::default() };
    let hits = store.recall(Corpus::Press, "a", 10, &filter).await.expect("recall");
    assert!(hits.iter().all(|h| h.payload["theme"] == "sport"));
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p knowledge --test knowledge_store
```
Expected: FAIL to compile (`EidosKnowledgeStore` missing).

- [ ] **Step 4: Implement the adapter** (`crates/knowledge/src/eidos_knowledge_store.rs`)

```rust
//! gRPC adapter implementing the KnowledgeStore port over EidosDB.

use std::sync::Arc;

use async_trait::async_trait;
use domain::knowledge::{
    Corpus, Embedder, KnowledgeError, KnowledgeStore, MemoryId, RecallFilter, RecallHit, Remembrance, Score,
};
use eidosdb_client::{CollectionSpec, EidosClient};
use eidosdb_core::{Dimension, Embedding, Metric, VectorId};
use eidosdb_lexical::Document;
use eidosdb_query::{Filter, HybridQuery, SearchHit, SearchQuery, Value};

use crate::mapping::{build_payload, payload_to_json, recall_filter_to_filter, KEY_MEMORY_ID};

const RRF_K: f64 = 60.0;
const OVERFETCH_FACTOR: usize = 3;

/// KnowledgeStore backed by an EidosDB gRPC server.
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
    /// # Errors
    ///
    /// Returns [`KnowledgeError::Backend`] on a transport or server error.
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
                    index_type: eidosdb_proto::convert::enums::IndexTypeChoice::Hnsw,
                    hnsw: Some(eidosdb_hnsw::HnswConfig::default()),
                })
                .await
                .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    fn theme_of(payload: &serde_json::Value) -> Option<&str> {
        payload.get("theme").and_then(serde_json::Value::as_str)
    }

    fn to_hit(hit: SearchHit) -> RecallHit {
        let payload = hit.payload.as_ref().map(payload_to_json).unwrap_or(serde_json::Value::Null);
        let id = hit
            .payload
            .as_ref()
            .and_then(|p| p.get(KEY_MEMORY_ID))
            .and_then(|f| match f {
                eidosdb_query::FieldValue::Scalar(Value::Text(text)) => uuid::Uuid::parse_str(text).ok(),
                _ => None,
            })
            .map_or_else(MemoryId::new, MemoryId::from_uuid);
        RecallHit { id, score: Score::new(hit.score.0), payload }
    }
}

#[async_trait]
impl KnowledgeStore for EidosKnowledgeStore {
    async fn remember(&self, corpus: Corpus, item: Remembrance) -> Result<(), KnowledgeError> {
        let vector = self.embedder.embed_passage(&item.text).await?;
        let embedding = Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let theme = Self::theme_of(&item.payload).map(str::to_string);
        let payload = build_payload(item.id, item.occurred_at, theme.as_deref(), &item.payload)?;
        let mut client = self.client.clone();
        let name = corpus.collection();
        client
            .delete_by_filter(name, Filter::Eq(KEY_MEMORY_ID.into(), Value::Text(item.id.to_string())))
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        client
            .upsert(name, VectorId::new(), embedding, Some(Document::from(item.text)), Some(payload))
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn remember_batch(&self, corpus: Corpus, items: Vec<Remembrance>) -> Result<(), KnowledgeError> {
        for item in items {
            self.remember(corpus, item).await?;
        }
        Ok(())
    }

    async fn recall(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError> {
        let vector = self.embedder.embed_query(query).await?;
        let embedding = Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let mut client = self.client.clone();
        let hits = client
            .search(corpus.collection(), SearchQuery {
                embedding,
                k,
                metric: None,
                filter: recall_filter_to_filter(filter),
            })
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(hits.into_iter().map(Self::to_hit).collect())
    }

    async fn recall_hybrid(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError> {
        let vector = self.embedder.embed_query(query).await?;
        let embedding = Embedding::new(vector).map_err(|e| KnowledgeError::Config(format!("{e:?}")))?;
        let mut client = self.client.clone();
        let hits = client
            .search_hybrid(corpus.collection(), HybridQuery {
                vector: Some(embedding),
                text: Some(query.to_string()),
                k,
                filter: recall_filter_to_filter(filter),
                metric: None,
                rrf_k: RRF_K,
                overfetch_factor: OVERFETCH_FACTOR,
            })
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(hits.into_iter().map(Self::to_hit).collect())
    }

    async fn forget(&self, corpus: Corpus, id: MemoryId) -> Result<bool, KnowledgeError> {
        let mut client = self.client.clone();
        let deleted = client
            .delete_by_filter(corpus.collection(), Filter::Eq(KEY_MEMORY_ID.into(), Value::Text(id.to_string())))
            .await
            .map_err(|e| KnowledgeError::Backend(e.to_string()))?;
        Ok(deleted > 0)
    }
}
```
Adjust import paths to the real re-exports: read `eidosdb-client/src/lib.rs` to confirm whether `CollectionSpec`, `IndexTypeChoice`, `HnswConfig`, `Metric`, `Dimension`, `VectorId`, `Embedding`, `Document`, `SearchQuery`, `HybridQuery`, `SearchHit`, `Value`, `FieldValue`, `Filter` are re-exported from the client (preferred) or must be pulled from their own crates. Confirm `hit.score.0` is accessible (`Score(pub f32)`); if private, use the public accessor.

- [ ] **Step 5: Wire, run, iterate to green**

In `crates/knowledge/src/lib.rs`:
```rust
mod eidos_knowledge_store;
pub use eidos_knowledge_store::EidosKnowledgeStore;
```
```bash
cargo test -p knowledge --test knowledge_store
```
Expected: PASS (4 tests).

- [ ] **Step 6: Gate and commit**

```bash
cargo fmt -p knowledge && cargo clippy -p knowledge --all-targets -- -D warnings
git add backend/crates/knowledge backend/Cargo.lock
git commit -m "feat(knowledge): EidosKnowledgeStore adapter over EidosDB"
```

---

### Task B5: `runner` wiring (owns the embedded server)

**Files:**
- Create: `backend/bins/runner/src/bootstrap.rs`
- Modify: `backend/bins/runner/src/main.rs` (use bootstrap)
- Create: `backend/bins/runner/tests/ingestion.rs`
- Modify: `backend/bins/runner/Cargo.toml` (deps: `domain`, `knowledge`, `eidos-embedded`, `eidosdb-client`, `serde_json`, `time`; dev-deps: `tempfile`, `tokio` macros)

**Interfaces:**
- Consumes: `EmbeddedEidos::start`, `EidosClient::connect`, `EidosKnowledgeStore::{new, ensure_collections}`, `Arc<dyn Embedder>`.
- Produces: `bootstrap(data_dir, addr, embedder) -> Result<(EidosKnowledgeStore, ShutdownHandle), BootstrapError>` (testable wiring, embedder injected so tests pass a stub and prod passes `CandleEmbedder`).

- [ ] **Step 1: Write the failing test** (`bins/runner/tests/ingestion.rs`)

```rust
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use domain::knowledge::{Corpus, EmbedError, Embedder, KnowledgeStore, MemoryId, RecallFilter, Remembrance};
use runner::bootstrap;
use time::OffsetDateTime;

struct StubEmbedder;
#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize { 384 }
    async fn embed_passage(&self, _t: &str) -> Result<Vec<f32>, EmbedError> { Ok(unit()) }
    async fn embed_query(&self, _t: &str) -> Result<Vec<f32>, EmbedError> { Ok(unit()) }
}
fn unit() -> Vec<f32> { let mut v = vec![0.0; 384]; v[0] = 1.0; v }

#[tokio::test]
async fn bootstrap_serves_and_ingests() {
    let dir = tempfile::tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (store, handle) = bootstrap(dir.path().to_path_buf(), addr, Arc::new(StubEmbedder)).await.expect("bootstrap");
    store.remember(Corpus::Press, Remembrance {
        id: MemoryId::new(), text: "hello".into(),
        payload: serde_json::json!({ "theme": "x" }),
        occurred_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
    }).await.expect("remember");
    let hits = store.recall(Corpus::Press, "hello", 5, &RecallFilter::default()).await.expect("recall");
    assert_eq!(hits.len(), 1);
    handle.shutdown();
}
```
This requires the runner to expose a library target (`runner::bootstrap`). Add `[lib]` to `bins/runner/Cargo.toml` with `path = "src/lib.rs"`, or set `name`/`path` so both bin and lib build.

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p runner --test ingestion
```
Expected: FAIL to compile (`runner::bootstrap` missing).

- [ ] **Step 3: Implement bootstrap** (`bins/runner/src/bootstrap.rs`, exported from a new `src/lib.rs`)

```rust
//! Runner bootstrap: embed the EidosDB server, ensure collections, build the store.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use domain::knowledge::Embedder;
use eidos_embedded::{EmbeddedEidos, ShutdownHandle};
use eidosdb_client::EidosClient;
use knowledge::EidosKnowledgeStore;

/// Failure modes while bootstrapping the runner.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// Embedded server failed to start.
    #[error("embedded server: {0}")]
    Embedded(String),
    /// Client could not connect.
    #[error("client connect: {0}")]
    Connect(String),
    /// Collection bootstrap failed.
    #[error("ensure collections: {0}")]
    Ensure(String),
}

/// Starts the embedded server, ensures collections, returns the store and a shutdown handle.
///
/// # Errors
///
/// Returns [`BootstrapError`] if any startup step fails.
pub async fn bootstrap(
    data_dir: PathBuf,
    addr: SocketAddr,
    embedder: Arc<dyn Embedder>,
) -> Result<(EidosKnowledgeStore, ShutdownHandle), BootstrapError> {
    let (endpoint, handle) = EmbeddedEidos::start(data_dir, addr)
        .await
        .map_err(|e| BootstrapError::Embedded(e.to_string()))?;
    let client = EidosClient::connect(endpoint).await.map_err(|e| BootstrapError::Connect(e.to_string()))?;
    let store = EidosKnowledgeStore::new(client, embedder);
    store.ensure_collections().await.map_err(|e| BootstrapError::Ensure(e.to_string()))?;
    Ok((store, handle))
}
```
`src/lib.rs`:
```rust
//! Runner library surface (bootstrap wiring), kept testable.
pub mod bootstrap;
pub use bootstrap::{bootstrap, BootstrapError};
```
Add `thiserror = "2"` to the runner deps.

- [ ] **Step 4: Use it in main** (`bins/runner/src/main.rs`, replace the body) - read `JOEL_EIDOS_DATA_DIR` / `JOEL_EIDOS_ADDR`, build `CandleEmbedder`, call `bootstrap`, keep the tick loop. Embedder is the real `CandleEmbedder::load()`. Keep the existing `ctrl_c` shutdown; on shutdown call `handle.shutdown()`.

```rust
let data_dir = std::env::var("JOEL_EIDOS_DATA_DIR").unwrap_or_else(|_| "./data/eidos".into());
let addr = std::env::var("JOEL_EIDOS_ADDR").unwrap_or_else(|_| "127.0.0.1:50100".into());
let embedder = std::sync::Arc::new(knowledge::CandleEmbedder::load().expect("model"));
let (_store, handle) = runner::bootstrap(data_dir.into(), addr.parse().expect("addr"), embedder)
    .await
    .expect("bootstrap");
// ... existing interval/ctrl_c loop; the ingestion tick will call _store.remember(...) once the RSS pipeline lands (out of B7.2)
handle.shutdown();
```
Note: `expect` in `main` is acceptable per convention only if the workspace allows it in binaries; if `expect_used = deny` applies to bins, map errors and `return Err(...)` from a `Result`-returning `main` instead.

- [ ] **Step 5: Run, gate, commit**

```bash
cargo test -p runner --test ingestion
cargo fmt -p runner && cargo clippy -p runner --all-targets -- -D warnings
git add backend/bins/runner backend/Cargo.lock
git commit -m "feat(runner): embed EidosDB, ensure collections, build store"
```
Expected: PASS, green clippy.

---

### Task B6: `api` wiring (remote client + recall route)

**Files:**
- Modify: `backend/bins/api/src/state.rs` (add `knowledge: Arc<dyn KnowledgeStore>` to `AppState` + build it)
- Create: `backend/bins/api/src/knowledge_routes.rs` (GET `/api/recall`)
- Modify: `backend/bins/api/src/lib.rs` (merge the new router)
- Create: `backend/bins/api/tests/recall.rs`
- Modify: `backend/bins/api/Cargo.toml` (deps: `knowledge`, `eidosdb-client`, `domain`; dev-deps: `eidos-embedded`, `tempfile`)

**Interfaces:**
- Consumes: `EidosKnowledgeStore`, `EidosClient::connect`, `CandleEmbedder` (prod), `KnowledgeStore` trait.
- Produces: `AppState.knowledge: Arc<dyn KnowledgeStore>`; route `GET /api/recall?corpus=&q=&k=` returning JSON `[{ id, score, payload }]`.

- [ ] **Step 1: Write the failing test** (`bins/api/tests/recall.rs`) - start an embedded server, build an `EidosKnowledgeStore` with a stub embedder, seed one memory, build a minimal router exposing the recall route with that store as state, and assert the HTTP response contains the hit.

```rust
// Uses axum::Router built from knowledge_routes::router(store) and tower::ServiceExt::oneshot.
// Asserts GET /api/recall?corpus=press&q=hello&k=5 returns 200 with a non-empty JSON array.
```
(Write the concrete test mirroring the existing `bins/api` test style: read `bins/api/tests` or `auth_routes` tests for the established `oneshot` request pattern, and reuse the `StubEmbedder` shape from Task B4/B5.)

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p api --test recall
```
Expected: FAIL to compile (`knowledge_routes` missing).

- [ ] **Step 3: Implement the route** (`bins/api/src/knowledge_routes.rs`)

```rust
//! HTTP routes for semantic recall.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use domain::knowledge::{Corpus, KnowledgeStore, RecallFilter};
use serde::Deserialize;

#[derive(Deserialize)]
struct RecallParams {
    corpus: String,
    q: String,
    #[serde(default = "default_k")]
    k: usize,
}
fn default_k() -> usize { 10 }

/// Builds the recall router over a knowledge store.
pub fn router(store: Arc<dyn KnowledgeStore>) -> Router {
    Router::new().route("/api/recall", get(recall)).with_state(store)
}

async fn recall(
    State(store): State<Arc<dyn KnowledgeStore>>,
    Query(params): Query<RecallParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let corpus = match params.corpus.as_str() {
        "press" => Corpus::Press,
        "memory" => Corpus::AgentMemory,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let hits = store
        .recall(corpus, &params.q, params.k, &RecallFilter::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let body = hits
        .into_iter()
        .map(|h| serde_json::json!({ "id": h.id.to_string(), "score": h.score.value(), "payload": h.payload }))
        .collect::<Vec<_>>();
    Ok(Json(serde_json::Value::Array(body)))
}
```
Note: the existing `AppState` uses a single state type; this route uses `Arc<dyn KnowledgeStore>` as its own state. Either mount it as a sub-router with its own state (shown above, simplest) or add `knowledge` to `AppState` and extract via `FromRef`. Pick the sub-router approach to avoid disturbing the existing `AppState` wiring, and in `lib.rs` merge it: `.merge(knowledge_routes::router(knowledge_store))`. In `build_router_with`, construct the store from `JOEL_EIDOS_ENDPOINT` + `CandleEmbedder::load()`.

- [ ] **Step 4: Run, gate, commit**

```bash
cargo test -p api --test recall
cargo fmt -p api && cargo clippy -p api --all-targets -- -D warnings
git add backend/bins/api backend/Cargo.lock
git commit -m "feat(api): expose semantic recall route over EidosDB"
```
Expected: PASS, green clippy.

---

### Task B7: full workspace gate + push + PR

**Files:** none (verification only)

- [ ] **Step 1: Full gate at CI scope**

```bash
cd "C:/Users/pierr/Documents/Developpements/Perso/joel/backend"
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo deny check
```
Expected: all green. The `sqlx::test` auth tests may fail locally without Postgres (pre-existing, environmental): confirm they pass under the CI Postgres service, do not fix here.

- [ ] **Step 2: Push the branch**

```bash
git push -u origin HEAD
```

- [ ] **Step 3: Open the PR**

Open a PR `feature/eidosdb-b7-2-knowledgestore -> main` on the Joel remote, titled `feat: EidosDB B7.2 KnowledgeStore`. Body: summary of the port, adapter, embedded crate, runner/api wiring, and the pinned EidosDB revision `EIDOSDB_REV` (note the matching EidosDB PR `feature/delete-by-filter`). Verify CI is green before requesting merge. Do NOT merge without explicit GO.

---

## Self-Review

**Spec coverage:**
- Sub-deliverable EidosDB `delete_by_filter` (spec §4) -> Tasks A1-A3. `derive(Clone)` (spec §2 dec.3) -> A4. Covered.
- Domain port (spec §5) -> B1. Covered.
- Adapter mapping, idempotent remember, filter mapping, payload reserved keys (spec §6) -> B3 + B4. Covered.
- `eidos-embedded` crate (spec §7) -> B2. Covered.
- Runner wiring + idempotent ensure_collections (spec §8.1) -> B4 (`ensure_collections`) + B5. Covered.
- API remote client + recall route (spec §8.2) -> B6. Covered.
- Config env vars (spec §8.3) -> B5 (`JOEL_EIDOS_DATA_DIR`/`ADDR`) + B6 (`JOEL_EIDOS_ENDPOINT`). Covered.
- Tests: StubEmbedder, in-process server, golden ignored (spec §9) -> B4/B5/B6 stubs; CandleEmbedder golden untouched (B7.1). `deny.toml [sources]` (spec §9) -> B2 step 1. Covered.

**Open verification points (flagged inline for the implementer, not gaps):**
- Exact re-export surface of `eidosdb-client` (whether `CollectionSpec`/`IndexTypeChoice`/`HnswConfig`/`SearchQuery`/`HybridQuery`/`Value`/`FieldValue`/`Filter` come from the client or their own crates): confirmed at B2/B4 by reading `eidosdb-client/src/lib.rs`.
- `tonic` version alignment between `eidos-embedded` and EidosDB: confirmed at B2 step 2.
- `Payload::get` accessor name and `Score` field visibility: confirmed at B3/B4.
- `HybridQuery` field set (no `Default` observed): explicit `rrf_k`/`overfetch_factor` provided in B4.

**Type consistency:** `MemoryId`, `Score`, `Corpus::collection`, `KnowledgeError`, reserved keys `KEY_*`, `EidosKnowledgeStore::{new, ensure_collections}`, `EmbeddedEidos::start`, `bootstrap` signatures are consistent across B1-B6.
