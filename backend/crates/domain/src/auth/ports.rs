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

/// Storage of `WebAuthn` credentials, serialized as JSON.
#[async_trait]
pub trait CredentialRepository: Send + Sync {
    /// Persists a passkey for a user under a human-readable label.
    async fn insert(
        &self,
        user_id: Uuid,
        label: &str,
        credential_json: &str,
    ) -> Result<(), AuthError>;
    /// Returns all passkeys of a user as raw JSON documents.
    async fn for_user(&self, user_id: Uuid) -> Result<Vec<String>, AuthError>;
}

/// Sink for security-relevant events.
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Records an action, optionally tied to a user.
    async fn record(&self, user_id: Option<Uuid>, action: &str, detail: serde_json::Value);
}
