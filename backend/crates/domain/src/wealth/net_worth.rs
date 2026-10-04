//! Net worth: the sum of the latest valuation of every active account, in euros.

use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;
use time::Date;

use super::account::{Account, AccountId, AccountKind, Owner};
use super::error::WealthError;
use super::money::{Currency, Money};
use super::valuation::Valuation;

/// A valuation older than this many days marks its account as stale.
pub const STALE_AFTER_DAYS: i64 = 45;

/// Units of a currency for one euro, keyed by currency and valuation date.
#[derive(Debug, Default, Clone)]
pub struct RateTable(pub HashMap<(Currency, Date), Decimal>);

/// Net worth on a date, in euros, with its breakdowns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetWorth {
    /// Date of the computation.
    pub as_of: Date,
    /// Total in euros.
    pub total: Money,
    /// Total per owner, in euros; owners without account are absent.
    pub by_owner: BTreeMap<Owner, Money>,
    /// Total per account kind, in euros; kinds without account are absent.
    pub by_kind: BTreeMap<AccountKind, Money>,
    /// Active accounts without valuation, or whose latest valuation is older
    /// than [`STALE_AFTER_DAYS`] days before `as_of`.
    pub stale_accounts: Vec<AccountId>,
}

impl NetWorth {
    /// Computes the net worth on `as_of`. Archived accounts are ignored. Each
    /// converted amount is rounded to the cent before summing.
    ///
    /// # Errors
    /// [`WealthError::ExchangeRateMissing`] when a non-euro valuation has no rate
    /// in `rates` for its own `as_of`.
    pub fn compute(
        as_of: Date,
        accounts: &[Account],
        latest: &[Valuation],
        rates: &RateTable,
    ) -> Result<Self, WealthError> {
        let latest_by_account: HashMap<AccountId, &Valuation> = latest
            .iter()
            .filter(|v| v.as_of <= as_of)
            .map(|v| (v.account_id, v))
            .collect();
        let mut total = Decimal::ZERO;
        let mut by_owner: BTreeMap<Owner, Money> = BTreeMap::new();
        let mut by_kind: BTreeMap<AccountKind, Money> = BTreeMap::new();
        let mut stale_accounts = Vec::new();

        for account in accounts.iter().filter(|a| !a.is_archived) {
            let Some(valuation) = latest_by_account.get(&account.id) else {
                stale_accounts.push(account.id);
                continue;
            };
            if (as_of - valuation.as_of).whole_days() > STALE_AFTER_DAYS {
                stale_accounts.push(account.id);
            }
            let in_euros = to_euros(valuation, rates)?;
            total += in_euros;
            add(&mut by_owner, account.owner, in_euros);
            add(&mut by_kind, account.kind, in_euros);
        }

        Ok(Self {
            as_of,
            total: Money::eur(total),
            by_owner,
            by_kind,
            stale_accounts,
        })
    }
}

fn to_euros(valuation: &Valuation, rates: &RateTable) -> Result<Decimal, WealthError> {
    let amount = valuation.amount;
    if amount.currency == Currency::EUR {
        return Ok(amount.amount.round_dp(2));
    }
    let missing = || WealthError::ExchangeRateMissing {
        currency: amount.currency,
        on: valuation.as_of,
    };
    let units_per_eur = rates
        .0
        .get(&(amount.currency, valuation.as_of))
        .ok_or_else(missing)?;
    amount
        .amount
        .checked_div(*units_per_eur)
        .map(|eur| eur.round_dp(2))
        .ok_or_else(missing)
}

fn add<K: Ord>(totals: &mut BTreeMap<K, Money>, key: K, amount: Decimal) {
    totals
        .entry(key)
        .and_modify(|money| money.amount += amount)
        .or_insert_with(|| Money::eur(amount));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::OffsetDateTime;
    use time::macros::{date, datetime};

    use super::*;
    use crate::wealth::account::BrokerageEnvelope;
    use crate::wealth::valuation::ValuationSource;

    const NOW: OffsetDateTime = datetime!(2026-10-04 12:00 UTC);

    fn account(kind: AccountKind, owner: Owner, currency: Currency) -> Account {
        Account::open("Compte", kind, owner, currency, None, NOW).unwrap()
    }

    fn valuation(account: &Account, as_of: Date, units: i64) -> Valuation {
        Valuation::record(
            account,
            as_of,
            Money::new(Decimal::new(units, 0), account.currency),
            ValuationSource::Manual,
            NOW,
        )
        .unwrap()
    }

    #[test]
    fn sums_assets_and_subtracts_loans_with_breakdowns() {
        let pea = account(
            AccountKind::Brokerage {
                envelope: BrokerageEnvelope::Pea,
            },
            Owner::Personal,
            Currency::EUR,
        );
        let house = account(AccountKind::RealEstate, Owner::Personal, Currency::EUR);
        let loan = account(AccountKind::Loan, Owner::Personal, Currency::EUR);
        let treasury = account(AccountKind::BankAccount, Owner::Company, Currency::EUR);
        let latest = vec![
            valuation(&pea, date!(2026 - 10 - 01), 50_000),
            valuation(&house, date!(2026 - 09 - 30), 300_000),
            valuation(&loan, date!(2026 - 09 - 30), -180_000),
            valuation(&treasury, date!(2026 - 10 - 02), 40_000),
        ];
        let accounts = vec![pea, house, loan, treasury];

        let net_worth = NetWorth::compute(
            date!(2026 - 10 - 04),
            &accounts,
            &latest,
            &RateTable::default(),
        )
        .unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(210_000, 0)));
        assert_eq!(
            net_worth.by_owner[&Owner::Personal],
            Money::eur(Decimal::new(170_000, 0))
        );
        assert_eq!(
            net_worth.by_owner[&Owner::Company],
            Money::eur(Decimal::new(40_000, 0))
        );
        assert_eq!(
            net_worth.by_kind[&AccountKind::Loan],
            Money::eur(Decimal::new(-180_000, 0))
        );
        assert!(net_worth.stale_accounts.is_empty());
    }

    #[test]
    fn converts_with_the_rate_of_the_valuation_date_and_rounds_to_the_cent() {
        let usd: Currency = "USD".parse().unwrap();
        let broker = account(
            AccountKind::Brokerage {
                envelope: BrokerageEnvelope::Cto,
            },
            Owner::Personal,
            usd,
        );
        let latest = vec![valuation(&broker, date!(2026 - 10 - 02), 1000)];
        let mut rates = RateTable::default();
        rates
            .0
            .insert((usd, date!(2026 - 10 - 02)), Decimal::new(10_850, 4));

        let net_worth =
            NetWorth::compute(date!(2026 - 10 - 04), &[broker], &latest, &rates).unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(92_166, 2)));
    }

    #[test]
    fn missing_rate_is_an_explicit_error() {
        let usd: Currency = "USD".parse().unwrap();
        let broker = account(AccountKind::Savings, Owner::Personal, usd);
        let latest = vec![valuation(&broker, date!(2026 - 10 - 02), 1000)];

        let result = NetWorth::compute(
            date!(2026 - 10 - 04),
            &[broker],
            &latest,
            &RateTable::default(),
        );

        assert_eq!(
            result,
            Err(WealthError::ExchangeRateMissing {
                currency: usd,
                on: date!(2026 - 10 - 02)
            })
        );
    }

    #[test]
    fn ignores_archived_accounts_and_flags_stale_ones() {
        let mut archived = account(AccountKind::Savings, Owner::Personal, Currency::EUR);
        let old = account(AccountKind::LifeInsurance, Owner::Personal, Currency::EUR);
        let empty = account(AccountKind::RetirementPlan, Owner::Personal, Currency::EUR);
        let latest = vec![
            valuation(&archived, date!(2026 - 10 - 01), 999),
            valuation(&old, date!(2026 - 08 - 19), 10_000),
        ];
        archived.is_archived = true;
        let (old_id, empty_id) = (old.id, empty.id);

        let net_worth = NetWorth::compute(
            date!(2026 - 10 - 04),
            &[archived, old, empty],
            &latest,
            &RateTable::default(),
        )
        .unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(10_000, 0)));
        assert_eq!(net_worth.stale_accounts, vec![old_id, empty_id]);
    }

    #[test]
    fn a_valuation_exactly_45_days_old_is_not_stale() {
        let account = account(AccountKind::LifeInsurance, Owner::Personal, Currency::EUR);
        let latest = vec![valuation(&account, date!(2026 - 08 - 20), 1)];
        let net_worth = NetWorth::compute(
            date!(2026 - 10 - 04),
            &[account],
            &latest,
            &RateTable::default(),
        )
        .unwrap();
        assert!(net_worth.stale_accounts.is_empty());
    }
}
