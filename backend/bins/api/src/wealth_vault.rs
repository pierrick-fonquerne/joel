//! Lazy, retrying access to the wealth use cases, which need the Egide-held key.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cipher_egide::{EgideClient, EgideEnvelopeCipher};
use domain::wealth::{FieldCipher, Wealth, WealthError};
use persistence::wealth::{PgAccounts, PgExchangeRates, PgValuations, PgWrappedKeys};
use sqlx::PgPool;
use tokio::sync::Mutex;

/// Minimum delay between two unlock attempts while the vault is sealed.
pub const UNLOCK_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Produces the field cipher once the vault is reachable.
#[async_trait]
pub trait VaultUnlocker: Send + Sync {
    /// Returns a ready cipher, or a log-safe reason (never key material).
    ///
    /// # Errors
    /// A log-safe reason when the vault cannot be unlocked.
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String>;
}

/// Unlocks through Egide Transit.
pub struct EgideUnlocker {
    client: EgideClient,
    keys: PgWrappedKeys,
}

impl EgideUnlocker {
    /// Builds the unlocker.
    #[must_use]
    pub const fn new(client: EgideClient, keys: PgWrappedKeys) -> Self {
        Self { client, keys }
    }
}

#[async_trait]
impl VaultUnlocker for EgideUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        EgideEnvelopeCipher::unlock(&self.client, &self.keys)
            .await
            .map(|cipher| Arc::new(cipher) as Arc<dyn FieldCipher>)
            .map_err(|error| error.to_string())
    }
}

/// Used when Egide is not configured: the vault stays sealed.
pub struct SealedUnlocker;

#[async_trait]
impl VaultUnlocker for SealedUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        Err("egide is not configured".to_owned())
    }
}

struct GateState {
    wealth: Option<Arc<Wealth>>,
    last_attempt: Option<Instant>,
}

/// Holds the wealth use cases once unlocked, and retries unlocking at most
/// once per `retry_interval` while sealed.
///
/// The state lock is held across the unlock await on purpose: unlocking is
/// single-flight, so two concurrent first starts cannot create two data keys.
pub struct WealthGate {
    pool: PgPool,
    unlocker: Arc<dyn VaultUnlocker>,
    retry_interval: Duration,
    state: Mutex<GateState>,
}

impl WealthGate {
    /// Builds a sealed gate; the first call to [`Self::wealth`] tries to unlock.
    #[must_use]
    pub fn new(pool: PgPool, unlocker: Arc<dyn VaultUnlocker>, retry_interval: Duration) -> Self {
        Self {
            pool,
            unlocker,
            retry_interval,
            state: Mutex::new(GateState {
                wealth: None,
                last_attempt: None,
            }),
        }
    }

    /// The wealth use cases, unlocking the vault when needed.
    ///
    /// # Errors
    /// [`WealthError::VaultSealed`] while the vault cannot be unlocked.
    pub async fn wealth(&self) -> Result<Arc<Wealth>, WealthError> {
        let mut state = self.state.lock().await;
        if let Some(wealth) = &state.wealth {
            return Ok(Arc::clone(wealth));
        }
        if state
            .last_attempt
            .is_some_and(|at| at.elapsed() < self.retry_interval)
        {
            return Err(WealthError::VaultSealed);
        }
        state.last_attempt = Some(Instant::now());
        match self.unlocker.unlock().await {
            Ok(cipher) => {
                let wealth = Arc::new(Wealth::new(
                    Arc::new(PgAccounts::new(self.pool.clone(), Arc::clone(&cipher))),
                    Arc::new(PgValuations::new(self.pool.clone(), cipher)),
                    Arc::new(PgExchangeRates::new(self.pool.clone())),
                ));
                state.wealth = Some(Arc::clone(&wealth));
                tracing::info!("wealth vault unlocked");
                Ok(wealth)
            }
            Err(reason) => {
                tracing::warn!(%reason, "wealth vault is sealed");
                Err(WealthError::VaultSealed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::wealth::test_support::FakeCipher;

    use super::*;

    struct FlakyUnlocker {
        failures_left: AtomicUsize,
        attempts: AtomicUsize,
    }

    #[async_trait]
    impl VaultUnlocker for FlakyUnlocker {
        async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.failures_left.load(Ordering::SeqCst) > 0 {
                self.failures_left.fetch_sub(1, Ordering::SeqCst);
                return Err("sealed".to_owned());
            }
            Ok(Arc::new(FakeCipher))
        }
    }

    struct SlowUnlocker {
        attempts: AtomicUsize,
    }

    #[async_trait]
    impl VaultUnlocker for SlowUnlocker {
        async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(Arc::new(FakeCipher))
        }
    }

    fn lazy_pool() -> PgPool {
        PgPool::connect_lazy("postgres://joel:unused@127.0.0.1:1/joel").unwrap()
    }

    #[tokio::test]
    async fn recovers_after_the_vault_is_unsealed() {
        let unlocker = Arc::new(FlakyUnlocker {
            failures_left: AtomicUsize::new(1),
            attempts: AtomicUsize::new(0),
        });
        let gate = WealthGate::new(lazy_pool(), unlocker.clone(), Duration::ZERO);

        assert!(matches!(gate.wealth().await, Err(WealthError::VaultSealed)));
        assert!(gate.wealth().await.is_ok());
        assert!(gate.wealth().await.is_ok());
        assert_eq!(unlocker.attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn does_not_hammer_egide_within_the_retry_interval() {
        let unlocker = Arc::new(FlakyUnlocker {
            failures_left: AtomicUsize::new(5),
            attempts: AtomicUsize::new(0),
        });
        let gate = WealthGate::new(lazy_pool(), unlocker.clone(), Duration::from_hours(1));

        for _ in 0..3 {
            assert!(matches!(gate.wealth().await, Err(WealthError::VaultSealed)));
        }
        assert_eq!(unlocker.attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_calls_unlock_only_once() {
        let unlocker = Arc::new(SlowUnlocker {
            attempts: AtomicUsize::new(0),
        });
        let gate = WealthGate::new(lazy_pool(), unlocker.clone(), Duration::ZERO);

        let (first, second) = tokio::join!(gate.wealth(), gate.wealth());

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_eq!(unlocker.attempts.load(Ordering::SeqCst), 1);
    }
}
