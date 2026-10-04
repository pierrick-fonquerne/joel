//! `PostgreSQL` adapters for the wealth ports. Sensitive fields are encrypted
//! through [`domain::wealth::FieldCipher`] before reaching the database.

pub mod accounts;
pub mod exchange_rates;
mod padding;
pub mod valuations;
pub mod wrapped_keys;

pub use accounts::PgAccounts;
pub use exchange_rates::PgExchangeRates;
pub use valuations::PgValuations;
pub use wrapped_keys::PgWrappedKeys;

use domain::wealth::WealthError;

pub(crate) fn storage(error: &sqlx::Error) -> WealthError {
    WealthError::Storage(error.to_string())
}
