//! `PostgreSQL` adapters for the Joel domain ports.

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
