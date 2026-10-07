//! Periodic refresh of the ECB exchange rate cache used by the wealth module.

use std::time::{Duration, Instant};

use domain::wealth::WealthError;
use exchange_rates_ecb::{EcbError, parse_feed};
use persistence::wealth::PgExchangeRates;

/// Delay between two refreshes. The ECB publishes once per business day.
pub const REFRESH_INTERVAL: Duration = Duration::from_hours(6);

/// Minimum wait before retrying after a failed refresh.
pub const FAILURE_BACKOFF: Duration = Duration::from_mins(30);

/// Whether a refresh should run now: at least [`REFRESH_INTERVAL`] after the
/// last success and at least [`FAILURE_BACKOFF`] after the last failure.
#[must_use]
pub fn refresh_due(
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    now: Instant,
) -> bool {
    let waited = |since: Option<Instant>, delay: Duration| {
        since.is_none_or(|at| now.saturating_duration_since(at) >= delay)
    };
    waited(last_success, REFRESH_INTERVAL) && waited(last_failure, FAILURE_BACKOFF)
}

/// Refresh failures.
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    /// Feed download or parsing failed.
    #[error(transparent)]
    Feed(#[from] EcbError),
    /// Database write failed.
    #[error(transparent)]
    Store(#[from] WealthError),
}

/// Parses an ECB document and stores its rates; returns the number of new rows.
///
/// # Errors
/// See [`RefreshError`].
pub async fn refresh_from_feed(xml: &str, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    let rates: Vec<_> = parse_feed(xml)?
        .into_iter()
        .map(|rate| (rate.currency, rate.on, rate.units_per_eur))
        .collect();
    Ok(store.store_rates(&rates).await?)
}

/// Downloads the rates and stores them: the full history when the cache is
/// empty (so backdated valuations can be converted), the last 90 days otherwise.
///
/// # Errors
/// See [`RefreshError`].
pub async fn refresh(http: &reqwest::Client, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    let feed = if store.is_empty().await? {
        exchange_rates_ecb::fetch_full_history(http).await?
    } else {
        exchange_rates_ecb::fetch_last_90_days(http).await?
    };
    let rates: Vec<_> = feed
        .into_iter()
        .map(|rate| (rate.currency, rate.on, rate.units_per_eur))
        .collect();
    Ok(store.store_rates(&rates).await?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::ExchangeRateSource;
    use rust_decimal::Decimal;
    use sqlx::PgPool;
    use time::macros::date;

    use super::*;

    #[test]
    fn refresh_is_due_without_history() {
        assert!(refresh_due(None, None, Instant::now()));
    }

    #[test]
    fn success_postpones_the_next_refresh_for_six_hours() {
        let start = Instant::now();
        let now = start + Duration::from_hours(10);
        assert!(!refresh_due(
            Some(start + Duration::from_hours(9)),
            None,
            now
        ));
        assert!(refresh_due(
            Some(start + Duration::from_hours(3)),
            None,
            now
        ));
    }

    #[test]
    fn failure_postpones_the_retry_for_thirty_minutes() {
        let start = Instant::now();
        let now = start + Duration::from_hours(10);
        let recent = start + Duration::from_mins(590);
        let old = start + Duration::from_mins(569);
        assert!(!refresh_due(None, Some(recent), now));
        assert!(refresh_due(None, Some(old), now));
        assert!(!refresh_due(Some(start), Some(recent), now));
    }

    const SAMPLE: &str = include_str!(
        "../../../crates/exchange-rates-ecb/tests/fixtures/eurofxref-hist-90d-sample.xml"
    );

    #[sqlx::test(migrations = "../../migrations")]
    async fn refresh_stores_new_rates_once(pool: PgPool) {
        let store = PgExchangeRates::new(pool);
        assert_eq!(refresh_from_feed(SAMPLE, &store).await.unwrap(), 3);
        assert_eq!(refresh_from_feed(SAMPLE, &store).await.unwrap(), 0);
        assert_eq!(
            store
                .units_per_eur("USD".parse().unwrap(), date!(2026 - 10 - 02))
                .await
                .unwrap(),
            Some(Decimal::new(10_850, 4))
        );
    }
}
