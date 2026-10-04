//! Encrypted account storage.

use std::sync::Arc;

use async_trait::async_trait;
use domain::wealth::{
    Account, AccountId, AccountKind, AccountRepository, CipherContext, FieldCipher, Owner,
    WealthError,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::padding::pad_to_block;
use super::storage;

const TABLE: &str = "wealth_accounts";
const SELECT: &str =
    "SELECT id, kind, owner, currency, name, notes, is_archived, created_at FROM wealth_accounts";

/// `PostgreSQL` implementation of [`AccountRepository`].
pub struct PgAccounts {
    pool: PgPool,
    cipher: Arc<dyn FieldCipher>,
}

type AccountRow = (
    Uuid,
    String,
    String,
    String,
    Vec<u8>,
    Option<Vec<u8>>,
    bool,
    OffsetDateTime,
);

impl PgAccounts {
    /// Builds the adapter.
    #[must_use]
    pub fn new(pool: PgPool, cipher: Arc<dyn FieldCipher>) -> Self {
        Self { pool, cipher }
    }

    fn seal(
        &self,
        column: &'static str,
        id: AccountId,
        text: &str,
    ) -> Result<Vec<u8>, WealthError> {
        self.cipher.encrypt(
            &pad_to_block(text),
            &CipherContext {
                table: TABLE,
                column,
                row_id: id.0,
            },
        )
    }

    fn open(
        &self,
        column: &'static str,
        id: AccountId,
        sealed: &[u8],
    ) -> Result<String, WealthError> {
        let bytes = self.cipher.decrypt(
            sealed,
            &CipherContext {
                table: TABLE,
                column,
                row_id: id.0,
            },
        )?;
        String::from_utf8(bytes)
            .map(|text| text.trim().to_owned())
            .map_err(|_| WealthError::Cipher)
    }

    fn to_account(&self, row: AccountRow) -> Result<Account, WealthError> {
        let (id, kind, owner, currency, name, notes, is_archived, created_at) = row;
        let id = AccountId(id);
        Ok(Account {
            id,
            name: self.open("name", id, &name)?,
            kind: AccountKind::from_code(&kind)?,
            owner: Owner::from_code(&owner)?,
            currency: currency.parse()?,
            is_archived,
            notes: notes
                .map(|sealed| self.open("notes", id, &sealed))
                .transpose()?,
            created_at,
        })
    }
}

#[async_trait]
impl AccountRepository for PgAccounts {
    async fn insert(&self, account: &Account) -> Result<(), WealthError> {
        let name = self.seal("name", account.id, &account.name)?;
        let notes = account
            .notes
            .as_deref()
            .map(|text| self.seal("notes", account.id, text))
            .transpose()?;
        sqlx::query(
            "INSERT INTO wealth_accounts (id, kind, owner, currency, name, notes, is_archived, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(account.id.0)
        .bind(account.kind.code())
        .bind(account.owner.code())
        .bind(account.currency.as_str())
        .bind(name)
        .bind(notes)
        .bind(account.is_archived)
        .bind(account.created_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| storage(&e))
    }

    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError> {
        sqlx::query_as::<_, AccountRow>(&format!("{SELECT} WHERE id = $1"))
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| storage(&e))?
            .map(|row| self.to_account(row))
            .transpose()
    }

    async fn list(&self) -> Result<Vec<Account>, WealthError> {
        sqlx::query_as::<_, AccountRow>(&format!("{SELECT} ORDER BY created_at, id"))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| storage(&e))?
            .into_iter()
            .map(|row| self.to_account(row))
            .collect()
    }

    async fn archive(&self, id: AccountId) -> Result<(), WealthError> {
        let result = sqlx::query("UPDATE wealth_accounts SET is_archived = true WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| storage(&e))?;
        if result.rows_affected() == 0 {
            return Err(WealthError::AccountNotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::test_support::FakeCipher;
    use domain::wealth::{BrokerageEnvelope, Currency};

    use super::*;

    fn repository(pool: PgPool) -> PgAccounts {
        PgAccounts::new(pool, Arc::new(FakeCipher))
    }

    fn pea() -> Account {
        Account::open(
            "PEA Bourso",
            AccountKind::Brokerage {
                envelope: BrokerageEnvelope::Pea,
            },
            Owner::Personal,
            Currency::EUR,
            Some("ouvert en 2019"),
            OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        )
        .unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn insert_find_list_and_archive(pool: PgPool) {
        let repository = repository(pool);
        let account = pea();
        repository.insert(&account).await.unwrap();

        assert_eq!(
            repository.find(account.id).await.unwrap(),
            Some(account.clone())
        );
        assert_eq!(repository.list().await.unwrap(), vec![account.clone()]);
        repository.archive(account.id).await.unwrap();
        assert!(
            repository
                .find(account.id)
                .await
                .unwrap()
                .unwrap()
                .is_archived
        );
        assert_eq!(
            repository.archive(AccountId::generate()).await,
            Err(WealthError::AccountNotFound)
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn name_and_notes_are_never_stored_in_clear(pool: PgPool) {
        let repository = repository(pool.clone());
        let account = pea();
        repository.insert(&account).await.unwrap();
        let (name, notes): (Vec<u8>, Option<Vec<u8>>) =
            sqlx::query_as("SELECT name, notes FROM wealth_accounts")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!name.windows(4).any(|w| w == b"PEA "));
        let notes = notes.unwrap();
        assert!(!notes.windows(6).any(|w| w == b"ouvert"));

        let id = account.id.0;
        let context = |column| CipherContext {
            table: "wealth_accounts",
            column,
            row_id: id,
        };
        assert_eq!(
            FakeCipher
                .decrypt(&name, &context("name"))
                .unwrap()
                .trim_ascii(),
            b"PEA Bourso"
        );
        assert_eq!(
            FakeCipher
                .decrypt(&notes, &context("notes"))
                .unwrap()
                .trim_ascii(),
            b"ouvert en 2019"
        );
        assert!(FakeCipher.decrypt(&name, &context("notes")).is_err());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ciphertext_length_does_not_depend_on_name_or_notes_length(pool: PgPool) {
        let repository = repository(pool.clone());
        let at = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let short = Account::open(
            "A",
            AccountKind::Savings,
            Owner::Personal,
            Currency::EUR,
            Some("n"),
            at,
        )
        .unwrap();
        let long_name = "B".repeat(40);
        let long = Account::open(
            &long_name,
            AccountKind::Savings,
            Owner::Personal,
            Currency::EUR,
            Some("une note un peu plus longue"),
            at,
        )
        .unwrap();
        repository.insert(&short).await.unwrap();
        repository.insert(&long).await.unwrap();

        let lengths: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT octet_length(name), octet_length(notes) FROM wealth_accounts ORDER BY created_at, id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(lengths[0], lengths[1]);
        let mut stored = repository.list().await.unwrap();
        stored.sort_by_key(|account| account.name.len());
        assert_eq!(stored, vec![short, long]);
    }
}
