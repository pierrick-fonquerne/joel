# Brique 0 « Socle » — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poser la fondation complète de Joel : monorepo compilable et testé en CI, authentification passkeys + TOTP, conteneurs durcis, NetBird self-hosted — jusqu'à ouvrir la PWA depuis l'iPhone en 4G et se connecter avec Face ID.

**Architecture:** Monolithe modulaire hexagonal — workspace Cargo (`domain` pur, `persistence` sqlx, binaires `api` Axum et `runner`) + front Angular PWA strict. PostgreSQL 17 unique état. Zone publique du VPS limitée à NetBird et SSH ; l'intranet n'écoute que sur le mesh.

**Tech Stack:** Rust edition 2024 (axum, sqlx, webauthn-rs, argon2, totp-rs, aes-gcm, tracing), Angular PWA (TypeScript strict, ESLint), PostgreSQL 17, Caddy (module dns ovh), Docker Compose, NetBird self-hosted, ntfy, CoreDNS.

**Spec de référence :** `docs/superpowers/specs/2026-06-06-joel-intranet-design.md`

**Conventions :**
- TDD systématique sur le code métier ; commits fréquents en anglais (conventional commits).
- Toute commande backend s'exécute depuis `backend/`, toute commande frontend depuis `frontend/`, sauf mention contraire.
- `$` = terminal local (PowerShell ou bash) ; `vps$` = session SSH sur le VPS.

---

## Partie A — Fondations du monorepo

### Task 0 : Établir le trunk et le repo distant

Le repo n'a qu'une branche `docs/initial-design` (design + ce plan). On établit `main` à partir d'elle (acte administratif de naissance du trunk — pas un commit direct), puis on crée le repo distant privé.

**Files:** aucun.

- [ ] **Step 0.1 : Établir main**

```bash
git checkout docs/initial-design
git branch main
git checkout main
```

- [ ] **Step 0.2 : Créer le repo distant privé et pousser**

```bash
gh repo create joel --private --source . --push
git push -u origin docs/initial-design
```

Expected: `main` et `docs/initial-design` visibles sur le remote.

- [ ] **Step 0.3 : Créer la branche de travail de la brique**

```bash
git checkout -b feature/brick-00-foundation
```

### Task 1 : Squelette du monorepo

**Files:**
- Create: `.gitignore`, `.editorconfig`, `README.md`

- [ ] **Step 1.1 : Écrire `.gitignore`**

```gitignore
# Rust
backend/target/

# Node / Angular
frontend/node_modules/
frontend/dist/
frontend/.angular/

# Environnements
.env
.env.*
!.env.example

# Outils locaux
.superpowers/
.remember/
```

- [ ] **Step 1.2 : Écrire `.editorconfig`**

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4

[*.{ts,html,scss,json,yml,yaml,toml,md}]
indent_size = 2
```

- [ ] **Step 1.3 : Écrire `README.md`**

```markdown
# Joel

Intranet personnel : agents, routines, planning, revue de presse.

- `backend/` — workspace Cargo (api, runner, domain, persistence)
- `frontend/` — Angular PWA
- `deploy/` — Docker Compose, Caddy, exploitation
- `docs/` — specs et plans

Voir `docs/superpowers/specs/2026-06-06-joel-intranet-design.md`.
```

- [ ] **Step 1.4 : Commit**

```bash
git add .gitignore .editorconfig README.md
git commit -m "chore: bootstrap monorepo skeleton"
```

### Task 2 : Workspace Cargo, crate domain, lints stricts

**Files:**
- Create: `backend/Cargo.toml`, `backend/rust-toolchain.toml`, `backend/deny.toml`, `backend/crates/domain/Cargo.toml`, `backend/crates/domain/src/lib.rs`

- [ ] **Step 2.1 : Écrire `backend/Cargo.toml` (workspace)**

```toml
[workspace]
resolver = "3"
members = ["crates/domain", "crates/persistence", "bins/api", "bins/runner"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "UNLICENSED"
publish = false

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
```

Note : `crates/persistence`, `bins/api`, `bins/runner` sont listés mais créés dans les tâches suivantes — le workspace ne compilera qu'à la fin de la Task 4. Pour compiler dès maintenant, ne garder que `crates/domain` dans `members` et ajouter les autres au fil des tâches.

- [ ] **Step 2.2 : Écrire `backend/rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

- [ ] **Step 2.3 : Écrire `backend/deny.toml`**

```toml
[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0", "Zlib", "MPL-2.0", "CDLA-Permissive-2.0"]

[advisories]
yanked = "deny"

[bans]
multiple-versions = "warn"
```

- [ ] **Step 2.4 : Créer le crate domain**

`backend/crates/domain/Cargo.toml` :

```toml
[package]
name = "domain"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
```

`backend/crates/domain/src/lib.rs` :

```rust
//! Joel core domain: business entities, ports and pure services.
//! This crate has zero infrastructure dependency by design.
```

- [ ] **Step 2.5 : Vérifier la compilation et le lint**

Run (depuis `backend/`, avec `members = ["crates/domain"]` temporairement) :

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: aucun warning, aucune erreur.

- [ ] **Step 2.6 : Commit**

```bash
git add backend/
git commit -m "feat(backend): cargo workspace with strict lints and empty domain crate"
```

### Task 3 : Binaire api — Axum, /api/healthz, tracing JSON (TDD)

**Files:**
- Create: `backend/bins/api/Cargo.toml`, `backend/bins/api/src/lib.rs`, `backend/bins/api/src/main.rs`, `backend/bins/api/tests/healthz.rs`

- [ ] **Step 3.1 : Créer le manifest du crate api**

`backend/bins/api/Cargo.toml` :

```toml
[package]
name = "api"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../../crates/domain" }
axum = "0.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

[dev-dependencies]
http-body-util = "0.1"
```

(Réactiver `bins/api` dans `members` du workspace.)

- [ ] **Step 3.2 : Écrire le test d'intégration AVANT l'implémentation**

`backend/bins/api/tests/healthz.rs` :

```rust
//! Integration test for the health endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok_with_version() {
    let app = api::build_router();

    let response = app
        .oneshot(Request::get("/api/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}
```

Note : `unwrap` est interdit par les lints en code de prod ; les tests l'autorisent via un override local. Ajouter en tête du fichier :

```rust
#![allow(clippy::unwrap_used)]
```

- [ ] **Step 3.3 : Vérifier que le test échoue**

```bash
cargo test -p api
```

Expected: échec de compilation — `api::build_router` n'existe pas.

- [ ] **Step 3.4 : Implémenter le router**

`backend/bins/api/src/lib.rs` :

```rust
//! Joel HTTP API: router assembly and HTTP adapters.

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

/// Health status payload returned by the health endpoint.
#[derive(Serialize)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

/// Builds the application router with all HTTP routes.
#[must_use]
pub fn build_router() -> Router {
    Router::new().route("/api/healthz", get(healthz))
}

async fn healthz() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
```

`backend/bins/api/src/main.rs` :

```rust
//! Joel API server entry point.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let bind = std::env::var("APP_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%bind, %error, "cannot bind listener");
            std::process::exit(1);
        }
    };
    tracing::info!(%bind, "api listening");

    let app = api::build_router().layer(tower_http::trace::TraceLayer::new_for_http());
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}
```

- [ ] **Step 3.5 : Vérifier que le test passe et que le lint est propre**

```bash
cargo test -p api
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: 1 test PASS, lint propre.

- [ ] **Step 3.6 : Commit**

```bash
git add backend/
git commit -m "feat(api): axum server with health endpoint and json tracing"
```

### Task 4 : Binaire runner — boucle tick, arrêt propre

**Files:**
- Create: `backend/bins/runner/Cargo.toml`, `backend/bins/runner/src/main.rs`

- [ ] **Step 4.1 : Créer le manifest**

`backend/bins/runner/Cargo.toml` :

```toml
[package]
name = "runner"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../../crates/domain" }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "time"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

(Réactiver `bins/runner` dans `members`.)

- [ ] **Step 4.2 : Implémenter la boucle**

`backend/bins/runner/src/main.rs` :

```rust
//! Joel runner: scheduler and job executor (skeleton for brick 0).

use std::time::Duration;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tracing::info!("runner started");
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                tracing::info!("runner tick: no scheduled work yet");
            }
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    tracing::error!(%error, "signal handler failure");
                }
                tracing::info!("runner shutting down");
                break;
            }
        }
    }
}
```

- [ ] **Step 4.3 : Vérification manuelle**

```bash
cargo run -p runner
```

Expected: log JSON `runner started` puis un tick immédiat ; `Ctrl+C` → `runner shutting down`, sortie code 0.

- [ ] **Step 4.4 : Commit**

```bash
git add backend/
git commit -m "feat(runner): tick loop skeleton with graceful shutdown"
```

### Task 5 : PostgreSQL dev, crate persistence, healthz avec ping DB (TDD)

**Files:**
- Create: `deploy/compose.dev.yml`, `.env.example`, `backend/migrations/0001_init.sql`, `backend/crates/persistence/Cargo.toml`, `backend/crates/persistence/src/lib.rs`
- Modify: `backend/bins/api/src/lib.rs`, `backend/bins/api/src/main.rs`, `backend/bins/api/tests/healthz.rs`, `backend/bins/api/Cargo.toml`

- [ ] **Step 5.1 : Écrire `deploy/compose.dev.yml`**

```yaml
services:
  postgres:
    image: postgres:17-alpine
    environment:
      POSTGRES_DB: joel
      POSTGRES_USER: joel
      POSTGRES_PASSWORD: joel-dev-only
    ports:
      - "127.0.0.1:5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U joel -d joel"]
      interval: 5s
      timeout: 3s
      retries: 10

volumes:
  pgdata:
```

- [ ] **Step 5.2 : Écrire `.env.example`**

```bash
DATABASE_URL=postgres://joel:joel-dev-only@localhost:5432/joel
APP_BIND=0.0.0.0:8080
RUST_LOG=info
```

Copier en `.env` local (non versionné) :

```bash
cp .env.example .env
```

- [ ] **Step 5.3 : Démarrer PostgreSQL et installer sqlx-cli**

```bash
docker compose -f deploy/compose.dev.yml up -d
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

- [ ] **Step 5.4 : Première migration**

`backend/migrations/0001_init.sql` :

```sql
-- Joel database baseline. Auth tables arrive with migration 0002.
CREATE TABLE schema_marker (
    id INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    installed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO schema_marker (id) VALUES (1);
```

Run (depuis `backend/`, `DATABASE_URL` exporté depuis `.env`) :

```bash
sqlx migrate run
```

Expected: `Applied 0001/migrate init`.

- [ ] **Step 5.5 : Créer le crate persistence avec son test**

`backend/crates/persistence/Cargo.toml` :

```toml
[package]
name = "persistence"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../domain" }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "time", "migrate"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

`backend/crates/persistence/src/lib.rs` — test d'abord (TDD), tout dans le même fichier pour cette tâche :

```rust
//! PostgreSQL adapters for the Joel domain ports.

use sqlx::PgPool;

/// Verifies database connectivity with a trivial round-trip query.
///
/// # Errors
/// Returns the underlying `sqlx` error when the database is unreachable.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::ping;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../../migrations")]
    async fn ping_succeeds_on_live_database(pool: PgPool) {
        ping(&pool).await.unwrap();
    }
}
```

- [ ] **Step 5.6 : Vérifier que le test passe**

```bash
cargo test -p persistence
```

Expected: PASS (sqlx crée une base éphémère et applique les migrations).

- [ ] **Step 5.7 : Brancher la DB dans /api/healthz — modifier le test d'abord**

`backend/bins/api/tests/healthz.rs` — remplacer le contenu :

```rust
//! Integration test for the health endpoint.
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "../../migrations")]
async fn healthz_reports_db_up(pool: PgPool) {
    let app = api::build_router(pool);

    let response = app
        .oneshot(Request::get("/api/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["db"], "up");
}
```

Ajouter aux deps de `backend/bins/api/Cargo.toml` :

```toml
persistence = { path = "../../crates/persistence" }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres"] }
```

- [ ] **Step 5.8 : Vérifier l'échec, puis implémenter**

`cargo test -p api` → échec de compilation (`build_router` ne prend pas de pool).

`backend/bins/api/src/lib.rs` — remplacer :

```rust
//! Joel HTTP API: router assembly and HTTP adapters.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;

/// Health status payload returned by the health endpoint.
#[derive(Serialize)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Database connectivity indicator.
    pub db: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

/// Builds the application router with all HTTP routes.
#[must_use]
pub fn build_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/healthz", get(healthz))
        .with_state(pool)
}

async fn healthz(State(pool): State<PgPool>) -> Json<Health> {
    let db = if persistence::ping(&pool).await.is_ok() { "up" } else { "down" };
    Json(Health {
        status: "ok",
        db,
        version: env!("CARGO_PKG_VERSION"),
    })
}
```

`backend/bins/api/src/main.rs` — remplacer le corps de `main` :

```rust
//! Joel API server entry point.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        tracing::error!("DATABASE_URL is not set");
        std::process::exit(1);
    };
    let pool = match sqlx::PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "cannot connect to database");
            std::process::exit(1);
        }
    };

    let bind = std::env::var("APP_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%bind, %error, "cannot bind listener");
            std::process::exit(1);
        }
    };
    tracing::info!(%bind, "api listening");

    let app = api::build_router(pool).layer(tower_http::trace::TraceLayer::new_for_http());
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}
```

- [ ] **Step 5.9 : Vérifier tests + lint, puis commit**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git add backend/ deploy/ .env.example
git commit -m "feat(persistence): postgres pool, migrations baseline, db-aware health endpoint"
```

### Task 6 : Frontend Angular — strict, ESLint, logger structuré, layout

**Files:**
- Create: `frontend/` (généré), `frontend/src/app/core/logging/logger.service.ts`, `frontend/src/app/core/logging/logger.service.spec.ts`, `frontend/src/app/core/layout/shell.component.ts`, `frontend/src/app/features/cockpit/cockpit.component.ts`, `frontend/proxy.conf.json`

- [ ] **Step 6.1 : Générer l'application (depuis la racine du repo)**

```bash
npx -y @angular/cli@latest new frontend --directory frontend --style=scss --ssr=false --skip-git
cd frontend
npx ng add angular-eslint --skip-confirmation
```

- [ ] **Step 6.2 : Interdire console.log via ESLint**

Dans `frontend/eslint.config.js`, ajouter à la section des règles TypeScript :

```javascript
rules: {
  "no-console": "error",
},
```

- [ ] **Step 6.3 : Test du logger structuré AVANT l'implémentation**

`frontend/src/app/core/logging/logger.service.spec.ts` :

```typescript
import { TestBed } from '@angular/core/testing';
import { LoggerService } from './logger.service';

describe('LoggerService', () => {
  let service: LoggerService;
  let emitted: unknown[];

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(LoggerService);
    emitted = [];
    service.sink = (entry) => emitted.push(entry);
  });

  it('emits a structured entry with level, message and context', () => {
    service.info('user.login', { method: 'passkey' });
    expect(emitted.length).toBe(1);
    const entry = emitted[0] as Record<string, unknown>;
    expect(entry['level']).toBe('info');
    expect(entry['message']).toBe('user.login');
    expect(entry['context']).toEqual({ method: 'passkey' });
    expect(typeof entry['timestamp']).toBe('string');
  });

  it('supports warn and error levels', () => {
    service.warn('job.retry', {});
    service.error('job.failed', { attempts: 3 });
    const levels = emitted.map((e) => (e as Record<string, unknown>)['level']);
    expect(levels).toEqual(['warn', 'error']);
  });
});
```

Run : `npm test -- --watch=false` → FAIL (`logger.service` introuvable).

- [ ] **Step 6.4 : Implémenter le logger**

`frontend/src/app/core/logging/logger.service.ts` :

```typescript
import { Injectable } from '@angular/core';

export type LogLevel = 'info' | 'warn' | 'error';

export interface LogEntry {
  level: LogLevel;
  message: string;
  context: Record<string, unknown>;
  timestamp: string;
}

/**
 * Structured JSON logger. Console transport lives here and only here:
 * application code must never call console directly.
 */
@Injectable({ providedIn: 'root' })
export class LoggerService {
  sink: (entry: LogEntry) => void = (entry) => {
    // eslint-disable-next-line no-console
    console[entry.level](JSON.stringify(entry));
  };

  info(message: string, context: Record<string, unknown> = {}): void {
    this.emit('info', message, context);
  }

  warn(message: string, context: Record<string, unknown> = {}): void {
    this.emit('warn', message, context);
  }

  error(message: string, context: Record<string, unknown> = {}): void {
    this.emit('error', message, context);
  }

  private emit(level: LogLevel, message: string, context: Record<string, unknown>): void {
    this.sink({ level, message, context, timestamp: new Date().toISOString() });
  }
}
```

Run : `npm test -- --watch=false` → PASS. Puis `npx ng lint` → propre.

- [ ] **Step 6.5 : Layout shell + cockpit (squelette navigable)**

`frontend/src/app/core/layout/shell.component.ts` :

```typescript
import { Component } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-shell',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  template: `
    <div class="shell">
      <aside class="shell__nav">
        <h1 class="shell__brand">Joel</h1>
        <nav>
          <a routerLink="/cockpit" routerLinkActive="active">Cockpit</a>
          <span class="shell__soon">Agents</span>
          <span class="shell__soon">Routines</span>
          <span class="shell__soon">Presse</span>
          <span class="shell__soon">Planning</span>
        </nav>
      </aside>
      <main class="shell__content"><router-outlet /></main>
    </div>
  `,
  styles: `
    .shell { display: flex; min-height: 100vh; }
    .shell__nav { width: 13rem; padding: 1rem; background: #101418; color: #e8eaed; }
    .shell__brand { font-size: 1.4rem; margin: 0 0 1.5rem; }
    .shell__nav nav { display: flex; flex-direction: column; gap: 0.75rem; }
    .shell__nav a { color: #e8eaed; text-decoration: none; }
    .shell__nav a.active { font-weight: 700; }
    .shell__soon { color: #5f6a72; cursor: default; }
    .shell__content { flex: 1; padding: 1.5rem; }
    @media (max-width: 700px) {
      .shell { flex-direction: column; }
      .shell__nav { width: auto; }
      .shell__nav nav { flex-direction: row; flex-wrap: wrap; }
    }
  `,
})
export class ShellComponent {}
```

`frontend/src/app/features/cockpit/cockpit.component.ts` :

```typescript
import { Component } from '@angular/core';

@Component({
  selector: 'app-cockpit',
  template: `
    <h2>Cockpit</h2>
    <p>Joel est opérationnel. Les premières tuiles arrivent avec les briques 1 à 4.</p>
  `,
})
export class CockpitComponent {}
```

`frontend/src/app/app.routes.ts` — remplacer :

```typescript
import { Routes } from '@angular/router';
import { ShellComponent } from './core/layout/shell.component';

export const routes: Routes = [
  {
    path: '',
    component: ShellComponent,
    children: [
      { path: '', pathMatch: 'full', redirectTo: 'cockpit' },
      {
        path: 'cockpit',
        loadComponent: () =>
          import('./features/cockpit/cockpit.component').then((m) => m.CockpitComponent),
      },
    ],
  },
];
```

`frontend/src/app/app.ts` (composant racine généré) — réduire le template à :

```typescript
template: `<router-outlet />`,
```

(et garder l'import `RouterOutlet`).

- [ ] **Step 6.6 : Proxy de dev vers l'api**

`frontend/proxy.conf.json` :

```json
{
  "/api": {
    "target": "http://localhost:8080",
    "secure": false
  }
}
```

Dans `frontend/angular.json`, section `serve.options`, ajouter :

```json
"proxyConfig": "proxy.conf.json"
```

- [ ] **Step 6.7 : Vérifier build, lint, tests puis commit**

```bash
npm run build
npx ng lint
npm test -- --watch=false
git add frontend/
git commit -m "feat(frontend): angular shell with strict lint, structured logger and cockpit skeleton"
```

### Task 7 : PWA installable

**Files:**
- Modify: `frontend/` (généré par le schématique), `frontend/public/manifest.webmanifest`

- [ ] **Step 7.1 : Ajouter le service worker PWA**

```bash
cd frontend
npx ng add @angular/pwa --skip-confirmation
```

- [ ] **Step 7.2 : Personnaliser le manifest**

`frontend/public/manifest.webmanifest` — remplacer les champs suivants (garder le bloc `icons` généré) :

```json
{
  "name": "Joel",
  "short_name": "Joel",
  "theme_color": "#101418",
  "background_color": "#101418",
  "display": "standalone",
  "scope": "./",
  "start_url": "./"
}
```

- [ ] **Step 7.3 : Vérifier le build et committer**

```bash
npm run build
git add frontend/
git commit -m "feat(frontend): installable pwa manifest and service worker"
```

### Task 8 : Contrat OpenAPI + client TypeScript généré

**Files:**
- Modify: `backend/bins/api/Cargo.toml`, `backend/bins/api/src/lib.rs`, `backend/bins/api/tests/healthz.rs`, `frontend/package.json`
- Create: `frontend/openapi-ts.config.ts`

- [ ] **Step 8.1 : Test d'abord — la spec OpenAPI est servie**

Ajouter à `backend/bins/api/tests/healthz.rs` :

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn openapi_spec_is_served(pool: PgPool) {
    let app = api::build_router(pool);

    let response = app
        .oneshot(Request::get("/api/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["info"]["title"], "Joel API");
    assert!(json["paths"]["/api/healthz"].is_object());
}
```

Run : `cargo test -p api` → FAIL (route absente).

- [ ] **Step 8.2 : Implémenter avec utoipa**

Ajouter aux dépendances de `backend/bins/api/Cargo.toml` :

```toml
utoipa = { version = "5", features = ["axum_extras"] }
```

`backend/bins/api/src/lib.rs` — remplacer :

```rust
//! Joel HTTP API: router assembly and HTTP adapters.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};

/// Health status payload returned by the health endpoint.
#[derive(Serialize, ToSchema)]
pub struct Health {
    /// Overall service status.
    pub status: &'static str,
    /// Database connectivity indicator.
    pub db: &'static str,
    /// Crate version, set at compile time.
    pub version: &'static str,
}

#[derive(OpenApi)]
#[openapi(info(title = "Joel API"), paths(healthz), components(schemas(Health)))]
struct ApiDoc;

/// Builds the application router with all HTTP routes.
#[must_use]
pub fn build_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/healthz", get(healthz))
        .route(
            "/api/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        .with_state(pool)
}

/// Reports liveness of the api process and its database connection.
#[utoipa::path(get, path = "/api/healthz", responses((status = 200, body = Health)))]
async fn healthz(State(pool): State<PgPool>) -> Json<Health> {
    let db = if persistence::ping(&pool).await.is_ok() { "up" } else { "down" };
    Json(Health {
        status: "ok",
        db,
        version: env!("CARGO_PKG_VERSION"),
    })
}
```

Run : `cargo test -p api` → 2 tests PASS.

- [ ] **Step 8.3 : Générer le client TypeScript**

```bash
cd frontend
npm install -D @hey-api/openapi-ts
```

`frontend/openapi-ts.config.ts` :

```typescript
import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: 'http://localhost:8080/api/openapi.json',
  output: 'src/app/core/api',
});
```

Ajouter aux scripts de `frontend/package.json` :

```json
"generate:api": "openapi-ts"
```

Lancer l'api (`cargo run -p api` dans `backend/`, avec PostgreSQL dev démarré) puis :

```bash
npm run generate:api
```

Expected: `frontend/src/app/core/api/` contient types et client générés (`Health` typé).

- [ ] **Step 8.4 : Brancher le cockpit sur /api/healthz**

`frontend/src/app/features/cockpit/cockpit.component.ts` — remplacer :

```typescript
import { Component, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { LoggerService } from '../../core/logging/logger.service';
import type { Health } from '../../core/api';

@Component({
  selector: 'app-cockpit',
  template: `
    <h2>Cockpit</h2>
    @if (health(); as h) {
      <p>Joel est opérationnel — api v{{ h.version }}, base {{ h.db }}.</p>
    } @else {
      <p>Connexion à l'api…</p>
    }
  `,
})
export class CockpitComponent {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);
  readonly health = signal<Health | null>(null);

  constructor() {
    this.http.get<Health>('/api/healthz').subscribe({
      next: (h) => this.health.set(h),
      error: () => this.logger.error('cockpit.health.unreachable', {}),
    });
  }
}
```

Ajouter `provideHttpClient()` aux providers dans `frontend/src/app/app.config.ts` :

```typescript
import { provideHttpClient } from '@angular/common/http';
// dans providers: [...existants, provideHttpClient()]
```

- [ ] **Step 8.5 : Vérification bout en bout dev + commit**

Avec PostgreSQL et `cargo run -p api` actifs :

```bash
npm start
```

Ouvrir `http://localhost:4200` → le Cockpit affiche « Joel est opérationnel — api v0.1.0, base up. »

```bash
npx ng lint && npm run build
git add frontend/ backend/
git commit -m "feat: openapi contract with generated typescript client wired to cockpit"
```

### Task 9 : CI complète

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 9.1 : Écrire le workflow**

```yaml
name: CI

on:
  push:
    branches: ["**"]
  pull_request:

jobs:
  backend:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: backend
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_DB: joel
          POSTGRES_USER: joel
          POSTGRES_PASSWORD: joel-ci
        ports: ["5432:5432"]
        options: >-
          --health-cmd "pg_isready -U joel -d joel"
          --health-interval 5s --health-timeout 3s --health-retries 10
    env:
      DATABASE_URL: postgres://joel:joel-ci@localhost:5432/joel
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          manifest-path: backend/Cargo.toml
      - run: cargo test --workspace

  frontend:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: frontend
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: frontend/package-lock.json
      - run: npm ci
      - run: npx ng lint
      - run: npm test -- --watch=false --browsers=ChromeHeadless
      - run: npm run build
```

- [ ] **Step 9.2 : Pousser et vérifier**

```bash
git add .github/
git commit -m "ci: backend and frontend pipelines"
git push -u origin feature/brick-00-foundation
```

Expected: les deux jobs verts sur le run de la branche.

---

## Partie B — Authentification (passkeys + secours mot de passe/TOTP)

Variables d'environnement nouvelles (ajouter à `.env.example`, puis à `.env`) :

```bash
MASTER_KEY=CHANGE_ME_openssl_rand_base64_32
WEBAUTHN_RP_ID=localhost
WEBAUTHN_ORIGIN=http://localhost:4200
SESSION_TTL_DAYS=30
```

Générer une vraie clé locale : `openssl rand -base64 32` (ou `node -e "console.log(require('crypto').randomBytes(32).toString('base64'))"`).

### Task 10 : Migration auth + entités et ports du domaine

**Files:**
- Create: `backend/migrations/0002_auth.sql`, `backend/crates/domain/src/auth/mod.rs`, `backend/crates/domain/src/auth/model.rs`, `backend/crates/domain/src/auth/ports.rs`
- Modify: `backend/crates/domain/Cargo.toml`, `backend/crates/domain/src/lib.rs`

- [ ] **Step 10.1 : Écrire la migration**

`backend/migrations/0002_auth.sql` :

```sql
CREATE TABLE app_user (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    totp_secret_enc BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE webauthn_credential (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_user(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    credential JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE TABLE session (
    token_hash BYTEA PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES app_user(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    user_agent TEXT
);
CREATE INDEX session_expires_at_idx ON session (expires_at);

CREATE TABLE audit_log (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id UUID REFERENCES app_user(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Run : `sqlx migrate run` → `Applied 0002/migrate auth`.

- [ ] **Step 10.2 : Dépendances du domaine**

`backend/crates/domain/Cargo.toml`, section `[dependencies]` :

```toml
argon2 = "0.5"
totp-rs = { version = "5", features = ["otpauth"] }
aes-gcm = "0.10"
rand = "0.9"
sha2 = "0.10"
uuid = { version = "1", features = ["v4", "serde"] }
time = { version = "0.3", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
async-trait = "0.1"
base64 = "0.22"
webauthn-rs = "0.5"
```

- [ ] **Step 10.3 : Modèle et erreurs**

`backend/crates/domain/src/auth/model.rs` :

```rust
//! Authentication entities and value objects.

use time::OffsetDateTime;
use uuid::Uuid;

/// A registered user account.
#[derive(Debug, Clone)]
pub struct User {
    /// Unique account identifier.
    pub id: Uuid,
    /// Login email, unique across the system.
    pub email: String,
    /// Name shown in the interface.
    pub display_name: String,
    /// Argon2id password hash (PHC string).
    pub password_hash: String,
    /// TOTP secret, encrypted at rest with the master key.
    pub totp_secret_enc: Vec<u8>,
}

/// A server-side session, identified by the hash of its bearer token.
#[derive(Debug, Clone)]
pub struct Session {
    /// SHA-256 hash of the opaque bearer token.
    pub token_hash: [u8; 32],
    /// Owning user.
    pub user_id: Uuid,
    /// Expiration instant; the session is invalid afterwards.
    pub expires_at: OffsetDateTime,
}

/// A session freshly issued to a client, carrying the only copy of the raw token.
#[derive(Debug)]
pub struct IssuedSession {
    /// Opaque bearer token to set as a cookie. Never stored server-side.
    pub token: String,
    /// Expiration instant.
    pub expires_at: OffsetDateTime,
}

/// Authentication failures. Credential-related causes are deliberately merged.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Wrong email, password or TOTP code — indistinguishable on purpose.
    #[error("invalid credentials")]
    InvalidCredentials,
    /// Session is absent or expired.
    #[error("not authenticated")]
    NotAuthenticated,
    /// Cryptographic material is malformed (master key, sealed payload).
    #[error("crypto failure")]
    Crypto,
    /// Storage adapter failure.
    #[error("storage failure: {0}")]
    Storage(String),
}
```

- [ ] **Step 10.4 : Ports**

`backend/crates/domain/src/auth/ports.rs` :

```rust
//! Outbound ports required by authentication use cases.

use async_trait::async_trait;
use uuid::Uuid;

use super::model::{AuthError, Session, User};

/// Read/write access to user accounts.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Finds a user by email; `Ok(None)` when unknown.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;
    /// Finds a user by id; `Ok(None)` when unknown.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError>;
    /// Persists a new user account.
    async fn insert(&self, user: &User) -> Result<(), AuthError>;
}

/// Storage of server-side sessions.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Persists a session.
    async fn insert(&self, session: &Session) -> Result<(), AuthError>;
    /// Returns the session matching the token hash, when present.
    async fn find(&self, token_hash: [u8; 32]) -> Result<Option<Session>, AuthError>;
    /// Deletes a session; deleting an absent session is not an error.
    async fn delete(&self, token_hash: [u8; 32]) -> Result<(), AuthError>;
}

/// Storage of WebAuthn credentials, serialized as JSON.
#[async_trait]
pub trait CredentialRepository: Send + Sync {
    /// Persists a passkey for a user under a human-readable label.
    async fn insert(&self, user_id: Uuid, label: &str, credential_json: &str) -> Result<(), AuthError>;
    /// Returns all passkeys of a user as raw JSON documents.
    async fn for_user(&self, user_id: Uuid) -> Result<Vec<String>, AuthError>;
}

/// Sink for security-relevant events.
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Records an action, optionally tied to a user.
    async fn record(&self, user_id: Option<Uuid>, action: &str, detail: serde_json::Value);
}
```

- [ ] **Step 10.5 : Câbler les modules**

`backend/crates/domain/src/auth/mod.rs` :

```rust
//! Authentication: entities, ports, pure services and use cases.

pub mod model;
pub mod ports;
```

`backend/crates/domain/src/lib.rs` — ajouter sous la doc de crate :

```rust
pub mod auth;
```

- [ ] **Step 10.6 : Vérifier compilation + lint, commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(domain): auth entities, errors and outbound ports with auth schema migration"
```

### Task 11 : Services cryptographiques purs (TDD)

**Files:**
- Create: `backend/crates/domain/src/auth/crypto.rs`
- Modify: `backend/crates/domain/src/auth/mod.rs`

- [ ] **Step 11.1 : Écrire les tests AVANT (dans le futur module)**

`backend/crates/domain/src/auth/crypto.rs` — commencer par le squelette de module avec UNIQUEMENT les tests :

```rust
//! Pure cryptographic services: password hashing, TOTP, secret sealing, session tokens.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn password_roundtrip_verifies() {
        let hash = PasswordService::hash("correct horse battery staple").unwrap();
        assert!(PasswordService::verify("correct horse battery staple", &hash));
        assert!(!PasswordService::verify("wrong", &hash));
    }

    #[test]
    fn totp_accepts_current_code_and_rejects_garbage() {
        let secret = TotpService::generate_secret();
        let now = 1_780_000_000;
        let code = TotpService::current_code(&secret, now).unwrap();
        assert!(TotpService::verify(&secret, &code, now));
        assert!(!TotpService::verify(&secret, "000000", now));
    }

    #[test]
    fn otpauth_url_contains_issuer_and_account() {
        let secret = TotpService::generate_secret();
        let url = TotpService::otpauth_url(&secret, "pierrick@example.com").unwrap();
        assert!(url.starts_with("otpauth://totp/"));
        assert!(url.contains("Joel"));
    }

    #[test]
    fn secret_box_roundtrip_and_tamper_detection() {
        let key = [42_u8; 32];
        let sealed = SecretBox::new(key).seal(b"sensitive");
        assert_eq!(SecretBox::new(key).open(&sealed).unwrap(), b"sensitive");
        let mut tampered = sealed;
        let last = tampered.last_mut().unwrap();
        *last = last.wrapping_add(1);
        assert!(SecretBox::new(key).open(&tampered).is_err());
    }

    #[test]
    fn session_token_hash_is_stable_and_token_is_long() {
        let (token, hash) = SessionToken::generate();
        assert!(token.len() >= 43);
        assert_eq!(SessionToken::hash(&token), hash);
    }
}
```

Run : `cargo test -p domain` → FAIL (types non définis). Ajouter `pub mod crypto;` à `auth/mod.rs`.

- [ ] **Step 11.2 : Implémenter au-dessus des tests, dans le même fichier**

```rust
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::password_hash::rand_core::OsRng as ArgonRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};

use super::model::AuthError;

/// Argon2id password hashing.
pub struct PasswordService;

impl PasswordService {
    /// Hashes a password into a PHC string.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when hashing fails.
    pub fn hash(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut ArgonRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| AuthError::Crypto)
    }

    /// Verifies a password against a stored PHC string.
    #[must_use]
    pub fn verify(password: &str, stored: &str) -> bool {
        PasswordHash::new(stored)
            .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
            .unwrap_or(false)
    }
}

/// RFC 6238 TOTP, 6 digits, 30 second step, SHA-1 (authenticator app default).
pub struct TotpService;

impl TotpService {
    /// Generates a 20-byte random secret.
    #[must_use]
    pub fn generate_secret() -> Vec<u8> {
        let mut secret = vec![0_u8; 20];
        rand::rng().fill_bytes(&mut secret);
        secret
    }

    fn totp(secret: &[u8], account: &str) -> Result<TOTP, AuthError> {
        TOTP::new(Algorithm::SHA1, 6, 1, 30, secret.to_vec(), Some("Joel".to_owned()), account.to_owned())
            .map_err(|_| AuthError::Crypto)
    }

    /// Computes the code for the given unix timestamp (test and seeding helper).
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] on malformed secrets.
    pub fn current_code(secret: &[u8], now_unix: u64) -> Result<String, AuthError> {
        Self::totp(secret, "joel").map(|t| t.generate(now_unix))
    }

    /// Verifies a user-provided code against the secret at the given instant.
    #[must_use]
    pub fn verify(secret: &[u8], code: &str, now_unix: u64) -> bool {
        Self::totp(secret, "joel").is_ok_and(|t| t.check(code, now_unix))
    }

    /// Builds the otpauth:// provisioning URL for authenticator apps.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] on malformed secrets.
    pub fn otpauth_url(secret: &[u8], account: &str) -> Result<String, AuthError> {
        Self::totp(secret, account).map(|t| t.get_url())
    }

    /// Encodes a secret in base32 for manual entry in authenticator apps.
    #[must_use]
    pub fn secret_base32(secret: &[u8]) -> String {
        Secret::Raw(secret.to_vec()).to_encoded().to_string()
    }
}

/// AES-256-GCM sealing of secrets at rest. Layout: `nonce (12) || ciphertext`.
pub struct SecretBox {
    key: [u8; 32],
}

impl SecretBox {
    /// Builds a box from a raw 32-byte key.
    #[must_use]
    pub const fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Builds a box from a base64-encoded 32-byte key (`MASTER_KEY`).
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when the key is not 32 bytes of valid base64.
    pub fn from_base64(key_b64: &str) -> Result<Self, AuthError> {
        let bytes = STANDARD.decode(key_b64.trim()).map_err(|_| AuthError::Crypto)?;
        let key: [u8; 32] = bytes.try_into().map_err(|_| AuthError::Crypto)?;
        Ok(Self::new(key))
    }

    /// Encrypts and authenticates a payload.
    #[must_use]
    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut out = nonce.to_vec();
        // Encryption with a fresh nonce and in-memory buffers cannot fail.
        if let Ok(ciphertext) = cipher.encrypt(&nonce, plaintext) {
            out.extend_from_slice(&ciphertext);
        }
        out
    }

    /// Decrypts a sealed payload, failing on any tampering.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when the payload is malformed or forged.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, AuthError> {
        if sealed.len() < 13 {
            return Err(AuthError::Crypto);
        }
        let (nonce, ciphertext) = sealed.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        cipher.decrypt(Nonce::from_slice(nonce), ciphertext).map_err(|_| AuthError::Crypto)
    }
}

/// Opaque session bearer tokens: 32 random bytes, stored as SHA-256 hashes only.
pub struct SessionToken;

impl SessionToken {
    /// Generates a fresh token and its storage hash.
    #[must_use]
    pub fn generate() -> (String, [u8; 32]) {
        let mut raw = [0_u8; 32];
        rand::rng().fill_bytes(&mut raw);
        let token = URL_SAFE_NO_PAD.encode(raw);
        let hash = Self::hash(&token);
        (token, hash)
    }

    /// Hashes a presented token for storage lookup.
    #[must_use]
    pub fn hash(token: &str) -> [u8; 32] {
        Sha256::digest(token.as_bytes()).into()
    }
}
```

- [ ] **Step 11.3 : Vérifier tests + lint, commit**

```bash
cargo test -p domain
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(domain): argon2, totp, aes-gcm secret box and session token services"
```

### Task 12 : Use cases d'authentification (TDD avec fakes)

**Files:**
- Create: `backend/crates/domain/src/auth/use_cases.rs`, `backend/crates/domain/src/auth/test_support.rs`
- Modify: `backend/crates/domain/src/auth/mod.rs`

- [ ] **Step 12.1 : Fakes in-memory réutilisables**

`backend/crates/domain/src/auth/test_support.rs` :

```rust
//! In-memory fakes for authentication ports, shared by domain and api tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use super::model::{AuthError, Session, User};
use super::ports::{AuditSink, SessionRepository, UserRepository};

/// In-memory [`UserRepository`].
#[derive(Default)]
pub struct FakeUsers {
    /// Stored users, keyed by id.
    pub users: Mutex<HashMap<Uuid, User>>,
}

#[async_trait]
impl UserRepository for FakeUsers {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        Ok(self.users.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.values().find(|u| u.email == email).cloned())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError> {
        Ok(self.users.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.get(&id).cloned())
    }
    async fn insert(&self, user: &User) -> Result<(), AuthError> {
        self.users.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.insert(user.id, user.clone());
        Ok(())
    }
}

/// In-memory [`SessionRepository`].
#[derive(Default)]
pub struct FakeSessions {
    /// Stored sessions, keyed by token hash.
    pub sessions: Mutex<HashMap<[u8; 32], Session>>,
}

#[async_trait]
impl SessionRepository for FakeSessions {
    async fn insert(&self, session: &Session) -> Result<(), AuthError> {
        self.sessions.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.insert(session.token_hash, session.clone());
        Ok(())
    }
    async fn find(&self, token_hash: [u8; 32]) -> Result<Option<Session>, AuthError> {
        Ok(self.sessions.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.get(&token_hash).cloned())
    }
    async fn delete(&self, token_hash: [u8; 32]) -> Result<(), AuthError> {
        self.sessions.lock().map_err(|_| AuthError::Storage("poisoned".into()))?.remove(&token_hash);
        Ok(())
    }
}

/// Audit sink that swallows everything.
#[derive(Default)]
pub struct NoopAudit;

#[async_trait]
impl AuditSink for NoopAudit {
    async fn record(&self, _user_id: Option<Uuid>, _action: &str, _detail: serde_json::Value) {}
}
```

Exposer dans `auth/mod.rs` :

```rust
pub mod crypto;
pub mod test_support;
pub mod use_cases;
```

- [ ] **Step 12.2 : Tests des use cases AVANT l'implémentation**

`backend/crates/domain/src/auth/use_cases.rs` — d'abord uniquement le bloc de tests :

```rust
//! Authentication use cases, infrastructure-agnostic.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;
    use crate::auth::crypto::{PasswordService, SecretBox, TotpService};
    use crate::auth::model::User;
    use crate::auth::ports::UserRepository;
    use crate::auth::test_support::{FakeSessions, FakeUsers, NoopAudit};

    const KEY: [u8; 32] = [7; 32];

    async fn seeded() -> (Auth, Vec<u8>) {
        let users = Arc::new(FakeUsers::default());
        let secret = TotpService::generate_secret();
        users
            .insert(&User {
                id: Uuid::new_v4(),
                email: "pierrick@example.com".into(),
                display_name: "Pierrick".into(),
                password_hash: PasswordService::hash("hunter2hunter2").unwrap(),
                totp_secret_enc: SecretBox::new(KEY).seal(&secret),
            })
            .await
            .unwrap();
        let auth = Auth::new(users, Arc::new(FakeSessions::default()), Arc::new(NoopAudit), SecretBox::new(KEY), 30);
        (auth, secret)
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap()
    }

    #[tokio::test]
    async fn password_login_issues_a_validatable_session() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();

        let issued = auth.password_login("pierrick@example.com", "hunter2hunter2", &code, now()).await.unwrap();
        let user = auth.validate_session(&issued.token, now()).await.unwrap();
        assert_eq!(user.email, "pierrick@example.com");
    }

    #[tokio::test]
    async fn wrong_password_and_wrong_totp_both_yield_invalid_credentials() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();

        let wrong_pwd = auth.password_login("pierrick@example.com", "nope", &code, now()).await;
        let wrong_totp = auth.password_login("pierrick@example.com", "hunter2hunter2", "000000", now()).await;
        assert!(matches!(wrong_pwd, Err(AuthError::InvalidCredentials)));
        assert!(matches!(wrong_totp, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn unknown_email_yields_invalid_credentials_not_a_distinct_error() {
        let (auth, _) = seeded().await;
        let result = auth.password_login("ghost@example.com", "x", "000000", now()).await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn expired_session_is_rejected() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();
        let issued = auth.password_login("pierrick@example.com", "hunter2hunter2", &code, now()).await.unwrap();

        let later = now() + time::Duration::days(31);
        assert!(matches!(auth.validate_session(&issued.token, later).await, Err(AuthError::NotAuthenticated)));
    }

    #[tokio::test]
    async fn logout_invalidates_the_session() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();
        let issued = auth.password_login("pierrick@example.com", "hunter2hunter2", &code, now()).await.unwrap();

        auth.logout(&issued.token).await.unwrap();
        assert!(matches!(auth.validate_session(&issued.token, now()).await, Err(AuthError::NotAuthenticated)));
    }
}
```

Run : `cargo test -p domain` → FAIL (`Auth` non défini).

- [ ] **Step 12.3 : Implémenter `Auth` au-dessus des tests**

```rust
use std::sync::Arc;

use time::{Duration, OffsetDateTime};

use super::crypto::{PasswordService, SecretBox, SessionToken, TotpService};
use super::model::{AuthError, IssuedSession, Session, User};
use super::ports::{AuditSink, SessionRepository, UserRepository};

/// Authentication service: groups the password/TOTP and session use cases.
pub struct Auth {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionRepository>,
    audit: Arc<dyn AuditSink>,
    secret_box: SecretBox,
    session_ttl_days: i64,
}

impl Auth {
    /// Assembles the service from its ports and crypto material.
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn SessionRepository>,
        audit: Arc<dyn AuditSink>,
        secret_box: SecretBox,
        session_ttl_days: i64,
    ) -> Self {
        Self { users, sessions, audit, secret_box, session_ttl_days }
    }

    /// Authenticates with email + password + TOTP and issues a session.
    ///
    /// # Errors
    /// [`AuthError::InvalidCredentials`] on any mismatch, without detail.
    pub async fn password_login(
        &self,
        email: &str,
        password: &str,
        totp_code: &str,
        now: OffsetDateTime,
    ) -> Result<IssuedSession, AuthError> {
        let Some(user) = self.users.find_by_email(email).await? else {
            self.audit.record(None, "auth.login.unknown_email", serde_json::json!({ "email": email })).await;
            return Err(AuthError::InvalidCredentials);
        };
        if !PasswordService::verify(password, &user.password_hash) {
            self.audit.record(Some(user.id), "auth.login.bad_password", serde_json::json!({})).await;
            return Err(AuthError::InvalidCredentials);
        }
        let secret = self.secret_box.open(&user.totp_secret_enc)?;
        let now_unix = u64::try_from(now.unix_timestamp()).map_err(|_| AuthError::Crypto)?;
        if !TotpService::verify(&secret, totp_code, now_unix) {
            self.audit.record(Some(user.id), "auth.login.bad_totp", serde_json::json!({})).await;
            return Err(AuthError::InvalidCredentials);
        }
        let issued = self.issue_session(user.id, now).await?;
        self.audit.record(Some(user.id), "auth.login.password", serde_json::json!({})).await;
        Ok(issued)
    }

    /// Issues a session for an already-authenticated user (passkey flow).
    ///
    /// # Errors
    /// Propagates storage failures.
    pub async fn issue_session(&self, user_id: uuid::Uuid, now: OffsetDateTime) -> Result<IssuedSession, AuthError> {
        let (token, token_hash) = SessionToken::generate();
        let expires_at = now + Duration::days(self.session_ttl_days);
        self.sessions.insert(&Session { token_hash, user_id, expires_at }).await?;
        Ok(IssuedSession { token, expires_at })
    }

    /// Resolves a bearer token into its user, enforcing expiry.
    ///
    /// # Errors
    /// [`AuthError::NotAuthenticated`] when absent or expired.
    pub async fn validate_session(&self, token: &str, now: OffsetDateTime) -> Result<User, AuthError> {
        let hash = SessionToken::hash(token);
        let Some(session) = self.sessions.find(hash).await? else {
            return Err(AuthError::NotAuthenticated);
        };
        if session.expires_at <= now {
            self.sessions.delete(hash).await?;
            return Err(AuthError::NotAuthenticated);
        }
        self.users.find_by_id(session.user_id).await?.ok_or(AuthError::NotAuthenticated)
    }

    /// Destroys the session associated with the token, when present.
    ///
    /// # Errors
    /// Propagates storage failures.
    pub async fn logout(&self, token: &str) -> Result<(), AuthError> {
        self.sessions.delete(SessionToken::hash(token)).await
    }
}
```

- [ ] **Step 12.4 : Vérifier tests + lint, commit**

```bash
cargo test -p domain
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(domain): password and session use cases with audit trail"
```

Note : `test_support` est compilé en build normal (pas seulement cfg(test)) pour être réutilisable par les tests d'`api` ; c'est volontaire.

### Task 13 : Adapters PostgreSQL des ports auth (sqlx::test)

**Files:**
- Create: `backend/crates/persistence/src/auth.rs`
- Modify: `backend/crates/persistence/src/lib.rs`, `backend/crates/persistence/Cargo.toml`

- [ ] **Step 13.1 : Dépendances**

Ajouter à `backend/crates/persistence/Cargo.toml` :

```toml
async-trait = "0.1"
serde_json = "1"
time = "0.3"
uuid = { version = "1", features = ["v4"] }
```

(et `"json"` à la liste de features de sqlx).

- [ ] **Step 13.2 : Tests d'intégration AVANT**

`backend/crates/persistence/src/auth.rs` — bloc de tests d'abord :

```rust
//! PostgreSQL adapters for the authentication ports.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::auth::model::{Session, User};
    use domain::auth::ports::{AuditSink, CredentialRepository, SessionRepository, UserRepository};
    use sqlx::PgPool;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn sample_user() -> User {
        User {
            id: Uuid::new_v4(),
            email: format!("u-{}@example.com", Uuid::new_v4()),
            display_name: "Test".into(),
            password_hash: "$argon2id$stub".into(),
            totp_secret_enc: vec![1, 2, 3],
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn user_roundtrip_by_email_and_id(pool: PgPool) {
        let repo = PgUsers::new(pool);
        let user = sample_user();
        repo.insert(&user).await.unwrap();
        assert_eq!(repo.find_by_email(&user.email).await.unwrap().unwrap().id, user.id);
        assert_eq!(repo.find_by_id(user.id).await.unwrap().unwrap().email, user.email);
        assert!(repo.find_by_email("absent@example.com").await.unwrap().is_none());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn session_roundtrip_and_delete(pool: PgPool) {
        let users = PgUsers::new(pool.clone());
        let user = sample_user();
        users.insert(&user).await.unwrap();

        let repo = PgSessions::new(pool);
        let session = Session {
            token_hash: [9; 32],
            user_id: user.id,
            expires_at: OffsetDateTime::from_unix_timestamp(1_900_000_000).unwrap(),
        };
        repo.insert(&session).await.unwrap();
        assert_eq!(repo.find([9; 32]).await.unwrap().unwrap().user_id, user.id);
        repo.delete([9; 32]).await.unwrap();
        assert!(repo.find([9; 32]).await.unwrap().is_none());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn credentials_are_stored_per_user(pool: PgPool) {
        let users = PgUsers::new(pool.clone());
        let user = sample_user();
        users.insert(&user).await.unwrap();

        let repo = PgCredentials::new(pool);
        repo.insert(user.id, "iPhone", r#"{"cred":"a"}"#).await.unwrap();
        repo.insert(user.id, "PC", r#"{"cred":"b"}"#).await.unwrap();
        assert_eq!(repo.for_user(user.id).await.unwrap().len(), 2);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn audit_records_do_not_fail(pool: PgPool) {
        let sink = PgAudit::new(pool.clone());
        sink.record(None, "test.event", serde_json::json!({ "k": 1 })).await;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1);
    }
}
```

Run : `cargo test -p persistence` → FAIL (types non définis).

- [ ] **Step 13.3 : Implémenter les adapters au-dessus des tests**

```rust
use async_trait::async_trait;
use domain::auth::model::{AuthError, Session, User};
use domain::auth::ports::{AuditSink, CredentialRepository, SessionRepository, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

fn storage(err: sqlx::Error) -> AuthError {
    AuthError::Storage(err.to_string())
}

/// PostgreSQL implementation of [`UserRepository`].
pub struct PgUsers {
    pool: PgPool,
}

impl PgUsers {
    /// Builds the adapter over a connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUsers {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        sqlx::query_as::<_, (Uuid, String, String, String, Vec<u8>)>(
            "SELECT id, email, display_name, password_hash, totp_secret_enc FROM app_user WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|(id, email, display_name, password_hash, totp_secret_enc)| User { id, email, display_name, password_hash, totp_secret_enc }))
        .map_err(storage)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError> {
        sqlx::query_as::<_, (Uuid, String, String, String, Vec<u8>)>(
            "SELECT id, email, display_name, password_hash, totp_secret_enc FROM app_user WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|(id, email, display_name, password_hash, totp_secret_enc)| User { id, email, display_name, password_hash, totp_secret_enc }))
        .map_err(storage)
    }

    async fn insert(&self, user: &User) -> Result<(), AuthError> {
        sqlx::query("INSERT INTO app_user (id, email, display_name, password_hash, totp_secret_enc) VALUES ($1, $2, $3, $4, $5)")
            .bind(user.id)
            .bind(&user.email)
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(&user.totp_secret_enc)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(storage)
    }
}

/// PostgreSQL implementation of [`SessionRepository`].
pub struct PgSessions {
    pool: PgPool,
}

impl PgSessions {
    /// Builds the adapter over a connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PgSessions {
    async fn insert(&self, session: &Session) -> Result<(), AuthError> {
        sqlx::query("INSERT INTO session (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(session.token_hash.as_slice())
            .bind(session.user_id)
            .bind(session.expires_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(storage)
    }

    async fn find(&self, token_hash: [u8; 32]) -> Result<Option<Session>, AuthError> {
        sqlx::query_as::<_, (Vec<u8>, Uuid, time::OffsetDateTime)>(
            "SELECT token_hash, user_id, expires_at FROM session WHERE token_hash = $1",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(|(hash, user_id, expires_at)| {
            let token_hash: [u8; 32] = hash.try_into().map_err(|_| AuthError::Storage("corrupt token hash".into()))?;
            Ok(Session { token_hash, user_id, expires_at })
        })
        .transpose()
    }

    async fn delete(&self, token_hash: [u8; 32]) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM session WHERE token_hash = $1")
            .bind(token_hash.as_slice())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(storage)
    }
}

/// PostgreSQL implementation of [`CredentialRepository`].
pub struct PgCredentials {
    pool: PgPool,
}

impl PgCredentials {
    /// Builds the adapter over a connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CredentialRepository for PgCredentials {
    async fn insert(&self, user_id: Uuid, label: &str, credential_json: &str) -> Result<(), AuthError> {
        let value: serde_json::Value = serde_json::from_str(credential_json).map_err(|e| AuthError::Storage(e.to_string()))?;
        sqlx::query("INSERT INTO webauthn_credential (user_id, label, credential) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(label)
            .bind(value)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(storage)
    }

    async fn for_user(&self, user_id: Uuid) -> Result<Vec<String>, AuthError> {
        sqlx::query_scalar::<_, serde_json::Value>("SELECT credential FROM webauthn_credential WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.into_iter().map(|v| v.to_string()).collect())
            .map_err(storage)
    }
}

/// PostgreSQL implementation of [`AuditSink`]. Failures are logged-and-dropped:
/// auditing must never break the authentication path.
pub struct PgAudit {
    pool: PgPool,
}

impl PgAudit {
    /// Builds the adapter over a connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditSink for PgAudit {
    async fn record(&self, user_id: Option<Uuid>, action: &str, detail: serde_json::Value) {
        let _unused = sqlx::query("INSERT INTO audit_log (user_id, action, detail) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(action)
            .bind(detail)
            .execute(&self.pool)
            .await;
    }
}
```

Ajouter à `backend/crates/persistence/src/lib.rs` :

```rust
pub mod auth;
```

- [ ] **Step 13.4 : Vérifier tests + lint, commit**

```bash
cargo test -p persistence
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(persistence): postgres adapters for users, sessions, credentials and audit"
```

### Task 14 : Routes password login + middleware session (TDD)

**Files:**
- Create: `backend/bins/api/src/auth_routes.rs`, `backend/bins/api/src/state.rs`, `backend/bins/api/tests/auth_password.rs`
- Modify: `backend/bins/api/src/lib.rs`, `backend/bins/api/src/main.rs`, `backend/bins/api/Cargo.toml`

- [ ] **Step 14.1 : Dépendances api**

Ajouter à `backend/bins/api/Cargo.toml` :

```toml
axum-extra = { version = "0.10", features = ["cookie"] }
time = "0.3"
uuid = { version = "1", features = ["v4", "serde"] }
webauthn-rs = "0.5"
url = "2"
```

et aux `[dev-dependencies]` :

```toml
domain = { path = "../../crates/domain" }
```

- [ ] **Step 14.2 : Test d'intégration AVANT**

`backend/bins/api/tests/auth_password.rs` :

```rust
//! Integration tests for password + TOTP login and the session middleware.
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
use domain::auth::model::User;
use domain::auth::ports::UserRepository;
use http_body_util::BodyExt;
use persistence::auth::PgUsers;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const KEY_B64: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

pub async fn seed(pool: &PgPool) -> Vec<u8> {
    let secret = TotpService::generate_secret();
    let secret_box = SecretBox::from_base64(KEY_B64).unwrap();
    PgUsers::new(pool.clone())
        .insert(&User {
            id: Uuid::new_v4(),
            email: "pierrick@example.com".into(),
            display_name: "Pierrick".into(),
            password_hash: PasswordService::hash("hunter2hunter2").unwrap(),
            totp_secret_enc: secret_box.seal(&secret),
        })
        .await
        .unwrap();
    secret
}

fn now_unix() -> u64 {
    u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap()
}

pub fn test_config() -> api::Config {
    api::Config {
        master_key_b64: KEY_B64.to_owned(),
        webauthn_rp_id: "localhost".to_owned(),
        webauthn_origin: "http://localhost:4200".to_owned(),
        session_ttl_days: 30,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_sets_cookie_and_me_returns_identity(pool: PgPool) {
    let secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();
    let code = TotpService::current_code(&secret, now_unix()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"pierrick@example.com","password":"hunter2hunter2","totp":"{code}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().to_owned();
    assert!(cookie.contains("joel_session="));
    assert!(cookie.contains("HttpOnly"));

    let me = app
        .oneshot(Request::get("/api/auth/me").header(header::COOKIE, cookie.split(';').next().unwrap()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let body = me.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email"], "pierrick@example.com");
}

#[sqlx::test(migrations = "../../migrations")]
async fn bad_credentials_yield_401_and_me_requires_session(pool: PgPool) {
    let _secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email":"pierrick@example.com","password":"wrong","totp":"000000"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::UNAUTHORIZED);

    let me = app.oneshot(Request::get("/api/auth/me").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_invalidates_the_cookie(pool: PgPool) {
    let secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();
    let code = TotpService::current_code(&secret, now_unix()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"pierrick@example.com","password":"hunter2hunter2","totp":"{code}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().split(';').next().unwrap().to_owned();

    let logout = app
        .clone()
        .oneshot(Request::post("/api/auth/logout").header(header::COOKIE, &cookie).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let me = app.oneshot(Request::get("/api/auth/me").header(header::COOKIE, &cookie).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}
```

Run : `cargo test -p api` → FAIL (`Config`, `build_router_with` absents).

- [ ] **Step 14.3 : Implémenter état partagé et routes**

`backend/bins/api/src/state.rs` :

```rust
//! Shared application state and configuration.

use std::collections::HashMap;
use std::sync::Arc;

use domain::auth::crypto::SecretBox;
use domain::auth::model::AuthError;
use domain::auth::use_cases::Auth;
use persistence::auth::{PgAudit, PgCredentials, PgSessions, PgUsers};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use webauthn_rs::prelude::{DiscoverableAuthentication, PasskeyRegistration, Url, Webauthn, WebauthnBuilder};

/// Runtime configuration, sourced from the environment.
pub struct Config {
    /// Base64-encoded 32-byte master key for secrets at rest.
    pub master_key_b64: String,
    /// WebAuthn relying-party id (the apex hostname).
    pub webauthn_rp_id: String,
    /// WebAuthn origin (scheme + host + port presented by the browser).
    pub webauthn_origin: String,
    /// Session lifetime in days.
    pub session_ttl_days: i64,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// # Errors
    /// Returns the name of the missing variable.
    pub fn from_env() -> Result<Self, String> {
        let need = |name: &str| std::env::var(name).map_err(|_| name.to_owned());
        Ok(Self {
            master_key_b64: need("MASTER_KEY")?,
            webauthn_rp_id: need("WEBAUTHN_RP_ID")?,
            webauthn_origin: need("WEBAUTHN_ORIGIN")?,
            session_ttl_days: need("SESSION_TTL_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(30),
        })
    }
}

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database pool (health checks and future modules).
    pub pool: PgPool,
    /// Authentication use cases.
    pub auth: Arc<Auth>,
    /// WebAuthn engine.
    pub webauthn: Arc<Webauthn>,
    /// Passkey storage adapter.
    pub credentials: Arc<PgCredentials>,
    /// In-flight passkey registration states, keyed by user id.
    pub reg_states: Arc<Mutex<HashMap<Uuid, PasskeyRegistration>>>,
    /// In-flight discoverable authentication states, keyed by challenge id.
    pub auth_states: Arc<Mutex<HashMap<Uuid, DiscoverableAuthentication>>>,
}

impl AppState {
    /// Assembles the full state from a pool and configuration.
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build(pool: PgPool, config: &Config) -> Result<Self, AuthError> {
        let secret_box = SecretBox::from_base64(&config.master_key_b64)?;
        let auth = Auth::new(
            Arc::new(PgUsers::new(pool.clone())),
            Arc::new(PgSessions::new(pool.clone())),
            Arc::new(PgAudit::new(pool.clone())),
            secret_box,
            config.session_ttl_days,
        );
        let origin = Url::parse(&config.webauthn_origin).map_err(|_| AuthError::Crypto)?;
        let webauthn = WebauthnBuilder::new(&config.webauthn_rp_id, &origin)
            .map_err(|_| AuthError::Crypto)?
            .rp_name("Joel")
            .build()
            .map_err(|_| AuthError::Crypto)?;
        Ok(Self {
            credentials: Arc::new(PgCredentials::new(pool.clone())),
            pool,
            auth: Arc::new(auth),
            webauthn: Arc::new(webauthn),
            reg_states: Arc::new(Mutex::new(HashMap::new())),
            auth_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}
```

`backend/bins/api/src/auth_routes.rs` :

```rust
//! HTTP adapters for the authentication use cases.

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use domain::auth::model::{AuthError, IssuedSession, User};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::state::AppState;

pub const SESSION_COOKIE: &str = "joel_session";

/// Request body of the password login endpoint.
#[derive(Deserialize)]
pub struct LoginRequest {
    /// Account email.
    pub email: String,
    /// Account password.
    pub password: String,
    /// Current 6-digit TOTP code.
    pub totp: String,
}

/// Public identity of the authenticated user.
#[derive(Serialize)]
pub struct Identity {
    /// Account email.
    pub email: String,
    /// Display name.
    pub display_name: String,
}

/// Authenticated user extractor: resolves the session cookie or rejects with 401.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()).ok_or(StatusCode::UNAUTHORIZED)?;
        state
            .auth
            .validate_session(&token, OffsetDateTime::now_utc())
            .await
            .map(CurrentUser)
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

fn auth_error_response(error: &AuthError) -> StatusCode {
    match error {
        AuthError::InvalidCredentials | AuthError::NotAuthenticated => StatusCode::UNAUTHORIZED,
        AuthError::Crypto | AuthError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub fn session_cookie(issued: &IssuedSession) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, issued.token.clone());
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path("/");
    cookie.set_expires(issued.expires_at);
    cookie
}

async fn login(State(state): State<AppState>, jar: CookieJar, Json(body): Json<LoginRequest>) -> Response {
    match state.auth.password_login(&body.email, &body.password, &body.totp, OffsetDateTime::now_utc()).await {
        Ok(issued) => (jar.add(session_cookie(&issued)), StatusCode::OK).into_response(),
        Err(error) => auth_error_response(&error).into_response(),
    }
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if state.auth.logout(cookie.value()).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    (jar.remove(Cookie::from(SESSION_COOKIE)), StatusCode::NO_CONTENT).into_response()
}

async fn me(CurrentUser(user): CurrentUser) -> Json<Identity> {
    Json(Identity { email: user.email, display_name: user.display_name })
}

/// Routes under `/api/auth`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
}
```

`backend/bins/api/src/lib.rs` — remplacer `build_router` et déclarer les modules :

```rust
pub mod auth_routes;
pub mod state;

pub use state::Config;

use crate::state::AppState;

/// Builds the application router from a pool and explicit configuration.
///
/// # Errors
/// Propagates [`domain::auth::model::AuthError`] when crypto material is invalid.
pub fn build_router_with(pool: PgPool, config: &Config) -> Result<Router, domain::auth::model::AuthError> {
    let state = AppState::build(pool.clone(), config)?;
    Ok(Router::new()
        .route("/api/healthz", get(healthz))
        .route("/api/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .merge(auth_routes::router())
        .with_state(state))
}
```

et adapter `healthz` à l'état (`State(state): State<AppState>` puis `persistence::ping(&state.pool)`). Dans `main.rs`, charger `Config::from_env()` (sortie code 1 avec log si variable manquante) et appeler `build_router_with(pool, &config)`.

**Mettre à jour `tests/healthz.rs`** (écrit en Tasks 5/8 contre l'ancienne signature) : remplacer chaque `api::build_router(pool)` par `api::build_router_with(pool, &test_config()).unwrap()` en réutilisant la même fonction `test_config()` (la dupliquer dans ce fichier ou l'y déclarer et l'inclure depuis `auth_password.rs`). Sans cela, `cargo test -p api` ne compile plus après cette tâche.

Note de design : le cookie `Secure` est posé même en dev — les navigateurs acceptent les cookies Secure sur `localhost`, aucun aménagement nécessaire.

- [ ] **Step 14.4 : Vérifier tests + lint, commit**

```bash
cargo test -p api
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(api): password login, session cookie middleware, identity and logout endpoints"
```

### Task 15 : Endpoints WebAuthn — enregistrement et login passkey (TDD partiel)

**Files:**
- Create: `backend/bins/api/src/webauthn_routes.rs`, `backend/bins/api/tests/auth_webauthn.rs`
- Modify: `backend/bins/api/src/lib.rs`

- [ ] **Step 15.1 : Tests AVANT (start + finish invalide ; le happy path complet se vérifie au navigateur en Task 18)**

`backend/bins/api/tests/auth_webauthn.rs` :

```rust
//! Integration tests for the WebAuthn endpoints (challenge issuance and rejection paths).
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

mod common {
    include!("auth_password.rs");
}

#[sqlx::test(migrations = "../../migrations")]
async fn register_start_requires_a_session(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(Request::post("/api/auth/webauthn/register/start").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_start_returns_a_challenge(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(Request::post("/api/auth/webauthn/login/start").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["challenge_id"].is_string());
    assert!(json["options"]["publicKey"]["challenge"].is_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_finish_with_unknown_challenge_is_rejected(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(
            Request::post("/api/auth/webauthn/login/finish")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"challenge_id":"00000000-0000-0000-0000-000000000000","credential":{"id":"x","rawId":"eA","response":{"authenticatorData":"eA","clientDataJSON":"eA","signature":"eA"},"type":"public-key"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

Note : le `mod common { include!(...) }` réutilise `seed`/`test_config` du fichier voisin ; les tests inclus s'exécutent une seconde fois sous ce module, c'est sans effet (bases éphémères distinctes).

Run : `cargo test -p api` → FAIL (routes absentes → 404 ≠ 401/200).

- [ ] **Step 15.2 : Implémenter les routes WebAuthn**

`backend/bins/api/src/webauthn_routes.rs` :

```rust
//! HTTP adapters for passkey registration and discoverable login.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;
use webauthn_rs::prelude::{DiscoverableKey, Passkey, PublicKeyCredential, RegisterPublicKeyCredential};

use crate::auth_routes::{session_cookie, CurrentUser};
use crate::state::AppState;

/// Request body completing a passkey registration.
#[derive(Deserialize)]
pub struct RegisterFinish {
    /// Human-readable device label ("iPhone", "PC").
    pub label: String,
    /// Credential produced by the browser.
    pub credential: RegisterPublicKeyCredential,
}

/// Request body completing a discoverable login.
#[derive(Deserialize)]
pub struct LoginFinish {
    /// Challenge identifier returned by the start endpoint.
    pub challenge_id: Uuid,
    /// Assertion produced by the browser.
    pub credential: PublicKeyCredential,
}

async fn register_start(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Response {
    let existing: Vec<Passkey> = match state.credentials.for_user(user.id).await {
        Ok(raw) => raw.iter().filter_map(|j| serde_json::from_str(j).ok()).collect(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let exclude = existing.iter().map(|p| p.cred_id().clone()).collect::<Vec<_>>();
    match state.webauthn.start_passkey_registration(user.id, &user.email, &user.display_name, Some(exclude)) {
        Ok((ccr, reg_state)) => {
            state.reg_states.lock().await.insert(user.id, reg_state);
            Json(ccr).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn register_finish(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<RegisterFinish>,
) -> Response {
    let Some(reg_state) = state.reg_states.lock().await.remove(&user.id) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(passkey) = state.webauthn.finish_passkey_registration(&body.credential, &reg_state) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(serialized) = serde_json::to_string(&passkey) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match state.credentials.insert(user.id, &body.label, &serialized).await {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn login_start(State(state): State<AppState>) -> Response {
    match state.webauthn.start_discoverable_authentication() {
        Ok((rcr, auth_state)) => {
            let challenge_id = Uuid::new_v4();
            state.auth_states.lock().await.insert(challenge_id, auth_state);
            Json(json!({ "challenge_id": challenge_id, "options": rcr })).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn login_finish(State(state): State<AppState>, jar: CookieJar, Json(body): Json<LoginFinish>) -> Response {
    let Some(auth_state) = state.auth_states.lock().await.remove(&body.challenge_id) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok((user_id, _cred_id)) = state.webauthn.identify_discoverable_authentication(&body.credential) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let keys: Vec<DiscoverableKey> = match state.credentials.for_user(user_id).await {
        Ok(raw) => raw
            .iter()
            .filter_map(|j| serde_json::from_str::<Passkey>(j).ok())
            .map(|p| DiscoverableKey::from(&p))
            .collect(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if keys.is_empty() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if state.webauthn.finish_discoverable_authentication(&body.credential, auth_state, &keys).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match state.auth.issue_session(user_id, OffsetDateTime::now_utc()).await {
        Ok(issued) => (jar.add(session_cookie(&issued)), StatusCode::OK).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Routes under `/api/auth/webauthn`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/webauthn/register/start", post(register_start))
        .route("/api/auth/webauthn/register/finish", post(register_finish))
        .route("/api/auth/webauthn/login/start", post(login_start))
        .route("/api/auth/webauthn/login/finish", post(login_finish))
}
```

Dans `lib.rs` : `pub mod webauthn_routes;` et `.merge(webauthn_routes::router())`.

Si la signature exacte d'un appel `webauthn-rs` diffère à la compilation (`DiscoverableKey::from`, type d'`exclude`), corriger en suivant les erreurs du compilateur — l'architecture des routes ne change pas.

- [ ] **Step 15.3 : Vérifier tests + lint, commit**

```bash
cargo test -p api
cargo clippy --workspace --all-targets -- -D warnings
git add backend/
git commit -m "feat(api): webauthn passkey registration and discoverable login endpoints"
```

### Task 16 : Commande seed-admin

**Files:**
- Modify: `backend/bins/api/src/main.rs`

- [ ] **Step 16.1 : Implémenter la sous-commande**

Dans `main.rs`, avant le démarrage du serveur, intercepter `std::env::args` :

```rust
async fn seed_admin(pool: sqlx::PgPool, config: &api::Config, email: &str, display_name: &str) {
    use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
    use domain::auth::model::User;
    use domain::auth::ports::UserRepository;

    let Ok(secret_box) = SecretBox::from_base64(&config.master_key_b64) else {
        tracing::error!("invalid MASTER_KEY");
        std::process::exit(1);
    };
    let mut password_bytes = [0_u8; 18];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut password_bytes);
    let password = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, password_bytes);
    let totp_secret = TotpService::generate_secret();

    let Ok(password_hash) = PasswordService::hash(&password) else {
        tracing::error!("password hashing failed");
        std::process::exit(1);
    };
    let user = User {
        id: uuid::Uuid::new_v4(),
        email: email.to_owned(),
        display_name: display_name.to_owned(),
        password_hash,
        totp_secret_enc: secret_box.seal(&totp_secret),
    };
    let users = persistence::auth::PgUsers::new(pool);
    if let Err(error) = users.insert(&user).await {
        tracing::error!(%error, "cannot insert admin user");
        std::process::exit(1);
    }
    let otpauth = TotpService::otpauth_url(&totp_secret, email).unwrap_or_default();
    println!("user created: {email}");
    println!("initial password: {password}");
    println!("totp secret (base32): {}", TotpService::secret_base32(&totp_secret));
    println!("otpauth url: {otpauth}");
    println!("Add the TOTP secret to your authenticator app, log in, then register a passkey.");
}
```

et dans `main`, après la création du pool :

```rust
let args: Vec<String> = std::env::args().collect();
if args.get(1).map(String::as_str) == Some("seed-admin") {
    let (Some(email), Some(display_name)) = (args.get(2), args.get(3)) else {
        tracing::error!("usage: api seed-admin <email> <display_name>");
        std::process::exit(2);
    };
    seed_admin(pool, &config, email, display_name).await;
    return;
}
```

Ajouter `rand = "0.9"` et `base64 = "0.22"` aux dépendances d'`api`. Les `println!` de la sous-commande sont une interface CLI légitime : si clippy s'en plaint (`clippy::print_stdout` n'est pas activé par défaut — rien à faire normalement), annoter la fonction avec `#[allow(clippy::print_stdout)]`.

- [ ] **Step 16.2 : Vérification manuelle**

```bash
cargo run -p api -- seed-admin pierrick@example.com Pierrick
```

Expected: affichage du mot de passe initial, du secret TOTP base32 et de l'URL otpauth. Scanner/saisir dans l'app authenticator.

- [ ] **Step 16.3 : Commit**

```bash
git add backend/
git commit -m "feat(api): seed-admin bootstrap command"
```

### Task 17 : Front — AuthService, helpers WebAuthn, page de connexion (TDD)

**Files:**
- Create: `frontend/src/app/core/auth/webauthn.helpers.ts`, `frontend/src/app/core/auth/webauthn.helpers.spec.ts`, `frontend/src/app/core/auth/auth.service.ts`, `frontend/src/app/core/auth/auth.service.spec.ts`, `frontend/src/app/features/login/login.component.ts`

- [ ] **Step 17.1 : Tests des helpers base64url AVANT**

`frontend/src/app/core/auth/webauthn.helpers.spec.ts` :

```typescript
import { base64UrlToBuffer, bufferToBase64Url } from './webauthn.helpers';

describe('webauthn helpers', () => {
  it('roundtrips binary data through base64url', () => {
    const bytes = new Uint8Array([0, 1, 250, 255, 32, 64]);
    const encoded = bufferToBase64Url(bytes.buffer);
    expect(encoded).not.toContain('+');
    expect(encoded).not.toContain('/');
    expect(encoded).not.toContain('=');
    expect(new Uint8Array(base64UrlToBuffer(encoded))).toEqual(bytes);
  });

  it('decodes standard base64url challenges', () => {
    const decoded = base64UrlToBuffer('AQID');
    expect(new Uint8Array(decoded)).toEqual(new Uint8Array([1, 2, 3]));
  });
});
```

Run : `npm test -- --watch=false` → FAIL.

- [ ] **Step 17.2 : Implémenter les helpers**

`frontend/src/app/core/auth/webauthn.helpers.ts` :

```typescript
/** Decodes a base64url string (no padding) into an ArrayBuffer. */
export function base64UrlToBuffer(value: string): ArrayBuffer {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const padded = base64 + '='.repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/** Encodes an ArrayBuffer into a base64url string without padding. */
export function bufferToBase64Url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/u, '');
}
```

Run : tests PASS.

- [ ] **Step 17.3 : Tests de l'AuthService AVANT**

`frontend/src/app/core/auth/auth.service.spec.ts` :

```typescript
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { AuthService } from './auth.service';

describe('AuthService', () => {
  let service: AuthService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(AuthService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('posts credentials on password login and loads identity', async () => {
    const login = service.loginWithPassword('a@b.c', 'pwd', '123456');
    const loginReq = http.expectOne('/api/auth/login');
    expect(loginReq.request.method).toBe('POST');
    expect(loginReq.request.body).toEqual({ email: 'a@b.c', password: 'pwd', totp: '123456' });
    loginReq.flush({});
    const meReq = http.expectOne('/api/auth/me');
    meReq.flush({ email: 'a@b.c', display_name: 'A' });
    await login;
    expect(service.identity()?.email).toBe('a@b.c');
  });

  it('clears identity on logout', async () => {
    const logout = service.logout();
    http.expectOne('/api/auth/logout').flush(null, { status: 204, statusText: 'No Content' });
    await logout;
    expect(service.identity()).toBeNull();
  });
});
```

Run : FAIL.

- [ ] **Step 17.4 : Implémenter l'AuthService**

`frontend/src/app/core/auth/auth.service.ts` :

```typescript
import { inject, Injectable, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { base64UrlToBuffer, bufferToBase64Url } from './webauthn.helpers';
import { LoggerService } from '../logging/logger.service';

export interface Identity {
  email: string;
  display_name: string;
}

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);

  /** Identity of the connected user; null when logged out or unknown. */
  readonly identity = signal<Identity | null>(null);

  async loginWithPassword(email: string, password: string, totp: string): Promise<void> {
    await firstValueFrom(this.http.post('/api/auth/login', { email, password, totp }));
    await this.refreshIdentity();
    this.logger.info('auth.login', { method: 'password' });
  }

  async loginWithPasskey(): Promise<void> {
    const start = await firstValueFrom(
      this.http.post<{ challenge_id: string; options: { publicKey: Record<string, unknown> } }>(
        '/api/auth/webauthn/login/start',
        {},
      ),
    );
    const publicKey = start.options.publicKey;
    const assertion = (await navigator.credentials.get({
      publicKey: {
        ...publicKey,
        challenge: base64UrlToBuffer(publicKey['challenge'] as string),
      } as PublicKeyCredentialRequestOptions,
    })) as PublicKeyCredential;
    const response = assertion.response as AuthenticatorAssertionResponse;
    await firstValueFrom(
      this.http.post('/api/auth/webauthn/login/finish', {
        challenge_id: start.challenge_id,
        credential: {
          id: assertion.id,
          rawId: bufferToBase64Url(assertion.rawId),
          type: assertion.type,
          response: {
            authenticatorData: bufferToBase64Url(response.authenticatorData),
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
            signature: bufferToBase64Url(response.signature),
            userHandle: response.userHandle ? bufferToBase64Url(response.userHandle) : null,
          },
        },
      }),
    );
    await this.refreshIdentity();
    this.logger.info('auth.login', { method: 'passkey' });
  }

  async registerPasskey(label: string): Promise<void> {
    const options = await firstValueFrom(
      this.http.post<{ publicKey: Record<string, unknown> }>('/api/auth/webauthn/register/start', {}),
    );
    const publicKey = options.publicKey;
    const user = publicKey['user'] as Record<string, unknown>;
    const credential = (await navigator.credentials.create({
      publicKey: {
        ...publicKey,
        challenge: base64UrlToBuffer(publicKey['challenge'] as string),
        user: { ...user, id: base64UrlToBuffer(user['id'] as string) },
        excludeCredentials: ((publicKey['excludeCredentials'] as { id: string; type: string }[] | undefined) ?? []).map(
          (c) => ({ ...c, id: base64UrlToBuffer(c.id) }),
        ),
      } as PublicKeyCredentialCreationOptions,
    })) as PublicKeyCredential;
    const response = credential.response as AuthenticatorAttestationResponse;
    await firstValueFrom(
      this.http.post('/api/auth/webauthn/register/finish', {
        label,
        credential: {
          id: credential.id,
          rawId: bufferToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            attestationObject: bufferToBase64Url(response.attestationObject),
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
          },
        },
      }),
    );
    this.logger.info('auth.passkey.registered', { label });
  }

  async refreshIdentity(): Promise<void> {
    try {
      this.identity.set(await firstValueFrom(this.http.get<Identity>('/api/auth/me')));
    } catch {
      this.identity.set(null);
    }
  }

  async logout(): Promise<void> {
    await firstValueFrom(this.http.post('/api/auth/logout', {}));
    this.identity.set(null);
    this.logger.info('auth.logout', {});
  }
}
```

Run : `npm test -- --watch=false` → PASS ; `npx ng lint` → propre.

- [ ] **Step 17.5 : Page de connexion**

`frontend/src/app/features/login/login.component.ts` :

```typescript
import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { AuthService } from '../../core/auth/auth.service';

@Component({
  selector: 'app-login',
  imports: [FormsModule],
  template: `
    <div class="login">
      <h1>Joel</h1>
      <p>Votre cabinet vous attend.</p>

      <button type="button" class="login__passkey" (click)="passkey()">Se connecter avec une passkey</button>

      @if (fallback()) {
        <form (ngSubmit)="password()">
          <input name="email" type="email" [(ngModel)]="email" placeholder="Email" required />
          <input name="password" type="password" [(ngModel)]="pwd" placeholder="Mot de passe" required />
          <input name="totp" inputmode="numeric" [(ngModel)]="totp" placeholder="Code TOTP" required />
          <button type="submit">Connexion</button>
        </form>
      } @else {
        <button type="button" class="login__alt" (click)="fallback.set(true)">Utiliser le mot de passe</button>
      }

      @if (error()) {
        <p class="login__error">Connexion impossible. Vérifiez vos informations.</p>
      }
    </div>
  `,
  styles: `
    .login { max-width: 22rem; margin: 15vh auto; display: flex; flex-direction: column; gap: 0.8rem; text-align: center; }
    .login form { display: flex; flex-direction: column; gap: 0.6rem; }
    .login input, .login button { padding: 0.7rem; font-size: 1rem; }
    .login__passkey { background: #101418; color: #fff; border: none; border-radius: 6px; }
    .login__alt { background: none; border: none; color: #444; text-decoration: underline; }
    .login__error { color: #b00020; }
  `,
})
export class LoginComponent {
  private readonly auth = inject(AuthService);
  private readonly router = inject(Router);

  email = '';
  pwd = '';
  totp = '';
  readonly fallback = signal(false);
  readonly error = signal(false);

  async passkey(): Promise<void> {
    await this.attempt(() => this.auth.loginWithPasskey());
  }

  async password(): Promise<void> {
    await this.attempt(() => this.auth.loginWithPassword(this.email, this.pwd, this.totp));
  }

  private async attempt(action: () => Promise<void>): Promise<void> {
    this.error.set(false);
    try {
      await action();
      await this.router.navigateByUrl('/cockpit');
    } catch {
      this.error.set(true);
    }
  }
}
```

- [ ] **Step 17.6 : Vérifier lint + tests + build, commit**

```bash
npx ng lint && npm test -- --watch=false && npm run build
git add frontend/
git commit -m "feat(frontend): auth service with passkey and password flows, login page"
```

### Task 18 : Front — guard, page Sécurité, vérification navigateur complète

**Files:**
- Create: `frontend/src/app/core/auth/auth.guard.ts`, `frontend/src/app/features/settings/security.component.ts`
- Modify: `frontend/src/app/app.routes.ts`, `frontend/src/app/core/layout/shell.component.ts`

- [ ] **Step 18.1 : Guard**

`frontend/src/app/core/auth/auth.guard.ts` :

```typescript
import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from './auth.service';

export const authGuard: CanActivateFn = async () => {
  const auth = inject(AuthService);
  const router = inject(Router);
  if (auth.identity()) {
    return true;
  }
  await auth.refreshIdentity();
  return auth.identity() ? true : router.parseUrl('/login');
};
```

- [ ] **Step 18.2 : Page Sécurité (enregistrement de passkeys)**

`frontend/src/app/features/settings/security.component.ts` :

```typescript
import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { AuthService } from '../../core/auth/auth.service';

@Component({
  selector: 'app-security',
  imports: [FormsModule],
  template: `
    <h2>Sécurité</h2>
    <p>Enregistrez une passkey sur cet appareil pour vous connecter avec Face ID ou Windows Hello.</p>
    <form (ngSubmit)="add()">
      <input name="label" [(ngModel)]="label" placeholder="Nom de l'appareil (iPhone, PC…)" required />
      <button type="submit">Ajouter une passkey</button>
    </form>
    @if (done()) {
      <p>Passkey enregistrée ✔</p>
    }
    @if (error()) {
      <p>Échec de l'enregistrement.</p>
    }
  `,
})
export class SecurityComponent {
  private readonly auth = inject(AuthService);
  label = '';
  readonly done = signal(false);
  readonly error = signal(false);

  async add(): Promise<void> {
    this.done.set(false);
    this.error.set(false);
    try {
      await this.auth.registerPasskey(this.label);
      this.done.set(true);
    } catch {
      this.error.set(true);
    }
  }
}
```

- [ ] **Step 18.3 : Routes protégées**

`frontend/src/app/app.routes.ts` — remplacer :

```typescript
import { Routes } from '@angular/router';
import { ShellComponent } from './core/layout/shell.component';
import { authGuard } from './core/auth/auth.guard';

export const routes: Routes = [
  {
    path: 'login',
    loadComponent: () => import('./features/login/login.component').then((m) => m.LoginComponent),
  },
  {
    path: '',
    component: ShellComponent,
    canActivate: [authGuard],
    children: [
      { path: '', pathMatch: 'full', redirectTo: 'cockpit' },
      {
        path: 'cockpit',
        loadComponent: () =>
          import('./features/cockpit/cockpit.component').then((m) => m.CockpitComponent),
      },
      {
        path: 'securite',
        loadComponent: () =>
          import('./features/settings/security.component').then((m) => m.SecurityComponent),
      },
    ],
  },
];
```

Ajouter le lien dans la nav du shell : `<a routerLink="/securite" routerLinkActive="active">Sécurité</a>`.

- [ ] **Step 18.4 : Vérification navigateur de bout en bout (dev)**

Avec PostgreSQL dev + `cargo run -p api` + `npm start` :

1. `cargo run -p api -- seed-admin pierrick@example.com Pierrick` → noter mot de passe + ajouter le TOTP à l'app authenticator.
2. Ouvrir `http://localhost:4200` → redirection `/login`.
3. « Utiliser le mot de passe » → email + mot de passe + code TOTP → arrivée sur le Cockpit.
4. Page Sécurité → « Ajouter une passkey » (Windows Hello se déclenche) → ✔.
5. Se déconnecter n'existe pas encore dans l'UI (volontaire, YAGNI) : supprimer le cookie via les outils de dev, revenir sur `/login`, cliquer « Se connecter avec une passkey » → Windows Hello → Cockpit. ✅

- [ ] **Step 18.5 : Lint + tests + build, commit**

```bash
npx ng lint && npm test -- --watch=false && npm run build
git add frontend/
git commit -m "feat(frontend): auth guard, security page and protected shell routes"
```

---

## Partie C — Conteneurs, VPS, NetBird, mise en production

Convention : `vps$` = session SSH sur le VPS. Les étapes marquées **[MANUEL]** demandent une action de Pierrick (interfaces web, App Store, console OVH).

### Task 19 : Migrations embarquées + Dockerfiles durcis

**Files:**
- Create: `deploy/Dockerfile.backend`, `deploy/Dockerfile.edge`, `deploy/Caddyfile`
- Modify: `backend/bins/api/src/main.rs`

- [ ] **Step 19.1 : Embarquer les migrations dans l'api**

Dans `main.rs`, juste après la création du pool (avant le branchement `seed-admin`) :

```rust
if let Err(error) = sqlx::migrate!("../../migrations").run(&pool).await {
    tracing::error!(%error, "database migration failed");
    std::process::exit(1);
}
```

Ajouter la feature `migrate` à la dépendance sqlx d'`api`. Vérifier : `cargo test -p api` toujours vert.

- [ ] **Step 19.2 : Dockerfile backend (api + runner, distroless, non-root)**

`deploy/Dockerfile.backend` :

```dockerfile
FROM rust:1-slim AS builder
WORKDIR /build
COPY backend/ .
RUN cargo build --release --bin api --bin runner

FROM gcr.io/distroless/cc-debian12:nonroot AS api
LABEL version="0.1.0" description="Joel API server"
COPY --from=builder /build/target/release/api /usr/local/bin/api
COPY --from=builder /build/migrations /migrations
USER nonroot
ENTRYPOINT ["/usr/local/bin/api"]

FROM gcr.io/distroless/cc-debian12:nonroot AS runner
LABEL version="0.1.0" description="Joel job runner"
COPY --from=builder /build/target/release/runner /usr/local/bin/runner
USER nonroot
ENTRYPOINT ["/usr/local/bin/runner"]
```

(`sqlx::migrate!` embarque les fichiers à la compilation ; la copie `/migrations` est superflue à l'exécution mais documente le contenu de l'image — la retirer si l'image doit rester minimale.)

- [ ] **Step 19.3 : Dockerfile edge (front Angular + Caddy avec module OVH)**

`deploy/Dockerfile.edge` :

```dockerfile
FROM node:22-alpine AS front
WORKDIR /build
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

FROM caddy:2-builder AS caddybuild
RUN xcaddy build --with github.com/caddy-dns/ovh

FROM caddy:2-alpine
LABEL version="0.1.0" description="Joel edge: internal tls, static front, api proxy"
COPY --from=caddybuild /usr/bin/caddy /usr/bin/caddy
COPY --from=front /build/dist/frontend/browser /srv/joel
COPY deploy/Caddyfile /etc/caddy/Caddyfile
USER 1000:1000
ENV XDG_DATA_HOME=/data XDG_CONFIG_HOME=/config
```

Note : le chemin de sortie Angular (`dist/frontend/browser`) est celui de la configuration par défaut ; vérifier avec `npm run build` local si le nom du dossier diffère.

- [ ] **Step 19.4 : Caddyfile (cert wildcard DNS-01, en-têtes durcis, ports non privilégiés)**

`deploy/Caddyfile` :

```caddyfile
{
	email pierrick.fonquerne@nubster.com
	http_port 8080
	https_port 8443
}

*.pierrickfonquerne.com:8443 {
	tls {
		dns ovh {
			endpoint ovh-eu
			application_key {env.OVH_APPLICATION_KEY}
			application_secret {env.OVH_APPLICATION_SECRET}
			consumer_key {env.OVH_CONSUMER_KEY}
		}
	}

	@intranet host intranet.pierrickfonquerne.com
	handle @intranet {
		header {
			Strict-Transport-Security "max-age=31536000"
			X-Content-Type-Options "nosniff"
			Referrer-Policy "no-referrer"
			Content-Security-Policy "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self'"
		}
		handle /api/* {
			reverse_proxy api:8080
		}
		handle {
			root * /srv/joel
			try_files {path} /index.html
			file_server
		}
	}

	@ntfy host ntfy.pierrickfonquerne.com
	handle @ntfy {
		reverse_proxy ntfy:80
	}

	handle {
		abort
	}
}
```

- [ ] **Step 19.5 : Build local de vérification + commit**

```bash
docker build -f deploy/Dockerfile.backend --target api -t joel-api .
docker build -f deploy/Dockerfile.backend --target runner -t joel-runner .
docker build -f deploy/Dockerfile.edge -t joel-edge .
```

Expected: 3 images construites sans erreur.

```bash
git add deploy/ backend/
git commit -m "feat(deploy): hardened container images and edge configuration"
```

### Task 20 : Compose de production — réseaux cloisonnés, CoreDNS, ntfy

**Files:**
- Create: `deploy/compose.prod.yml`, `deploy/Corefile`, `deploy/ntfy/server.yml`, `deploy/.env.prod.example`

- [ ] **Step 20.1 : Écrire `deploy/compose.prod.yml`**

```yaml
services:
  caddy:
    build:
      context: ..
      dockerfile: deploy/Dockerfile.edge
    restart: unless-stopped
    ports:
      - "${NETBIRD_IP}:443:8443"
    environment:
      OVH_APPLICATION_KEY: ${OVH_APPLICATION_KEY}
      OVH_APPLICATION_SECRET: ${OVH_APPLICATION_SECRET}
      OVH_CONSUMER_KEY: ${OVH_CONSUMER_KEY}
    volumes:
      - caddy_data:/data
      - caddy_config:/config
    networks: [edge, internet]
    depends_on: [api, ntfy]

  api:
    build:
      context: ..
      dockerfile: deploy/Dockerfile.backend
      target: api
    restart: unless-stopped
    read_only: true
    environment:
      DATABASE_URL: postgres://joel:${POSTGRES_PASSWORD}@postgres:5432/joel
      MASTER_KEY: ${MASTER_KEY}
      WEBAUTHN_RP_ID: intranet.pierrickfonquerne.com
      WEBAUTHN_ORIGIN: https://intranet.pierrickfonquerne.com
      SESSION_TTL_DAYS: "30"
      RUST_LOG: info
    networks: [edge, data]
    depends_on:
      postgres:
        condition: service_healthy

  runner:
    build:
      context: ..
      dockerfile: deploy/Dockerfile.backend
      target: runner
    restart: unless-stopped
    read_only: true
    environment:
      DATABASE_URL: postgres://joel:${POSTGRES_PASSWORD}@postgres:5432/joel
      RUST_LOG: info
    networks: [data, internet]
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: joel
      POSTGRES_USER: joel
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    networks: [data]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U joel -d joel"]
      interval: 5s
      timeout: 3s
      retries: 10

  ntfy:
    image: binwiederhier/ntfy
    restart: unless-stopped
    command: serve
    user: "1000:1000"
    volumes:
      - ./ntfy/server.yml:/etc/ntfy/server.yml:ro
      - ntfy_cache:/var/cache/ntfy
      - ntfy_data:/var/lib/ntfy
    networks: [edge, internet]

  coredns:
    image: coredns/coredns
    restart: unless-stopped
    command: ["-conf", "/etc/coredns/Corefile"]
    ports:
      - "${NETBIRD_IP}:53:53/udp"
      - "${NETBIRD_IP}:53:53/tcp"
    volumes:
      - ./Corefile:/etc/coredns/Corefile:ro
    networks: [internet]

networks:
  edge:
    internal: true
  data:
    internal: true
  internet:

volumes:
  caddy_data:
  caddy_config:
  pgdata:
  ntfy_cache:
  ntfy_data:
```

Cloisonnement obtenu : `api` et `postgres` n'appartiennent qu'à des réseaux `internal` (aucun accès Internet sortant) ; seuls `caddy` (ACME/OVH), `runner` (API IA futures), `ntfy` (upstream APNs) et `coredns` (résolution amont) peuvent sortir.

- [ ] **Step 20.2 : Écrire `deploy/Corefile`**

```
pierrickfonquerne.com:53 {
    hosts {
        NETBIRD_IP_PLACEHOLDER intranet.pierrickfonquerne.com
        NETBIRD_IP_PLACEHOLDER ntfy.pierrickfonquerne.com
        PUBLIC_IP_PLACEHOLDER netbird.pierrickfonquerne.com
        fallthrough
    }
    forward . 1.1.1.1 9.9.9.9
}

.:53 {
    forward . 1.1.1.1 9.9.9.9
}
```

Les deux placeholders sont remplacés en Task 22 (une fois l'IP mesh connue) — c'est un fichier de config statique, pas de variable d'environnement possible ici.

- [ ] **Step 20.3 : Écrire `deploy/ntfy/server.yml`**

```yaml
base-url: https://ntfy.pierrickfonquerne.com
upstream-base-url: https://ntfy.sh
auth-file: /var/lib/ntfy/auth.db
auth-default-access: deny-all
behind-proxy: true
cache-file: /var/cache/ntfy/cache.db
```

- [ ] **Step 20.4 : Écrire `deploy/.env.prod.example`**

```bash
NETBIRD_IP=100.0.0.0
POSTGRES_PASSWORD=CHANGE_ME_openssl_rand_hex_24
MASTER_KEY=CHANGE_ME_openssl_rand_base64_32
OVH_APPLICATION_KEY=CHANGE_ME
OVH_APPLICATION_SECRET=CHANGE_ME
OVH_CONSUMER_KEY=CHANGE_ME
```

- [ ] **Step 20.5 : Commit**

```bash
git add deploy/
git commit -m "feat(deploy): production compose with isolated networks, internal dns and ntfy"
git push
```

Expected: CI verte sur la branche.

### Task 21 : VPS — Installation de NetBird self-hosted

**Files:** aucun (opérations serveur).

- [ ] **Step 21.1 [MANUEL] : DNS public chez OVH**

Console OVH → Zone DNS `pierrickfonquerne.com` → ajouter :

```
netbird.pierrickfonquerne.com.  A  <IP_PUBLIQUE_VPS>
```

Aucun autre enregistrement : `intranet.` et `ntfy.` n'existeront QUE dans le DNS interne.

- [ ] **Step 21.2 : Pare-feu du VPS**

```bash
vps$ sudo ufw allow 80/tcp 443/tcp 443/udp 3478/udp 49152:65535/udp
vps$ sudo ufw status
```

Expected: règles actives (en plus du port SSH existant). Les ports 80/443 servent NetBird (management/signal/dashboard derrière son propre proxy) — PAS l'intranet.

- [ ] **Step 21.3 : Lancer l'installeur officiel NetBird (avec Zitadel embarqué)**

```bash
vps$ mkdir -p ~/netbird-selfhosted && cd ~/netbird-selfhosted
vps$ export NETBIRD_DOMAIN=netbird.pierrickfonquerne.com
vps$ curl -fsSL https://raw.githubusercontent.com/netbirdio/netbird/main/infrastructure_files/getting-started-with-zitadel.sh | bash
```

Expected: à la fin, le script affiche l'URL du dashboard (`https://netbird.pierrickfonquerne.com`) et les identifiants admin Zitadel initiaux. **Les noter immédiatement dans le gestionnaire de mots de passe.**

- [ ] **Step 21.4 [MANUEL] : Premier login dashboard**

Ouvrir `https://netbird.pierrickfonquerne.com`, se connecter, changer le mot de passe admin, activer la MFA Zitadel.

- [ ] **Step 21.5 : Enrôler le VPS lui-même dans le mesh**

Le VPS doit être un peer du mesh pour posséder une IP `100.x` :

```bash
vps$ curl -fsSL https://pkgs.netbird.io/install.sh | sh
vps$ sudo netbird up --management-url https://netbird.pierrickfonquerne.com
```

Suivre le lien d'autorisation affiché (login Zitadel). Puis :

```bash
vps$ netbird status
```

Expected: `Management: Connected`, et une **IP NetBird `100.x.y.z`** — c'est la valeur de `NETBIRD_IP`. La noter.

### Task 22 : Enrôlement des appareils + DNS interne

**Files:**
- Modify: `deploy/Corefile` (sur le VPS, via le clone — voir Task 23 ; les valeurs sont désormais connues)

- [ ] **Step 22.1 [MANUEL] : Enrôler le PC Windows**

Installer le client NetBird (site officiel), démarrer, `Settings → Management URL` = `https://netbird.pierrickfonquerne.com`, se connecter. Vérifier : `ping <IP_mesh_du_VPS>` répond.

- [ ] **Step 22.2 [MANUEL] : Enrôler l'iPhone**

App Store → NetBird → ouvrir → « Use self-hosted » → URL `https://netbird.pierrickfonquerne.com` → login. Activer la connexion. Dans Réglages iOS → VPN : NetBird peut rester actif en permanence (consommation négligeable).

- [ ] **Step 22.3 [MANUEL] : Déclarer le DNS interne dans NetBird**

Dashboard NetBird → `DNS → Nameservers → Add nameserver` :
- Nameserver : `<IP_mesh_du_VPS>`, port 53
- Match domains : `pierrickfonquerne.com`
- Distribution groups : All

Ainsi, toute résolution `*.pierrickfonquerne.com` des appareils enrôlés passe par le CoreDNS du VPS (qui renvoie l'IP mesh pour `intranet.`/`ntfy.`, l'IP publique pour `netbird.`, et fait suivre le reste).

- [ ] **Step 22.4 : Renseigner le Corefile**

Remplacer dans `deploy/Corefile` les deux placeholders par les vraies valeurs (`NETBIRD_IP_PLACEHOLDER` → IP mesh, `PUBLIC_IP_PLACEHOLDER` → IP publique), committer :

```bash
git add deploy/Corefile
git commit -m "feat(deploy): pin internal dns records"
git push
```

### Task 23 : Premier déploiement de Joel

**Files:** aucun (opérations serveur).

- [ ] **Step 23.1 [MANUEL] : Créer les clés API OVH pour le DNS-01**

`https://eu.api.ovh.com/createToken` — droits strictement limités :

```
GET    /domain/zone/pierrickfonquerne.com/*
POST   /domain/zone/pierrickfonquerne.com/*
DELETE /domain/zone/pierrickfonquerne.com/*
```

Noter Application Key / Application Secret / Consumer Key.

- [ ] **Step 23.2 : Cloner le repo sur le VPS (deploy key lecture seule)**

```bash
vps$ ssh-keygen -t ed25519 -f ~/.ssh/joel_deploy -N "" -C "joel-deploy"
vps$ cat ~/.ssh/joel_deploy.pub
```

**[MANUEL]** Ajouter cette clé publique comme deploy key (lecture seule) du repo `joel` sur la forge. Puis :

```bash
vps$ GIT_SSH_COMMAND="ssh -i ~/.ssh/joel_deploy" git clone <url_ssh_du_repo> ~/joel
vps$ cd ~/joel && git checkout feature/brick-00-foundation
```

- [ ] **Step 23.3 : Environnement de production**

```bash
vps$ cd ~/joel/deploy
vps$ cp .env.prod.example .env
vps$ openssl rand -hex 24    # → POSTGRES_PASSWORD
vps$ openssl rand -base64 32 # → MASTER_KEY
```

Éditer `.env` : renseigner `NETBIRD_IP`, les deux valeurs générées, les trois clés OVH. `chmod 600 .env`.

- [ ] **Step 23.4 : Démarrer**

```bash
vps$ docker compose -f compose.prod.yml --env-file .env up -d --build
vps$ docker compose -f compose.prod.yml ps
```

Expected: 6 services `running`, postgres `healthy`. Vérifier l'obtention du wildcard :

```bash
vps$ docker compose -f compose.prod.yml logs caddy | grep -i "certificate obtained"
```

- [ ] **Step 23.5 : Créer le compte de Pierrick**

```bash
vps$ docker compose -f compose.prod.yml --env-file .env run --rm api seed-admin pierrick.fonquerne@nubster.com Pierrick
```

Expected: mot de passe initial + secret TOTP affichés. **[MANUEL]** Ajouter le TOTP à l'app authenticator, stocker le mot de passe au gestionnaire.

- [ ] **Step 23.6 : Vérification depuis le PC (enrôlé NetBird)**

Ouvrir `https://intranet.pierrickfonquerne.com` → page de connexion Joel, cadenas TLS valide (wildcard). Login mot de passe + TOTP → Cockpit « Joel est opérationnel — base up ». Page Sécurité → enregistrer la passkey « PC » (Windows Hello).

Contre-vérification sécurité : depuis un réseau SANS NetBird (par ex. la 4G du téléphone avec NetBird désactivé), `https://intranet.pierrickfonquerne.com` ne doit PAS répondre (résolution impossible ou timeout). ✅ = l'intranet est bien invisible.

### Task 24 : ntfy — la première notification de Joel

**Files:** aucun (opérations serveur + iPhone).

- [ ] **Step 24.1 : Créer l'utilisateur et le token ntfy**

```bash
vps$ docker compose -f compose.prod.yml exec ntfy ntfy user add --role=admin pierrick
vps$ docker compose -f compose.prod.yml exec ntfy ntfy token add pierrick
```

Noter le token (`tk_...`) — il servira au runner dans la brique 1.

- [ ] **Step 24.2 [MANUEL] : App ntfy sur l'iPhone**

App Store → ntfy → Settings → Default server : `https://ntfy.pierrickfonquerne.com` + user/mot de passe. S'abonner au topic `joel`.

- [ ] **Step 24.3 : Première notification**

```bash
vps$ curl -H "Authorization: Bearer <tk_...>" -d "Joel est en ligne. À votre service." https://ntfy.pierrickfonquerne.com/joel
```

Expected: notification push sur l'iPhone (NetBird actif). 🎩

### Task 25 : Critère de fin, PR et clôture de la brique

**Files:** aucun.

- [ ] **Step 25.1 [MANUEL] : LE test — iPhone, 4G, Face ID**

1. iPhone en 4G (WiFi coupé), NetBird actif.
2. Safari → `https://intranet.pierrickfonquerne.com` → page de connexion.
3. Première fois : login mot de passe + TOTP, puis page Sécurité → « Ajouter une passkey » (label `iPhone`) → Face ID.
4. Partage → « Sur l'écran d'accueil » → l'icône **Joel** s'installe (PWA standalone).
5. Fermer Safari, ouvrir l'app Joel → `/login` → « Se connecter avec une passkey » → **Face ID** → Cockpit.

✅ **Critère de fin de la brique 0 atteint.**

- [ ] **Step 25.2 : Pull request**

```bash
git push
gh pr create --base main --head feature/brick-00-foundation --title "Brick 0: foundation" --body "Monorepo, hardened containers, passkey auth, self-hosted mesh access. Closes the brick 0 milestone."
```

Vérifier la CI verte sur la PR. **Merge `--no-ff` vers `main` uniquement après GO explicite de Pierrick.**

- [ ] **Step 25.3 : Sauvegardes (dette assumée → brique 1)**

La sauvegarde chiffrée quotidienne de PostgreSQL (spec §3) est implémentée en brique 1 avec la première routine — c'est son cas d'usage de test idéal. D'ici là, sauvegarde manuelle après le seed :

```bash
vps$ docker compose -f compose.prod.yml exec postgres pg_dump -U joel joel | gzip > ~/joel-backup-initial.sql.gz
```

---

## Couverture du spec (auto-vérification)

| Exigence du spec | Tâche(s) |
|---|---|
| Monorepo workspace Cargo + Angular | 1, 2, 6 |
| api Axum REST + tracing JSON | 3, 5, 14 |
| runner séparé | 4, 19, 20 |
| PostgreSQL 17, migrations | 5, 10, 19 |
| Contrat OpenAPI → client TS | 8 |
| CI complète (clippy pedantic, cargo-deny, ESLint, tests) | 9 |
| Auth passkeys + secours mot de passe/TOTP | 10–18 |
| Secrets chiffrés au repos (AES-256-GCM) | 11, 16 |
| Audit log | 10, 12, 13 |
| Conteneurs durcis (multi-stage, distroless, non-root, read-only, LABEL) | 19, 20 |
| Réseaux cloisonnés (seuls runner/caddy/ntfy sortent) | 20 |
| NetBird self-hosted + enrôlement | 21, 22 |
| Cert wildcard DNS-01 OVH (zéro CT leak) | 19, 23 |
| DNS interne (intranet/ntfy invisibles publiquement) | 20, 22 |
| ntfy self-hosted + iOS | 20, 24 |
| PWA installable + Face ID en 4G (critère de fin) | 7, 25 |
| SSE temps réel | — (brique 2, premier consommateur réel) |
| Sauvegardes chiffrées quotidiennes | — (brique 1, première routine ; manuel en 25.3) |

