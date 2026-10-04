//! Encrypted, append-only valuation storage.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use domain::wealth::{
    AccountId, CipherContext, Currency, FieldCipher, Money, Valuation, ValuationId,
    ValuationRepository, ValuationSource, WealthError,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::storage;

const TABLE: &str = "wealth_valuations";
const AMOUNT_COLUMN: &str = "amount";
const COLUMNS: &str = "id, account_id, as_of, currency, amount, source, recorded_at";

type ValuationRow = (Uuid, Uuid, Date, String, Vec<u8>, String, OffsetDateTime);

/// `PostgreSQL` implementation of [`ValuationRepository`].
pub struct PgValuations {
    pool: PgPool,
    cipher: Arc<dyn FieldCipher>,
}

impl PgValuations {
    /// Builds the adapter.
    #[must_use]
    pub fn new(pool: PgPool, cipher: Arc<dyn FieldCipher>) -> Self {
        Self { pool, cipher }
    }

    fn context(id: ValuationId) -> CipherContext {
        CipherContext {
            table: TABLE,
            column: AMOUNT_COLUMN,
            row_id: id.0,
        }
    }

    fn to_valuation(&self, row: ValuationRow) -> Result<Valuation, WealthError> {
        let (id, account_id, as_of, currency, amount, source, recorded_at) = row;
        let id = ValuationId(id);
        let clear = self.cipher.decrypt(&amount, &Self::context(id))?;
        let text = std::str::from_utf8(&clear).map_err(|_| WealthError::Cipher)?;
        Ok(Valuation {
            id,
            account_id: AccountId(account_id),
            as_of,
            amount: Money::new(
                Decimal::from_str(text).map_err(|_| WealthError::Cipher)?,
                Currency::from_str(&currency)?,
            ),
            source: ValuationSource::from_code(&source)?,
            recorded_at,
        })
    }
}

#[async_trait]
impl ValuationRepository for PgValuations {
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError> {
        let amount = self.cipher.encrypt(
            valuation.amount.amount.to_string().as_bytes(),
            &Self::context(valuation.id),
        )?;
        sqlx::query(&format!(
            "INSERT INTO wealth_valuations ({COLUMNS}) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        ))
        .bind(valuation.id.0)
        .bind(valuation.account_id.0)
        .bind(valuation.as_of)
        .bind(valuation.amount.currency.as_str())
        .bind(amount)
        .bind(valuation.source.code())
        .bind(valuation.recorded_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| storage(&e))
    }

    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError> {
        sqlx::query_as::<_, ValuationRow>(&format!(
            "SELECT DISTINCT ON (account_id) {COLUMNS} FROM wealth_valuations WHERE as_of <= $1 ORDER BY account_id, as_of DESC, recorded_at DESC"
        ))
        .bind(at)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .into_iter()
        .map(|row| self.to_valuation(row))
        .collect()
    }

    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        sqlx::query_as::<_, ValuationRow>(&format!(
            "SELECT {COLUMNS} FROM wealth_valuations WHERE account_id = $1 ORDER BY as_of, recorded_at"
        ))
        .bind(account_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .into_iter()
        .map(|row| self.to_valuation(row))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::test_support::FakeCipher;
    use domain::wealth::{Account, AccountKind, AccountRepository, Owner};
    use time::macros::{date, datetime};

    use super::*;
    use crate::wealth::PgAccounts;

    async fn seeded_account(pool: &PgPool) -> Account {
        let account = Account::open(
            "Livret",
            AccountKind::Savings,
            Owner::Personal,
            Currency::EUR,
            None,
            datetime!(2026-01-01 0:00 UTC),
        )
        .unwrap();
        PgAccounts::new(pool.clone(), Arc::new(FakeCipher))
            .insert(&account)
            .await
            .unwrap();
        account
    }

    fn valuation(
        account: &Account,
        as_of: Date,
        cents: i64,
        recorded_at: OffsetDateTime,
    ) -> Valuation {
        Valuation::record(
            account,
            as_of,
            Money::eur(Decimal::new(cents, 2)),
            ValuationSource::Manual,
            recorded_at,
        )
        .unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn latest_per_account_picks_greatest_date_then_latest_record(pool: PgPool) {
        let account = seeded_account(&pool).await;
        let repository = PgValuations::new(pool, Arc::new(FakeCipher));
        repository
            .insert(&valuation(
                &account,
                date!(2026 - 09 - 01),
                10_000,
                datetime!(2026-09-01 9:00 UTC),
            ))
            .await
            .unwrap();
        repository
            .insert(&valuation(
                &account,
                date!(2026 - 10 - 01),
                20_000,
                datetime!(2026-10-01 9:00 UTC),
            ))
            .await
            .unwrap();
        let correction = valuation(
            &account,
            date!(2026 - 10 - 01),
            21_050,
            datetime!(2026-10-01 9:05 UTC),
        );
        repository.insert(&correction).await.unwrap();
        repository
            .insert(&valuation(
                &account,
                date!(2026 - 10 - 03),
                99_900,
                datetime!(2026-10-03 9:00 UTC),
            ))
            .await
            .unwrap();

        let latest = repository
            .latest_per_account(date!(2026 - 10 - 02))
            .await
            .unwrap();

        assert_eq!(latest, vec![correction]);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn for_account_is_ordered_and_amounts_are_encrypted(pool: PgPool) {
        let account = seeded_account(&pool).await;
        let repository = PgValuations::new(pool.clone(), Arc::new(FakeCipher));
        let later = valuation(
            &account,
            date!(2026 - 10 - 01),
            12_345_678,
            datetime!(2026-10-01 9:00 UTC),
        );
        let earlier = valuation(
            &account,
            date!(2026 - 09 - 01),
            100,
            datetime!(2026-10-01 9:01 UTC),
        );
        repository.insert(&later).await.unwrap();
        repository.insert(&earlier).await.unwrap();

        assert_eq!(
            repository.for_account(account.id).await.unwrap(),
            vec![earlier, later]
        );
        let amounts: Vec<Vec<u8>> = sqlx::query_scalar("SELECT amount FROM wealth_valuations")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(
            amounts
                .iter()
                .all(|bytes| !bytes.windows(6).any(|w| w == b"123456"))
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn valuations_cannot_be_updated_or_deleted(pool: PgPool) {
        let account = seeded_account(&pool).await;
        PgValuations::new(pool.clone(), Arc::new(FakeCipher))
            .insert(&valuation(
                &account,
                date!(2026 - 10 - 01),
                100,
                datetime!(2026-10-01 9:00 UTC),
            ))
            .await
            .unwrap();
        assert!(
            sqlx::query("UPDATE wealth_valuations SET as_of = as_of")
                .execute(&pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM wealth_valuations")
                .execute(&pool)
                .await
                .is_err()
        );
    }
}
