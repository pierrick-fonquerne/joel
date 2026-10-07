//! Outbound ports of the wealth module.

use async_trait::async_trait;
use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use super::account::{Account, AccountId};
use super::error::WealthError;
use super::money::Currency;
use super::valuation::Valuation;

/// Persistent accounts. Names and notes are encrypted by the adapter.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Persists a new account.
    async fn insert(&self, account: &Account) -> Result<(), WealthError>;
    /// Finds an account; `Ok(None)` when unknown.
    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError>;
    /// Lists every account, archived ones included, in creation order.
    async fn list(&self) -> Result<Vec<Account>, WealthError>;
    /// Marks an account archived; [`WealthError::AccountNotFound`] when unknown.
    async fn archive(&self, id: AccountId) -> Result<(), WealthError>;
}

/// Append-only valuations. Amounts are encrypted by the adapter.
#[async_trait]
pub trait ValuationRepository: Send + Sync {
    /// Appends a valuation.
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError>;
    /// For each account, the valuation with the greatest `as_of <= at`, the
    /// latest `recorded_at` winning ties. Archived accounts are included.
    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError>;
    /// Every valuation of an account, by `as_of` then `recorded_at` ascending.
    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError>;
}

/// Cached ECB exchange rates.
#[async_trait]
pub trait ExchangeRateSource: Send + Sync {
    /// Units of `currency` for one euro on `on`, or on the closest earlier day
    /// within seven days. EUR always yields one.
    async fn units_per_eur(
        &self,
        currency: Currency,
        on: Date,
    ) -> Result<Option<Decimal>, WealthError>;
    /// True when at least one rate is known for `currency`. EUR is always supported.
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError>;
}

/// Storage of the Egide-wrapped data key, one row per key version.
#[async_trait]
pub trait WrappedKeyStore: Send + Sync {
    /// The most recent wrapped key, when one exists.
    async fn current(&self) -> Result<Option<String>, WealthError>;
    /// Appends a new wrapped key version.
    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError>;
}

/// Binds a ciphertext to its exact storage location (AES-GCM associated data),
/// so that a value copied to another row or column fails to decrypt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CipherContext {
    /// Table name.
    pub table: &'static str,
    /// Column name.
    pub column: &'static str,
    /// Primary key of the row.
    pub row_id: Uuid,
}

impl CipherContext {
    /// Associated data bytes: `table|column|row_id`.
    #[must_use]
    pub fn aad(&self) -> Vec<u8> {
        format!("{}|{}|{}", self.table, self.column, self.row_id).into_bytes()
    }
}

/// Field-level authenticated encryption.
pub trait FieldCipher: Send + Sync {
    /// Encrypts a field bound to its context.
    ///
    /// # Errors
    /// [`WealthError::Cipher`] when encryption fails.
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError>;
    /// Decrypts a field, failing on tampering or a different context.
    ///
    /// # Errors
    /// [`WealthError::Cipher`] when the ciphertext is malformed, forged or bound elsewhere.
    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError>;
}
