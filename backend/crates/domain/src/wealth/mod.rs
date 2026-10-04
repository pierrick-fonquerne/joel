//! Personal and company wealth: accounts, valuations and net worth.

pub mod account;
pub mod error;
pub mod money;
pub mod net_worth;
pub mod ports;
pub mod test_support;
pub mod use_cases;
pub mod valuation;

pub use account::{Account, AccountId, AccountKind, BrokerageEnvelope, Owner};
pub use error::WealthError;
pub use money::{Currency, Money, parse_amount};
pub use net_worth::{NetWorth, RateTable, STALE_AFTER_DAYS};
pub use ports::{
    AccountRepository, CipherContext, ExchangeRateSource, FieldCipher, ValuationRepository,
    WrappedKeyStore,
};
pub use use_cases::{AccountSummary, NewAccount, Wealth, month_ends};
pub use valuation::{Valuation, ValuationId, ValuationSource};
