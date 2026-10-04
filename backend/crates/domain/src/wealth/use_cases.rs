//! Wealth use cases, orchestrating the ports.

use std::sync::Arc;

use rust_decimal::Decimal;
use time::{Date, Month, OffsetDateTime};

use super::account::{Account, AccountId, AccountKind, Owner};
use super::error::WealthError;
use super::money::{Currency, Money};
use super::net_worth::{NetWorth, RateTable, STALE_AFTER_DAYS};
use super::ports::{AccountRepository, ExchangeRateSource, ValuationRepository};
use super::valuation::{Valuation, ValuationSource};

/// Input of [`Wealth::create_account`].
#[derive(Debug, Clone)]
pub struct NewAccount {
    /// Human name.
    pub name: String,
    /// Nature.
    pub kind: AccountKind,
    /// Owner.
    pub owner: Owner,
    /// Currency of every valuation.
    pub currency: Currency,
    /// Free notes.
    pub notes: Option<String>,
}

/// An account with its latest valuation, for lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    /// The account.
    pub account: Account,
    /// Latest valuation on or before the reference date.
    pub latest_valuation: Option<Valuation>,
    /// True when the account has no valuation or a valuation older than
    /// [`STALE_AFTER_DAYS`] days. Always false for archived accounts.
    pub is_stale: bool,
}

/// Wealth use cases.
pub struct Wealth {
    accounts: Arc<dyn AccountRepository>,
    valuations: Arc<dyn ValuationRepository>,
    rates: Arc<dyn ExchangeRateSource>,
}

impl Wealth {
    /// Wires the use cases to their ports.
    #[must_use]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        valuations: Arc<dyn ValuationRepository>,
        rates: Arc<dyn ExchangeRateSource>,
    ) -> Self {
        Self {
            accounts,
            valuations,
            rates,
        }
    }

    /// Opens an account in a supported currency.
    ///
    /// # Errors
    /// [`WealthError::UnsupportedCurrency`], [`WealthError::EmptyAccountName`], storage errors.
    pub async fn create_account(
        &self,
        input: NewAccount,
        now: OffsetDateTime,
    ) -> Result<Account, WealthError> {
        if !self.rates.is_supported(input.currency).await? {
            return Err(WealthError::UnsupportedCurrency(input.currency));
        }
        let account = Account::open(
            &input.name,
            input.kind,
            input.owner,
            input.currency,
            input.notes.as_deref(),
            now,
        )?;
        self.accounts.insert(&account).await?;
        Ok(account)
    }

    /// Archives an account.
    ///
    /// # Errors
    /// [`WealthError::AccountNotFound`], storage errors.
    pub async fn archive_account(&self, id: AccountId) -> Result<(), WealthError> {
        self.accounts.archive(id).await
    }

    /// Lists every account with its latest valuation on or before `today`.
    ///
    /// # Errors
    /// Storage and cipher errors.
    pub async fn account_summaries(&self, today: Date) -> Result<Vec<AccountSummary>, WealthError> {
        let accounts = self.accounts.list().await?;
        let latest = self.valuations.latest_per_account(today).await?;
        Ok(accounts
            .into_iter()
            .map(|account| {
                let latest_valuation = latest.iter().find(|v| v.account_id == account.id).cloned();
                let is_stale = !account.is_archived
                    && latest_valuation
                        .as_ref()
                        .is_none_or(|v| (today - v.as_of).whole_days() > STALE_AFTER_DAYS);
                AccountSummary {
                    account,
                    latest_valuation,
                    is_stale,
                }
            })
            .collect())
    }

    /// Records a manual valuation in the account currency.
    ///
    /// # Errors
    /// [`WealthError::AccountNotFound`] and every invariant of [`Valuation::record`].
    pub async fn record_valuation(
        &self,
        account_id: AccountId,
        as_of: Date,
        amount: Decimal,
        now: OffsetDateTime,
    ) -> Result<Valuation, WealthError> {
        let account = self
            .accounts
            .find(account_id)
            .await?
            .ok_or(WealthError::AccountNotFound)?;
        let valuation = Valuation::record(
            &account,
            as_of,
            Money::new(amount, account.currency),
            ValuationSource::Manual,
            now,
        )?;
        self.valuations.insert(&valuation).await?;
        Ok(valuation)
    }

    /// Every valuation of an account, oldest first.
    ///
    /// # Errors
    /// [`WealthError::AccountNotFound`], storage errors.
    pub async fn account_history(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<Valuation>, WealthError> {
        if self.accounts.find(account_id).await?.is_none() {
            return Err(WealthError::AccountNotFound);
        }
        self.valuations.for_account(account_id).await
    }

    /// Net worth on `at`.
    ///
    /// # Errors
    /// [`WealthError::ExchangeRateMissing`], storage errors.
    pub async fn net_worth(&self, at: Date) -> Result<NetWorth, WealthError> {
        let accounts = self.accounts.list().await?;
        let latest = self.valuations.latest_per_account(at).await?;
        let mut rates = RateTable::default();
        for valuation in &latest {
            let currency = valuation.amount.currency;
            let is_active = accounts
                .iter()
                .any(|a| a.id == valuation.account_id && !a.is_archived);
            if currency == Currency::EUR || !is_active {
                continue;
            }
            let units_per_eur = self
                .rates
                .units_per_eur(currency, valuation.as_of)
                .await?
                .ok_or(WealthError::ExchangeRateMissing {
                    currency,
                    on: valuation.as_of,
                })?;
            rates.0.insert((currency, valuation.as_of), units_per_eur);
        }
        NetWorth::compute(at, &accounts, &latest, &rates)
    }

    /// One net worth per month end in `[from, to)`, then one on `to`.
    ///
    /// # Errors
    /// [`WealthError::InvalidRange`] when `to < from`, and the errors of [`Self::net_worth`].
    pub async fn net_worth_history(
        &self,
        from: Date,
        to: Date,
    ) -> Result<Vec<NetWorth>, WealthError> {
        if to < from {
            return Err(WealthError::InvalidRange);
        }
        let mut points = Vec::new();
        for day in month_ends(from, to) {
            points.push(self.net_worth(day).await?);
        }
        Ok(points)
    }
}

/// Last day of each month from the month of `from`, strictly before `to`,
/// followed by `to` itself. Empty when `to < from`.
#[must_use]
pub fn month_ends(from: Date, to: Date) -> Vec<Date> {
    if to < from {
        return Vec::new();
    }
    let mut points = Vec::new();
    let mut cursor = last_day_of_month(from);
    while cursor < to {
        points.push(cursor);
        let Some(next_month_day) = cursor.next_day() else {
            break;
        };
        cursor = last_day_of_month(next_month_day);
    }
    points.push(to);
    points
}

fn last_day_of_month(date: Date) -> Date {
    let (year, month) = if date.month() == Month::December {
        (date.year() + 1, Month::January)
    } else {
        (date.year(), date.month().next())
    };
    Date::from_calendar_date(year, month, 1)
        .ok()
        .and_then(Date::previous_day)
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::macros::{date, datetime};

    use super::*;
    use crate::wealth::test_support::{FakeAccounts, FakeRates, FakeValuations};

    const NOW: OffsetDateTime = datetime!(2026-10-04 12:00 UTC);

    fn wealth_with(rates: FakeRates) -> Wealth {
        Wealth::new(
            Arc::new(FakeAccounts::default()),
            Arc::new(FakeValuations::default()),
            Arc::new(rates),
        )
    }

    fn savings(currency: Currency) -> NewAccount {
        NewAccount {
            name: "Livret A".into(),
            kind: AccountKind::Savings,
            owner: Owner::Personal,
            currency,
            notes: None,
        }
    }

    #[tokio::test]
    async fn create_account_rejects_currency_without_rate() {
        let wealth = wealth_with(FakeRates::default());
        let btc: Currency = "BTC".parse().unwrap();
        assert_eq!(
            wealth.create_account(savings(btc), NOW).await,
            Err(WealthError::UnsupportedCurrency(btc))
        );
        assert!(
            wealth
                .create_account(savings(Currency::EUR), NOW)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn record_valuation_uses_account_currency_and_checks_existence() {
        let wealth = wealth_with(FakeRates::default());
        let account = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        let valuation = wealth
            .record_valuation(
                account.id,
                date!(2026 - 10 - 04),
                Decimal::new(22_950, 0),
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(valuation.amount, Money::eur(Decimal::new(22_950, 0)));
        assert_eq!(valuation.source, ValuationSource::Manual);
        assert_eq!(
            wealth
                .record_valuation(
                    AccountId::generate(),
                    date!(2026 - 10 - 04),
                    Decimal::ONE,
                    NOW
                )
                .await,
            Err(WealthError::AccountNotFound)
        );
    }

    #[tokio::test]
    async fn correction_on_same_date_wins() {
        let wealth = wealth_with(FakeRates::default());
        let account = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        wealth
            .record_valuation(
                account.id,
                date!(2026 - 10 - 01),
                Decimal::new(1000, 0),
                NOW,
            )
            .await
            .unwrap();
        wealth
            .record_valuation(
                account.id,
                date!(2026 - 10 - 01),
                Decimal::new(1100, 0),
                NOW + time::Duration::minutes(1),
            )
            .await
            .unwrap();
        let net_worth = wealth.net_worth(date!(2026 - 10 - 04)).await.unwrap();
        assert_eq!(net_worth.total, Money::eur(Decimal::new(1100, 0)));
    }

    #[tokio::test]
    async fn net_worth_falls_back_to_previous_business_day_rate() {
        let usd: Currency = "USD".parse().unwrap();
        let rates = FakeRates::default().with(usd, date!(2026 - 10 - 02), Decimal::new(2, 0));
        let wealth = wealth_with(rates);
        let account = wealth.create_account(savings(usd), NOW).await.unwrap();
        wealth
            .record_valuation(account.id, date!(2026 - 10 - 04), Decimal::new(100, 0), NOW)
            .await
            .unwrap();
        let net_worth = wealth.net_worth(date!(2026 - 10 - 04)).await.unwrap();
        assert_eq!(net_worth.total, Money::eur(Decimal::new(50, 0)));
    }

    #[tokio::test]
    async fn net_worth_reports_missing_rate_beyond_seven_days() {
        let usd: Currency = "USD".parse().unwrap();
        let rates = FakeRates::default().with(usd, date!(2026 - 09 - 20), Decimal::new(2, 0));
        let wealth = wealth_with(rates);
        let account = wealth.create_account(savings(usd), NOW).await.unwrap();
        wealth
            .record_valuation(account.id, date!(2026 - 10 - 04), Decimal::new(100, 0), NOW)
            .await
            .unwrap();
        assert_eq!(
            wealth.net_worth(date!(2026 - 10 - 04)).await,
            Err(WealthError::ExchangeRateMissing {
                currency: usd,
                on: date!(2026 - 10 - 04)
            })
        );
    }

    #[tokio::test]
    async fn summaries_flag_stale_accounts_but_not_archived_ones() {
        let wealth = wealth_with(FakeRates::default());
        let fresh = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        let never_valued = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        let archived = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        wealth
            .record_valuation(fresh.id, date!(2026 - 10 - 01), Decimal::ONE, NOW)
            .await
            .unwrap();
        wealth.archive_account(archived.id).await.unwrap();

        let summaries = wealth
            .account_summaries(date!(2026 - 10 - 04))
            .await
            .unwrap();

        let stale_of = |id| {
            summaries
                .iter()
                .find(|s| s.account.id == id)
                .unwrap()
                .is_stale
        };
        assert!(!stale_of(fresh.id));
        assert!(stale_of(never_valued.id));
        assert!(!stale_of(archived.id));
    }

    #[tokio::test]
    async fn staleness_boundary_is_forty_five_days() {
        let wealth = wealth_with(FakeRates::default());
        let on_boundary = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        let past_boundary = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        wealth
            .record_valuation(on_boundary.id, date!(2026 - 08 - 20), Decimal::ONE, NOW)
            .await
            .unwrap();
        wealth
            .record_valuation(past_boundary.id, date!(2026 - 08 - 19), Decimal::ONE, NOW)
            .await
            .unwrap();

        let summaries = wealth
            .account_summaries(date!(2026 - 10 - 04))
            .await
            .unwrap();

        let stale_of = |id| {
            summaries
                .iter()
                .find(|s| s.account.id == id)
                .unwrap()
                .is_stale
        };
        assert!(!stale_of(on_boundary.id));
        assert!(stale_of(past_boundary.id));
    }

    #[tokio::test]
    async fn history_returns_one_point_per_month_end_then_the_end_date() {
        let wealth = wealth_with(FakeRates::default());
        let account = wealth
            .create_account(savings(Currency::EUR), NOW)
            .await
            .unwrap();
        wealth
            .record_valuation(
                account.id,
                date!(2026 - 08 - 15),
                Decimal::new(1000, 0),
                NOW,
            )
            .await
            .unwrap();
        wealth
            .record_valuation(
                account.id,
                date!(2026 - 09 - 20),
                Decimal::new(1500, 0),
                NOW,
            )
            .await
            .unwrap();

        let points = wealth
            .net_worth_history(date!(2026 - 08 - 01), date!(2026 - 10 - 04))
            .await
            .unwrap();

        let observed: Vec<_> = points
            .iter()
            .map(|point| (point.as_of, point.total))
            .collect();
        assert_eq!(
            observed,
            vec![
                (date!(2026 - 08 - 31), Money::eur(Decimal::new(1000, 0))),
                (date!(2026 - 09 - 30), Money::eur(Decimal::new(1500, 0))),
                (date!(2026 - 10 - 04), Money::eur(Decimal::new(1500, 0))),
            ]
        );
    }

    #[tokio::test]
    async fn net_worth_skips_archived_account_without_rate() {
        let usd: Currency = "USD".parse().unwrap();
        let rates = FakeRates::default().with(usd, date!(2026 - 10 - 02), Decimal::new(2, 0));
        let wealth = wealth_with(rates);
        let account = wealth.create_account(savings(usd), NOW).await.unwrap();
        wealth
            .record_valuation(account.id, date!(2026 - 01 - 05), Decimal::new(100, 0), NOW)
            .await
            .unwrap();
        wealth.archive_account(account.id).await.unwrap();

        let net_worth = wealth.net_worth(date!(2026 - 10 - 04)).await.unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::ZERO));
    }

    #[tokio::test]
    async fn history_rejects_inverted_range() {
        let wealth = wealth_with(FakeRates::default());
        assert_eq!(
            wealth
                .net_worth_history(date!(2026 - 10 - 04), date!(2026 - 01 - 01))
                .await,
            Err(WealthError::InvalidRange)
        );
    }

    #[test]
    fn month_ends_lists_each_month_end_then_the_end_date() {
        assert_eq!(
            month_ends(date!(2026 - 01 - 15), date!(2026 - 03 - 10)),
            vec![
                date!(2026 - 01 - 31),
                date!(2026 - 02 - 28),
                date!(2026 - 03 - 10)
            ]
        );
        assert_eq!(
            month_ends(date!(2025 - 12 - 01), date!(2026 - 01 - 31)),
            vec![date!(2025 - 12 - 31), date!(2026 - 01 - 31)]
        );
        assert_eq!(
            month_ends(date!(2026 - 10 - 04), date!(2026 - 10 - 04)),
            vec![date!(2026 - 10 - 04)]
        );
        assert!(month_ends(date!(2026 - 10 - 04), date!(2026 - 10 - 01)).is_empty());
    }
}
