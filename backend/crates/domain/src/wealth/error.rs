//! Errors raised by the wealth domain.

use time::Date;

use super::money::Currency;

/// Every failure of the wealth module, with a stable business code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WealthError {
    /// The currency code is not three uppercase ASCII letters.
    #[error("invalid currency code")]
    InvalidCurrency,
    /// No exchange rate is published for this currency.
    #[error("unsupported currency: {0}")]
    UnsupportedCurrency(Currency),
    /// The amount is not a plain decimal with at most two fraction digits.
    #[error("invalid amount")]
    InvalidAmount,
    /// The account kind code is unknown.
    #[error("invalid account kind")]
    InvalidAccountKind,
    /// The owner code is unknown.
    #[error("invalid owner")]
    InvalidOwner,
    /// The valuation source code is unknown.
    #[error("invalid valuation source")]
    InvalidValuationSource,
    /// The account name is empty once trimmed.
    #[error("account name must not be empty")]
    EmptyAccountName,
    /// A loan valuation must be negative or zero.
    #[error("loan valuations must be negative or zero")]
    PositiveLoanValuation,
    /// An asset valuation must be positive or zero.
    #[error("asset valuations must be positive or zero")]
    NegativeAssetValuation,
    /// The valuation currency differs from the account currency.
    #[error("valuation currency differs from account currency")]
    CurrencyMismatch,
    /// The valuation date is after tomorrow (UTC).
    #[error("valuation date is in the future")]
    FutureValuation,
    /// The account is archived and accepts no new valuation.
    #[error("account is archived")]
    ArchivedAccount,
    /// The requested date range ends before it starts.
    #[error("invalid date range")]
    InvalidRange,
    /// No account matches the identifier.
    #[error("account not found")]
    AccountNotFound,
    /// No exchange rate was found within seven days before the date.
    #[error("no exchange rate for {currency} on {on}")]
    ExchangeRateMissing {
        /// Currency to convert.
        currency: Currency,
        /// Valuation date that needed a rate.
        on: Date,
    },
    /// The encryption key is not available (vault sealed or unreachable).
    #[error("wealth vault is sealed")]
    VaultSealed,
    /// A ciphertext could not be produced or opened.
    #[error("cipher failure")]
    Cipher,
    /// Storage adapter failure.
    #[error("storage failure: {0}")]
    Storage(String),
}

impl WealthError {
    /// Stable machine-readable code, exposed by the API.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidCurrency => "invalid_currency",
            Self::UnsupportedCurrency(_) => "unsupported_currency",
            Self::InvalidAmount => "invalid_amount",
            Self::InvalidAccountKind => "invalid_account_kind",
            Self::InvalidOwner => "invalid_owner",
            Self::InvalidValuationSource => "invalid_valuation_source",
            Self::EmptyAccountName => "empty_account_name",
            Self::PositiveLoanValuation => "positive_loan_valuation",
            Self::NegativeAssetValuation => "negative_asset_valuation",
            Self::CurrencyMismatch => "currency_mismatch",
            Self::FutureValuation => "future_valuation",
            Self::ArchivedAccount => "archived_account",
            Self::InvalidRange => "invalid_range",
            Self::AccountNotFound => "account_not_found",
            Self::ExchangeRateMissing { .. } => "exchange_rate_missing",
            Self::VaultSealed => "wealth_vault_sealed",
            Self::Cipher | Self::Storage(_) => "internal",
        }
    }
}
