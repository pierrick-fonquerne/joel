//! `PostgreSQL` adapters for the authentication ports.

use async_trait::async_trait;
use domain::auth::model::{AuthError, Session, User};
use domain::auth::ports::{AuditSink, CredentialRepository, SessionRepository, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

fn storage(err: &sqlx::Error) -> AuthError {
    AuthError::Storage(err.to_string())
}

/// `PostgreSQL` implementation of [`UserRepository`].
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
        .map(|row| {
            row.map(|(id, email, display_name, password_hash, totp_secret_enc)| User {
                id,
                email,
                display_name,
                password_hash,
                totp_secret_enc,
            })
        })
        .map_err(|e| storage(&e))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError> {
        sqlx::query_as::<_, (Uuid, String, String, String, Vec<u8>)>(
            "SELECT id, email, display_name, password_hash, totp_secret_enc FROM app_user WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| {
            row.map(|(id, email, display_name, password_hash, totp_secret_enc)| User {
                id,
                email,
                display_name,
                password_hash,
                totp_secret_enc,
            })
        })
        .map_err(|e| storage(&e))
    }

    async fn insert(&self, user: &User) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO app_user (id, email, display_name, password_hash, totp_secret_enc) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(&user.totp_secret_enc)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| storage(&e))
    }
}

/// `PostgreSQL` implementation of [`SessionRepository`].
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
            .map_err(|e| storage(&e))
    }

    async fn find(&self, token_hash: [u8; 32]) -> Result<Option<Session>, AuthError> {
        sqlx::query_as::<_, (Vec<u8>, Uuid, time::OffsetDateTime)>(
            "SELECT token_hash, user_id, expires_at FROM session WHERE token_hash = $1",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(|(hash, user_id, expires_at)| {
            let token_hash: [u8; 32] = hash
                .try_into()
                .map_err(|_| AuthError::Storage("corrupt token hash".into()))?;
            Ok(Session {
                token_hash,
                user_id,
                expires_at,
            })
        })
        .transpose()
    }

    async fn delete(&self, token_hash: [u8; 32]) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM session WHERE token_hash = $1")
            .bind(token_hash.as_slice())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| storage(&e))
    }
}

/// `PostgreSQL` implementation of [`CredentialRepository`].
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
    async fn insert(
        &self,
        user_id: Uuid,
        label: &str,
        credential_json: &str,
    ) -> Result<(), AuthError> {
        let value: serde_json::Value =
            serde_json::from_str(credential_json).map_err(|e| AuthError::Storage(e.to_string()))?;
        sqlx::query(
            "INSERT INTO webauthn_credential (user_id, label, credential) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(label)
        .bind(value)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| storage(&e))
    }

    async fn for_user(&self, user_id: Uuid) -> Result<Vec<String>, AuthError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT credential FROM webauthn_credential WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|v| v.to_string()).collect())
        .map_err(|e| storage(&e))
    }
}

/// `PostgreSQL` implementation of [`AuditSink`]. Failures are logged-and-dropped:
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
        let _unused =
            sqlx::query("INSERT INTO audit_log (user_id, action, detail) VALUES ($1, $2, $3)")
                .bind(user_id)
                .bind(action)
                .bind(detail)
                .execute(&self.pool)
                .await;
    }
}

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
        assert_eq!(
            repo.find_by_email(&user.email).await.unwrap().unwrap().id,
            user.id
        );
        assert_eq!(
            repo.find_by_id(user.id).await.unwrap().unwrap().email,
            user.email
        );
        assert!(
            repo.find_by_email("absent@example.com")
                .await
                .unwrap()
                .is_none()
        );
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
        repo.insert(user.id, "iPhone", r#"{"cred":"a"}"#)
            .await
            .unwrap();
        repo.insert(user.id, "PC", r#"{"cred":"b"}"#).await.unwrap();
        assert_eq!(repo.for_user(user.id).await.unwrap().len(), 2);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn audit_records_do_not_fail(pool: PgPool) {
        let sink = PgAudit::new(pool.clone());
        sink.record(None, "test.event", serde_json::json!({ "k": 1 }))
            .await;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
