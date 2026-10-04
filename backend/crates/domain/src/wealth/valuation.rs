//! Valuations: the value of an account on a given date.

use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::account::{Account, AccountId};
use super::error::WealthError;
use super::money::Money;

/// Where a valuation comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValuationSource {
    /// Typed by hand.
    Manual,
    /// Imported from a broker CSV export.
    CsvImport,
    /// Pulled from a bank aggregation provider.
    BankAggregation,
    /// Computed from market prices.
    PriceFeed,
}

impl ValuationSource {
    /// Stable storage and API code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::CsvImport => "csv_import",
            Self::BankAggregation => "bank_aggregation",
            Self::PriceFeed => "price_feed",
        }
    }

    /// Parses a storage code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidValuationSource`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        match code {
            "manual" => Ok(Self::Manual),
            "csv_import" => Ok(Self::CsvImport),
            "bank_aggregation" => Ok(Self::BankAggregation),
            "price_feed" => Ok(Self::PriceFeed),
            _ => Err(WealthError::InvalidValuationSource),
        }
    }
}

/// Valuation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValuationId(pub Uuid);

impl ValuationId {
    /// Generates a new time-ordered identifier.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

/// The value of an account on a date. Valuations are appended, never modified:
/// a correction is a newer valuation with the same `as_of`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valuation {
    /// Identifier.
    pub id: ValuationId,
    /// Valued account.
    pub account_id: AccountId,
    /// Date the value applies to.
    pub as_of: Date,
    /// Value, in the account currency.
    pub amount: Money,
    /// Origin of the value.
    pub source: ValuationSource,
    /// Instant the valuation was recorded; breaks ties between corrections.
    pub recorded_at: OffsetDateTime,
}

impl Valuation {
    /// Records a valuation after checking every invariant against its account.
    /// `as_of` may be at most one day after the UTC date of `now`, so that a
    /// user ahead of UTC (Paris after midnight) can date a valuation today.
    ///
    /// # Errors
    /// [`WealthError::ArchivedAccount`], [`WealthError::CurrencyMismatch`],
    /// [`WealthError::FutureValuation`], [`WealthError::PositiveLoanValuation`]
    /// or [`WealthError::NegativeAssetValuation`].
    pub fn record(
        account: &Account,
        as_of: Date,
        amount: Money,
        source: ValuationSource,
        now: OffsetDateTime,
    ) -> Result<Self, WealthError> {
        if account.is_archived {
            return Err(WealthError::ArchivedAccount);
        }
        if amount.currency != account.currency {
            return Err(WealthError::CurrencyMismatch);
        }
        let latest_allowed = now.date().next_day().unwrap_or(now.date());
        if as_of > latest_allowed {
            return Err(WealthError::FutureValuation);
        }
        if account.kind.is_liability()
            && amount.amount.is_sign_positive()
            && !amount.amount.is_zero()
        {
            return Err(WealthError::PositiveLoanValuation);
        }
        if !account.kind.is_liability()
            && amount.amount.is_sign_negative()
            && !amount.amount.is_zero()
        {
            return Err(WealthError::NegativeAssetValuation);
        }
        Ok(Self {
            id: ValuationId::generate(),
            account_id: account.id,
            as_of,
            amount,
            source,
            recorded_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use rust_decimal::Decimal;
    use time::macros::{date, datetime};

    use super::*;
    use crate::wealth::account::{AccountKind, Owner};
    use crate::wealth::money::Currency;

    const NOW: OffsetDateTime = datetime!(2026-10-04 22:30 UTC);

    fn account(kind: AccountKind) -> Account {
        Account::open("Compte", kind, Owner::Personal, Currency::EUR, None, NOW).unwrap()
    }

    fn eur(units: i64) -> Money {
        Money::eur(Decimal::new(units, 0))
    }

    #[test]
    fn records_a_valid_asset_valuation() {
        let account = account(AccountKind::Savings);
        let valuation = Valuation::record(
            &account,
            date!(2026 - 10 - 04),
            eur(1000),
            ValuationSource::Manual,
            NOW,
        )
        .unwrap();
        assert_eq!(valuation.account_id, account.id);
        assert_eq!(valuation.recorded_at, NOW);
    }

    #[test]
    fn accepts_tomorrow_utc_for_timezones_ahead_of_utc() {
        let account = account(AccountKind::Savings);
        assert!(
            Valuation::record(
                &account,
                date!(2026 - 10 - 05),
                eur(1),
                ValuationSource::Manual,
                NOW
            )
            .is_ok()
        );
        assert_eq!(
            Valuation::record(
                &account,
                date!(2026 - 10 - 06),
                eur(1),
                ValuationSource::Manual,
                NOW
            ),
            Err(WealthError::FutureValuation)
        );
    }

    #[test]
    fn loans_must_be_negative_and_assets_positive() {
        let loan = account(AccountKind::Loan);
        assert!(
            Valuation::record(
                &loan,
                date!(2026 - 10 - 01),
                eur(-1),
                ValuationSource::Manual,
                NOW
            )
            .is_ok()
        );
        assert_eq!(
            Valuation::record(
                &loan,
                date!(2026 - 10 - 01),
                eur(1),
                ValuationSource::Manual,
                NOW
            ),
            Err(WealthError::PositiveLoanValuation)
        );
        let asset = account(AccountKind::RealEstate);
        assert_eq!(
            Valuation::record(
                &asset,
                date!(2026 - 10 - 01),
                eur(-1),
                ValuationSource::Manual,
                NOW
            ),
            Err(WealthError::NegativeAssetValuation)
        );
        assert!(
            Valuation::record(
                &asset,
                date!(2026 - 10 - 01),
                eur(0),
                ValuationSource::Manual,
                NOW
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_currency_mismatch_and_archived_accounts() {
        let mut account = account(AccountKind::BankAccount);
        let usd = Money::new(Decimal::ONE, "USD".parse().unwrap());
        assert_eq!(
            Valuation::record(
                &account,
                date!(2026 - 10 - 01),
                usd,
                ValuationSource::Manual,
                NOW
            ),
            Err(WealthError::CurrencyMismatch)
        );
        account.is_archived = true;
        assert_eq!(
            Valuation::record(
                &account,
                date!(2026 - 10 - 01),
                eur(1),
                ValuationSource::Manual,
                NOW
            ),
            Err(WealthError::ArchivedAccount)
        );
    }

    #[test]
    fn source_codes_roundtrip() {
        for source in [
            ValuationSource::Manual,
            ValuationSource::CsvImport,
            ValuationSource::BankAggregation,
            ValuationSource::PriceFeed,
        ] {
            assert_eq!(ValuationSource::from_code(source.code()).unwrap(), source);
        }
        assert_eq!(
            ValuationSource::from_code("guess"),
            Err(WealthError::InvalidValuationSource)
        );
    }
}
