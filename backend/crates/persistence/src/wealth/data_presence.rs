//! Detects whether encrypted wealth data already exists.

use domain::wealth::WealthError;
use sqlx::PgPool;

use super::storage;

/// Whether any wealth account is stored, meaning a data key must already exist.
///
/// # Errors
/// [`WealthError::Storage`].
pub async fn has_wealth_data(pool: &PgPool) -> Result<bool, WealthError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM wealth_accounts)")
        .fetch_one(pool)
        .await
        .map_err(|e| storage(&e))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn reports_wealth_data_once_an_account_exists(pool: PgPool) {
        assert!(!has_wealth_data(&pool).await.unwrap());
        sqlx::query(
            "INSERT INTO wealth_accounts (id, kind, owner, currency, name, notes, is_archived, created_at) VALUES ($1, 'savings', 'personal', 'EUR', $2, NULL, false, now())",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(b"sealed".as_slice())
        .execute(&pool)
        .await
        .unwrap();
        assert!(has_wealth_data(&pool).await.unwrap());
    }
}
