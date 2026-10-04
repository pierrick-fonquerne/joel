//! Cached ECB exchange rates, written by the runner, read by the api.

use async_trait::async_trait;
use domain::wealth::{Currency, ExchangeRateSource, WealthError};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::Date;

use super::storage;

/// Rates older than this many days before the requested date are ignored.
pub const MAX_FALLBACK_DAYS: i32 = 7;

/// `PostgreSQL` implementation of [`ExchangeRateSource`].
pub struct PgExchangeRates {
    pool: PgPool,
}

impl PgExchangeRates {
    /// Builds the adapter.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inserts rates, ignoring the ones already stored.
    ///
    /// # Errors
    /// [`WealthError::Storage`].
    pub async fn store_rates(
        &self,
        rates: &[(Currency, Date, Decimal)],
    ) -> Result<u64, WealthError> {
        let mut inserted = 0;
        for (currency, on, units_per_eur) in rates {
            inserted += sqlx::query(
                "INSERT INTO wealth_exchange_rates (currency, on_date, units_per_eur) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(currency.as_str())
            .bind(on)
            .bind(units_per_eur)
            .execute(&self.pool)
            .await
            .map_err(|e| storage(&e))?
            .rows_affected();
        }
        Ok(inserted)
    }
}

#[async_trait]
impl ExchangeRateSource for PgExchangeRates {
    async fn units_per_eur(
        &self,
        currency: Currency,
        on: Date,
    ) -> Result<Option<Decimal>, WealthError> {
        if currency == Currency::EUR {
            return Ok(Some(Decimal::ONE));
        }
        sqlx::query_scalar(
            "SELECT units_per_eur FROM wealth_exchange_rates WHERE currency = $1 AND on_date <= $2 AND on_date >= $2 - $3 ORDER BY on_date DESC LIMIT 1",
        )
        .bind(currency.as_str())
        .bind(on)
        .bind(MAX_FALLBACK_DAYS)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))
    }

    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError> {
        if currency == Currency::EUR {
            return Ok(true);
        }
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM wealth_exchange_rates WHERE currency = $1)",
        )
        .bind(currency.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| storage(&e))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::macros::date;

    use super::*;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn store_is_idempotent_and_lookup_falls_back_up_to_seven_days(pool: PgPool) {
        let rates = PgExchangeRates::new(pool);
        let rows = [(usd(), date!(2026 - 10 - 02), Decimal::new(10_850, 4))];
        assert_eq!(rates.store_rates(&rows).await.unwrap(), 1);
        assert_eq!(rates.store_rates(&rows).await.unwrap(), 0);

        assert_eq!(
            rates
                .units_per_eur(usd(), date!(2026 - 10 - 04))
                .await
                .unwrap(),
            Some(Decimal::new(10_850, 4))
        );
        assert_eq!(
            rates
                .units_per_eur(usd(), date!(2026 - 10 - 09))
                .await
                .unwrap(),
            Some(Decimal::new(10_850, 4))
        );
        assert_eq!(
            rates
                .units_per_eur(usd(), date!(2026 - 10 - 10))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            rates
                .units_per_eur(usd(), date!(2026 - 10 - 01))
                .await
                .unwrap(),
            None
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn euro_is_always_supported_and_one(pool: PgPool) {
        let rates = PgExchangeRates::new(pool);
        assert!(rates.is_supported(Currency::EUR).await.unwrap());
        assert!(!rates.is_supported(usd()).await.unwrap());
        assert_eq!(
            rates
                .units_per_eur(Currency::EUR, date!(2026 - 10 - 04))
                .await
                .unwrap(),
            Some(Decimal::ONE)
        );
        rates
            .store_rates(&[(usd(), date!(2026 - 10 - 02), Decimal::ONE)])
            .await
            .unwrap();
        assert!(rates.is_supported(usd()).await.unwrap());
    }
}
