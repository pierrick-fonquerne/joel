# EidosDB B7.2 : KnowledgeStore de Joel sur EidosDB (design)

- Date : 2026-06-29
- Slug : eidosdb-b7-2-knowledgestore
- Statut : valide en brainstorming, en attente de revue de la spec ecrite
- Lot parent : B7 (`Nubster/_specs/2026-06-24-eidosdb-b7-joel-knowledgestore-design.md`)
- Predecesseur : B7.1 embedder (livre + merge, PR #6)
- Depots touches : `Perso/joel` (port + adapter + wiring) et `Nubster/eidosdb` (sous-livrable API, branche dediee)

## 1. Objet et reframing par le code reel

B7.2 cree la couche memoire de Joel : un port `KnowledgeStore` (domaine) et son adaptateur
`EidosKnowledgeStore` (gRPC) consommant EidosDB, avec un serveur EidosDB embarque. Objectif
produit : remplacer pgvector (baseline du bench differentiel B7.3).

L'exploration du code reel (29/06/2026) a invalide trois hypotheses de la spec B7 ; le present
design les corrige :

1. **Pas de crate umbrella `eidosdb`.** L'adaptateur depend des crates granulaires :
   `eidosdb-client`, `eidosdb-core`, `eidosdb-query`, `eidosdb-hnsw`, `eidosdb-lexical` ; le
   serveur embarque depend de `eidosdb-server`.
2. **Le client est `&mut self` partout** (`EidosClient`, wrapper tonic ; `connect`, pas `new`).
   Non partageable tel quel derriere un port `Send + Sync` a `&self`.
3. **`VectorId` a un champ prive et ne genere qu'un UUID v7 aleatoire** (`VectorId::new()`),
   sans constructeur `from(Uuid)`. Il n'existe ni `get` ni `delete-by-filter` ; `delete` prend
   un `VectorId`. `forget` par l'identite Joel etait donc impossible en l'etat.

Bonne surprise : le `Filter` AST d'EidosDB est complet (`Eq/Ne/Lt/Lte/Gt/Gte/In/Contains/
Exists/And/Or/Not`), donc le mapping `RecallFilter` (theme/since/until) est natif, sans
post-filtrage cote client.

## 2. Decisions verrouillees (brainstorming 28-29/06)

| # | Sujet | Decision |
|---|-------|----------|
| 1 | Reconciliation des identites + `forget` | Ajouter **`delete_by_filter`** a EidosDB (capacite de premier ordre : retention, RGPD, "oublie le theme X"). `forget` filtre sur un payload `memory_id`. Choisi contre un simple `VectorId::from(Uuid)`, plus etroit |
| 2 | Semantique de `remember` | **Idempotent** : `delete_by_filter(memory_id)` puis `upsert`. Pas de doublon a la re-ingestion, coherent avec le nom. Cout : 2 aller-retours par item (mitige en batch) |
| 3 | Detention du client gRPC | **Cloner le client par appel** (Channel tonic partage, HTTP/2 multiplexe). Idiomatique, non serialisant, fidele au bench. Exige `#[derive(Clone)]` sur `EidosClient` |
| 4 | Bootstrap serveur embarque | **Nouveau crate `eidos-embedded`** isolant `eidosdb-server` (le moteur) de l'adaptateur client. L'`api` ne tire pas le moteur |
| 5 | Topologie des binaires | **`runner` = proprietaire du serveur** (ouvre le data-dir redb, sert sur loopback) ; **`api` = client gRPC distant** vers ce port. Contrainte : redb verrouille le data-dir en mono-processus. Les deux bins fonctionnels des B7.2 |

Heritees de B7 et inchangees : un port / deux collections (`press`, `memory`) ; frontiere
embeddings via sous-port `Embedder` ; `candle` + `multilingual-e5-small` (384d, cosinus) ;
dependance **git** epinglee pendant B7.2 puis bascule crates.io en B7.0 ; oracle Flat pour le
recall ; serveur in-process pour contourner l'issue #14 (arret gracieux).

## 3. Architecture

```
Joel (Perso/joel)                                EidosDB (git, rev epinglee)
crates/domain/src/knowledge/                     eidosdb-client (EidosClient, gRPC)
  Embedder, EmbedError        (livre B7.1)       eidosdb-core  (VectorId, Embedding,
  KnowledgeStore, KnowledgeError                                Dimension, Metric, Score)
  Corpus, MemoryId, Score                        eidosdb-query (SearchQuery, HybridQuery,
  Remembrance, RecallHit, RecallFilter                          Payload, FieldValue, Value,
crates/knowledge/            (adaptateurs)                       Filter, SearchHit)
  CandleEmbedder              (livre B7.1)       eidosdb-hnsw  (HnswConfig)
  EidosKnowledgeStore         (B7.2)             eidosdb-lexical (Document)
crates/eidos-embedded/       (B7.2, nouveau)     eidosdb-server (Registry, EidosDbService) <- embedded
  EmbeddedEidos::start(data_dir, addr)
bins/runner : serveur embarque + ingestion (remember)
bins/api    : client distant + route recall
```

### 3.1 Placement hexagonal
- **Domaine** (`crates/domain`, module `knowledge`) : ports + types purs, zero dependance
  technique (ni `eidosdb-*`, ni `candle`). Un fichier par concept.
- **Adaptateur** (`crates/knowledge`) : depend de `eidosdb-client` + `-core/-query/-hnsw/
  -lexical`. Heberge `EidosKnowledgeStore` aux cotes de `CandleEmbedder`.
- **Serveur embarque** (`crates/eidos-embedded`, nouveau) : depend de `eidosdb-server`.
  Consomme par `runner` et les tests d'integration uniquement.

### 3.2 Topologie runtime (decision #5)
redb verrouille le data-dir en mono-processus : un seul binaire ouvre le serveur embarque.
Le `runner` (longue duree) est ce proprietaire : `EmbeddedEidos::start(data_dir, 127.0.0.1:PORT)`,
puis il sert et ingere. L'`api` se connecte a `http://127.0.0.1:PORT` comme client gRPC. Le
port loopback est une configuration partagee. Le label "in-process" ne vaut que pour le runner
et les tests ; pour l'api c'est un client local.

## 4. Sous-livrable EidosDB (repo `Nubster/eidosdb`, branche `feature/delete-by-filter`)

Deux ajouts, dont on epingle la rev dans Joel (dep git) :

1. **`delete_by_filter`**, traversant trois crates :
   - `eidosdb-proto` : message de requete (`collection`, `filter`) + reponse (`deleted: u64`) + RPC.
   - `eidosdb-server` : handler qui evalue le `Filter` AST sur les payloads et supprime les ids
     correspondants ; renvoie le compte.
   - `eidosdb-client` : `async fn delete_by_filter(&mut self, collection: &str, filter: Filter)
     -> Result<u64, ClientError>`.
2. **`#[derive(Clone)]` sur `EidosClient`** (Channel tonic clonable) pour le partage par clone.

Ces ajouts respectent l'ordre des dependances pour la future publication B7.0. `delete_by_filter`
n'est pas specifique a Joel : c'est une capacite generale d'EidosDB.

## 5. Port de domaine (`crates/domain/src/knowledge/`)

Squelette Planning (signatures ; un fichier par concept) :

```rust
pub enum Corpus { Press, AgentMemory }
impl Corpus { pub fn collection(self) -> &'static str; } // "press" / "memory"

pub struct MemoryId(Uuid);   // newtype ; genere v7 a la creation cote appelant
pub struct Score(f32);        // newtype ; jamais compare par ==

pub struct Remembrance {
    pub id: MemoryId,
    pub text: String,
    pub payload: serde_json::Value,   // titre, url, theme, source...
    pub occurred_at: OffsetDateTime,
}
pub struct RecallHit { pub id: MemoryId, pub score: Score, pub payload: serde_json::Value }
pub struct RecallFilter {
    pub theme: Option<String>,
    pub since: Option<OffsetDateTime>,
    pub until: Option<OffsetDateTime>,
}

#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    async fn remember(&self, corpus: Corpus, item: Remembrance) -> Result<(), KnowledgeError>;
    async fn remember_batch(&self, corpus: Corpus, items: Vec<Remembrance>) -> Result<(), KnowledgeError>;
    async fn recall(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError>;
    async fn recall_hybrid(&self, corpus: Corpus, query: &str, k: usize, filter: &RecallFilter)
        -> Result<Vec<RecallHit>, KnowledgeError>;
    async fn forget(&self, corpus: Corpus, id: MemoryId) -> Result<bool, KnowledgeError>;
}

pub enum KnowledgeError { Embed(EmbedError), Backend(String), NotFound, Config(String) }
```

Le domaine n'importe aucun type `eidosdb-*` : `RecallFilter`/`RecallHit` sont des types Joel ; la
traduction vers `Filter`/`SearchHit` se fait dans l'adaptateur.

## 6. Adaptateur `EidosKnowledgeStore` (`crates/knowledge`)

Detient un `EidosClient` clonable + `Arc<dyn Embedder>`. Chaque methode `client.clone()` puis
appelle (concurrence via le Channel partage).

### 6.1 Payload reserve
Aux cotes du `payload` metier, deux cles reservees, ecrites comme `eidosdb_query::Value` :
- `memory_id` : `Value::Text(id.to_string())` : cible de `delete_by_filter` (idempotence + forget).
- `occurred_at` : `Value::Integer(epoch_seconds)` : permet le filtre temporel `Gte/Lte`.

### 6.2 Mapping des operations
- `remember` (idempotent) : `embed_passage(text)` -> `delete_by_filter(coll, Eq("memory_id", id))`
  -> `upsert(coll, VectorId::new(), Embedding::new(vec)?, Some(Document(text)), Some(payload))`.
  Le `document = texte brut` alimente l'index lexical (BM25) requis par l'hybride.
- `remember_batch` : meme logique groupee ; `delete_by_filter(coll, In("memory_id", [ids]))`
  (le `Filter::In` evite N appels) puis `batch_upsert(coll, points)`.
- `recall` : `embed_query(q)` -> `search(coll, SearchQuery { embedding, k, metric: None, filter })`.
- `recall_hybrid` : `search_hybrid(coll, HybridQuery { vector: Some(embed_query(q)),
  text: Some(q.into()), k, filter, rrf_k: default, overfetch_factor: default, metric: None })`.
- `forget` : `delete_by_filter(coll, Eq("memory_id", id))` -> `Ok(deleted > 0)`.

### 6.3 `RecallFilter` -> `eidosdb_query::Filter`
Conjonction (`And`) des clauses presentes :
- `theme` -> `Filter::Eq("theme", Value::Text(..))`.
- `since` -> `Filter::Gte("occurred_at", Value::Integer(epoch))`.
- `until` -> `Filter::Lte("occurred_at", Value::Integer(epoch))`.
Aucun filtre -> `None`.

### 6.4 Collections, metrique, erreurs
- Chaque collection : `Metric::Cosine`, `IndexTypeChoice::Hnsw`, `HnswConfig::default()`
  (`m=16`, `ef_construction=200`, `ef_search=64`, seed fixe), `Dimension(embedder.dimension())`.
- Vecteurs deja **L2-normalises** par l'`Embedder` (acquis B7.1) : pas de double normalisation.
- `SearchHit.score` (`Score(f32)`, plus grand = plus proche) -> `domain::Score`.
- `ClientError` -> `KnowledgeError::Backend(..)` ; `EmbedError` -> `KnowledgeError::Embed(..)` ;
  dimension incoherente -> `Config(..)`.

## 7. Crate `eidos-embedded` (nouveau)

```rust
pub struct EmbeddedEidos;
impl EmbeddedEidos {
    /// Ouvre le data-dir (Registry::open) et monte le serveur tonic sur addr.
    /// Pour un port ephemere (tests), passer un addr sur le port 0.
    pub async fn start(data_dir: PathBuf, addr: SocketAddr)
        -> Result<(String /* endpoint http://host:port */, ShutdownHandle), EmbeddedError>;
}
pub struct ShutdownHandle(/* arret + JoinHandle */);
```

Implementation : `Registry::open(data_dir)?` -> `TcpListener::bind(addr)` -> `local_addr()` pour
recuperer le port effectif -> `Server::builder().add_service(EidosDbServer::new(
EidosDbService::new(registry))).serve_with_incoming(TcpListenerStream::new(listener))` dans une
tache tokio. Depend de `eidosdb-server`, `eidosdb-proto`, `tonic`, `tokio`, `tokio-stream`.

## 8. Wiring des binaires

### 8.1 `runner` (proprietaire)
1. `EmbeddedEidos::start(JOEL_EIDOS_DATA_DIR, JOEL_EIDOS_ADDR)`.
2. `ensure_collections` **idempotent** : pour `press` et `memory`, `describe_collection` ; si
   absente, `create_collection` (cosinus, HNSW). Ne recree jamais une collection existante.
3. Construit `CandleEmbedder::load()` + `EidosKnowledgeStore` (client vers le port local).
4. Le tick d'ingestion appelle `remember`/`remember_batch`. (Le pipeline RSS reel reste hors B7.)
5. Arret du runner = arret du serveur (contourne #14).

### 8.2 `api` (client distant)
- Ajoute `Arc<dyn KnowledgeStore>` a `AppState` (connecte a `JOEL_EIDOS_ENDPOINT`), assemble
  dans `AppState::build`.
- Expose une route `recall` (GET) renvoyant les `RecallHit`. N'embarque pas le serveur, ne cree
  pas les collections.

### 8.3 Configuration (`Config::from_env`)
- `runner` : `JOEL_EIDOS_DATA_DIR` (chemin redb), `JOEL_EIDOS_ADDR` (ex `127.0.0.1:50100`).
- `api` : `JOEL_EIDOS_ENDPOINT` (ex `http://127.0.0.1:50100`).

## 9. Tests (TDD strict)

- **Domaine** : tests purs : `Corpus::collection`, `RecallFilter -> Filter`, newtypes
  (`MemoryId`, `Score`), pas de comparaison de `Score` par `==`.
- **Adaptateur** : integration sur un **vrai serveur EidosDB in-process** (`eidos-embedded`,
  port 0 ephemere, `tempfile` data-dir) + un **`StubEmbedder` deterministe** (vecteur 384d
  derive par hash du texte, normalise L2) en dev-dependency, pour eviter tout telechargement de
  modele en CI. Couvre : remember -> recall, recall_hybrid, idempotence (remember x2 -> 1 hit),
  forget (-> recall vide), filtre theme + fenetre temporelle, `delete_by_filter` du sous-livrable.
- **`CandleEmbedder` reel** : golden `#[ignore]` (inchange, B7.1).
- `deny.toml` : ajouter une section `[sources]` autorisant la source git EidosDB
  (`allow-git` / `unknown-git`), sinon `cargo deny` refuse la dependance git. Toujours lancer
  `cargo deny check` en local avant push (lecon B7.1).

## 10. Hors perimetre B7.2

- Bench differentiel + `PgVectorKnowledgeStore` : B7.3.
- Publication crates.io + bascule git -> version : B7.0.
- Issue #13 (`ef_search` par requete) : a evaluer pendant B7.2 ; non bloquante (l'`ef_search`
  de collection suffit). Si le bench B7.3 la requiert, la fermer avant le gel 0.1.0.
- Pipeline presse live (ingestion RSS, scheduler) : le tick reel reste un squelette.
- UI de recherche frontend : l'`api` expose `recall`, la UI viendra hors B7.

## 11. Risques

- **Sous-livrable cross-repo** : `delete_by_filter` touche proto/server/client d'EidosDB. Le
  faire en TDD sur la branche dediee, epingler la rev, puis seulement consommer cote Joel.
- **Dep git, premiere du repo Joel** : ajuster `deny.toml` (`[sources]`) ; verifier que la CI a
  acces au depot EidosDB (public OSS).
- **uuid v4 (Joel) vs v7 (EidosDB)** : sans incidence ; `VectorId` est genere par EidosDB et
  l'identite Joel vit dans le payload `memory_id`. `MemoryId` peut adopter v7 par coherence.
- **Cout des 2 aller-retours de `remember`** : acceptable ; le batch amortit via
  `delete_by_filter` groupe.
- **Cloisonnement** : aucune specificite Joel ne remonte dans EidosDB ; `delete_by_filter` est
  une feature generale.
