//! Authentication use cases, infrastructure-agnostic.

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
        Self {
            users,
            sessions,
            audit,
            secret_box,
            session_ttl_days,
        }
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
            self.audit
                .record(
                    None,
                    "auth.login.unknown_email",
                    serde_json::json!({ "email": email }),
                )
                .await;
            return Err(AuthError::InvalidCredentials);
        };
        if !PasswordService::verify(password, &user.password_hash) {
            self.audit
                .record(
                    Some(user.id),
                    "auth.login.bad_password",
                    serde_json::json!({}),
                )
                .await;
            return Err(AuthError::InvalidCredentials);
        }
        let secret = self.secret_box.open(&user.totp_secret_enc)?;
        let now_unix = u64::try_from(now.unix_timestamp()).map_err(|_| AuthError::Crypto)?;
        if !TotpService::verify(&secret, totp_code, now_unix) {
            self.audit
                .record(Some(user.id), "auth.login.bad_totp", serde_json::json!({}))
                .await;
            return Err(AuthError::InvalidCredentials);
        }
        let issued = self.issue_session(user.id, now).await?;
        self.audit
            .record(Some(user.id), "auth.login.password", serde_json::json!({}))
            .await;
        Ok(issued)
    }

    /// Issues a session for an already-authenticated user (passkey flow).
    ///
    /// # Errors
    /// Propagates storage failures.
    pub async fn issue_session(
        &self,
        user_id: uuid::Uuid,
        now: OffsetDateTime,
    ) -> Result<IssuedSession, AuthError> {
        let (token, token_hash) = SessionToken::generate();
        let expires_at = now + Duration::days(self.session_ttl_days);
        self.sessions
            .insert(&Session {
                token_hash,
                user_id,
                expires_at,
            })
            .await?;
        Ok(IssuedSession { token, expires_at })
    }

    /// Resolves a bearer token into its user, enforcing expiry.
    ///
    /// # Errors
    /// [`AuthError::NotAuthenticated`] when absent or expired.
    pub async fn validate_session(
        &self,
        token: &str,
        now: OffsetDateTime,
    ) -> Result<User, AuthError> {
        let hash = SessionToken::hash(token);
        let Some(session) = self.sessions.find(hash).await? else {
            return Err(AuthError::NotAuthenticated);
        };
        if session.expires_at <= now {
            self.sessions.delete(hash).await?;
            return Err(AuthError::NotAuthenticated);
        }
        self.users
            .find_by_id(session.user_id)
            .await?
            .ok_or(AuthError::NotAuthenticated)
    }

    /// Destroys the session associated with the token, when present.
    ///
    /// # Errors
    /// Propagates storage failures.
    pub async fn logout(&self, token: &str) -> Result<(), AuthError> {
        self.sessions.delete(SessionToken::hash(token)).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;
    use crate::auth::crypto::{PasswordService, SecretBox, TotpService};
    use crate::auth::model::{AuthError, User};
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
        let auth = Auth::new(
            users,
            Arc::new(FakeSessions::default()),
            Arc::new(NoopAudit),
            SecretBox::new(KEY),
            30,
        );
        (auth, secret)
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap()
    }

    #[tokio::test]
    async fn password_login_issues_a_validatable_session() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();

        let issued = auth
            .password_login("pierrick@example.com", "hunter2hunter2", &code, now())
            .await
            .unwrap();
        let user = auth.validate_session(&issued.token, now()).await.unwrap();
        assert_eq!(user.email, "pierrick@example.com");
    }

    #[tokio::test]
    async fn wrong_password_and_wrong_totp_both_yield_invalid_credentials() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();

        let wrong_pwd = auth
            .password_login("pierrick@example.com", "nope", &code, now())
            .await;
        let wrong_totp = auth
            .password_login("pierrick@example.com", "hunter2hunter2", "000000", now())
            .await;
        assert!(matches!(wrong_pwd, Err(AuthError::InvalidCredentials)));
        assert!(matches!(wrong_totp, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn unknown_email_yields_invalid_credentials_not_a_distinct_error() {
        let (auth, _) = seeded().await;
        let result = auth
            .password_login("ghost@example.com", "x", "000000", now())
            .await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn expired_session_is_rejected() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();
        let issued = auth
            .password_login("pierrick@example.com", "hunter2hunter2", &code, now())
            .await
            .unwrap();

        let later = now() + time::Duration::days(31);
        assert!(matches!(
            auth.validate_session(&issued.token, later).await,
            Err(AuthError::NotAuthenticated)
        ));
    }

    #[tokio::test]
    async fn logout_invalidates_the_session() {
        let (auth, secret) = seeded().await;
        let code = TotpService::current_code(&secret, 1_780_000_000).unwrap();
        let issued = auth
            .password_login("pierrick@example.com", "hunter2hunter2", &code, now())
            .await
            .unwrap();

        auth.logout(&issued.token).await.unwrap();
        assert!(matches!(
            auth.validate_session(&issued.token, now()).await,
            Err(AuthError::NotAuthenticated)
        ));
    }
}
