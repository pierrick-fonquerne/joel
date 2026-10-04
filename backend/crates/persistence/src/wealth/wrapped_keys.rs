//! Versioned storage of the Egide-wrapped wealth data key.

use async_trait::async_trait;
use domain::wealth::{WealthError, WrappedKeyStore};
use sqlx::PgPool;

use super::storage;

/// `PostgreSQL` implementation of [`WrappedKeyStore`].
pub struct PgWrappedKeys {
    pool: PgPool,
}

impl PgWrappedKeys {
    /// Builds the adapter.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrappedKeyStore for PgWrappedKeys {
    async fn current(&self) -> Result<Option<String>, WealthError> {
        sqlx::query_scalar("SELECT wrapped_key FROM wealth_keys ORDER BY version DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| storage(&e))
    }

    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError> {
        sqlx::query("INSERT INTO wealth_keys (egide_key_name, wrapped_key) VALUES ($1, $2)")
            .bind(egide_key_name)
            .bind(wrapped_key)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| storage(&e))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn current_returns_the_latest_version(pool: PgPool) {
        let store = PgWrappedKeys::new(pool);
        assert_eq!(store.current().await.unwrap(), None);
        store.insert("egide:v1:a", "joel-wealth").await.unwrap();
        store.insert("egide:v2:b", "joel-wealth").await.unwrap();
        assert_eq!(
            store.current().await.unwrap().as_deref(),
            Some("egide:v2:b")
        );
    }
}
