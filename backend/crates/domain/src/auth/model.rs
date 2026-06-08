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
    /// Wrong email, password or TOTP code, indistinguishable on purpose.
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
