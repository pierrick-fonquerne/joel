//! Periodic refresh of the ECB exchange rate cache used by the wealth module.

use std::time::Duration;

use domain::wealth::WealthError;
use exchange_rates_ecb::{EcbError, parse_feed};
use persistence::wealth::PgExchangeRates;

/// Delay between two refreshes. The ECB publishes once per business day.
pub const REFRESH_INTERVAL: Duration = Duration::from_hours(6);

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

/// Downloads the last 90 days and stores them.
///
/// # Errors
/// See [`RefreshError`].
pub async fn refresh(http: &reqwest::Client, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    let rates: Vec<_> = exchange_rates_ecb::fetch_last_90_days(http)
        .await?
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
