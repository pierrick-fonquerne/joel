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
