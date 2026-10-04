//! In-memory fakes for the wealth ports, shared by domain, persistence and api tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rust_decimal::Decimal;
use time::{Date, Duration};

use super::account::{Account, AccountId};
use super::error::WealthError;
use super::money::Currency;
use super::ports::{
    AccountRepository, CipherContext, ExchangeRateSource, FieldCipher, ValuationRepository,
    WrappedKeyStore,
};
use super::valuation::Valuation;

fn poisoned() -> WealthError {
    WealthError::Storage("poisoned".into())
}

/// In-memory [`AccountRepository`].
#[derive(Default)]
pub struct FakeAccounts {
    /// Stored accounts, in insertion order.
    pub accounts: Mutex<Vec<Account>>,
}

#[async_trait]
impl AccountRepository for FakeAccounts {
    async fn insert(&self, account: &Account) -> Result<(), WealthError> {
        self.accounts
            .lock()
            .map_err(|_| poisoned())?
            .push(account.clone());
        Ok(())
    }
    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError> {
        Ok(self
            .accounts
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }
    async fn list(&self) -> Result<Vec<Account>, WealthError> {
        Ok(self.accounts.lock().map_err(|_| poisoned())?.clone())
    }
    async fn archive(&self, id: AccountId) -> Result<(), WealthError> {
        let mut accounts = self.accounts.lock().map_err(|_| poisoned())?;
        let account = accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(WealthError::AccountNotFound)?;
        account.is_archived = true;
        Ok(())
    }
}

/// In-memory [`ValuationRepository`].
#[derive(Default)]
pub struct FakeValuations {
    /// Stored valuations, in insertion order.
    pub valuations: Mutex<Vec<Valuation>>,
}

#[async_trait]
impl ValuationRepository for FakeValuations {
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError> {
        self.valuations
            .lock()
            .map_err(|_| poisoned())?
            .push(valuation.clone());
        Ok(())
    }
    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError> {
        let valuations = self.valuations.lock().map_err(|_| poisoned())?;
        let mut latest: HashMap<AccountId, Valuation> = HashMap::new();
        for valuation in valuations.iter().filter(|v| v.as_of <= at) {
            let is_newer = latest.get(&valuation.account_id).is_none_or(|current| {
                (valuation.as_of, valuation.recorded_at) >= (current.as_of, current.recorded_at)
            });
            if is_newer {
                latest.insert(valuation.account_id, valuation.clone());
            }
        }
        Ok(latest.into_values().collect())
    }
    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        let mut found: Vec<Valuation> = self
            .valuations
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|v| v.account_id == account_id)
            .cloned()
            .collect();
        found.sort_by_key(|v| (v.as_of, v.recorded_at));
        Ok(found)
    }
}

/// In-memory [`ExchangeRateSource`] with the seven-day fallback.
#[derive(Default)]
pub struct FakeRates {
    /// Units per euro, keyed by currency and date.
    pub rates: Mutex<HashMap<(Currency, Date), Decimal>>,
}

impl FakeRates {
    /// Adds a rate.
    #[must_use]
    pub fn with(self, currency: Currency, on: Date, units_per_eur: Decimal) -> Self {
        if let Ok(mut rates) = self.rates.lock() {
            rates.insert((currency, on), units_per_eur);
        }
        self
    }
}

#[async_trait]
impl ExchangeRateSource for FakeRates {
    async fn units_per_eur(
        &self,
        currency: Currency,
        on: Date,
    ) -> Result<Option<Decimal>, WealthError> {
        if currency == Currency::EUR {
            return Ok(Some(Decimal::ONE));
        }
        let rates = self.rates.lock().map_err(|_| poisoned())?;
        Ok((0..=7)
            .filter_map(|days_back| on.checked_sub(Duration::days(days_back)))
            .find_map(|day| rates.get(&(currency, day)).copied()))
    }
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError> {
        Ok(currency == Currency::EUR
            || self
                .rates
                .lock()
                .map_err(|_| poisoned())?
                .keys()
                .any(|(c, _)| *c == currency))
    }
}

/// In-memory [`WrappedKeyStore`].
#[derive(Default)]
pub struct FakeWrappedKeys {
    /// Stored wrapped keys, oldest first.
    pub keys: Mutex<Vec<String>>,
}

#[async_trait]
impl WrappedKeyStore for FakeWrappedKeys {
    async fn current(&self) -> Result<Option<String>, WealthError> {
        Ok(self.keys.lock().map_err(|_| poisoned())?.last().cloned())
    }
    async fn insert(&self, wrapped_key: &str, _egide_key_name: &str) -> Result<(), WealthError> {
        self.keys
            .lock()
            .map_err(|_| poisoned())?
            .push(wrapped_key.to_owned());
        Ok(())
    }
}

/// Deterministic, context-bound [`FieldCipher`] for tests: the output never
/// contains the plaintext in clear, and decrypting with another context fails.
/// Not cryptographically secure.
#[derive(Default)]
pub struct FakeCipher;

const FAKE_MASK: u8 = 0x5A;

impl FieldCipher for FakeCipher {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let aad = context.aad();
        let aad_length = u32::try_from(aad.len()).map_err(|_| WealthError::Cipher)?;
        let mut out = aad_length.to_be_bytes().to_vec();
        out.extend(aad.iter().map(|b| b ^ FAKE_MASK));
        out.extend(plaintext.iter().map(|b| b ^ FAKE_MASK));
        Ok(out)
    }
    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let (length_bytes, rest) = ciphertext.split_at_checked(4).ok_or(WealthError::Cipher)?;
        let length_array: [u8; 4] = length_bytes.try_into().map_err(|_| WealthError::Cipher)?;
        let aad_length =
            usize::try_from(u32::from_be_bytes(length_array)).map_err(|_| WealthError::Cipher)?;
        let (aad_masked, body) = rest
            .split_at_checked(aad_length)
            .ok_or(WealthError::Cipher)?;
        let aad: Vec<u8> = aad_masked.iter().map(|b| b ^ FAKE_MASK).collect();
        if aad != context.aad() {
            return Err(WealthError::Cipher);
        }
        Ok(body.iter().map(|b| b ^ FAKE_MASK).collect())
    }
}
