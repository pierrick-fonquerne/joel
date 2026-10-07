# Socle patrimoine W0 : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pierrick saisit ses comptes depuis la PWA de Joel sur iPhone et lit son patrimoine net, sa répartition et son évolution, avec les montants et les noms de compte chiffrés au repos via Egide.

**Architecture:** Nouveau module pur `domain/wealth` (modèle, invariants, calcul du patrimoine net, ports). Deux nouveaux crates adaptateurs : `cipher-egide` (chiffrement par enveloppe, clé de données obtenue d'Egide Transit) et `exchange-rates-ecb` (flux BCE). Les dépôts Postgres chiffrent et déchiffrent via le port `FieldCipher`. L'`api` sert `/api/wealth/*` derrière un `WealthGate` qui rend `503` tant que le coffre est scellé. Le `runner`, seul à avoir Internet, alimente la table des taux. La PWA Angular ajoute `features/wealth`.

**Tech Stack:** Rust 2024 (axum 0.8, sqlx 0.8 Postgres, rust_decimal, aes-gcm, zeroize, reqwest rustls, roxmltree, wiremock pour les tests), Angular 19 standalone + signals, Karma/Jasmine, Docker Compose, Egide 0.1.0.

**Spec:** `docs/superpowers/specs/2026-10-04-joel-wealth-foundation-design.md`

## Global Constraints

- Branche d'implémentation : `feature/joel-wealth-foundation`, créée depuis `docs/joel-wealth-foundation` (spec et plan inclus). Jamais de commit sur `main`. Une PR à la fin, merge seulement après le GO de Pierrick.
- Avant tout code : milestone et issues sur le dépôt distant (Task 0). Chaque commit de tâche cite son issue (`Refs #N`).
- CI à respecter à chaque commit : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clippy `all` deny, `pedantic` warn, donc bloquant en CI), `cargo deny`, `cargo test --workspace`, `npx ng lint`, `npm test -- --watch=false --browsers=ChromeHeadless`, `npm run build`.
- Rust : pas de `unwrap`/`expect` hors tests (les modules de test portent `#![allow(clippy::unwrap_used)]` ou `#[allow]` comme l'existant). Doc comments en anglais sur tout item public, section `# Errors` sur toute fonction publique qui renvoie `Result`, `#[must_use]` sur les fonctions publiques pures qui renvoient une valeur.
- Montants : `rust_decimal::Decimal` côté Rust, chaîne décimale en JSON (`"12345.67"`), jamais de flottant côté backend. Deux décimales maximum en saisie.
- Journaux : aucun montant, nom de compte, note, clé ni token dans `tracing` ou le logger front.
- Identifiants en anglais, sans abréviation. Textes d'interface en français avec accents. Aucun em-dash nulle part (code, docs, commits).
- TypeScript strict, ESLint, pas de `console.log` : utiliser `LoggerService` (`core/logging/logger.service`).
- Messages de commit : Conventional Commits, sans aucune mention d'IA ni ligne d'attribution.
- Vocabulaire figé (spec section 3) : `Owner`, `Account`, `AccountKind`, `BrokerageEnvelope`, `Valuation`, `ValuationSource`, `Money`, `NetWorth`, `ExchangeRate`, `FieldCipher`, `StaleAccount`.
- Écart assumé avec la spec, section 6 : le type de compte est stocké dans une seule colonne `kind` dont le code porte l'enveloppe (`brokerage_pea`, `brokerage_cto`). La colonne `brokerage_envelope` disparaît. Le modèle Rust garde `AccountKind::Brokerage { envelope }`. La Task 6 met la spec à jour en même temps que la migration.

## Review Focus

1. **Saisie au clavier français** : « 1 234,56 » tapé sur iPhone doit être accepté (virgule, espaces, espace insécable) et envoyé `"1234.56"`. Une saisie invalide (« 12,345 », « 1e3 », « abc ») est refusée côté front et, si elle passe, rejetée en `400 invalid_amount` côté API. Tests : Task 1 (`parse_amount`), Task 10 (`normalizeAmount`), Task 9 (route).
2. **Fuseau horaire** : à 00h30 à Paris, la date du jour est encore la veille en UTC. Un relevé daté d'aujourd'hui (heure de Paris) ne doit pas être refusé comme « futur ». Règle : `as_of` est accepté jusqu'à la date UTC du jour plus un jour. Test : Task 1.
3. **Devise sans taux BCE** (crypto en BTC, devise exotique) : refus clair à la création du compte (`400 unsupported_currency`), pour qu'un compte ne casse jamais le calcul global plus tard. Test : Task 3 et Task 9.
4. **Relevé un week-end, un jour férié ou avant le premier taux en cache** : on prend le taux du jour ouvré précédent, jusqu'à 7 jours en arrière. Au-delà, `422 exchange_rate_missing` explicite, jamais une conversion à 1. Tests : Task 3 et Task 6.
5. **Egide descellé après le démarrage de Joel** : les routes patrimoine reprennent seules, sans redémarrer Joel, au plus 30 secondes après le descellement. Test : Task 8 (`WealthGate` avec un `VaultUnlocker` qui échoue puis réussit).

---

### Task 0: Suivi sur le dépôt distant et branche

**Files:** aucun fichier du dépôt.

- [ ] **Step 1: Créer la branche d'implémentation**

```bash
cd joel
git checkout docs/joel-wealth-foundation
git checkout -b feature/joel-wealth-foundation
git rev-parse --abbrev-ref HEAD
```
Expected: `feature/joel-wealth-foundation`

- [ ] **Step 2: Créer la milestone et une issue par tâche**

```bash
gh api repos/{owner}/{repo}/milestones -f title="W0 Socle patrimoine" -f description="Spec docs/superpowers/specs/2026-10-04-joel-wealth-foundation-design.md"
for title in \
  "W0.1 Domaine wealth : types et invariants" \
  "W0.1 Domaine wealth : ports, faux et calcul du patrimoine net" \
  "W0.1 Domaine wealth : use cases" \
  "W0.2 cipher-egide : client Transit" \
  "W0.2 cipher-egide : chiffrement par enveloppe" \
  "W0.3 Persistance wealth : migration et dépôts chiffrés" \
  "W0.3 Taux BCE : crate et tâche du runner" \
  "W0.4 API : WealthGate et configuration Egide" \
  "W0.4 API : routes /api/wealth" \
  "W0.5 PWA : modèles, service, mode discret" \
  "W0.5 PWA : tableau de bord et tuile cockpit" \
  "W0.5 PWA : comptes et saisie d'un relevé" \
  "W0.6 Déploiement Egide et runbook" ; do
  gh issue create --title "$title" --milestone "W0 Socle patrimoine" --body "Plan : docs/superpowers/plans/2026-10-04-joel-wealth-foundation.md"
done
gh issue list --milestone "W0 Socle patrimoine"
```
Expected: 13 issues listées. Noter leurs numéros : chaque commit de tâche finit par `Refs #N`.

---

### Task 1: Domaine wealth, types et invariants

**Files:**
- Modify: `backend/crates/domain/Cargo.toml`
- Modify: `backend/crates/domain/src/lib.rs`
- Create: `backend/crates/domain/src/wealth/mod.rs`
- Create: `backend/crates/domain/src/wealth/error.rs`
- Create: `backend/crates/domain/src/wealth/money.rs`
- Create: `backend/crates/domain/src/wealth/account.rs`
- Create: `backend/crates/domain/src/wealth/valuation.rs`

**Interfaces:**
- Produces: `WealthError` (avec `code()`), `Currency` (`EUR`, `FromStr`, `as_str`), `Money`, `parse_amount(&str) -> Result<Decimal, WealthError>`, `Owner`, `BrokerageEnvelope`, `AccountKind` (`code`, `from_code`, `is_liability`), `AccountId`, `Account::open(...)`, `ValuationId`, `ValuationSource` (`code`, `from_code`), `Valuation::record(...)`.

- [ ] **Step 1: Ajouter la dépendance et déclarer le module**

Dans `backend/crates/domain/Cargo.toml`, section `[dependencies]`, ajouter :

```toml
rust_decimal = "1"
```

Dans `backend/crates/domain/src/lib.rs`, ajouter après `pub mod knowledge;` :

```rust
pub mod wealth;
```

Créer `backend/crates/domain/src/wealth/mod.rs` :

```rust
//! Personal and company wealth: accounts, valuations and net worth.

pub mod account;
pub mod error;
pub mod money;
pub mod valuation;

pub use account::{Account, AccountId, AccountKind, BrokerageEnvelope, Owner};
pub use error::WealthError;
pub use money::{Currency, Money, parse_amount};
pub use valuation::{Valuation, ValuationId, ValuationSource};
```

- [ ] **Step 2: Écrire les tests qui échouent**

Créer `backend/crates/domain/src/wealth/error.rs` :

```rust
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
```

Créer `backend/crates/domain/src/wealth/money.rs` avec uniquement les signatures et les tests :

```rust
//! Exact monetary amounts and ISO 4217 currencies.

use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

use super::error::WealthError;

/// ISO 4217 currency code: three uppercase ASCII letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Currency([u8; 3]);

impl Currency {
    /// The euro, reference currency of every net worth.
    pub const EUR: Self = Self(*b"EUR");

    /// Returns the code as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl FromStr for Currency {
    type Err = WealthError;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// An exact amount of money in a given currency. Never a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    /// Exact decimal amount.
    pub amount: Decimal,
    /// Currency of the amount.
    pub currency: Currency,
}

impl Money {
    /// Builds an amount in a currency.
    #[must_use]
    pub const fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Builds an amount in euros.
    #[must_use]
    pub const fn eur(amount: Decimal) -> Self {
        Self::new(amount, Currency::EUR)
    }
}

/// Parses a user-supplied amount: optional minus sign, digits, optional dot and
/// at most two fraction digits. No exponent, no thousands separator.
///
/// # Errors
/// Returns [`WealthError::InvalidAmount`] for any other shape.
pub fn parse_amount(input: &str) -> Result<Decimal, WealthError> {
    todo!()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn currency_accepts_three_uppercase_letters() {
        let usd: Currency = "USD".parse().unwrap();
        assert_eq!(usd.as_str(), "USD");
        assert_eq!(Currency::EUR.to_string(), "EUR");
    }

    #[test]
    fn currency_rejects_other_shapes() {
        for code in ["usd", "US", "EURO", "U$D", ""] {
            assert_eq!(code.parse::<Currency>(), Err(WealthError::InvalidCurrency), "{code}");
        }
    }

    #[test]
    fn parse_amount_accepts_plain_decimals() {
        assert_eq!(parse_amount("1234.56").unwrap(), Decimal::new(123_456, 2));
        assert_eq!(parse_amount("-250000").unwrap(), Decimal::new(-250_000, 0));
        assert_eq!(parse_amount("0.5").unwrap(), Decimal::new(5, 1));
    }

    #[test]
    fn parse_amount_rejects_ambiguous_or_exotic_input() {
        for input in ["1,5", "1 234.56", "1e3", "12.345", "abc", "", ".", "1.", "+3"] {
            assert_eq!(parse_amount(input), Err(WealthError::InvalidAmount), "{input}");
        }
    }
}
```

Créer `backend/crates/domain/src/wealth/account.rs` avec signatures et tests :

```rust
//! Wealth accounts: the envelopes whose value is tracked over time.

use time::OffsetDateTime;
use uuid::Uuid;

use super::error::WealthError;
use super::money::Currency;

/// Who owns an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Owner {
    /// Pierrick personally.
    Personal,
    /// The company (Labade Conseil).
    Company,
}

impl Owner {
    /// Stable storage and API code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        todo!()
    }

    /// Parses a storage or API code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidOwner`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        todo!()
    }
}

/// Tax envelope of a brokerage account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BrokerageEnvelope {
    /// Plan d'épargne en actions.
    Pea,
    /// Compte-titres ordinaire.
    Cto,
}

/// Nature of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountKind {
    /// Securities account, with its tax envelope.
    Brokerage {
        /// PEA or CTO.
        envelope: BrokerageEnvelope,
    },
    /// Life insurance (euro funds and unit-linked).
    LifeInsurance,
    /// Retirement plan (PER).
    RetirementPlan,
    /// Current account.
    BankAccount,
    /// Savings account.
    Savings,
    /// Crypto wallet, valued manually in a supported currency.
    CryptoWallet,
    /// Real estate asset.
    RealEstate,
    /// Shares of a company.
    CompanyShares,
    /// Loan: a liability, valued negatively.
    Loan,
}

impl AccountKind {
    /// Every kind, in display order.
    pub const ALL: [Self; 10] = [
        Self::Brokerage { envelope: BrokerageEnvelope::Pea },
        Self::Brokerage { envelope: BrokerageEnvelope::Cto },
        Self::LifeInsurance,
        Self::RetirementPlan,
        Self::BankAccount,
        Self::Savings,
        Self::CryptoWallet,
        Self::RealEstate,
        Self::CompanyShares,
        Self::Loan,
    ];

    /// True for liabilities, whose valuations are negative.
    #[must_use]
    pub const fn is_liability(self) -> bool {
        matches!(self, Self::Loan)
    }

    /// Stable storage and API code; the brokerage envelope is part of the code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        todo!()
    }

    /// Parses a storage or API code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidAccountKind`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        todo!()
    }
}

/// Account identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(pub Uuid);

impl AccountId {
    /// Generates a new time-ordered identifier.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

/// A wealth envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Identifier.
    pub id: AccountId,
    /// Human name, encrypted at rest.
    pub name: String,
    /// Nature of the account.
    pub kind: AccountKind,
    /// Owner.
    pub owner: Owner,
    /// Currency of every valuation of this account.
    pub currency: Currency,
    /// Archived accounts accept no valuation and leave the net worth.
    pub is_archived: bool,
    /// Free notes, encrypted at rest.
    pub notes: Option<String>,
    /// Creation instant.
    pub created_at: OffsetDateTime,
}

impl Account {
    /// Opens a new, non-archived account. The name and notes are trimmed;
    /// blank notes become `None`.
    ///
    /// # Errors
    /// Returns [`WealthError::EmptyAccountName`] when the trimmed name is empty.
    pub fn open(
        name: &str,
        kind: AccountKind,
        owner: Owner,
        currency: Currency,
        notes: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<Self, WealthError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn kind_codes_roundtrip_for_every_kind() {
        for kind in AccountKind::ALL {
            assert_eq!(AccountKind::from_code(kind.code()).unwrap(), kind);
        }
        assert_eq!(
            AccountKind::Brokerage { envelope: BrokerageEnvelope::Pea }.code(),
            "brokerage_pea"
        );
        assert_eq!(AccountKind::from_code("stocks"), Err(WealthError::InvalidAccountKind));
    }

    #[test]
    fn owner_codes_roundtrip() {
        assert_eq!(Owner::from_code("personal").unwrap(), Owner::Personal);
        assert_eq!(Owner::from_code(Owner::Company.code()).unwrap(), Owner::Company);
        assert_eq!(Owner::from_code("spouse"), Err(WealthError::InvalidOwner));
    }

    #[test]
    fn only_loans_are_liabilities() {
        assert!(AccountKind::Loan.is_liability());
        assert!(!AccountKind::RealEstate.is_liability());
    }

    #[test]
    fn open_trims_name_and_drops_blank_notes() {
        let account = Account::open(
            "  PEA Bourso ",
            AccountKind::Brokerage { envelope: BrokerageEnvelope::Pea },
            Owner::Personal,
            Currency::EUR,
            Some("   "),
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(account.name, "PEA Bourso");
        assert_eq!(account.notes, None);
        assert!(!account.is_archived);
    }

    #[test]
    fn open_rejects_blank_name() {
        let result = Account::open(
            "   ",
            AccountKind::Savings,
            Owner::Personal,
            Currency::EUR,
            None,
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(result, Err(WealthError::EmptyAccountName));
    }
}
```

Créer `backend/crates/domain/src/wealth/valuation.rs` avec signatures et tests :

```rust
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
        todo!()
    }

    /// Parses a storage code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidValuationSource`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        todo!()
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
        todo!()
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
        let valuation =
            Valuation::record(&account, date!(2026-10-04), eur(1000), ValuationSource::Manual, NOW)
                .unwrap();
        assert_eq!(valuation.account_id, account.id);
        assert_eq!(valuation.recorded_at, NOW);
    }

    #[test]
    fn accepts_tomorrow_utc_for_timezones_ahead_of_utc() {
        let account = account(AccountKind::Savings);
        assert!(
            Valuation::record(&account, date!(2026-10-05), eur(1), ValuationSource::Manual, NOW)
                .is_ok()
        );
        assert_eq!(
            Valuation::record(&account, date!(2026-10-06), eur(1), ValuationSource::Manual, NOW),
            Err(WealthError::FutureValuation)
        );
    }

    #[test]
    fn loans_must_be_negative_and_assets_positive() {
        let loan = account(AccountKind::Loan);
        assert!(Valuation::record(&loan, date!(2026-10-01), eur(-1), ValuationSource::Manual, NOW).is_ok());
        assert_eq!(
            Valuation::record(&loan, date!(2026-10-01), eur(1), ValuationSource::Manual, NOW),
            Err(WealthError::PositiveLoanValuation)
        );
        let asset = account(AccountKind::RealEstate);
        assert_eq!(
            Valuation::record(&asset, date!(2026-10-01), eur(-1), ValuationSource::Manual, NOW),
            Err(WealthError::NegativeAssetValuation)
        );
        assert!(Valuation::record(&asset, date!(2026-10-01), eur(0), ValuationSource::Manual, NOW).is_ok());
    }

    #[test]
    fn rejects_currency_mismatch_and_archived_accounts() {
        let mut account = account(AccountKind::BankAccount);
        let usd = Money::new(Decimal::ONE, "USD".parse().unwrap());
        assert_eq!(
            Valuation::record(&account, date!(2026-10-01), usd, ValuationSource::Manual, NOW),
            Err(WealthError::CurrencyMismatch)
        );
        account.is_archived = true;
        assert_eq!(
            Valuation::record(&account, date!(2026-10-01), eur(1), ValuationSource::Manual, NOW),
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
        assert_eq!(ValuationSource::from_code("guess"), Err(WealthError::InvalidValuationSource));
    }
}
```

Ajouter `"macros"` aux features de `time` dans `[dev-dependencies]` du domaine :

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
time = { version = "0.3", features = ["serde", "macros"] }
```

- [ ] **Step 3: Lancer les tests et vérifier qu'ils échouent**

Run: `cd backend && cargo test -p domain wealth`
Expected: FAIL, panics `not yet implemented` dans chaque test.

- [ ] **Step 4: Implémenter**

Dans `money.rs`, remplacer les `todo!()` :

```rust
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("???")
    }
```

```rust
    fn from_str(code: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; 3] = code
            .as_bytes()
            .try_into()
            .map_err(|_| WealthError::InvalidCurrency)?;
        if bytes.iter().all(u8::is_ascii_uppercase) {
            Ok(Self(bytes))
        } else {
            Err(WealthError::InvalidCurrency)
        }
    }
```

```rust
pub fn parse_amount(input: &str) -> Result<Decimal, WealthError> {
    let unsigned = input.strip_prefix('-').unwrap_or(input);
    let (integer, fraction) = match unsigned.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (unsigned, None),
    };
    let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let is_valid_fraction = fraction.is_none_or(|part| is_digits(part) && part.len() <= 2);
    if !is_digits(integer) || !is_valid_fraction {
        return Err(WealthError::InvalidAmount);
    }
    Decimal::from_str(input).map_err(|_| WealthError::InvalidAmount)
}
```

Dans `account.rs` :

```rust
    pub const fn code(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Company => "company",
        }
    }

    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        match code {
            "personal" => Ok(Self::Personal),
            "company" => Ok(Self::Company),
            _ => Err(WealthError::InvalidOwner),
        }
    }
```

```rust
    pub const fn code(self) -> &'static str {
        match self {
            Self::Brokerage { envelope: BrokerageEnvelope::Pea } => "brokerage_pea",
            Self::Brokerage { envelope: BrokerageEnvelope::Cto } => "brokerage_cto",
            Self::LifeInsurance => "life_insurance",
            Self::RetirementPlan => "retirement_plan",
            Self::BankAccount => "bank_account",
            Self::Savings => "savings",
            Self::CryptoWallet => "crypto_wallet",
            Self::RealEstate => "real_estate",
            Self::CompanyShares => "company_shares",
            Self::Loan => "loan",
        }
    }

    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.code() == code)
            .ok_or(WealthError::InvalidAccountKind)
    }
```

```rust
    pub fn open(
        name: &str,
        kind: AccountKind,
        owner: Owner,
        currency: Currency,
        notes: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<Self, WealthError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WealthError::EmptyAccountName);
        }
        Ok(Self {
            id: AccountId::generate(),
            name: name.to_owned(),
            kind,
            owner,
            currency,
            is_archived: false,
            notes: notes
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
            created_at: now,
        })
    }
```

Dans `valuation.rs` :

```rust
    pub const fn code(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::CsvImport => "csv_import",
            Self::BankAggregation => "bank_aggregation",
            Self::PriceFeed => "price_feed",
        }
    }

    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        match code {
            "manual" => Ok(Self::Manual),
            "csv_import" => Ok(Self::CsvImport),
            "bank_aggregation" => Ok(Self::BankAggregation),
            "price_feed" => Ok(Self::PriceFeed),
            _ => Err(WealthError::InvalidValuationSource),
        }
    }
```

```rust
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
        if account.kind.is_liability() && amount.amount.is_sign_positive() && !amount.amount.is_zero() {
            return Err(WealthError::PositiveLoanValuation);
        }
        if !account.kind.is_liability() && amount.amount.is_sign_negative() && !amount.amount.is_zero() {
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
```

- [ ] **Step 5: Lancer les tests et clippy**

Run: `cd backend && cargo test -p domain wealth && cargo clippy -p domain --all-targets -- -D warnings && cargo fmt --check`
Expected: tous les tests `wealth::` PASS, clippy et fmt sans sortie d'erreur.

- [ ] **Step 6: Commit**

```bash
git add backend/crates/domain
git commit -m "feat(wealth): add wealth domain types and invariants

Refs #<issue W0.1 types>"
```

---

### Task 2: Domaine wealth, ports, faux et calcul du patrimoine net

**Files:**
- Create: `backend/crates/domain/src/wealth/ports.rs`
- Create: `backend/crates/domain/src/wealth/net_worth.rs`
- Create: `backend/crates/domain/src/wealth/test_support.rs`
- Modify: `backend/crates/domain/src/wealth/mod.rs`

**Interfaces:**
- Consumes: tout ce que produit la Task 1.
- Produces:
  - `CipherContext { table: &'static str, column: &'static str, row_id: Uuid }` avec `aad() -> Vec<u8>`.
  - Traits (`#[async_trait]` sauf `FieldCipher`, synchrone) : `AccountRepository` (`insert`, `find`, `list`, `archive`), `ValuationRepository` (`insert`, `latest_per_account(at: Date)`, `for_account(AccountId)`), `ExchangeRateSource` (`units_per_eur(Currency, Date) -> Result<Option<Decimal>>`, `is_supported(Currency) -> Result<bool>`), `WrappedKeyStore` (`current() -> Result<Option<String>>`, `insert(wrapped_key: &str, egide_key_name: &str)`), `FieldCipher` (`encrypt`, `decrypt`).
  - `STALE_AFTER_DAYS: i64 = 45`, `RateTable`, `NetWorth::compute(as_of, &[Account], &[Valuation], &RateTable) -> Result<NetWorth, WealthError>`.
  - Faux : `FakeAccounts`, `FakeValuations`, `FakeRates`, `FakeWrappedKeys`, `FakeCipher`.

- [ ] **Step 1: Écrire les ports**

Créer `backend/crates/domain/src/wealth/ports.rs` :

```rust
//! Outbound ports of the wealth module.

use async_trait::async_trait;
use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use super::account::{Account, AccountId};
use super::error::WealthError;
use super::money::Currency;
use super::valuation::Valuation;

/// Persistent accounts. Names and notes are encrypted by the adapter.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Persists a new account.
    async fn insert(&self, account: &Account) -> Result<(), WealthError>;
    /// Finds an account; `Ok(None)` when unknown.
    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError>;
    /// Lists every account, archived ones included, in creation order.
    async fn list(&self) -> Result<Vec<Account>, WealthError>;
    /// Marks an account archived; [`WealthError::AccountNotFound`] when unknown.
    async fn archive(&self, id: AccountId) -> Result<(), WealthError>;
}

/// Append-only valuations. Amounts are encrypted by the adapter.
#[async_trait]
pub trait ValuationRepository: Send + Sync {
    /// Appends a valuation.
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError>;
    /// For each account, the valuation with the greatest `as_of <= at`, the
    /// latest `recorded_at` winning ties. Archived accounts are included.
    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError>;
    /// Every valuation of an account, by `as_of` then `recorded_at` ascending.
    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError>;
}

/// Cached ECB exchange rates.
#[async_trait]
pub trait ExchangeRateSource: Send + Sync {
    /// Units of `currency` for one euro on `on`, or on the closest earlier day
    /// within seven days. EUR always yields one.
    async fn units_per_eur(&self, currency: Currency, on: Date) -> Result<Option<Decimal>, WealthError>;
    /// True when at least one rate is known for `currency`. EUR is always supported.
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError>;
}

/// Storage of the Egide-wrapped data key, one row per key version.
#[async_trait]
pub trait WrappedKeyStore: Send + Sync {
    /// The most recent wrapped key, when one exists.
    async fn current(&self) -> Result<Option<String>, WealthError>;
    /// Appends a new wrapped key version.
    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError>;
}

/// Binds a ciphertext to its exact storage location (AES-GCM associated data),
/// so that a value copied to another row or column fails to decrypt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CipherContext {
    /// Table name.
    pub table: &'static str,
    /// Column name.
    pub column: &'static str,
    /// Primary key of the row.
    pub row_id: Uuid,
}

impl CipherContext {
    /// Associated data bytes: `table|column|row_id`.
    #[must_use]
    pub fn aad(&self) -> Vec<u8> {
        format!("{}|{}|{}", self.table, self.column, self.row_id).into_bytes()
    }
}

/// Field-level authenticated encryption.
pub trait FieldCipher: Send + Sync {
    /// Encrypts a field bound to its context.
    ///
    /// # Errors
    /// [`WealthError::Cipher`] when encryption fails.
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError>;
    /// Decrypts a field, failing on tampering or a different context.
    ///
    /// # Errors
    /// [`WealthError::Cipher`] when the ciphertext is malformed, forged or bound elsewhere.
    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError>;
}
```

- [ ] **Step 2: Écrire les faux**

Créer `backend/crates/domain/src/wealth/test_support.rs` :

```rust
//! In-memory fakes for the wealth ports, shared by domain, persistence and api tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rust_decimal::Decimal;
use time::{Date, Duration};

use super::account::{Account, AccountId};
use super::error::WealthError;
use super::money::Currency;
use super::ports::{
    AccountRepository, CipherContext, ExchangeRateSource, FieldCipher, ValuationRepository,
    WrappedKeyStore,
};
use super::valuation::Valuation;

fn poisoned() -> WealthError {
    WealthError::Storage("poisoned".into())
}

/// In-memory [`AccountRepository`].
#[derive(Default)]
pub struct FakeAccounts {
    /// Stored accounts, in insertion order.
    pub accounts: Mutex<Vec<Account>>,
}

#[async_trait]
impl AccountRepository for FakeAccounts {
    async fn insert(&self, account: &Account) -> Result<(), WealthError> {
        self.accounts.lock().map_err(|_| poisoned())?.push(account.clone());
        Ok(())
    }
    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError> {
        Ok(self.accounts.lock().map_err(|_| poisoned())?.iter().find(|a| a.id == id).cloned())
    }
    async fn list(&self) -> Result<Vec<Account>, WealthError> {
        Ok(self.accounts.lock().map_err(|_| poisoned())?.clone())
    }
    async fn archive(&self, id: AccountId) -> Result<(), WealthError> {
        let mut accounts = self.accounts.lock().map_err(|_| poisoned())?;
        let account = accounts.iter_mut().find(|a| a.id == id).ok_or(WealthError::AccountNotFound)?;
        account.is_archived = true;
        Ok(())
    }
}

/// In-memory [`ValuationRepository`].
#[derive(Default)]
pub struct FakeValuations {
    /// Stored valuations, in insertion order.
    pub valuations: Mutex<Vec<Valuation>>,
}

#[async_trait]
impl ValuationRepository for FakeValuations {
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError> {
        self.valuations.lock().map_err(|_| poisoned())?.push(valuation.clone());
        Ok(())
    }
    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError> {
        let valuations = self.valuations.lock().map_err(|_| poisoned())?;
        let mut latest: HashMap<AccountId, Valuation> = HashMap::new();
        for valuation in valuations.iter().filter(|v| v.as_of <= at) {
            let is_newer = latest
                .get(&valuation.account_id)
                .is_none_or(|current| (valuation.as_of, valuation.recorded_at) >= (current.as_of, current.recorded_at));
            if is_newer {
                latest.insert(valuation.account_id, valuation.clone());
            }
        }
        Ok(latest.into_values().collect())
    }
    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        let mut found: Vec<Valuation> = self
            .valuations
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|v| v.account_id == account_id)
            .cloned()
            .collect();
        found.sort_by_key(|v| (v.as_of, v.recorded_at));
        Ok(found)
    }
}

/// In-memory [`ExchangeRateSource`] with the seven-day fallback.
#[derive(Default)]
pub struct FakeRates {
    /// Units per euro, keyed by currency and date.
    pub rates: Mutex<HashMap<(Currency, Date), Decimal>>,
}

impl FakeRates {
    /// Adds a rate.
    ///
    /// # Panics
    /// Never in practice; a poisoned lock is ignored.
    pub fn with(self, currency: Currency, on: Date, units_per_eur: Decimal) -> Self {
        if let Ok(mut rates) = self.rates.lock() {
            rates.insert((currency, on), units_per_eur);
        }
        self
    }
}

#[async_trait]
impl ExchangeRateSource for FakeRates {
    async fn units_per_eur(&self, currency: Currency, on: Date) -> Result<Option<Decimal>, WealthError> {
        if currency == Currency::EUR {
            return Ok(Some(Decimal::ONE));
        }
        let rates = self.rates.lock().map_err(|_| poisoned())?;
        Ok((0..=7)
            .filter_map(|days_back| on.checked_sub(Duration::days(days_back)))
            .find_map(|day| rates.get(&(currency, day)).copied()))
    }
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError> {
        Ok(currency == Currency::EUR
            || self.rates.lock().map_err(|_| poisoned())?.keys().any(|(c, _)| *c == currency))
    }
}

/// In-memory [`WrappedKeyStore`].
#[derive(Default)]
pub struct FakeWrappedKeys {
    /// Stored wrapped keys, oldest first.
    pub keys: Mutex<Vec<String>>,
}

#[async_trait]
impl WrappedKeyStore for FakeWrappedKeys {
    async fn current(&self) -> Result<Option<String>, WealthError> {
        Ok(self.keys.lock().map_err(|_| poisoned())?.last().cloned())
    }
    async fn insert(&self, wrapped_key: &str, _egide_key_name: &str) -> Result<(), WealthError> {
        self.keys.lock().map_err(|_| poisoned())?.push(wrapped_key.to_owned());
        Ok(())
    }
}

/// Deterministic, context-bound [`FieldCipher`] for tests: the output never
/// contains the plaintext in clear, and decrypting with another context fails.
/// Not cryptographically secure.
#[derive(Default)]
pub struct FakeCipher;

const FAKE_MASK: u8 = 0x5A;

impl FieldCipher for FakeCipher {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let aad = context.aad();
        let aad_length = u32::try_from(aad.len()).map_err(|_| WealthError::Cipher)?;
        let mut out = aad_length.to_be_bytes().to_vec();
        out.extend(aad.iter().map(|b| b ^ FAKE_MASK));
        out.extend(plaintext.iter().map(|b| b ^ FAKE_MASK));
        Ok(out)
    }
    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let (length_bytes, rest) = ciphertext.split_at_checked(4).ok_or(WealthError::Cipher)?;
        let length_array: [u8; 4] = length_bytes.try_into().map_err(|_| WealthError::Cipher)?;
        let aad_length = usize::try_from(u32::from_be_bytes(length_array)).map_err(|_| WealthError::Cipher)?;
        let (aad_masked, body) = rest.split_at_checked(aad_length).ok_or(WealthError::Cipher)?;
        let aad: Vec<u8> = aad_masked.iter().map(|b| b ^ FAKE_MASK).collect();
        if aad != context.aad() {
            return Err(WealthError::Cipher);
        }
        Ok(body.iter().map(|b| b ^ FAKE_MASK).collect())
    }
}
```

- [ ] **Step 3: Écrire le calcul avec ses tests, qui échouent**

Créer `backend/crates/domain/src/wealth/net_worth.rs` :

```rust
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
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::macros::{date, datetime};
    use time::OffsetDateTime;

    use super::*;
    use crate::wealth::account::BrokerageEnvelope;
    use crate::wealth::valuation::ValuationSource;

    const NOW: OffsetDateTime = datetime!(2026-10-04 12:00 UTC);

    fn account(kind: AccountKind, owner: Owner, currency: Currency) -> Account {
        Account::open("Compte", kind, owner, currency, None, NOW).unwrap()
    }

    fn valuation(account: &Account, as_of: Date, units: i64) -> Valuation {
        Valuation::record(account, as_of, Money::new(Decimal::new(units, 0), account.currency), ValuationSource::Manual, NOW).unwrap()
    }

    #[test]
    fn sums_assets_and_subtracts_loans_with_breakdowns() {
        let pea = account(AccountKind::Brokerage { envelope: BrokerageEnvelope::Pea }, Owner::Personal, Currency::EUR);
        let house = account(AccountKind::RealEstate, Owner::Personal, Currency::EUR);
        let loan = account(AccountKind::Loan, Owner::Personal, Currency::EUR);
        let treasury = account(AccountKind::BankAccount, Owner::Company, Currency::EUR);
        let latest = vec![
            valuation(&pea, date!(2026-10-01), 50_000),
            valuation(&house, date!(2026-09-30), 300_000),
            valuation(&loan, date!(2026-09-30), -180_000),
            valuation(&treasury, date!(2026-10-02), 40_000),
        ];
        let accounts = vec![pea, house, loan, treasury];

        let net_worth = NetWorth::compute(date!(2026-10-04), &accounts, &latest, &RateTable::default()).unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(210_000, 0)));
        assert_eq!(net_worth.by_owner[&Owner::Personal], Money::eur(Decimal::new(170_000, 0)));
        assert_eq!(net_worth.by_owner[&Owner::Company], Money::eur(Decimal::new(40_000, 0)));
        assert_eq!(net_worth.by_kind[&AccountKind::Loan], Money::eur(Decimal::new(-180_000, 0)));
        assert!(net_worth.stale_accounts.is_empty());
    }

    #[test]
    fn converts_with_the_rate_of_the_valuation_date_and_rounds_to_the_cent() {
        let usd: Currency = "USD".parse().unwrap();
        let broker = account(AccountKind::Brokerage { envelope: BrokerageEnvelope::Cto }, Owner::Personal, usd);
        let latest = vec![valuation(&broker, date!(2026-10-02), 1000)];
        let mut rates = RateTable::default();
        rates.0.insert((usd, date!(2026-10-02)), Decimal::new(10_850, 4));

        let net_worth = NetWorth::compute(date!(2026-10-04), &[broker], &latest, &rates).unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(92_166, 2)));
    }

    #[test]
    fn missing_rate_is_an_explicit_error() {
        let usd: Currency = "USD".parse().unwrap();
        let broker = account(AccountKind::Savings, Owner::Personal, usd);
        let latest = vec![valuation(&broker, date!(2026-10-02), 1000)];

        let result = NetWorth::compute(date!(2026-10-04), &[broker], &latest, &RateTable::default());

        assert_eq!(result, Err(WealthError::ExchangeRateMissing { currency: usd, on: date!(2026-10-02) }));
    }

    #[test]
    fn ignores_archived_accounts_and_flags_stale_ones() {
        let mut archived = account(AccountKind::Savings, Owner::Personal, Currency::EUR);
        let old = account(AccountKind::LifeInsurance, Owner::Personal, Currency::EUR);
        let empty = account(AccountKind::RetirementPlan, Owner::Personal, Currency::EUR);
        let latest = vec![
            valuation(&archived, date!(2026-10-01), 999),
            valuation(&old, date!(2026-08-19), 10_000),
        ];
        archived.is_archived = true;
        let (old_id, empty_id) = (old.id, empty.id);

        let net_worth = NetWorth::compute(date!(2026-10-04), &[archived, old, empty], &latest, &RateTable::default()).unwrap();

        assert_eq!(net_worth.total, Money::eur(Decimal::new(10_000, 0)));
        assert_eq!(net_worth.stale_accounts, vec![old_id, empty_id]);
    }

    #[test]
    fn a_valuation_exactly_45_days_old_is_not_stale() {
        let account = account(AccountKind::LifeInsurance, Owner::Personal, Currency::EUR);
        let latest = vec![valuation(&account, date!(2026-08-20), 1)];
        let net_worth = NetWorth::compute(date!(2026-10-04), &[account], &latest, &RateTable::default()).unwrap();
        assert!(net_worth.stale_accounts.is_empty());
    }
}
```

Mettre à jour `backend/crates/domain/src/wealth/mod.rs` :

```rust
//! Personal and company wealth: accounts, valuations and net worth.

pub mod account;
pub mod error;
pub mod money;
pub mod net_worth;
pub mod ports;
pub mod test_support;
pub mod valuation;

pub use account::{Account, AccountId, AccountKind, BrokerageEnvelope, Owner};
pub use error::WealthError;
pub use money::{Currency, Money, parse_amount};
pub use net_worth::{NetWorth, RateTable, STALE_AFTER_DAYS};
pub use ports::{
    AccountRepository, CipherContext, ExchangeRateSource, FieldCipher, ValuationRepository,
    WrappedKeyStore,
};
pub use valuation::{Valuation, ValuationId, ValuationSource};
```

- [ ] **Step 4: Vérifier l'échec**

Run: `cd backend && cargo test -p domain net_worth`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 5: Implémenter `NetWorth::compute`**

```rust
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
    let missing = || WealthError::ExchangeRateMissing { currency: amount.currency, on: valuation.as_of };
    let units_per_eur = rates.0.get(&(amount.currency, valuation.as_of)).ok_or_else(missing)?;
    amount.amount.checked_div(*units_per_eur).map(|eur| eur.round_dp(2)).ok_or_else(missing)
}

fn add<K: Ord>(totals: &mut BTreeMap<K, Money>, key: K, amount: Decimal) {
    totals
        .entry(key)
        .and_modify(|money| money.amount += amount)
        .or_insert_with(|| Money::eur(amount));
}
```

Note : supprimer l'accolade fermante du bloc `impl NetWorth` d'origine puisque l'implémentation ci-dessus la ferme elle-même.

- [ ] **Step 6: Tests, clippy, fmt**

Run: `cd backend && cargo test -p domain wealth && cargo clippy -p domain --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS, aucun avertissement.

- [ ] **Step 7: Commit**

```bash
git add backend/crates/domain/src/wealth
git commit -m "feat(wealth): add wealth ports, fakes and net worth computation

Refs #<issue W0.1 ports>"
```

---

### Task 3: Domaine wealth, use cases

**Files:**
- Create: `backend/crates/domain/src/wealth/use_cases.rs`
- Modify: `backend/crates/domain/src/wealth/mod.rs` (ajouter `pub mod use_cases;` et `pub use use_cases::{AccountSummary, NewAccount, Wealth, month_ends};`)

**Interfaces:**
- Consumes: Tasks 1 et 2.
- Produces:
  - `NewAccount { name: String, kind: AccountKind, owner: Owner, currency: Currency, notes: Option<String> }`
  - `AccountSummary { account: Account, latest_valuation: Option<Valuation>, is_stale: bool }`
  - `Wealth::new(Arc<dyn AccountRepository>, Arc<dyn ValuationRepository>, Arc<dyn ExchangeRateSource>) -> Wealth`
  - `Wealth::create_account(&self, NewAccount, now: OffsetDateTime) -> Result<Account>`
  - `Wealth::archive_account(&self, AccountId) -> Result<()>`
  - `Wealth::account_summaries(&self, today: Date) -> Result<Vec<AccountSummary>>`
  - `Wealth::record_valuation(&self, AccountId, as_of: Date, amount: Decimal, now: OffsetDateTime) -> Result<Valuation>`
  - `Wealth::account_history(&self, AccountId) -> Result<Vec<Valuation>>`
  - `Wealth::net_worth(&self, at: Date) -> Result<NetWorth>`
  - `Wealth::net_worth_history(&self, from: Date, to: Date) -> Result<Vec<NetWorth>>`
  - `month_ends(from: Date, to: Date) -> Vec<Date>`

- [ ] **Step 1: Écrire signatures et tests qui échouent**

```rust
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
        Self { accounts, valuations, rates }
    }

    /// Opens an account in a supported currency.
    ///
    /// # Errors
    /// [`WealthError::UnsupportedCurrency`], [`WealthError::EmptyAccountName`], storage errors.
    pub async fn create_account(&self, input: NewAccount, now: OffsetDateTime) -> Result<Account, WealthError> {
        todo!()
    }

    /// Archives an account.
    ///
    /// # Errors
    /// [`WealthError::AccountNotFound`], storage errors.
    pub async fn archive_account(&self, id: AccountId) -> Result<(), WealthError> {
        todo!()
    }

    /// Lists every account with its latest valuation on or before `today`.
    ///
    /// # Errors
    /// Storage and cipher errors.
    pub async fn account_summaries(&self, today: Date) -> Result<Vec<AccountSummary>, WealthError> {
        todo!()
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
        todo!()
    }

    /// Every valuation of an account, oldest first.
    ///
    /// # Errors
    /// [`WealthError::AccountNotFound`], storage errors.
    pub async fn account_history(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        todo!()
    }

    /// Net worth on `at`.
    ///
    /// # Errors
    /// [`WealthError::ExchangeRateMissing`], storage errors.
    pub async fn net_worth(&self, at: Date) -> Result<NetWorth, WealthError> {
        todo!()
    }

    /// One net worth per month end in `[from, to)`, then one on `to`.
    ///
    /// # Errors
    /// [`WealthError::InvalidRange`] when `to < from`, and the errors of [`Self::net_worth`].
    pub async fn net_worth_history(&self, from: Date, to: Date) -> Result<Vec<NetWorth>, WealthError> {
        todo!()
    }
}

/// Last day of each month from the month of `from`, strictly before `to`,
/// followed by `to` itself. Empty when `to < from`.
#[must_use]
pub fn month_ends(from: Date, to: Date) -> Vec<Date> {
    todo!()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::macros::{date, datetime};

    use super::*;
    use crate::wealth::test_support::{FakeAccounts, FakeRates, FakeValuations};

    const NOW: OffsetDateTime = datetime!(2026-10-04 12:00 UTC);

    fn wealth_with(rates: FakeRates) -> Wealth {
        Wealth::new(Arc::new(FakeAccounts::default()), Arc::new(FakeValuations::default()), Arc::new(rates))
    }

    fn savings(currency: Currency) -> NewAccount {
        NewAccount { name: "Livret A".into(), kind: AccountKind::Savings, owner: Owner::Personal, currency, notes: None }
    }

    #[tokio::test]
    async fn create_account_rejects_currency_without_rate() {
        let wealth = wealth_with(FakeRates::default());
        let btc: Currency = "BTC".parse().unwrap();
        assert_eq!(
            wealth.create_account(savings(btc), NOW).await,
            Err(WealthError::UnsupportedCurrency(btc))
        );
        assert!(wealth.create_account(savings(Currency::EUR), NOW).await.is_ok());
    }

    #[tokio::test]
    async fn record_valuation_uses_account_currency_and_checks_existence() {
        let wealth = wealth_with(FakeRates::default());
        let account = wealth.create_account(savings(Currency::EUR), NOW).await.unwrap();
        let valuation = wealth
            .record_valuation(account.id, date!(2026-10-04), Decimal::new(22_950, 0), NOW)
            .await
            .unwrap();
        assert_eq!(valuation.amount, Money::eur(Decimal::new(22_950, 0)));
        assert_eq!(valuation.source, ValuationSource::Manual);
        assert_eq!(
            wealth.record_valuation(AccountId::generate(), date!(2026-10-04), Decimal::ONE, NOW).await,
            Err(WealthError::AccountNotFound)
        );
    }

    #[tokio::test]
    async fn correction_on_same_date_wins() {
        let wealth = wealth_with(FakeRates::default());
        let account = wealth.create_account(savings(Currency::EUR), NOW).await.unwrap();
        wealth.record_valuation(account.id, date!(2026-10-01), Decimal::new(1000, 0), NOW).await.unwrap();
        wealth
            .record_valuation(account.id, date!(2026-10-01), Decimal::new(1100, 0), NOW + time::Duration::minutes(1))
            .await
            .unwrap();
        let net_worth = wealth.net_worth(date!(2026-10-04)).await.unwrap();
        assert_eq!(net_worth.total, Money::eur(Decimal::new(1100, 0)));
    }

    #[tokio::test]
    async fn net_worth_falls_back_to_previous_business_day_rate() {
        let usd: Currency = "USD".parse().unwrap();
        let rates = FakeRates::default().with(usd, date!(2026-10-02), Decimal::new(2, 0));
        let wealth = wealth_with(rates);
        let account = wealth.create_account(savings(usd), NOW).await.unwrap();
        wealth.record_valuation(account.id, date!(2026-10-04), Decimal::new(100, 0), NOW).await.unwrap();
        let net_worth = wealth.net_worth(date!(2026-10-04)).await.unwrap();
        assert_eq!(net_worth.total, Money::eur(Decimal::new(50, 0)));
    }

    #[tokio::test]
    async fn net_worth_reports_missing_rate_beyond_seven_days() {
        let usd: Currency = "USD".parse().unwrap();
        let rates = FakeRates::default().with(usd, date!(2026-09-20), Decimal::new(2, 0));
        let wealth = wealth_with(rates);
        let account = wealth.create_account(savings(usd), NOW).await.unwrap();
        wealth.record_valuation(account.id, date!(2026-10-04), Decimal::new(100, 0), NOW).await.unwrap();
        assert_eq!(
            wealth.net_worth(date!(2026-10-04)).await,
            Err(WealthError::ExchangeRateMissing { currency: usd, on: date!(2026-10-04) })
        );
    }

    #[tokio::test]
    async fn summaries_flag_stale_accounts_but_not_archived_ones() {
        let wealth = wealth_with(FakeRates::default());
        let fresh = wealth.create_account(savings(Currency::EUR), NOW).await.unwrap();
        let never_valued = wealth.create_account(savings(Currency::EUR), NOW).await.unwrap();
        let archived = wealth.create_account(savings(Currency::EUR), NOW).await.unwrap();
        wealth.record_valuation(fresh.id, date!(2026-10-01), Decimal::ONE, NOW).await.unwrap();
        wealth.archive_account(archived.id).await.unwrap();

        let summaries = wealth.account_summaries(date!(2026-10-04)).await.unwrap();

        let stale_of = |id| summaries.iter().find(|s| s.account.id == id).unwrap().is_stale;
        assert!(!stale_of(fresh.id));
        assert!(stale_of(never_valued.id));
        assert!(!stale_of(archived.id));
    }

    #[tokio::test]
    async fn history_rejects_inverted_range() {
        let wealth = wealth_with(FakeRates::default());
        assert_eq!(
            wealth.net_worth_history(date!(2026-10-04), date!(2026-01-01)).await,
            Err(WealthError::InvalidRange)
        );
    }

    #[test]
    fn month_ends_lists_each_month_end_then_the_end_date() {
        assert_eq!(
            month_ends(date!(2026-01-15), date!(2026-03-10)),
            vec![date!(2026-01-31), date!(2026-02-28), date!(2026-03-10)]
        );
        assert_eq!(
            month_ends(date!(2025-12-01), date!(2026-01-31)),
            vec![date!(2025-12-31), date!(2026-01-31)]
        );
        assert_eq!(month_ends(date!(2026-10-04), date!(2026-10-04)), vec![date!(2026-10-04)]);
        assert!(month_ends(date!(2026-10-04), date!(2026-10-01)).is_empty());
    }
}
```

Mettre à jour `mod.rs` comme indiqué dans **Files**.

- [ ] **Step 2: Vérifier l'échec**

Run: `cd backend && cargo test -p domain use_cases`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 3: Implémenter**

```rust
    pub async fn create_account(&self, input: NewAccount, now: OffsetDateTime) -> Result<Account, WealthError> {
        if !self.rates.is_supported(input.currency).await? {
            return Err(WealthError::UnsupportedCurrency(input.currency));
        }
        let account = Account::open(&input.name, input.kind, input.owner, input.currency, input.notes.as_deref(), now)?;
        self.accounts.insert(&account).await?;
        Ok(account)
    }

    pub async fn archive_account(&self, id: AccountId) -> Result<(), WealthError> {
        self.accounts.archive(id).await
    }

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
                AccountSummary { account, latest_valuation, is_stale }
            })
            .collect())
    }

    pub async fn record_valuation(
        &self,
        account_id: AccountId,
        as_of: Date,
        amount: Decimal,
        now: OffsetDateTime,
    ) -> Result<Valuation, WealthError> {
        let account = self.accounts.find(account_id).await?.ok_or(WealthError::AccountNotFound)?;
        let valuation = Valuation::record(&account, as_of, Money::new(amount, account.currency), ValuationSource::Manual, now)?;
        self.valuations.insert(&valuation).await?;
        Ok(valuation)
    }

    pub async fn account_history(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        if self.accounts.find(account_id).await?.is_none() {
            return Err(WealthError::AccountNotFound);
        }
        self.valuations.for_account(account_id).await
    }

    pub async fn net_worth(&self, at: Date) -> Result<NetWorth, WealthError> {
        let accounts = self.accounts.list().await?;
        let latest = self.valuations.latest_per_account(at).await?;
        let mut rates = RateTable::default();
        for valuation in &latest {
            let currency = valuation.amount.currency;
            let is_active = accounts.iter().any(|a| a.id == valuation.account_id && !a.is_archived);
            if currency == Currency::EUR || !is_active {
                continue;
            }
            let units_per_eur = self
                .rates
                .units_per_eur(currency, valuation.as_of)
                .await?
                .ok_or(WealthError::ExchangeRateMissing { currency, on: valuation.as_of })?;
            rates.0.insert((currency, valuation.as_of), units_per_eur);
        }
        NetWorth::compute(at, &accounts, &latest, &rates)
    }

    pub async fn net_worth_history(&self, from: Date, to: Date) -> Result<Vec<NetWorth>, WealthError> {
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

pub fn month_ends(from: Date, to: Date) -> Vec<Date> {
    if to < from {
        return Vec::new();
    }
    let mut points = Vec::new();
    let mut cursor = last_day_of_month(from);
    while cursor < to {
        points.push(cursor);
        let Some(next_month_day) = cursor.next_day() else { break };
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
```

Comme en Task 2, l'implémentation ferme elle-même le bloc `impl Wealth` : retirer l'accolade d'origine et le `todo!()` de `month_ends`.

- [ ] **Step 4: Tests, clippy, fmt**

Run: `cd backend && cargo test -p domain && cargo clippy -p domain --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/crates/domain/src/wealth
git commit -m "feat(wealth): add wealth use cases and month-end history

Refs #<issue W0.1 use cases>"
```

---

### Task 4: cipher-egide, client Transit

**Files:**
- Modify: `backend/Cargo.toml` (ajouter `"crates/cipher-egide"` aux `members`)
- Create: `backend/crates/cipher-egide/Cargo.toml`
- Create: `backend/crates/cipher-egide/src/lib.rs`
- Create: `backend/crates/cipher-egide/src/egide_client.rs`

**Interfaces:**
- Produces:
  - `EgideClient::new(base_url: impl Into<String>, token: impl Into<String>) -> EgideClient` (pas de `Debug`, le token ne doit jamais s'afficher)
  - `EgideClient::generate_datakey(&self, key_name: &str) -> Result<GeneratedDatakey, EgideError>`
  - `EgideClient::decrypt(&self, key_name: &str, ciphertext: &str) -> Result<Zeroizing<Vec<u8>>, EgideError>`
  - `EgideClient::rewrap(&self, key_name: &str, ciphertext: &str) -> Result<String, EgideError>`
  - `GeneratedDatakey { plaintext: Zeroizing<Vec<u8>>, ciphertext: String }`
  - `EgideError { Sealed, Unauthorized, Unexpected(u16), Transport(String), Malformed }` (`Display` sans secret)

- [ ] **Step 1: Créer le crate**

`backend/crates/cipher-egide/Cargo.toml` :

```toml
[package]
name = "cipher-egide"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../domain" }
aes-gcm = "0.10"
async-trait = "0.1"
base64 = "0.22"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
zeroize = "1"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
uuid = { version = "1", features = ["v4"] }
wiremock = "0.6"
```

`backend/crates/cipher-egide/src/lib.rs` :

```rust
//! Egide Transit adapter for the wealth [`domain::wealth::FieldCipher`] port:
//! envelope encryption with a data key wrapped by Egide.

pub mod egide_client;

pub use egide_client::{EgideClient, EgideError, GeneratedDatakey};
```

- [ ] **Step 2: Écrire les tests qui échouent**

`backend/crates/cipher-egide/src/egide_client.rs` :

```rust
//! Minimal HTTP client for the Egide Transit engine.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Egide failures. `Display` never contains a token, a key or a ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EgideError {
    /// Egide answered 503: it is sealed.
    #[error("egide is sealed")]
    Sealed,
    /// Egide refused the token (401 or 403).
    #[error("egide refused the token")]
    Unauthorized,
    /// Any other non-success status.
    #[error("egide answered status {0}")]
    Unexpected(u16),
    /// Network failure before any answer.
    #[error("egide unreachable: {0}")]
    Transport(String),
    /// The answer could not be decoded.
    #[error("egide answer is malformed")]
    Malformed,
}

/// A freshly generated data key: the plaintext to use now, the ciphertext to store.
pub struct GeneratedDatakey {
    /// Raw key bytes, wiped on drop.
    pub plaintext: Zeroizing<Vec<u8>>,
    /// Key wrapped by Egide (`egide:vN:...`).
    pub ciphertext: String,
}

/// Egide Transit client authenticated by a service token.
pub struct EgideClient {
    http: reqwest::Client,
    base_url: String,
    token: Zeroizing<String>,
}

#[derive(Serialize)]
struct CiphertextBody<'a> {
    ciphertext: &'a str,
}

#[derive(Deserialize)]
struct PlaintextAnswer {
    plaintext: String,
}

#[derive(Deserialize)]
struct CiphertextAnswer {
    ciphertext: String,
}

#[derive(Deserialize)]
struct DatakeyAnswer {
    plaintext: String,
    ciphertext: String,
}

impl EgideClient {
    /// Builds a client for `base_url` (for example `http://egide:8200`).
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        todo!()
    }

    /// Generates a data key wrapped under `key_name`.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn generate_datakey(&self, key_name: &str) -> Result<GeneratedDatakey, EgideError> {
        todo!()
    }

    /// Unwraps a data key.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn decrypt(&self, key_name: &str, ciphertext: &str) -> Result<Zeroizing<Vec<u8>>, EgideError> {
        todo!()
    }

    /// Re-wraps a ciphertext with the latest version of `key_name`.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn rewrap(&self, key_name: &str, ciphertext: &str) -> Result<String, EgideError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

    #[tokio::test]
    async fn generate_datakey_sends_bearer_and_decodes_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/datakey/joel-wealth"))
            .and(header("authorization", "Bearer egst_test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": KEY_B64, "ciphertext": "egide:v1:wrapped" }),
            ))
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");

        let datakey = client.generate_datakey("joel-wealth").await.unwrap();

        assert_eq!(datakey.plaintext.len(), 32);
        assert_eq!(datakey.plaintext[1], 1);
        assert_eq!(datakey.ciphertext, "egide:v1:wrapped");
    }

    #[tokio::test]
    async fn decrypt_and_rewrap_send_the_ciphertext() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/decrypt/joel-wealth"))
            .and(body_json(serde_json::json!({ "ciphertext": "egide:v1:wrapped" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "plaintext": KEY_B64 })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/rewrap/joel-wealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ciphertext": "egide:v2:wrapped" })))
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");

        assert_eq!(client.decrypt("joel-wealth", "egide:v1:wrapped").await.unwrap().len(), 32);
        assert_eq!(client.rewrap("joel-wealth", "egide:v1:wrapped").await.unwrap(), "egide:v2:wrapped");
    }

    #[tokio::test]
    async fn maps_statuses_to_errors() {
        for (status, expected) in [
            (503, EgideError::Sealed),
            (401, EgideError::Unauthorized),
            (403, EgideError::Unauthorized),
            (500, EgideError::Unexpected(500)),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;
            let client = EgideClient::new(server.uri(), "egst_test");
            assert_eq!(client.generate_datakey("joel-wealth").await.err(), Some(expected));
        }
    }

    #[tokio::test]
    async fn malformed_answer_and_unreachable_server_are_reported() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "plaintext": "%%%" })))
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");
        assert_eq!(client.decrypt("joel-wealth", "x").await.err(), Some(EgideError::Malformed));

        let unreachable = EgideClient::new("http://127.0.0.1:9", "egst_test");
        assert!(matches!(unreachable.decrypt("joel-wealth", "x").await, Err(EgideError::Transport(_))));
    }
}
```

Ajouter `serde_json = "1"` aux `[dev-dependencies]`.

- [ ] **Step 3: Vérifier l'échec**

Run: `cd backend && cargo test -p cipher-egide`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 4: Implémenter**

```rust
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: Zeroizing::new(token.into()),
        }
    }

    pub async fn generate_datakey(&self, key_name: &str) -> Result<GeneratedDatakey, EgideError> {
        let answer: DatakeyAnswer = self.post(&format!("datakey/{key_name}"), None).await?;
        Ok(GeneratedDatakey {
            plaintext: decode_key(&answer.plaintext)?,
            ciphertext: answer.ciphertext,
        })
    }

    pub async fn decrypt(&self, key_name: &str, ciphertext: &str) -> Result<Zeroizing<Vec<u8>>, EgideError> {
        let answer: PlaintextAnswer = self
            .post(&format!("decrypt/{key_name}"), Some(&CiphertextBody { ciphertext }))
            .await?;
        decode_key(&answer.plaintext)
    }

    pub async fn rewrap(&self, key_name: &str, ciphertext: &str) -> Result<String, EgideError> {
        let answer: CiphertextAnswer = self
            .post(&format!("rewrap/{key_name}"), Some(&CiphertextBody { ciphertext }))
            .await?;
        Ok(answer.ciphertext)
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        operation: &str,
        body: Option<&CiphertextBody<'_>>,
    ) -> Result<T, EgideError> {
        let mut request = self
            .http
            .post(format!("{}/v1/transit/{operation}", self.base_url))
            .bearer_auth(self.token.as_str());
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| EgideError::Transport(error.without_url().to_string()))?;
        match response.status() {
            status if status.is_success() => response.json::<T>().await.map_err(|_| EgideError::Malformed),
            StatusCode::SERVICE_UNAVAILABLE => Err(EgideError::Sealed),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(EgideError::Unauthorized),
            status => Err(EgideError::Unexpected(status.as_u16())),
        }
    }
}

fn decode_key(encoded: &str) -> Result<Zeroizing<Vec<u8>>, EgideError> {
    STANDARD
        .decode(encoded)
        .map(Zeroizing::new)
        .map_err(|_| EgideError::Malformed)
}
```

L'implémentation ferme elle-même le bloc `impl EgideClient`.

- [ ] **Step 5: Tests, clippy, fmt, deny**

Run: `cd backend && cargo test -p cipher-egide && cargo clippy -p cipher-egide --all-targets -- -D warnings && cargo fmt --check && cargo deny check`
Expected: PASS. Si `cargo deny` refuse une licence d'une nouvelle dépendance transitive, l'ajouter à `deny.toml` seulement si elle est permissive (MIT, Apache-2.0, BSD, ISC, Unicode-3.0) et le signaler dans le compte rendu de tâche.

- [ ] **Step 6: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/crates/cipher-egide
git commit -m "feat(cipher-egide): add Egide Transit client

Refs #<issue W0.2 client>"
```

---

### Task 5: cipher-egide, chiffrement par enveloppe

**Files:**
- Create: `backend/crates/cipher-egide/src/envelope_cipher.rs`
- Modify: `backend/crates/cipher-egide/src/lib.rs`

**Interfaces:**
- Consumes: `EgideClient`, `EgideError` (Task 4) ; `FieldCipher`, `CipherContext`, `WrappedKeyStore`, `WealthError`, `FakeWrappedKeys` (Task 2).
- Produces:
  - `WEALTH_KEY_NAME: &str = "joel-wealth"`
  - `EgideEnvelopeCipher::from_key(key: Zeroizing<[u8; 32]>) -> Self`
  - `EgideEnvelopeCipher::unlock(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<Self, UnlockError>`
  - `rewrap_stored_key(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<(), UnlockError>`
  - `UnlockError { Egide(EgideError), Store(WealthError), NoStoredKey, InvalidKeyLength }`
  - `impl FieldCipher for EgideEnvelopeCipher` (AES-256-GCM, `nonce (12) || ciphertext`, AAD = `context.aad()`)

- [ ] **Step 1: Écrire signatures et tests qui échouent**

```rust
//! Envelope encryption: a 256-bit data key, wrapped by Egide, encrypts every
//! field locally with AES-256-GCM. The clear key lives only in memory.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use domain::wealth::{CipherContext, FieldCipher, WealthError, WrappedKeyStore};
use zeroize::Zeroizing;

use crate::egide_client::{EgideClient, EgideError};

/// Name of the Egide Transit key that wraps the wealth data key.
pub const WEALTH_KEY_NAME: &str = "joel-wealth";

/// Why the vault could not be unlocked. `Display` never contains key material.
#[derive(Debug, thiserror::Error)]
pub enum UnlockError {
    /// Egide refused or could not answer.
    #[error("egide: {0}")]
    Egide(#[from] EgideError),
    /// The wrapped key store failed.
    #[error("wrapped key store: {0}")]
    Store(#[from] WealthError),
    /// Rewrap requested while no key was ever stored.
    #[error("no wrapped key is stored")]
    NoStoredKey,
    /// Egide returned a key that is not 32 bytes long.
    #[error("data key has an invalid length")]
    InvalidKeyLength,
}

/// [`FieldCipher`] backed by an Egide-wrapped data key.
pub struct EgideEnvelopeCipher {
    key: Zeroizing<[u8; 32]>,
}

impl EgideEnvelopeCipher {
    /// Builds a cipher from a clear data key (tests and unlock).
    #[must_use]
    pub const fn from_key(key: Zeroizing<[u8; 32]>) -> Self {
        Self { key }
    }

    /// Unwraps the stored data key, or generates and stores one on first start.
    ///
    /// # Errors
    /// See [`UnlockError`].
    pub async fn unlock(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<Self, UnlockError> {
        todo!()
    }
}

/// Re-wraps the current data key with the latest Egide key version and stores
/// it as a new version. Encrypted fields are untouched.
///
/// # Errors
/// See [`UnlockError`].
pub async fn rewrap_stored_key(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<(), UnlockError> {
    todo!()
}

impl FieldCipher for EgideEnvelopeCipher {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        todo!()
    }

    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::test_support::FakeWrappedKeys;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

    fn context(column: &'static str, row_id: Uuid) -> CipherContext {
        CipherContext { table: "wealth_valuations", column, row_id }
    }

    fn cipher() -> EgideEnvelopeCipher {
        EgideEnvelopeCipher::from_key(Zeroizing::new([7_u8; 32]))
    }

    #[test]
    fn roundtrip_restores_the_plaintext() {
        let ctx = context("amount", Uuid::new_v4());
        let sealed = cipher().encrypt(b"12345.67", &ctx).unwrap();
        assert_ne!(&sealed[12..], b"12345.67");
        assert_eq!(cipher().decrypt(&sealed, &ctx).unwrap(), b"12345.67");
    }

    #[test]
    fn decrypt_fails_with_another_row_or_column() {
        let row_id = Uuid::new_v4();
        let sealed = cipher().encrypt(b"12345.67", &context("amount", row_id)).unwrap();
        assert_eq!(cipher().decrypt(&sealed, &context("amount", Uuid::new_v4())), Err(WealthError::Cipher));
        assert_eq!(cipher().decrypt(&sealed, &context("notes", row_id)), Err(WealthError::Cipher));
    }

    #[test]
    fn decrypt_fails_on_tampering_and_truncation() {
        let ctx = context("amount", Uuid::new_v4());
        let mut sealed = cipher().encrypt(b"12345.67", &ctx).unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert_eq!(cipher().decrypt(&sealed, &ctx), Err(WealthError::Cipher));
        assert_eq!(cipher().decrypt(&[1, 2, 3], &ctx), Err(WealthError::Cipher));
    }

    #[test]
    fn two_encryptions_differ_thanks_to_random_nonces() {
        let ctx = context("amount", Uuid::new_v4());
        assert_ne!(cipher().encrypt(b"1", &ctx).unwrap(), cipher().encrypt(b"1", &ctx).unwrap());
    }

    #[tokio::test]
    async fn first_unlock_generates_and_stores_the_wrapped_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/datakey/joel-wealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": KEY_B64, "ciphertext": "egide:v1:wrapped" }),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();

        EgideEnvelopeCipher::unlock(&EgideClient::new(server.uri(), "t"), &store).await.unwrap();

        assert_eq!(store.current().await.unwrap().as_deref(), Some("egide:v1:wrapped"));
    }

    #[tokio::test]
    async fn later_unlock_decrypts_the_stored_key_and_reads_previous_data() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/decrypt/joel-wealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "plaintext": KEY_B64 })))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();
        store.insert("egide:v1:wrapped", WEALTH_KEY_NAME).await.unwrap();
        let mut key = [0_u8; 32];
        for (index, byte) in key.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        let ctx = context("amount", Uuid::new_v4());
        let sealed = EgideEnvelopeCipher::from_key(Zeroizing::new(key)).encrypt(b"42", &ctx).unwrap();

        let unlocked = EgideEnvelopeCipher::unlock(&EgideClient::new(server.uri(), "t"), &store).await.unwrap();

        assert_eq!(unlocked.decrypt(&sealed, &ctx).unwrap(), b"42");
    }

    #[tokio::test]
    async fn sealed_egide_and_short_keys_fail_to_unlock() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let result = EgideEnvelopeCipher::unlock(&EgideClient::new(server.uri(), "t"), &FakeWrappedKeys::default()).await;
        assert!(matches!(result, Err(UnlockError::Egide(EgideError::Sealed))));

        let short = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": "AAEC", "ciphertext": "egide:v1:short" }),
            ))
            .mount(&short)
            .await;
        let result = EgideEnvelopeCipher::unlock(&EgideClient::new(short.uri(), "t"), &FakeWrappedKeys::default()).await;
        assert!(matches!(result, Err(UnlockError::InvalidKeyLength)));
    }

    #[tokio::test]
    async fn rewrap_appends_a_new_key_version() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/rewrap/joel-wealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ciphertext": "egide:v2:wrapped" })))
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();
        store.insert("egide:v1:wrapped", WEALTH_KEY_NAME).await.unwrap();

        rewrap_stored_key(&EgideClient::new(server.uri(), "t"), &store).await.unwrap();

        assert_eq!(store.current().await.unwrap().as_deref(), Some("egide:v2:wrapped"));
        assert!(matches!(
            rewrap_stored_key(&EgideClient::new(server.uri(), "t"), &FakeWrappedKeys::default()).await,
            Err(UnlockError::NoStoredKey)
        ));
    }
}
```

Dans `lib.rs`, ajouter :

```rust
pub mod envelope_cipher;

pub use envelope_cipher::{EgideEnvelopeCipher, UnlockError, WEALTH_KEY_NAME, rewrap_stored_key};
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd backend && cargo test -p cipher-egide envelope`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 3: Implémenter**

```rust
    pub async fn unlock(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<Self, UnlockError> {
        let clear = match store.current().await? {
            Some(wrapped) => client.decrypt(WEALTH_KEY_NAME, &wrapped).await?,
            None => {
                let datakey = client.generate_datakey(WEALTH_KEY_NAME).await?;
                to_key(&datakey.plaintext)?;
                store.insert(&datakey.ciphertext, WEALTH_KEY_NAME).await?;
                datakey.plaintext
            }
        };
        Ok(Self::from_key(to_key(&clear)?))
    }
}

fn to_key(bytes: &[u8]) -> Result<Zeroizing<[u8; 32]>, UnlockError> {
    let array: [u8; 32] = bytes.try_into().map_err(|_| UnlockError::InvalidKeyLength)?;
    Ok(Zeroizing::new(array))
}

pub async fn rewrap_stored_key(client: &EgideClient, store: &dyn WrappedKeyStore) -> Result<(), UnlockError> {
    let current = store.current().await?.ok_or(UnlockError::NoStoredKey)?;
    let rewrapped = client.rewrap(WEALTH_KEY_NAME, &current).await?;
    if rewrapped != current {
        store.insert(&rewrapped, WEALTH_KEY_NAME).await?;
    }
    Ok(())
}

impl FieldCipher for EgideEnvelopeCipher {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.as_ref()));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let aad = context.aad();
        let sealed = cipher
            .encrypt(&nonce, Payload { msg: plaintext, aad: &aad })
            .map_err(|_| WealthError::Cipher)?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let (nonce, sealed) = ciphertext.split_at_checked(12).ok_or(WealthError::Cipher)?;
        if sealed.is_empty() {
            return Err(WealthError::Cipher);
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.as_ref()));
        let aad = context.aad();
        cipher
            .decrypt(Nonce::from_slice(nonce), Payload { msg: sealed, aad: &aad })
            .map_err(|_| WealthError::Cipher)
    }
}
```

Retirer les `todo!()` et l'accolade d'origine du bloc `impl EgideEnvelopeCipher` (l'implémentation la ferme).

- [ ] **Step 4: Tests, clippy, fmt**

Run: `cd backend && cargo test -p cipher-egide && cargo clippy -p cipher-egide --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/crates/cipher-egide backend/Cargo.lock
git commit -m "feat(cipher-egide): add Egide envelope field cipher with key rewrap

Refs #<issue W0.2 enveloppe>"
```

---

### Task 6: Persistance wealth, migration et dépôts chiffrés

**Files:**
- Create: `backend/migrations/0003_wealth.sql`
- Modify: `backend/crates/persistence/Cargo.toml`
- Modify: `backend/crates/persistence/src/lib.rs` (ajouter `pub mod wealth;`)
- Create: `backend/crates/persistence/src/wealth/mod.rs`
- Create: `backend/crates/persistence/src/wealth/accounts.rs`
- Create: `backend/crates/persistence/src/wealth/valuations.rs`
- Create: `backend/crates/persistence/src/wealth/exchange_rates.rs`
- Create: `backend/crates/persistence/src/wealth/wrapped_keys.rs`
- Modify: `docs/superpowers/specs/2026-10-04-joel-wealth-foundation-design.md` (section 6, colonne `kind` unique)

**Interfaces:**
- Consumes: ports, modèle et `FakeCipher` (Tasks 1-2).
- Produces:
  - `PgAccounts::new(pool: PgPool, cipher: Arc<dyn FieldCipher>)`, `PgValuations::new(pool, cipher)` : implémentent `AccountRepository` et `ValuationRepository`.
  - `PgExchangeRates::new(pool)` : implémente `ExchangeRateSource`, plus `store_rates(&self, rates: &[(Currency, Date, Decimal)]) -> Result<u64, WealthError>` (insère, ignore les doublons, renvoie le nombre de lignes ajoutées).
  - `PgWrappedKeys::new(pool)` : implémente `WrappedKeyStore`.

- [ ] **Step 1: Écrire la migration**

`backend/migrations/0003_wealth.sql` :

```sql
CREATE TABLE wealth_accounts (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN (
        'brokerage_pea', 'brokerage_cto', 'life_insurance', 'retirement_plan', 'bank_account',
        'savings', 'crypto_wallet', 'real_estate', 'company_shares', 'loan')),
    owner TEXT NOT NULL CHECK (owner IN ('personal', 'company')),
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    name BYTEA NOT NULL,
    notes BYTEA,
    is_archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE wealth_valuations (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES wealth_accounts(id),
    as_of DATE NOT NULL,
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    amount BYTEA NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('manual', 'csv_import', 'bank_aggregation', 'price_feed')),
    recorded_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX wealth_valuations_latest_idx
    ON wealth_valuations (account_id, as_of DESC, recorded_at DESC);

CREATE FUNCTION wealth_valuations_reject_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'wealth_valuations is append-only';
END;
$$;
CREATE TRIGGER wealth_valuations_append_only
    BEFORE UPDATE OR DELETE ON wealth_valuations
    FOR EACH ROW EXECUTE FUNCTION wealth_valuations_reject_change();

CREATE TABLE wealth_exchange_rates (
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    on_date DATE NOT NULL,
    units_per_eur NUMERIC(20, 10) NOT NULL CHECK (units_per_eur > 0),
    PRIMARY KEY (currency, on_date)
);

CREATE TABLE wealth_keys (
    version INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    egide_key_name TEXT NOT NULL,
    wrapped_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- [ ] **Step 2: Dépendances et module**

Dans `backend/crates/persistence/Cargo.toml` :

```toml
[dependencies]
domain = { path = "../domain" }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "time", "json", "migrate", "rust_decimal"] }
async-trait = "0.1"
rust_decimal = "1"
serde_json = "1"
time = "0.3"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
time = { version = "0.3", features = ["macros"] }
```

`backend/crates/persistence/src/wealth/mod.rs` :

```rust
//! `PostgreSQL` adapters for the wealth ports. Sensitive fields are encrypted
//! through [`domain::wealth::FieldCipher`] before reaching the database.

pub mod accounts;
pub mod exchange_rates;
pub mod valuations;
pub mod wrapped_keys;

pub use accounts::PgAccounts;
pub use exchange_rates::PgExchangeRates;
pub use valuations::PgValuations;
pub use wrapped_keys::PgWrappedKeys;

use domain::wealth::WealthError;

pub(crate) fn storage(error: &sqlx::Error) -> WealthError {
    WealthError::Storage(error.to_string())
}
```

- [ ] **Step 3: Écrire `accounts.rs` avec ses tests, qui échouent**

```rust
//! Encrypted account storage.

use std::sync::Arc;

use async_trait::async_trait;
use domain::wealth::{
    Account, AccountId, AccountKind, AccountRepository, CipherContext, FieldCipher, Owner, WealthError,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::storage;

const TABLE: &str = "wealth_accounts";

/// `PostgreSQL` implementation of [`AccountRepository`].
pub struct PgAccounts {
    pool: PgPool,
    cipher: Arc<dyn FieldCipher>,
}

type AccountRow = (Uuid, String, String, String, Vec<u8>, Option<Vec<u8>>, bool, OffsetDateTime);

impl PgAccounts {
    /// Builds the adapter.
    #[must_use]
    pub fn new(pool: PgPool, cipher: Arc<dyn FieldCipher>) -> Self {
        Self { pool, cipher }
    }

    fn seal(&self, column: &'static str, id: AccountId, text: &str) -> Result<Vec<u8>, WealthError> {
        self.cipher.encrypt(text.as_bytes(), &CipherContext { table: TABLE, column, row_id: id.0 })
    }

    fn open(&self, column: &'static str, id: AccountId, sealed: &[u8]) -> Result<String, WealthError> {
        let bytes = self.cipher.decrypt(sealed, &CipherContext { table: TABLE, column, row_id: id.0 })?;
        String::from_utf8(bytes).map_err(|_| WealthError::Cipher)
    }

    fn to_account(&self, row: AccountRow) -> Result<Account, WealthError> {
        todo!()
    }
}

#[async_trait]
impl AccountRepository for PgAccounts {
    async fn insert(&self, account: &Account) -> Result<(), WealthError> {
        todo!()
    }
    async fn find(&self, id: AccountId) -> Result<Option<Account>, WealthError> {
        todo!()
    }
    async fn list(&self) -> Result<Vec<Account>, WealthError> {
        todo!()
    }
    async fn archive(&self, id: AccountId) -> Result<(), WealthError> {
        todo!()
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
            AccountKind::Brokerage { envelope: BrokerageEnvelope::Pea },
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

        assert_eq!(repository.find(account.id).await.unwrap(), Some(account.clone()));
        assert_eq!(repository.list().await.unwrap(), vec![account.clone()]);
        repository.archive(account.id).await.unwrap();
        assert!(repository.find(account.id).await.unwrap().unwrap().is_archived);
        assert_eq!(repository.archive(AccountId::generate()).await, Err(WealthError::AccountNotFound));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn name_and_notes_are_never_stored_in_clear(pool: PgPool) {
        let repository = repository(pool.clone());
        repository.insert(&pea()).await.unwrap();
        let (name, notes): (Vec<u8>, Option<Vec<u8>>) =
            sqlx::query_as("SELECT name, notes FROM wealth_accounts").fetch_one(&pool).await.unwrap();
        assert!(!name.windows(4).any(|w| w == b"PEA "));
        assert!(!notes.unwrap().windows(6).any(|w| w == b"ouvert"));
    }
}
```

- [ ] **Step 4: Écrire `valuations.rs` avec ses tests, qui échouent**

```rust
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
        CipherContext { table: TABLE, column: "amount", row_id: id.0 }
    }

    fn to_valuation(&self, row: ValuationRow) -> Result<Valuation, WealthError> {
        todo!()
    }
}

#[async_trait]
impl ValuationRepository for PgValuations {
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError> {
        todo!()
    }
    async fn latest_per_account(&self, at: Date) -> Result<Vec<Valuation>, WealthError> {
        todo!()
    }
    async fn for_account(&self, account_id: AccountId) -> Result<Vec<Valuation>, WealthError> {
        todo!()
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
        let account = Account::open("Livret", AccountKind::Savings, Owner::Personal, Currency::EUR, None, datetime!(2026-01-01 0:00 UTC)).unwrap();
        PgAccounts::new(pool.clone(), Arc::new(FakeCipher)).insert(&account).await.unwrap();
        account
    }

    fn valuation(account: &Account, as_of: Date, cents: i64, recorded_at: OffsetDateTime) -> Valuation {
        Valuation::record(account, as_of, Money::eur(Decimal::new(cents, 2)), ValuationSource::Manual, recorded_at).unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn latest_per_account_picks_greatest_date_then_latest_record(pool: PgPool) {
        let account = seeded_account(&pool).await;
        let repository = PgValuations::new(pool, Arc::new(FakeCipher));
        repository.insert(&valuation(&account, date!(2026-09-01), 100_00, datetime!(2026-09-01 9:00 UTC))).await.unwrap();
        repository.insert(&valuation(&account, date!(2026-10-01), 200_00, datetime!(2026-10-01 9:00 UTC))).await.unwrap();
        let correction = valuation(&account, date!(2026-10-01), 210_50, datetime!(2026-10-01 9:05 UTC));
        repository.insert(&correction).await.unwrap();
        repository.insert(&valuation(&account, date!(2026-10-03), 999_00, datetime!(2026-10-03 9:00 UTC))).await.unwrap();

        let latest = repository.latest_per_account(date!(2026-10-02)).await.unwrap();

        assert_eq!(latest, vec![correction]);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn for_account_is_ordered_and_amounts_are_encrypted(pool: PgPool) {
        let account = seeded_account(&pool).await;
        let repository = PgValuations::new(pool.clone(), Arc::new(FakeCipher));
        let later = valuation(&account, date!(2026-10-01), 123_456_78, datetime!(2026-10-01 9:00 UTC));
        let earlier = valuation(&account, date!(2026-09-01), 1_00, datetime!(2026-10-01 9:01 UTC));
        repository.insert(&later).await.unwrap();
        repository.insert(&earlier).await.unwrap();

        assert_eq!(repository.for_account(account.id).await.unwrap(), vec![earlier, later]);
        let amounts: Vec<Vec<u8>> = sqlx::query_scalar("SELECT amount FROM wealth_valuations").fetch_all(&pool).await.unwrap();
        assert!(amounts.iter().all(|bytes| !bytes.windows(6).any(|w| w == b"123456")));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn valuations_cannot_be_updated_or_deleted(pool: PgPool) {
        let account = seeded_account(&pool).await;
        PgValuations::new(pool.clone(), Arc::new(FakeCipher))
            .insert(&valuation(&account, date!(2026-10-01), 1_00, datetime!(2026-10-01 9:00 UTC)))
            .await
            .unwrap();
        assert!(sqlx::query("UPDATE wealth_valuations SET as_of = as_of").execute(&pool).await.is_err());
        assert!(sqlx::query("DELETE FROM wealth_valuations").execute(&pool).await.is_err());
    }
}
```

- [ ] **Step 5: Écrire `exchange_rates.rs` et `wrapped_keys.rs` avec leurs tests, qui échouent**

`exchange_rates.rs` :

```rust
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
    pub async fn store_rates(&self, rates: &[(Currency, Date, Decimal)]) -> Result<u64, WealthError> {
        todo!()
    }
}

#[async_trait]
impl ExchangeRateSource for PgExchangeRates {
    async fn units_per_eur(&self, currency: Currency, on: Date) -> Result<Option<Decimal>, WealthError> {
        todo!()
    }
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError> {
        todo!()
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
        let rows = [(usd(), date!(2026-10-02), Decimal::new(10_850, 4))];
        assert_eq!(rates.store_rates(&rows).await.unwrap(), 1);
        assert_eq!(rates.store_rates(&rows).await.unwrap(), 0);

        assert_eq!(rates.units_per_eur(usd(), date!(2026-10-04)).await.unwrap(), Some(Decimal::new(10_850, 4)));
        assert_eq!(rates.units_per_eur(usd(), date!(2026-10-09)).await.unwrap(), Some(Decimal::new(10_850, 4)));
        assert_eq!(rates.units_per_eur(usd(), date!(2026-10-10)).await.unwrap(), None);
        assert_eq!(rates.units_per_eur(usd(), date!(2026-10-01)).await.unwrap(), None);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn euro_is_always_supported_and_one(pool: PgPool) {
        let rates = PgExchangeRates::new(pool);
        assert!(rates.is_supported(Currency::EUR).await.unwrap());
        assert!(!rates.is_supported(usd()).await.unwrap());
        assert_eq!(rates.units_per_eur(Currency::EUR, date!(2026-10-04)).await.unwrap(), Some(Decimal::ONE));
        rates.store_rates(&[(usd(), date!(2026-10-02), Decimal::ONE)]).await.unwrap();
        assert!(rates.is_supported(usd()).await.unwrap());
    }
}
```

`wrapped_keys.rs` :

```rust
//! Versioned storage of the Egide-wrapped wealth data key.

use async_trait::async_trait;
use domain::wealth::{WealthError, WrappedKeyStore};
use sqlx::PgPool;

use super::storage;

/// `PostgreSQL` implementation of [`WrappedKeyStore`].
pub struct PgWrappedKeys {
    pool: PgPool,
}

impl PgWrappedKeys {
    /// Builds the adapter.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrappedKeyStore for PgWrappedKeys {
    async fn current(&self) -> Result<Option<String>, WealthError> {
        todo!()
    }
    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn current_returns_the_latest_version(pool: PgPool) {
        let store = PgWrappedKeys::new(pool);
        assert_eq!(store.current().await.unwrap(), None);
        store.insert("egide:v1:a", "joel-wealth").await.unwrap();
        store.insert("egide:v2:b", "joel-wealth").await.unwrap();
        assert_eq!(store.current().await.unwrap().as_deref(), Some("egide:v2:b"));
    }
}
```

Ajouter `pub mod wealth;` dans `backend/crates/persistence/src/lib.rs`.

- [ ] **Step 6: Vérifier l'échec**

Run (Postgres local lancé via `deploy/compose.dev.yml`, `DATABASE_URL` exporté comme en CI) : `cd backend && cargo test -p persistence wealth`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 7: Implémenter**

`accounts.rs` :

```rust
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
            notes: notes.map(|sealed| self.open("notes", id, &sealed)).transpose()?,
            created_at,
        })
    }
```

```rust
const SELECT: &str = "SELECT id, kind, owner, currency, name, notes, is_archived, created_at FROM wealth_accounts";

#[async_trait]
impl AccountRepository for PgAccounts {
    async fn insert(&self, account: &Account) -> Result<(), WealthError> {
        let name = self.seal("name", account.id, &account.name)?;
        let notes = account.notes.as_deref().map(|text| self.seal("notes", account.id, text)).transpose()?;
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
```

`valuations.rs` :

```rust
    fn to_valuation(&self, row: ValuationRow) -> Result<Valuation, WealthError> {
        let (id, account_id, as_of, currency, amount, source, recorded_at) = row;
        let id = ValuationId(id);
        let clear = self.cipher.decrypt(&amount, &Self::context(id))?;
        let text = std::str::from_utf8(&clear).map_err(|_| WealthError::Cipher)?;
        Ok(Valuation {
            id,
            account_id: AccountId(account_id),
            as_of,
            amount: Money::new(Decimal::from_str(text).map_err(|_| WealthError::Cipher)?, Currency::from_str(&currency)?),
            source: ValuationSource::from_code(&source)?,
            recorded_at,
        })
    }
```

```rust
#[async_trait]
impl ValuationRepository for PgValuations {
    async fn insert(&self, valuation: &Valuation) -> Result<(), WealthError> {
        let amount = self.cipher.encrypt(valuation.amount.amount.to_string().as_bytes(), &Self::context(valuation.id))?;
        sqlx::query(&format!("INSERT INTO wealth_valuations ({COLUMNS}) VALUES ($1, $2, $3, $4, $5, $6, $7)"))
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
```

`exchange_rates.rs` :

```rust
    pub async fn store_rates(&self, rates: &[(Currency, Date, Decimal)]) -> Result<u64, WealthError> {
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
```

```rust
#[async_trait]
impl ExchangeRateSource for PgExchangeRates {
    async fn units_per_eur(&self, currency: Currency, on: Date) -> Result<Option<Decimal>, WealthError> {
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
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM wealth_exchange_rates WHERE currency = $1)")
            .bind(currency.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| storage(&e))
    }
}
```

`wrapped_keys.rs` :

```rust
#[async_trait]
impl WrappedKeyStore for PgWrappedKeys {
    async fn current(&self) -> Result<Option<String>, WealthError> {
        sqlx::query_scalar("SELECT wrapped_key FROM wealth_keys ORDER BY version DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| storage(&e))
    }

    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError> {
        sqlx::query("INSERT INTO wealth_keys (egide_key_name, wrapped_key) VALUES ($1, $2)")
            .bind(egide_key_name)
            .bind(wrapped_key)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| storage(&e))
    }
}
```

- [ ] **Step 8: Mettre la spec à jour**

Dans la spec, section 6, remplacer la ligne de `wealth_accounts` par :

```markdown
| `wealth_accounts` | `id`, `kind` (le code porte l'enveloppe : `brokerage_pea`, `brokerage_cto`), `owner`, `currency`, `is_archived`, `created_at` | `name`, `notes` |
```

- [ ] **Step 9: Tests, clippy, fmt**

Run: `cd backend && cargo test -p persistence && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS (les tests auth existants passent toujours avec la nouvelle migration).

- [ ] **Step 10: Commit**

```bash
git add backend/migrations/0003_wealth.sql backend/crates/persistence backend/Cargo.lock docs/superpowers/specs/2026-10-04-joel-wealth-foundation-design.md
git commit -m "feat(persistence): add encrypted wealth repositories and exchange rate cache

Refs #<issue W0.3 persistance>"
```

---

### Task 7: Taux BCE, crate et tâche du runner

**Files:**
- Modify: `backend/Cargo.toml` (ajouter `"crates/exchange-rates-ecb"` aux `members`)
- Create: `backend/crates/exchange-rates-ecb/Cargo.toml`
- Create: `backend/crates/exchange-rates-ecb/src/lib.rs`
- Create: `backend/crates/exchange-rates-ecb/tests/fixtures/eurofxref-hist-90d-sample.xml`
- Modify: `backend/bins/runner/Cargo.toml`
- Create: `backend/bins/runner/src/exchange_rate_refresh.rs`
- Modify: `backend/bins/runner/src/lib.rs`
- Modify: `backend/bins/runner/src/main.rs`

**Interfaces:**
- Consumes: `Currency` (Task 1), `PgExchangeRates::store_rates` (Task 6).
- Produces:
  - `ECB_LAST_90_DAYS_URL: &str`
  - `EcbRate { currency: Currency, on: Date, units_per_eur: Decimal }`
  - `parse_feed(xml: &str) -> Result<Vec<EcbRate>, EcbError>`
  - `fetch_last_90_days(http: &reqwest::Client) -> Result<Vec<EcbRate>, EcbError>`
  - runner : `refresh_from_feed(xml: &str, store: &PgExchangeRates) -> Result<u64, RefreshError>`, `REFRESH_INTERVAL: Duration` (6 h)

- [ ] **Step 1: Créer le crate et la fixture**

`backend/crates/exchange-rates-ecb/Cargo.toml` :

```toml
[package]
name = "exchange-rates-ecb"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
domain = { path = "../domain" }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
roxmltree = "0.20"
rust_decimal = "1"
thiserror = "2"
time = { version = "0.3", features = ["parsing", "macros"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

`backend/crates/exchange-rates-ecb/tests/fixtures/eurofxref-hist-90d-sample.xml` :

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
  <gesmes:subject>Reference rates</gesmes:subject>
  <gesmes:Sender><gesmes:name>European Central Bank</gesmes:name></gesmes:Sender>
  <Cube>
    <Cube time="2026-10-02">
      <Cube currency="USD" rate="1.0850"/>
      <Cube currency="CHF" rate="0.9412"/>
    </Cube>
    <Cube time="2026-10-01">
      <Cube currency="USD" rate="1.0832"/>
    </Cube>
  </Cube>
</gesmes:Envelope>
```

- [ ] **Step 2: Écrire `lib.rs` avec ses tests, qui échouent**

```rust
//! Client of the European Central Bank daily reference rates feed.

use std::str::FromStr;

use domain::wealth::Currency;
use rust_decimal::Decimal;
use time::Date;
use time::macros::format_description;

/// Reference rates of the last 90 business days.
pub const ECB_LAST_90_DAYS_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml";

/// One ECB reference rate: units of `currency` for one euro on `on`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcbRate {
    /// Quoted currency.
    pub currency: Currency,
    /// Business day of the rate.
    pub on: Date,
    /// Units of the currency for one euro.
    pub units_per_eur: Decimal,
}

/// Feed failures.
#[derive(Debug, thiserror::Error)]
pub enum EcbError {
    /// Download failed.
    #[error("ecb feed unreachable: {0}")]
    Transport(String),
    /// The document is not the expected XML.
    #[error("ecb feed is malformed: {0}")]
    Malformed(String),
}

/// Parses an `eurofxref` document (daily, 90 days or full history).
///
/// # Errors
/// [`EcbError::Malformed`] on invalid XML, date, currency or rate.
pub fn parse_feed(xml: &str) -> Result<Vec<EcbRate>, EcbError> {
    todo!()
}

/// Downloads and parses the last 90 days of reference rates.
///
/// # Errors
/// [`EcbError::Transport`] or [`EcbError::Malformed`].
pub async fn fetch_last_90_days(http: &reqwest::Client) -> Result<Vec<EcbRate>, EcbError> {
    todo!()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use time::macros::date;

    use super::*;

    const SAMPLE: &str = include_str!("../tests/fixtures/eurofxref-hist-90d-sample.xml");

    #[test]
    fn parses_every_day_and_currency() {
        let rates = parse_feed(SAMPLE).unwrap();
        assert_eq!(rates.len(), 3);
        assert!(rates.contains(&EcbRate {
            currency: "USD".parse().unwrap(),
            on: date!(2026-10-02),
            units_per_eur: Decimal::new(10_850, 4),
        }));
        assert!(rates.iter().any(|r| r.currency.as_str() == "CHF" && r.on == date!(2026-10-02)));
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(parse_feed("not xml"), Err(EcbError::Malformed(_))));
        let bad_rate = SAMPLE.replace("1.0850", "abc");
        assert!(matches!(parse_feed(&bad_rate), Err(EcbError::Malformed(_))));
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cd backend && cargo test -p exchange-rates-ecb`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 4: Implémenter**

```rust
pub fn parse_feed(xml: &str) -> Result<Vec<EcbRate>, EcbError> {
    let malformed = |what: &str| EcbError::Malformed(what.to_owned());
    let document = roxmltree::Document::parse(xml).map_err(|error| EcbError::Malformed(error.to_string()))?;
    let date_format = format_description!("[year]-[month]-[day]");
    let mut rates = Vec::new();
    for day in document.descendants().filter(|node| node.has_tag_name("Cube") && node.has_attribute("time")) {
        let on = Date::parse(day.attribute("time").unwrap_or_default(), date_format).map_err(|_| malformed("time"))?;
        for quote in day.children().filter(|node| node.has_tag_name("Cube")) {
            let currency = quote.attribute("currency").ok_or_else(|| malformed("currency"))?;
            let rate = quote.attribute("rate").ok_or_else(|| malformed("rate"))?;
            rates.push(EcbRate {
                currency: currency.parse().map_err(|_| malformed("currency"))?,
                on,
                units_per_eur: Decimal::from_str(rate).map_err(|_| malformed("rate"))?,
            });
        }
    }
    Ok(rates)
}

pub async fn fetch_last_90_days(http: &reqwest::Client) -> Result<Vec<EcbRate>, EcbError> {
    let body = http
        .get(ECB_LAST_90_DAYS_URL)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| EcbError::Transport(error.to_string()))?
        .text()
        .await
        .map_err(|error| EcbError::Transport(error.to_string()))?;
    parse_feed(&body)
}
```

- [ ] **Step 5: Tâche du runner, test qui échoue**

Dans `backend/bins/runner/Cargo.toml`, `[dependencies]`, ajouter :

```toml
exchange-rates-ecb = { path = "../../crates/exchange-rates-ecb" }
persistence = { path = "../../crates/persistence" }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "migrate"] }
```

et dans `[dev-dependencies]` :

```toml
rust_decimal = "1"
time = { version = "0.3", features = ["macros"] }
```

Créer `backend/bins/runner/src/exchange_rate_refresh.rs` :

```rust
//! Periodic refresh of the ECB exchange rate cache used by the wealth module.

use std::time::Duration;

use domain::wealth::WealthError;
use exchange_rates_ecb::{EcbError, parse_feed};
use persistence::wealth::PgExchangeRates;

/// Delay between two refreshes. The ECB publishes once per business day.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

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
    todo!()
}

/// Downloads the last 90 days and stores them.
///
/// # Errors
/// See [`RefreshError`].
pub async fn refresh(http: &reqwest::Client, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    todo!()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::ExchangeRateSource;
    use rust_decimal::Decimal;
    use sqlx::PgPool;
    use time::macros::date;

    use super::*;

    const SAMPLE: &str = include_str!("../../../crates/exchange-rates-ecb/tests/fixtures/eurofxref-hist-90d-sample.xml");

    #[sqlx::test(migrations = "../../migrations")]
    async fn refresh_stores_new_rates_once(pool: PgPool) {
        let store = PgExchangeRates::new(pool);
        assert_eq!(refresh_from_feed(SAMPLE, &store).await.unwrap(), 3);
        assert_eq!(refresh_from_feed(SAMPLE, &store).await.unwrap(), 0);
        assert_eq!(
            store.units_per_eur("USD".parse().unwrap(), date!(2026-10-02)).await.unwrap(),
            Some(Decimal::new(10_850, 4))
        );
    }
}
```

Dans `backend/bins/runner/src/lib.rs`, ajouter `pub mod exchange_rate_refresh;`.

Run: `cd backend && cargo test -p runner exchange_rate_refresh`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 6: Implémenter la tâche et la brancher**

```rust
pub async fn refresh_from_feed(xml: &str, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    let rates: Vec<_> = parse_feed(xml)?
        .into_iter()
        .map(|rate| (rate.currency, rate.on, rate.units_per_eur))
        .collect();
    Ok(store.store_rates(&rates).await?)
}

pub async fn refresh(http: &reqwest::Client, store: &PgExchangeRates) -> Result<u64, RefreshError> {
    let rates: Vec<_> = exchange_rates_ecb::fetch_last_90_days(http)
        .await?
        .into_iter()
        .map(|rate| (rate.currency, rate.on, rate.units_per_eur))
        .collect();
    Ok(store.store_rates(&rates).await?)
}
```

Dans `backend/bins/runner/src/main.rs`, après le bootstrap d'EidosDB et avant la boucle :

```rust
    let exchange_rates = match std::env::var("DATABASE_URL") {
        Ok(url) => match sqlx::PgPool::connect(&url).await {
            Ok(pool) => Some(persistence::wealth::PgExchangeRates::new(pool)),
            Err(error) => {
                tracing::error!(%error, "cannot connect to database, exchange rates disabled");
                None
            }
        },
        Err(_) => None,
    };
    let http = reqwest::Client::new();
    let mut last_rate_refresh: Option<std::time::Instant> = None;
```

et dans la branche `interval.tick()` de la boucle, remplacer le log « no scheduled work yet » par :

```rust
                let is_due = last_rate_refresh
                    .is_none_or(|at| at.elapsed() >= runner::exchange_rate_refresh::REFRESH_INTERVAL);
                if let (true, Some(store)) = (is_due, exchange_rates.as_ref()) {
                    last_rate_refresh = Some(std::time::Instant::now());
                    match runner::exchange_rate_refresh::refresh(&http, store).await {
                        Ok(inserted) => tracing::info!(inserted, "exchange rates refreshed"),
                        Err(error) => tracing::warn!(%error, "exchange rate refresh failed"),
                    }
                }
```

Le runner ne lance pas les migrations : c'est l'`api` qui les applique au démarrage (`depends_on` garantit Postgres, pas l'ordre api/runner). Si la table n'existe pas encore au premier tick, l'erreur est journalisée et la tentative suivante a lieu 6 h plus tard. Pour ne pas attendre, ne marquer `last_rate_refresh` qu'en cas de succès : déplacer `last_rate_refresh = Some(...)` dans la branche `Ok`. Le runner retentera alors à chaque tick (1 min) tant que ça échoue.

- [ ] **Step 7: Tests, clippy, fmt, deny**

Run: `cd backend && cargo test -p exchange-rates-ecb -p runner && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && cargo deny check`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/crates/exchange-rates-ecb backend/bins/runner
git commit -m "feat(runner): refresh ECB exchange rates for the wealth module

Refs #<issue W0.3 taux BCE>"
```

---

### Task 8: API, WealthGate et configuration Egide

**Files:**
- Modify: `backend/bins/api/Cargo.toml`
- Create: `backend/bins/api/src/wealth_vault.rs`
- Modify: `backend/bins/api/src/state.rs`
- Modify: `backend/bins/api/src/lib.rs`
- Modify: `backend/bins/api/src/main.rs`
- Modify: `backend/bins/api/tests/auth_password.rs` et `backend/bins/api/tests/auth_webauthn.rs` (champs ajoutés à `Config`)

**Interfaces:**
- Consumes: `Wealth`, `FieldCipher`, `WealthError`, `FakeCipher` (Tasks 2-3), `EgideClient`, `EgideEnvelopeCipher`, `rewrap_stored_key` (Tasks 4-5), dépôts Pg (Task 6).
- Produces:
  - `Config` gagne `egide_url: Option<String>` et `egide_token: Option<String>` (lus depuis `EGIDE_URL` et le fichier pointé par `EGIDE_TOKEN_FILE`).
  - `trait VaultUnlocker { async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String>; }`, `EgideUnlocker`, `SealedUnlocker`.
  - `WealthGate::new(pool: PgPool, unlocker: Arc<dyn VaultUnlocker>, retry_interval: Duration)`, `WealthGate::wealth(&self) -> Result<Arc<Wealth>, WealthError>`, `UNLOCK_RETRY_INTERVAL` (30 s).
  - `AppState.wealth: Arc<WealthGate>` ; `AppState::build_with_gate(pool, config, gate)` ; `api::build_router_with_gate(pool, config, gate)`.
  - Sous-commande `api rewrap-wealth-key`.

- [ ] **Step 1: Dépendances**

Dans `backend/bins/api/Cargo.toml`, `[dependencies]` :

```toml
cipher-egide = { path = "../../crates/cipher-egide" }
async-trait = "0.1"
rust_decimal = "1"
```

et remplacer `time = "0.3"` par :

```toml
time = { version = "0.3", features = ["macros", "parsing", "formatting"] }
```

- [ ] **Step 2: Écrire `wealth_vault.rs` avec ses tests, qui échouent**

```rust
//! Lazy, retrying access to the wealth use cases, which need the Egide-held key.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cipher_egide::{EgideClient, EgideEnvelopeCipher};
use domain::wealth::{FieldCipher, Wealth, WealthError};
use persistence::wealth::{PgAccounts, PgExchangeRates, PgValuations, PgWrappedKeys};
use sqlx::PgPool;
use tokio::sync::Mutex;

/// Minimum delay between two unlock attempts while the vault is sealed.
pub const UNLOCK_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Produces the field cipher once the vault is reachable.
#[async_trait]
pub trait VaultUnlocker: Send + Sync {
    /// Returns a ready cipher, or a log-safe reason (never key material).
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String>;
}

/// Unlocks through Egide Transit.
pub struct EgideUnlocker {
    client: EgideClient,
    keys: PgWrappedKeys,
}

impl EgideUnlocker {
    /// Builds the unlocker.
    #[must_use]
    pub const fn new(client: EgideClient, keys: PgWrappedKeys) -> Self {
        Self { client, keys }
    }
}

#[async_trait]
impl VaultUnlocker for EgideUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        EgideEnvelopeCipher::unlock(&self.client, &self.keys)
            .await
            .map(|cipher| Arc::new(cipher) as Arc<dyn FieldCipher>)
            .map_err(|error| error.to_string())
    }
}

/// Used when Egide is not configured: the vault stays sealed.
pub struct SealedUnlocker;

#[async_trait]
impl VaultUnlocker for SealedUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        Err("egide is not configured".to_owned())
    }
}

struct GateState {
    wealth: Option<Arc<Wealth>>,
    last_attempt: Option<Instant>,
}

/// Holds the wealth use cases once unlocked, and retries unlocking at most
/// once per `retry_interval` while sealed.
pub struct WealthGate {
    pool: PgPool,
    unlocker: Arc<dyn VaultUnlocker>,
    retry_interval: Duration,
    state: Mutex<GateState>,
}

impl WealthGate {
    /// Builds a sealed gate; the first call to [`Self::wealth`] tries to unlock.
    #[must_use]
    pub fn new(pool: PgPool, unlocker: Arc<dyn VaultUnlocker>, retry_interval: Duration) -> Self {
        Self {
            pool,
            unlocker,
            retry_interval,
            state: Mutex::new(GateState { wealth: None, last_attempt: None }),
        }
    }

    /// The wealth use cases, unlocking the vault when needed.
    ///
    /// # Errors
    /// [`WealthError::VaultSealed`] while the vault cannot be unlocked.
    pub async fn wealth(&self) -> Result<Arc<Wealth>, WealthError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::wealth::test_support::FakeCipher;

    use super::*;

    struct FlakyUnlocker {
        failures_left: AtomicUsize,
        attempts: AtomicUsize,
    }

    #[async_trait]
    impl VaultUnlocker for FlakyUnlocker {
        async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.failures_left.load(Ordering::SeqCst) > 0 {
                self.failures_left.fetch_sub(1, Ordering::SeqCst);
                return Err("sealed".to_owned());
            }
            Ok(Arc::new(FakeCipher))
        }
    }

    fn lazy_pool() -> PgPool {
        PgPool::connect_lazy("postgres://joel:unused@127.0.0.1:1/joel").unwrap()
    }

    #[tokio::test]
    async fn recovers_after_the_vault_is_unsealed() {
        let unlocker = Arc::new(FlakyUnlocker { failures_left: AtomicUsize::new(1), attempts: AtomicUsize::new(0) });
        let gate = WealthGate::new(lazy_pool(), unlocker.clone(), Duration::ZERO);

        assert!(matches!(gate.wealth().await, Err(WealthError::VaultSealed)));
        assert!(gate.wealth().await.is_ok());
        assert!(gate.wealth().await.is_ok());
        assert_eq!(unlocker.attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn does_not_hammer_egide_within_the_retry_interval() {
        let unlocker = Arc::new(FlakyUnlocker { failures_left: AtomicUsize::new(5), attempts: AtomicUsize::new(0) });
        let gate = WealthGate::new(lazy_pool(), unlocker.clone(), Duration::from_secs(3600));

        for _ in 0..3 {
            assert!(matches!(gate.wealth().await, Err(WealthError::VaultSealed)));
        }
        assert_eq!(unlocker.attempts.load(Ordering::SeqCst), 1);
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Ajouter `pub mod wealth_vault;` dans `lib.rs`, puis :
Run: `cd backend && cargo test -p api wealth_vault`
Expected: FAIL, `not yet implemented`.

- [ ] **Step 4: Implémenter `WealthGate::wealth`**

```rust
    pub async fn wealth(&self) -> Result<Arc<Wealth>, WealthError> {
        let mut state = self.state.lock().await;
        if let Some(wealth) = &state.wealth {
            return Ok(Arc::clone(wealth));
        }
        if state.last_attempt.is_some_and(|at| at.elapsed() < self.retry_interval) {
            return Err(WealthError::VaultSealed);
        }
        state.last_attempt = Some(Instant::now());
        match self.unlocker.unlock().await {
            Ok(cipher) => {
                let wealth = Arc::new(Wealth::new(
                    Arc::new(PgAccounts::new(self.pool.clone(), Arc::clone(&cipher))),
                    Arc::new(PgValuations::new(self.pool.clone(), cipher)),
                    Arc::new(PgExchangeRates::new(self.pool.clone())),
                ));
                state.wealth = Some(Arc::clone(&wealth));
                tracing::info!("wealth vault unlocked");
                Ok(wealth)
            }
            Err(reason) => {
                tracing::warn!(%reason, "wealth vault is sealed");
                Err(WealthError::VaultSealed)
            }
        }
    }
```

- [ ] **Step 5: Configuration et état**

Dans `state.rs`, ajouter à `Config` :

```rust
    /// Egide base url (`EGIDE_URL`); the wealth vault stays sealed when absent.
    pub egide_url: Option<String>,
    /// Egide service token, read from the file named by `EGIDE_TOKEN_FILE`.
    pub egide_token: Option<String>,
```

et dans `Config::from_env`, à la fin du `Ok(Self { ... })` :

```rust
            egide_url: std::env::var("EGIDE_URL").ok(),
            egide_token: std::env::var("EGIDE_TOKEN_FILE")
                .ok()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|token| token.trim().to_owned())
                .filter(|token| !token.is_empty()),
```

Ajouter à `AppState` :

```rust
    /// Wealth use cases behind the vault gate.
    pub wealth: Arc<crate::wealth_vault::WealthGate>,
```

Remplacer `AppState::build` par deux fonctions :

```rust
    /// Assembles the state, unlocking the wealth vault through Egide when configured.
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build(pool: PgPool, config: &Config) -> Result<Self, AuthError> {
        let gate = Arc::new(WealthGate::new(pool.clone(), unlocker_from(&pool, config), UNLOCK_RETRY_INTERVAL));
        Self::build_with_gate(pool, config, gate)
    }

    /// Assembles the state with an explicit wealth gate (tests).
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build_with_gate(pool: PgPool, config: &Config, wealth: Arc<WealthGate>) -> Result<Self, AuthError> {
        // corps actuel de `build`, avec `wealth,` ajouté dans le `Ok(Self { ... })`
    }
```

et la fonction libre :

```rust
fn unlocker_from(pool: &PgPool, config: &Config) -> Arc<dyn VaultUnlocker> {
    match (&config.egide_url, &config.egide_token) {
        (Some(url), Some(token)) => Arc::new(EgideUnlocker::new(
            EgideClient::new(url.clone(), token.clone()),
            PgWrappedKeys::new(pool.clone()),
        )),
        _ => Arc::new(SealedUnlocker),
    }
}
```

avec les imports `use cipher_egide::EgideClient;`, `use persistence::wealth::PgWrappedKeys;`, `use crate::wealth_vault::{EgideUnlocker, SealedUnlocker, UNLOCK_RETRY_INTERVAL, VaultUnlocker, WealthGate};`.

Dans `lib.rs`, ajouter :

```rust
/// Builds the core router with an explicit wealth gate (tests).
///
/// # Errors
/// Propagates [`domain::auth::model::AuthError`] when crypto material is invalid.
pub fn build_router_with_gate(
    pool: PgPool,
    config: &Config,
    gate: Arc<wealth_vault::WealthGate>,
) -> Result<Router, domain::auth::model::AuthError> {
    let state = AppState::build_with_gate(pool, config, gate)?;
    Ok(core_router(state))
}
```

et factoriser le corps de `build_router_with` dans `fn core_router(state: AppState) -> Router` (mêmes routes qu'aujourd'hui), appelé par les deux fonctions.

Dans les deux fichiers de test `tests/auth_password.rs` et `tests/auth_webauthn.rs`, ajouter aux `api::Config { ... }` :

```rust
        egide_url: None,
        egide_token: None,
```

(`grep -rn "session_ttl_days: 30" backend/bins/api/tests` liste tous les endroits.)

- [ ] **Step 6: Déverrouillage au démarrage et sous-commande de rotation**

Dans `main.rs`, après le bloc `seed-admin`, ajouter :

```rust
    if args.get(1).map(String::as_str) == Some("rewrap-wealth-key") {
        let (Some(url), Some(token)) = (&config.egide_url, &config.egide_token) else {
            tracing::error!("EGIDE_URL and EGIDE_TOKEN_FILE are required");
            std::process::exit(2);
        };
        let client = cipher_egide::EgideClient::new(url.clone(), token.clone());
        let keys = persistence::wealth::PgWrappedKeys::new(pool);
        if let Err(error) = cipher_egide::rewrap_stored_key(&client, &keys).await {
            tracing::error!(%error, "wealth key rewrap failed");
            std::process::exit(1);
        }
        println!("wealth key rewrapped with the latest Egide key version");
        return;
    }
```

Pour déverrouiller dès le démarrage plutôt qu'au premier appel, remplacer dans `build_app` (`lib.rs`) la ligne `let mut app = build_router_with(pool, config)?;` par :

```rust
    let state = AppState::build(pool.clone(), config)?;
    if state.wealth.wealth().await.is_err() {
        tracing::warn!("wealth vault sealed at startup, wealth routes answer 503 until unsealed");
    }
    let mut app = core_router(state);
```

- [ ] **Step 7: Tests, clippy, fmt**

Run: `cd backend && cargo test -p api && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS, tests auth existants compris.

- [ ] **Step 8: Commit**

```bash
git add backend/bins/api backend/Cargo.lock
git commit -m "feat(api): add wealth vault gate with Egide unlock and key rewrap command

Refs #<issue W0.4 gate>"
```

---

### Task 9: API, routes /api/wealth

**Files:**
- Create: `backend/bins/api/src/wealth_routes.rs`
- Modify: `backend/bins/api/src/lib.rs` (`pub mod wealth_routes;` et `.merge(wealth_routes::router())` dans `core_router`)
- Create: `backend/bins/api/tests/wealth.rs`

**Interfaces:**
- Consumes: `WealthGate::wealth`, `CurrentUser`, `Wealth` et ses types (Tasks 3, 8).
- Produces (contrat consommé par la PWA, Tasks 10-12) :
  - `GET /api/wealth/accounts` -> `AccountResponse[]`
  - `POST /api/wealth/accounts` body `{ name, kind, owner, currency, notes? }` -> `201 AccountResponse`
  - `POST /api/wealth/accounts/{id}/archive` -> `204`
  - `POST /api/wealth/accounts/{id}/valuations` body `{ amount: "1234.56", as_of: "YYYY-MM-DD" }` -> `201 ValuationResponse`
  - `GET /api/wealth/accounts/{id}/valuations` -> `ValuationResponse[]`
  - `GET /api/wealth/net-worth?at=YYYY-MM-DD` -> `NetWorthResponse`
  - `GET /api/wealth/net-worth/history?from=YYYY-MM-DD&to=YYYY-MM-DD` -> `NetWorthResponse[]`
  - Erreurs : `{ "code": "<WealthError::code()>" }`, `400` validation, `404` compte inconnu, `422` taux manquant, `503` coffre scellé, `401` sans session, `500` interne.
  - `AccountResponse { id, name, kind, owner, currency, is_archived, notes, latest_valuation: ValuationResponse | null, is_stale }`
  - `ValuationResponse { id, as_of, amount, currency, source, recorded_at }`
  - `NetWorthResponse { as_of, total, by_owner: { personal?, company? }, by_kind: { <code>?: string }, stale_account_ids }`

- [ ] **Step 1: Écrire les tests d'intégration qui échouent**

`backend/bins/api/tests/wealth.rs` :

```rust
//! Integration tests of the wealth routes, with a fake cipher behind the gate.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

use std::sync::Arc;
use std::time::Duration;

use api::wealth_vault::{SealedUnlocker, VaultUnlocker, WealthGate};
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
use domain::auth::model::User;
use domain::auth::ports::UserRepository;
use domain::wealth::FieldCipher;
use domain::wealth::test_support::FakeCipher;
use http_body_util::BodyExt;
use persistence::auth::PgUsers;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const KEY_B64: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

struct FakeUnlocker;

#[async_trait]
impl VaultUnlocker for FakeUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        Ok(Arc::new(FakeCipher))
    }
}

fn config() -> api::Config {
    api::Config {
        master_key_b64: KEY_B64.to_owned(),
        webauthn_rp_id: "localhost".to_owned(),
        webauthn_origin: "http://localhost:4200".to_owned(),
        session_ttl_days: 30,
        egide_url: None,
        egide_token: None,
    }
}

async fn app_and_cookie(pool: PgPool, unlocker: Arc<dyn VaultUnlocker>) -> (Router, String) {
    let secret = TotpService::generate_secret();
    PgUsers::new(pool.clone())
        .insert(&User {
            id: Uuid::new_v4(),
            email: "pierrick@example.com".into(),
            display_name: "Pierrick".into(),
            password_hash: PasswordService::hash("hunter2hunter2").unwrap(),
            totp_secret_enc: SecretBox::from_base64(KEY_B64).unwrap().seal(&secret),
        })
        .await
        .unwrap();
    let gate = Arc::new(WealthGate::new(pool.clone(), unlocker, Duration::ZERO));
    let app = api::build_router_with_gate(pool, &config(), gate).unwrap();
    let now = u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap();
    let code = TotpService::current_code(&secret, now).unwrap();
    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "pierrick@example.com", "password": "hunter2hunter2", "totp": code }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers()[header::SET_COOKIE].to_str().unwrap().split(';').next().unwrap().to_owned();
    (app, cookie)
}

async fn call(app: &Router, cookie: &str, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn today() -> String {
    time::OffsetDateTime::now_utc().date().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn full_flow_create_value_and_read_net_worth(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;

    let (status, pea) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "PEA Bourso", "kind": "brokerage_pea", "owner": "personal", "currency": "EUR" }))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, loan) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "Prêt maison", "kind": "loan", "owner": "personal", "currency": "EUR" }))).await;

    let pea_id = pea["id"].as_str().unwrap();
    let loan_id = loan["id"].as_str().unwrap();
    let (status, valuation) = call(&app, &cookie, "POST", &format!("/api/wealth/accounts/{pea_id}/valuations"),
        Some(json!({ "amount": "50000.50", "as_of": today() }))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(valuation["amount"], "50000.50");
    call(&app, &cookie, "POST", &format!("/api/wealth/accounts/{loan_id}/valuations"),
        Some(json!({ "amount": "-20000", "as_of": today() }))).await;

    let (status, net_worth) = call(&app, &cookie, "GET", "/api/wealth/net-worth", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(net_worth["total"], "30000.50");
    assert_eq!(net_worth["by_owner"]["personal"], "30000.50");
    assert_eq!(net_worth["by_kind"]["loan"], "-20000");

    let (status, accounts) = call(&app, &cookie, "GET", "/api/wealth/accounts", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(accounts[0]["name"], "PEA Bourso");
    assert_eq!(accounts[0]["is_stale"], false);
    assert_eq!(accounts[0]["latest_valuation"]["amount"], "50000.50");

    let (status, history) = call(&app, &cookie, "GET", &format!("/api/wealth/accounts/{pea_id}/valuations"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history.as_array().unwrap().len(), 1);

    let (status, points) = call(&app, &cookie, "GET", &format!("/api/wealth/net-worth/history?from=2026-01-01&to={}", today()), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(points.as_array().unwrap().last().unwrap()["total"], "30000.50");

    let (status, _) = call(&app, &cookie, "POST", &format!("/api/wealth/accounts/{pea_id}/archive"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn validation_errors_carry_business_codes(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;
    let (_, account) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "Livret", "kind": "savings", "owner": "personal", "currency": "EUR" }))).await;
    let id = account["id"].as_str().unwrap();

    let cases = [
        (json!({ "name": "X", "kind": "savings", "owner": "personal", "currency": "BTC" }), "/api/wealth/accounts".to_owned(), "unsupported_currency"),
        (json!({ "name": "X", "kind": "stocks", "owner": "personal", "currency": "EUR" }), "/api/wealth/accounts".to_owned(), "invalid_account_kind"),
        (json!({ "amount": "1,5", "as_of": today() }), format!("/api/wealth/accounts/{id}/valuations"), "invalid_amount"),
        (json!({ "amount": "-5", "as_of": today() }), format!("/api/wealth/accounts/{id}/valuations"), "negative_asset_valuation"),
        (json!({ "amount": "5", "as_of": "2999-01-01" }), format!("/api/wealth/accounts/{id}/valuations"), "future_valuation"),
        (json!({ "amount": "5", "as_of": "04/10/2026" }), format!("/api/wealth/accounts/{id}/valuations"), "invalid_date"),
    ];
    for (body, uri, code) in cases {
        let (status, error) = call(&app, &cookie, "POST", &uri, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{code}");
        assert_eq!(error["code"], code);
    }

    let unknown = Uuid::new_v4();
    let (status, error) = call(&app, &cookie, "POST", &format!("/api/wealth/accounts/{unknown}/valuations"),
        Some(json!({ "amount": "5", "as_of": today() }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["code"], "account_not_found");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sealed_vault_answers_503_and_anonymous_answers_401(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(SealedUnlocker)).await;
    let (status, error) = call(&app, &cookie, "GET", "/api/wealth/accounts", None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error["code"], "wealth_vault_sealed");

    let (status, _) = call(&app, "joel_session=nope", "GET", "/api/wealth/net-worth", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_exchange_rate_answers_422(pool: PgPool) {
    sqlx::query("INSERT INTO wealth_exchange_rates (currency, on_date, units_per_eur) VALUES ('USD', '2026-01-02', 1.1)")
        .execute(&pool)
        .await
        .unwrap();
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;
    let (_, account) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "Broker US", "kind": "brokerage_cto", "owner": "personal", "currency": "USD" }))).await;
    let id = account["id"].as_str().unwrap();
    call(&app, &cookie, "POST", &format!("/api/wealth/accounts/{id}/valuations"),
        Some(json!({ "amount": "100", "as_of": today() }))).await;

    let (status, error) = call(&app, &cookie, "GET", "/api/wealth/net-worth", None).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["code"], "exchange_rate_missing");
}
```

Ajouter aux `[dev-dependencies]` de l'api : `serde_json = "1"` (déjà en dépendance normale, rien à faire) et vérifier que `async-trait` y est déjà (oui).

- [ ] **Step 2: Vérifier l'échec**

Run: `cd backend && cargo test -p api --test wealth`
Expected: FAIL à la compilation, `wealth_routes` et les routes n'existent pas encore (ou 404 sur chaque appel).

- [ ] **Step 3: Implémenter `wealth_routes.rs`**

```rust
//! HTTP adapters for the wealth use cases.

use std::collections::BTreeMap;
use std::str::FromStr;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::wealth::{
    AccountId, AccountKind, AccountSummary, Currency, NetWorth, NewAccount, Owner, Valuation,
    WealthError, parse_amount,
};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::auth_routes::CurrentUser;
use crate::state::AppState;

/// Wealth sub-router; every route requires a session.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/wealth/accounts", get(list_accounts).post(create_account))
        .route("/api/wealth/accounts/{id}/archive", post(archive_account))
        .route("/api/wealth/accounts/{id}/valuations", get(account_history).post(record_valuation))
        .route("/api/wealth/net-worth", get(net_worth))
        .route("/api/wealth/net-worth/history", get(net_worth_history))
}

/// Body of `POST /api/wealth/accounts`.
#[derive(Deserialize)]
pub struct CreateAccountRequest {
    name: String,
    kind: String,
    owner: String,
    currency: String,
    notes: Option<String>,
}

/// Body of `POST /api/wealth/accounts/{id}/valuations`.
#[derive(Deserialize)]
pub struct RecordValuationRequest {
    amount: String,
    as_of: String,
}

/// Query of `GET /api/wealth/net-worth`.
#[derive(Deserialize)]
pub struct NetWorthQuery {
    at: Option<String>,
}

/// Query of `GET /api/wealth/net-worth/history`.
#[derive(Deserialize)]
pub struct HistoryQuery {
    from: String,
    to: Option<String>,
}

/// A valuation as exposed to the PWA.
#[derive(Serialize)]
pub struct ValuationResponse {
    id: Uuid,
    as_of: String,
    amount: String,
    currency: String,
    source: &'static str,
    recorded_at: String,
}

/// An account with its latest valuation.
#[derive(Serialize)]
pub struct AccountResponse {
    id: Uuid,
    name: String,
    kind: &'static str,
    owner: &'static str,
    currency: String,
    is_archived: bool,
    notes: Option<String>,
    latest_valuation: Option<ValuationResponse>,
    is_stale: bool,
}

/// Net worth with breakdowns, every amount as a decimal string in euros.
#[derive(Serialize)]
pub struct NetWorthResponse {
    as_of: String,
    total: String,
    by_owner: BTreeMap<&'static str, String>,
    by_kind: BTreeMap<&'static str, String>,
    stale_account_ids: Vec<Uuid>,
}

/// API error: HTTP status plus business code.
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl From<WealthError> for ApiError {
    fn from(error: WealthError) -> Self {
        let status = match &error {
            WealthError::AccountNotFound => StatusCode::NOT_FOUND,
            WealthError::ExchangeRateMissing { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            WealthError::VaultSealed => StatusCode::SERVICE_UNAVAILABLE,
            WealthError::Cipher | WealthError::Storage(_) => {
                tracing::error!(code = error.code(), "wealth internal failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::BAD_REQUEST,
        };
        Self { status, code: error.code() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "code": self.code }))).into_response()
    }
}

const INVALID_DATE: ApiError = ApiError { status: StatusCode::BAD_REQUEST, code: "invalid_date" };

fn parse_date(input: &str) -> Result<Date, ApiError> {
    Date::parse(input, format_description!("[year]-[month]-[day]")).map_err(|_| INVALID_DATE)
}

fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

fn to_valuation_response(valuation: &Valuation) -> ValuationResponse {
    ValuationResponse {
        id: valuation.id.0,
        as_of: valuation.as_of.to_string(),
        amount: valuation.amount.amount.to_string(),
        currency: valuation.amount.currency.to_string(),
        source: valuation.source.code(),
        recorded_at: valuation.recorded_at.format(&Rfc3339).unwrap_or_default(),
    }
}

fn to_account_response(summary: AccountSummary) -> AccountResponse {
    let AccountSummary { account, latest_valuation, is_stale } = summary;
    AccountResponse {
        id: account.id.0,
        name: account.name,
        kind: account.kind.code(),
        owner: account.owner.code(),
        currency: account.currency.to_string(),
        is_archived: account.is_archived,
        notes: account.notes,
        latest_valuation: latest_valuation.as_ref().map(to_valuation_response),
        is_stale,
    }
}

fn to_net_worth_response(net_worth: &NetWorth) -> NetWorthResponse {
    NetWorthResponse {
        as_of: net_worth.as_of.to_string(),
        total: net_worth.total.amount.to_string(),
        by_owner: net_worth.by_owner.iter().map(|(owner, money)| (owner.code(), money.amount.to_string())).collect(),
        by_kind: net_worth.by_kind.iter().map(|(kind, money)| (kind.code(), money.amount.to_string())).collect(),
        stale_account_ids: net_worth.stale_accounts.iter().map(|id| id.0).collect(),
    }
}

async fn list_accounts(_user: CurrentUser, State(state): State<AppState>) -> Result<Json<Vec<AccountResponse>>, ApiError> {
    let summaries = state.wealth.wealth().await?.account_summaries(today()).await?;
    Ok(Json(summaries.into_iter().map(to_account_response).collect()))
}

async fn create_account(
    _user: CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountResponse>), ApiError> {
    let input = NewAccount {
        name: body.name,
        kind: AccountKind::from_code(&body.kind)?,
        owner: Owner::from_code(&body.owner)?,
        currency: Currency::from_str(&body.currency)?,
        notes: body.notes,
    };
    let account = state.wealth.wealth().await?.create_account(input, OffsetDateTime::now_utc()).await?;
    let summary = AccountSummary { account, latest_valuation: None, is_stale: true };
    Ok((StatusCode::CREATED, Json(to_account_response(summary))))
}

async fn archive_account(_user: CurrentUser, State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, ApiError> {
    state.wealth.wealth().await?.archive_account(AccountId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn record_valuation(
    _user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RecordValuationRequest>,
) -> Result<(StatusCode, Json<ValuationResponse>), ApiError> {
    let amount = parse_amount(&body.amount)?;
    let as_of = parse_date(&body.as_of)?;
    let valuation = state
        .wealth
        .wealth()
        .await?
        .record_valuation(AccountId(id), as_of, amount, OffsetDateTime::now_utc())
        .await?;
    Ok((StatusCode::CREATED, Json(to_valuation_response(&valuation))))
}

async fn account_history(
    _user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ValuationResponse>>, ApiError> {
    let valuations = state.wealth.wealth().await?.account_history(AccountId(id)).await?;
    Ok(Json(valuations.iter().map(to_valuation_response).collect()))
}

async fn net_worth(
    _user: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<NetWorthQuery>,
) -> Result<Json<NetWorthResponse>, ApiError> {
    let at = query.at.as_deref().map(parse_date).transpose()?.unwrap_or_else(today);
    let net_worth = state.wealth.wealth().await?.net_worth(at).await?;
    Ok(Json(to_net_worth_response(&net_worth)))
}

async fn net_worth_history(
    _user: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<NetWorthResponse>>, ApiError> {
    let from = parse_date(&query.from)?;
    let to = query.to.as_deref().map(parse_date).transpose()?.unwrap_or_else(today);
    let points = state.wealth.wealth().await?.net_worth_history(from, to).await?;
    Ok(Json(points.iter().map(to_net_worth_response).collect()))
}
```

Une dette assumée : un corps JSON mal formé (champ manquant) est rejeté par l'extracteur `Json` d'axum avec son propre `422` et sans `code`. La PWA n'envoie que des corps complets ; on ne le couvre pas en W0.

- [ ] **Step 4: Tests, clippy, fmt**

Run: `cd backend && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && cargo deny check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/bins/api
git commit -m "feat(api): expose wealth accounts, valuations and net worth routes

Refs #<issue W0.4 routes>"
```

---

### Task 10: PWA, modèles, service, mode discret

**Files:**
- Create: `frontend/src/app/features/wealth/wealth.models.ts`
- Create: `frontend/src/app/features/wealth/amount-input.ts` + `amount-input.spec.ts`
- Create: `frontend/src/app/features/wealth/wealth.service.ts` + `wealth.service.spec.ts`
- Create: `frontend/src/app/features/wealth/discreet-mode.service.ts` + `discreet-mode.service.spec.ts`
- Create: `frontend/src/app/features/wealth/wealth-amount.pipe.ts` + `wealth-amount.pipe.spec.ts`

**Interfaces:**
- Consumes: le contrat HTTP de la Task 9.
- Produces:
  - Types `Owner`, `AccountKind`, `Valuation`, `Account`, `NetWorth`, `NewAccount`, constantes `ACCOUNT_KINDS`, `ACCOUNT_KIND_LABELS`, `OWNER_LABELS`.
  - `normalizeAmount(input: string): string | null`
  - `WealthService` : `vaultSealed` (signal), `listAccounts()`, `createAccount(NewAccount)`, `archiveAccount(id)`, `recordValuation(id, amount, asOf)`, `accountHistory(id)`, `netWorth(at?)`, `netWorthHistory(from, to?)`, tous en `Promise`.
  - `DiscreetModeService` : `isDiscreet` (signal), `toggle()`.
  - Pipe `wealthAmount` : `amount | wealthAmount: currency : discreet`.

- [ ] **Step 1: Modèles**

`wealth.models.ts` :

```ts
export type Owner = 'personal' | 'company';

export type AccountKind =
  | 'brokerage_pea'
  | 'brokerage_cto'
  | 'life_insurance'
  | 'retirement_plan'
  | 'bank_account'
  | 'savings'
  | 'crypto_wallet'
  | 'real_estate'
  | 'company_shares'
  | 'loan';

export interface Valuation {
  id: string;
  as_of: string;
  amount: string;
  currency: string;
  source: string;
  recorded_at: string;
}

export interface Account {
  id: string;
  name: string;
  kind: AccountKind;
  owner: Owner;
  currency: string;
  is_archived: boolean;
  notes: string | null;
  latest_valuation: Valuation | null;
  is_stale: boolean;
}

export interface NetWorth {
  as_of: string;
  total: string;
  by_owner: Partial<Record<Owner, string>>;
  by_kind: Partial<Record<AccountKind, string>>;
  stale_account_ids: string[];
}

export interface NewAccount {
  name: string;
  kind: AccountKind;
  owner: Owner;
  currency: string;
  notes: string | null;
}

export const ACCOUNT_KINDS: readonly AccountKind[] = [
  'brokerage_pea',
  'brokerage_cto',
  'life_insurance',
  'retirement_plan',
  'bank_account',
  'savings',
  'crypto_wallet',
  'real_estate',
  'company_shares',
  'loan',
];

export const ACCOUNT_KIND_LABELS: Record<AccountKind, string> = {
  brokerage_pea: 'PEA',
  brokerage_cto: 'Compte-titres',
  life_insurance: 'Assurance-vie',
  retirement_plan: 'PER',
  bank_account: 'Compte bancaire',
  savings: 'Épargne',
  crypto_wallet: 'Crypto',
  real_estate: 'Immobilier',
  company_shares: 'Parts de société',
  loan: 'Prêt',
};

export const OWNER_LABELS: Record<Owner, string> = {
  personal: 'Perso',
  company: 'SASU',
};
```

- [ ] **Step 2: Écrire les tests qui échouent**

`amount-input.spec.ts` :

```ts
import { normalizeAmount } from './amount-input';

describe('normalizeAmount', () => {
  it('accepts French keyboard input', () => {
    expect(normalizeAmount('1 234,56')).toBe('1234.56');
    expect(normalizeAmount('1 234,5')).toBe('1234.5');
    expect(normalizeAmount(' 250000 ')).toBe('250000');
    expect(normalizeAmount('-180 000')).toBe('-180000');
    expect(normalizeAmount('12.30')).toBe('12.30');
  });

  it('rejects ambiguous or invalid input', () => {
    for (const input of ['', 'abc', '12,345', '1e3', '1,2,3', '1.', ',5', '+3']) {
      expect(normalizeAmount(input)).withContext(input).toBeNull();
    }
  });
});
```

`wealth.service.spec.ts` :

```ts
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { WealthService } from './wealth.service';

describe('WealthService', () => {
  let service: WealthService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideHttpClient(), provideHttpClientTesting()] });
    service = TestBed.inject(WealthService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('posts a valuation with a decimal string amount', fakeAsync(() => {
    service.recordValuation('a1', '1234.56', '2026-10-04');
    const request = http.expectOne('/api/wealth/accounts/a1/valuations');
    expect(request.request.method).toBe('POST');
    expect(request.request.body).toEqual({ amount: '1234.56', as_of: '2026-10-04' });
    request.flush({});
    flushMicrotasks();
  }));

  it('requests net worth history with its range', fakeAsync(() => {
    service.netWorthHistory('2025-10-04', '2026-10-04');
    http.expectOne('/api/wealth/net-worth/history?from=2025-10-04&to=2026-10-04').flush([]);
    flushMicrotasks();
  }));

  it('flags the sealed vault on a 503 wealth_vault_sealed and clears it on success', fakeAsync(() => {
    let rejected = false;
    service.netWorth().catch(() => { rejected = true; });
    http.expectOne('/api/wealth/net-worth').flush({ code: 'wealth_vault_sealed' }, { status: 503, statusText: 'Service Unavailable' });
    flushMicrotasks();
    expect(rejected).toBeTrue();
    expect(service.vaultSealed()).toBeTrue();

    service.listAccounts();
    http.expectOne('/api/wealth/accounts').flush([]);
    flushMicrotasks();
    expect(service.vaultSealed()).toBeFalse();
  }));
});
```

`discreet-mode.service.spec.ts` :

```ts
import { TestBed } from '@angular/core/testing';
import { DiscreetModeService, DISCREET_MODE_STORAGE_KEY } from './discreet-mode.service';

describe('DiscreetModeService', () => {
  beforeEach(() => localStorage.removeItem(DISCREET_MODE_STORAGE_KEY));

  it('toggles and persists the mode', () => {
    const service = TestBed.inject(DiscreetModeService);
    expect(service.isDiscreet()).toBeFalse();
    service.toggle();
    expect(service.isDiscreet()).toBeTrue();
    expect(localStorage.getItem(DISCREET_MODE_STORAGE_KEY)).toBe('true');
  });

  it('restores the persisted mode', () => {
    localStorage.setItem(DISCREET_MODE_STORAGE_KEY, 'true');
    expect(TestBed.inject(DiscreetModeService).isDiscreet()).toBeTrue();
  });
});
```

`wealth-amount.pipe.spec.ts` :

```ts
import { WealthAmountPipe } from './wealth-amount.pipe';

describe('WealthAmountPipe', () => {
  const pipe = new WealthAmountPipe();

  it('formats euros the French way', () => {
    expect(pipe.transform('1234.5', 'EUR', false).replace(/\s/g, ' ')).toBe('1 234,50 €');
  });

  it('masks amounts in discreet mode', () => {
    expect(pipe.transform('1234.5', 'EUR', true)).toBe('••••• €');
  });

  it('shows a dash when there is no amount', () => {
    expect(pipe.transform(null, 'EUR', false)).toBe('-');
  });
});
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless`
Expected: FAIL, modules `./amount-input`, `./wealth.service`, etc. introuvables.

- [ ] **Step 4: Implémenter**

`amount-input.ts` :

```ts
const AMOUNT_PATTERN = /^-?\d+(\.\d{1,2})?$/;

/**
 * Turns what a French keyboard produces ("1 234,56") into the API format ("1234.56").
 * Returns null when the input is not an amount with at most two decimals.
 */
export function normalizeAmount(input: string): string | null {
  const compact = input.replace(/[\s  ]/g, '');
  if ((compact.match(/,/g) ?? []).length > 1) {
    return null;
  }
  const normalized = compact.replace(',', '.');
  return AMOUNT_PATTERN.test(normalized) ? normalized : null;
}
```

`wealth.service.ts` :

```ts
import { inject, Injectable, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom, Observable } from 'rxjs';
import { Account, NetWorth, NewAccount, Valuation } from './wealth.models';

const VAULT_SEALED_CODE = 'wealth_vault_sealed';

@Injectable({ providedIn: 'root' })
export class WealthService {
  private readonly http = inject(HttpClient);

  /** True after the api reported the wealth vault as sealed, until the next success. */
  readonly vaultSealed = signal(false);

  listAccounts(): Promise<Account[]> {
    return this.call(this.http.get<Account[]>('/api/wealth/accounts'));
  }

  createAccount(account: NewAccount): Promise<Account> {
    return this.call(this.http.post<Account>('/api/wealth/accounts', account));
  }

  archiveAccount(id: string): Promise<unknown> {
    return this.call(this.http.post(`/api/wealth/accounts/${id}/archive`, {}));
  }

  recordValuation(accountId: string, amount: string, asOf: string): Promise<Valuation> {
    return this.call(
      this.http.post<Valuation>(`/api/wealth/accounts/${accountId}/valuations`, { amount, as_of: asOf }),
    );
  }

  accountHistory(accountId: string): Promise<Valuation[]> {
    return this.call(this.http.get<Valuation[]>(`/api/wealth/accounts/${accountId}/valuations`));
  }

  netWorth(at?: string): Promise<NetWorth> {
    const query = at ? `?at=${at}` : '';
    return this.call(this.http.get<NetWorth>(`/api/wealth/net-worth${query}`));
  }

  netWorthHistory(from: string, to?: string): Promise<NetWorth[]> {
    const query = to ? `?from=${from}&to=${to}` : `?from=${from}`;
    return this.call(this.http.get<NetWorth[]>(`/api/wealth/net-worth/history${query}`));
  }

  private async call<T>(request: Observable<T>): Promise<T> {
    try {
      const result = await firstValueFrom(request);
      this.vaultSealed.set(false);
      return result;
    } catch (error) {
      if (error instanceof HttpErrorResponse && error.status === 503 && error.error?.code === VAULT_SEALED_CODE) {
        this.vaultSealed.set(true);
      }
      throw error;
    }
  }
}
```

`discreet-mode.service.ts` :

```ts
import { Injectable, signal } from '@angular/core';

export const DISCREET_MODE_STORAGE_KEY = 'joel.wealth.discreet';

function readStoredMode(): boolean {
  try {
    return localStorage.getItem(DISCREET_MODE_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

@Injectable({ providedIn: 'root' })
export class DiscreetModeService {
  /** When true, every wealth amount is masked on screen. */
  readonly isDiscreet = signal(readStoredMode());

  toggle(): void {
    const next = !this.isDiscreet();
    this.isDiscreet.set(next);
    try {
      localStorage.setItem(DISCREET_MODE_STORAGE_KEY, String(next));
    } catch {
      return;
    }
  }
}
```

`wealth-amount.pipe.ts` :

```ts
import { Pipe, PipeTransform } from '@angular/core';

const MASK = '•••••';

@Pipe({ name: 'wealthAmount' })
export class WealthAmountPipe implements PipeTransform {
  /** Formats a decimal string for display only; arithmetic stays on the server. */
  transform(amount: string | null | undefined, currency = 'EUR', isDiscreet = false): string {
    if (amount === null || amount === undefined) {
      return '-';
    }
    if (isDiscreet) {
      const symbol = currency === 'EUR' ? '€' : currency;
      return `${MASK} ${symbol}`;
    }
    return new Intl.NumberFormat('fr-FR', { style: 'currency', currency }).format(Number(amount));
  }
}
```

- [ ] **Step 5: Tests et lint**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless && npx ng lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/app/features/wealth
git commit -m "feat(pwa): add wealth models, service, amount input and discreet mode

Refs #<issue W0.5 modèles>"
```

---

### Task 11: PWA, tableau de bord et tuile cockpit

**Files:**
- Create: `frontend/src/app/features/wealth/net-worth-chart.component.ts` + `.spec.ts`
- Create: `frontend/src/app/features/wealth/wealth-dashboard.component.ts` + `.spec.ts`
- Create: `frontend/src/app/features/wealth/vault-sealed.component.ts`
- Modify: `frontend/src/app/app.routes.ts`
- Modify: `frontend/src/app/core/layout/shell.component.ts`
- Modify: `frontend/src/app/features/cockpit/cockpit.component.ts`

**Interfaces:**
- Consumes: `WealthService`, `DiscreetModeService`, `WealthAmountPipe`, modèles (Task 10).
- Produces: route `/patrimoine` (tableau de bord), composant `<app-net-worth-chart [points]>` avec `points: { date: string; value: number }[]`, `<app-vault-sealed>`.

- [ ] **Step 1: Écrire les tests qui échouent**

`net-worth-chart.component.spec.ts` :

```ts
import { TestBed } from '@angular/core/testing';
import { NetWorthChartComponent } from './net-worth-chart.component';

describe('NetWorthChartComponent', () => {
  it('draws one polyline point per value, scaled into the view box', () => {
    const fixture = TestBed.createComponent(NetWorthChartComponent);
    fixture.componentRef.setInput('points', [
      { date: '2026-08-31', value: 100 },
      { date: '2026-09-30', value: 300 },
      { date: '2026-10-04', value: 200 },
    ]);
    fixture.detectChanges();
    const polyline = fixture.nativeElement.querySelector('polyline') as SVGPolylineElement;
    expect(polyline.getAttribute('points')).toBe('0,100 150,0 300,50');
  });

  it('renders nothing with fewer than two points', () => {
    const fixture = TestBed.createComponent(NetWorthChartComponent);
    fixture.componentRef.setInput('points', [{ date: '2026-10-04', value: 1 }]);
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('polyline')).toBeNull();
  });
});
```

`wealth-dashboard.component.spec.ts` :

```ts
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { WealthDashboardComponent } from './wealth-dashboard.component';
import { WealthService } from './wealth.service';
import { NetWorth } from './wealth.models';
import { signal } from '@angular/core';

function netWorth(total: string, personal: string, company: string, asOf: string): NetWorth {
  return {
    as_of: asOf,
    total,
    by_owner: { personal, company },
    by_kind: { brokerage_pea: personal, bank_account: company },
    stale_account_ids: ['a1'],
  };
}

describe('WealthDashboardComponent', () => {
  const current = netWorth('150000', '110000', '40000', '2026-10-04');
  const previous = netWorth('140000', '100000', '40000', '2026-09-30');
  let wealth: jasmine.SpyObj<WealthService> & { vaultSealed: ReturnType<typeof signal<boolean>> };

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['netWorth', 'netWorthHistory']), {
      vaultSealed: signal(false),
    });
    wealth.netWorth.and.resolveTo(current);
    wealth.netWorthHistory.and.resolveTo([previous, current]);
    TestBed.configureTestingModule({
      providers: [provideRouter([]), { provide: WealthService, useValue: wealth }],
    });
    localStorage.removeItem('joel.wealth.discreet');
  });

  it('shows the total, the delta since last month end and the stale count', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!.replace(/\s/g, ' ');
    expect(text).toContain('150 000,00 €');
    expect(text).toContain('+10 000,00 €');
    expect(text).toContain('+7,1 %');
    expect(text).toContain('1 compte à mettre à jour');
  }));

  it('switches to the company total', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.componentInstance.selectedOwner.set('company');
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).querySelector('.wealth__total')!.textContent!.replace(/\s/g, ' ')).toContain('40 000,00 €');
  }));

  it('masks every amount in discreet mode', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    (fixture.nativeElement as HTMLElement).querySelector<HTMLButtonElement>('.wealth__discreet')!.click();
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).not.toContain('150');
  }));

  it('explains the sealed vault instead of an error', fakeAsync(() => {
    wealth.netWorth.and.rejectWith(new Error('sealed'));
    wealth.netWorthHistory.and.rejectWith(new Error('sealed'));
    wealth.vaultSealed.set(true);
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Le coffre patrimoine est scellé');
  }));
});
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless`
Expected: FAIL, composants introuvables.

- [ ] **Step 3: Implémenter les composants**

`net-worth-chart.component.ts` :

```ts
import { Component, computed, input } from '@angular/core';

export interface ChartPoint {
  date: string;
  value: number;
}

const WIDTH = 300;
const HEIGHT = 100;

@Component({
  selector: 'app-net-worth-chart',
  template: `
    @if (polylinePoints(); as line) {
      <svg [attr.viewBox]="'0 0 ' + width + ' ' + height" preserveAspectRatio="none" class="chart" role="img" aria-label="Évolution du patrimoine net">
        <polyline [attr.points]="line" fill="none" stroke="currentColor" stroke-width="2" vector-effect="non-scaling-stroke" />
      </svg>
    }
  `,
  styles: `.chart { width: 100%; height: 8rem; color: #4f8cc9; }`,
})
export class NetWorthChartComponent {
  readonly points = input.required<ChartPoint[]>();
  protected readonly width = WIDTH;
  protected readonly height = HEIGHT;

  protected readonly polylinePoints = computed(() => {
    const points = this.points();
    if (points.length < 2) {
      return null;
    }
    const values = points.map((point) => point.value);
    const minimum = Math.min(...values);
    const range = Math.max(...values) - minimum || 1;
    const step = WIDTH / (points.length - 1);
    return points
      .map((point, index) => `${Math.round(index * step)},${Math.round(HEIGHT - ((point.value - minimum) / range) * HEIGHT)}`)
      .join(' ');
  });
}
```

`vault-sealed.component.ts` :

```ts
import { Component } from '@angular/core';

@Component({
  selector: 'app-vault-sealed',
  template: `
    <section class="sealed">
      <h3>Le coffre patrimoine est scellé</h3>
      <p>Descelle Egide avec tes parts, puis recharge la page d'ici 30 secondes.</p>
    </section>
  `,
  styles: `.sealed { padding: 1rem; border-radius: 0.5rem; background: #2a2116; color: #f3d9a4; }`,
})
export class VaultSealedComponent {}
```

`wealth-dashboard.component.ts` :

```ts
import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LoggerService } from '../../core/logging/logger.service';
import { DiscreetModeService } from './discreet-mode.service';
import { NetWorthChartComponent } from './net-worth-chart.component';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { WealthService } from './wealth.service';
import { ACCOUNT_KIND_LABELS, AccountKind, NetWorth, Owner, OWNER_LABELS } from './wealth.models';

type OwnerSelection = Owner | 'total';

function isoDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

@Component({
  selector: 'app-wealth-dashboard',
  imports: [RouterLink, NetWorthChartComponent, VaultSealedComponent, WealthAmountPipe],
  template: `
    <header class="wealth__header">
      <h2>Patrimoine</h2>
      <button type="button" class="wealth__discreet" (click)="discreet.toggle()">
        {{ discreet.isDiscreet() ? 'Afficher' : 'Masquer' }}
      </button>
    </header>

    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else if (current(); as netWorth) {
      <nav class="wealth__owners">
        @for (option of ownerOptions; track option.value) {
          <button type="button" [class.active]="selectedOwner() === option.value" (click)="selectedOwner.set(option.value)">
            {{ option.label }}
          </button>
        }
      </nav>

      <p class="wealth__total">{{ selectedTotal() | wealthAmount: 'EUR' : discreet.isDiscreet() }}</p>
      @if (delta(); as change) {
        <p class="wealth__delta">
          {{ change.sign }}{{ change.signedAmount | wealthAmount: 'EUR' : discreet.isDiscreet() }} ({{ change.percent }})
          depuis la fin du mois dernier
        </p>
      }

      <app-net-worth-chart [points]="chartPoints()" />

      @if (selectedOwner() === 'total') {
        <ul class="wealth__kinds">
          @for (entry of kindBreakdown(); track entry.kind) {
            <li><span>{{ entry.label }}</span><span>{{ entry.amount | wealthAmount: 'EUR' : discreet.isDiscreet() }}</span></li>
          }
        </ul>
      }

      <a routerLink="/patrimoine/comptes" class="wealth__accounts">
        @if (netWorth.stale_account_ids.length > 0) {
          {{ netWorth.stale_account_ids.length }} compte{{ netWorth.stale_account_ids.length > 1 ? 's' : '' }} à mettre à jour
        } @else {
          Voir les comptes
        }
      </a>
    } @else {
      <p>Chargement du patrimoine...</p>
    }
  `,
  styles: `
    .wealth__header { display: flex; justify-content: space-between; align-items: center; }
    .wealth__owners { display: flex; gap: 0.5rem; margin: 0.5rem 0; }
    .wealth__owners .active { font-weight: 700; text-decoration: underline; }
    .wealth__total { font-size: 2.2rem; font-weight: 700; margin: 0.5rem 0 0; }
    .wealth__delta { color: #8fa3b1; margin: 0 0 1rem; }
    .wealth__kinds { list-style: none; padding: 0; }
    .wealth__kinds li { display: flex; justify-content: space-between; padding: 0.35rem 0; border-bottom: 1px solid #222a31; }
    .wealth__accounts { display: block; margin-top: 1rem; }
    button { min-height: 2.75rem; padding: 0 0.9rem; }
  `,
})
export class WealthDashboardComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly logger = inject(LoggerService);

  readonly selectedOwner = signal<OwnerSelection>('total');
  protected readonly current = signal<NetWorth | null>(null);
  protected readonly history = signal<NetWorth[]>([]);
  protected readonly ownerOptions: { value: OwnerSelection; label: string }[] = [
    { value: 'total', label: 'Total' },
    { value: 'personal', label: OWNER_LABELS.personal },
    { value: 'company', label: OWNER_LABELS.company },
  ];

  protected readonly selectedTotal = computed(() => {
    const netWorth = this.current();
    return netWorth ? this.totalFor(netWorth) : null;
  });

  protected readonly delta = computed(() => {
    const points = this.history();
    if (points.length < 2) {
      return null;
    }
    const now = Number(this.totalFor(points[points.length - 1]));
    const before = Number(this.totalFor(points[points.length - 2]));
    const difference = now - before;
    const sign = difference >= 0 ? '+' : '';
    const percent = before === 0
      ? '-'
      : `${sign}${new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 1 }).format((difference / Math.abs(before)) * 100)} %`;
    return { signedAmount: `${difference.toFixed(2)}`, percent, sign };
  });

  protected readonly chartPoints = computed(() =>
    this.history().map((point) => ({ date: point.as_of, value: Number(this.totalFor(point)) })),
  );

  protected readonly kindBreakdown = computed(() => {
    const netWorth = this.current();
    if (!netWorth) {
      return [];
    }
    return (Object.entries(netWorth.by_kind) as [AccountKind, string][])
      .map(([kind, amount]) => ({ kind, label: ACCOUNT_KIND_LABELS[kind], amount }))
      .sort((left, right) => Number(right.amount) - Number(left.amount));
  });

  constructor() {
    void this.load();
  }

  private totalFor(netWorth: NetWorth): string {
    const owner = this.selectedOwner();
    return owner === 'total' ? netWorth.total : (netWorth.by_owner[owner] ?? '0');
  }

  private async load(): Promise<void> {
    const today = new Date();
    const oneYearAgo = new Date(today.getFullYear() - 1, today.getMonth(), today.getDate());
    try {
      const [current, history] = await Promise.all([
        this.wealth.netWorth(),
        this.wealth.netWorthHistory(isoDate(oneYearAgo), isoDate(today)),
      ]);
      this.current.set(current);
      this.history.set(history);
    } catch {
      this.logger.error('wealth.dashboard.load_failed', { isVaultSealed: this.wealth.vaultSealed() });
    }
  }
}
```

Le pipe `wealthAmount` affiche `-10 000,00 €` pour un écart négatif et `10 000,00 €` pour un écart positif : le template préfixe donc `change.sign` (`+` ou rien) pour obtenir `+10 000,00 €`. En mode discret, le signe reste visible, le montant non.

- [ ] **Step 4: Routes, navigation, tuile cockpit**

Dans `app.routes.ts`, ajouter dans `children`, après `cockpit` :

```ts
      {
        path: 'patrimoine',
        loadComponent: () =>
          import('./features/wealth/wealth-dashboard.component').then((m) => m.WealthDashboardComponent),
      },
```

Dans `shell.component.ts`, ajouter après le lien Cockpit :

```html
          <a routerLink="/patrimoine" routerLinkActive="active">Patrimoine</a>
```

Dans `cockpit.component.ts`, ajouter `RouterLink` et `WealthAmountPipe` aux `imports`, injecter `WealthService` et `DiscreetModeService`, et ajouter au template, sous le paragraphe de santé :

```html
    <a routerLink="/patrimoine" class="cockpit__tile">
      <h3>Patrimoine</h3>
      @if (wealth.vaultSealed()) {
        <p>Coffre scellé</p>
      } @else if (netWorth(); as value) {
        <p>{{ value.total | wealthAmount: 'EUR' : discreet.isDiscreet() }}</p>
        @if (value.stale_account_ids.length > 0) {
          <p>{{ value.stale_account_ids.length }} à mettre à jour</p>
        }
      }
    </a>
```

et dans la classe :

```ts
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  readonly netWorth = signal<NetWorth | null>(null);
```

avec, dans le constructeur :

```ts
    this.wealth
      .netWorth()
      .then((value) => this.netWorth.set(value))
      .catch(() => this.logger.error('cockpit.wealth.unavailable', {}));
```

- [ ] **Step 5: Tests, lint, build**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless && npx ng lint && npm run build`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/app
git commit -m "feat(pwa): add wealth dashboard, net worth chart and cockpit tile

Refs #<issue W0.5 tableau de bord>"
```

---

### Task 12: PWA, comptes et saisie d'un relevé

**Files:**
- Create: `frontend/src/app/features/wealth/accounts.component.ts` + `.spec.ts`
- Create: `frontend/src/app/features/wealth/valuation-form.component.ts` + `.spec.ts`
- Modify: `frontend/src/app/app.routes.ts`

**Interfaces:**
- Consumes: Task 10 (service, modèles, `normalizeAmount`, pipe, mode discret), `VaultSealedComponent` (Task 11).
- Produces: routes `/patrimoine/comptes` et `/patrimoine/comptes/:id/releve`.

- [ ] **Step 1: Écrire les tests qui échouent**

`valuation-form.component.spec.ts` :

```ts
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { ActivatedRoute, Router, convertToParamMap, provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { ValuationFormComponent } from './valuation-form.component';
import { WealthService } from './wealth.service';

describe('ValuationFormComponent', () => {
  let wealth: jasmine.SpyObj<WealthService>;

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['recordValuation', 'accountHistory']), {
      vaultSealed: signal(false),
    });
    wealth.accountHistory.and.resolveTo([]);
    wealth.recordValuation.and.resolveTo({ id: 'v1', as_of: '2026-10-04', amount: '1234.56', currency: 'EUR', source: 'manual', recorded_at: '' });
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        { provide: WealthService, useValue: wealth },
        { provide: ActivatedRoute, useValue: { snapshot: { paramMap: convertToParamMap({ id: 'a1' }) } } },
      ],
    });
  });

  it('normalizes a French amount, posts it and goes back to the accounts', fakeAsync(() => {
    const router = TestBed.inject(Router);
    spyOn(router, 'navigateByUrl').and.resolveTo(true);
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('1 234,56');
    fixture.componentInstance.asOf.set('2026-10-04');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    expect(wealth.recordValuation).toHaveBeenCalledWith('a1', '1234.56', '2026-10-04');
    expect(router.navigateByUrl).toHaveBeenCalledWith('/patrimoine/comptes');
  }));

  it('refuses an invalid amount without calling the api', fakeAsync(() => {
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('12,345');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    fixture.detectChanges();
    expect(wealth.recordValuation).not.toHaveBeenCalled();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Montant invalide');
  }));

  it('prefills the date with today', () => {
    const fixture = TestBed.createComponent(ValuationFormComponent);
    const now = new Date();
    const local = new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
    expect(fixture.componentInstance.asOf()).toBe(local);
  });
});
```

`accounts.component.spec.ts` :

```ts
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { AccountsComponent } from './accounts.component';
import { WealthService } from './wealth.service';
import { Account } from './wealth.models';

const accounts: Account[] = [
  { id: 'a1', name: 'PEA Bourso', kind: 'brokerage_pea', owner: 'personal', currency: 'EUR', is_archived: false, notes: null, latest_valuation: null, is_stale: true },
  { id: 'a2', name: 'Trésorerie', kind: 'bank_account', owner: 'company', currency: 'EUR', is_archived: false, notes: null,
    latest_valuation: { id: 'v', as_of: '2026-10-01', amount: '40000', currency: 'EUR', source: 'manual', recorded_at: '' }, is_stale: false },
];

describe('AccountsComponent', () => {
  let wealth: jasmine.SpyObj<WealthService>;

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['listAccounts', 'createAccount', 'archiveAccount']), {
      vaultSealed: signal(false),
    });
    wealth.listAccounts.and.resolveTo(accounts);
    wealth.createAccount.and.resolveTo(accounts[0]);
    wealth.archiveAccount.and.resolveTo({});
    TestBed.configureTestingModule({ providers: [provideRouter([]), { provide: WealthService, useValue: wealth }] });
    localStorage.removeItem('joel.wealth.discreet');
  });

  it('groups accounts by owner and flags stale ones', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!;
    expect(text).toContain('Perso');
    expect(text).toContain('SASU');
    expect(text).toContain('À mettre à jour');
  }));

  it('creates an account then reloads the list', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    const component = fixture.componentInstance;
    component.draftName.set('Livret A');
    component.draftKind.set('savings');
    void component.create();
    flushMicrotasks();
    expect(wealth.createAccount).toHaveBeenCalledWith({ name: 'Livret A', kind: 'savings', owner: 'personal', currency: 'EUR', notes: null });
    expect(wealth.listAccounts).toHaveBeenCalledTimes(2);
  }));

  it('asks for a second tap before archiving', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    const component = fixture.componentInstance;
    void component.archive('a1');
    flushMicrotasks();
    expect(wealth.archiveAccount).not.toHaveBeenCalled();
    void component.archive('a1');
    flushMicrotasks();
    expect(wealth.archiveAccount).toHaveBeenCalledWith('a1');
  }));
});
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless`
Expected: FAIL, composants introuvables.

- [ ] **Step 3: Implémenter**

`valuation-form.component.ts` :

```ts
import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';
import { LoggerService } from '../../core/logging/logger.service';
import { normalizeAmount } from './amount-input';
import { DiscreetModeService } from './discreet-mode.service';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { WealthService } from './wealth.service';
import { Valuation } from './wealth.models';

const ERROR_MESSAGES: Record<string, string> = {
  invalid_amount: 'Montant invalide',
  negative_asset_valuation: 'Un actif ne peut pas être négatif',
  positive_loan_valuation: 'Un prêt se saisit en négatif',
  future_valuation: 'La date est dans le futur',
  archived_account: 'Ce compte est archivé',
  account_not_found: 'Compte introuvable',
};

function localToday(): string {
  const now = new Date();
  return new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
}

@Component({
  selector: 'app-valuation-form',
  imports: [FormsModule, RouterLink, VaultSealedComponent, WealthAmountPipe],
  template: `
    <h2>Nouveau relevé</h2>
    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else {
      <form (ngSubmit)="submit()" class="form">
        <label>
          Montant
          <input name="amount" inputmode="decimal" autocomplete="off" [ngModel]="amount()" (ngModelChange)="amount.set($event)" required />
        </label>
        <label>
          Date
          <input name="asOf" type="date" [ngModel]="asOf()" (ngModelChange)="asOf.set($event)" required />
        </label>
        @if (errorMessage(); as message) {
          <p class="form__error" role="alert">{{ message }}</p>
        }
        <button type="submit" [disabled]="isSaving()">Enregistrer</button>
        <a routerLink="/patrimoine/comptes">Annuler</a>
      </form>

      @if (history().length > 0) {
        <h3>Derniers relevés</h3>
        <ul class="history">
          @for (valuation of history(); track valuation.id) {
            <li><span>{{ valuation.as_of }}</span><span>{{ valuation.amount | wealthAmount: valuation.currency : discreet.isDiscreet() }}</span></li>
          }
        </ul>
      }
    }
  `,
  styles: `
    .form { display: flex; flex-direction: column; gap: 0.9rem; max-width: 24rem; }
    .form input { font-size: 1.3rem; min-height: 2.9rem; width: 100%; }
    .form button { min-height: 3rem; font-size: 1.1rem; }
    .form__error { color: #f08a7e; }
    .history { list-style: none; padding: 0; }
    .history li { display: flex; justify-content: space-between; padding: 0.3rem 0; }
  `,
})
export class ValuationFormComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly router = inject(Router);
  private readonly logger = inject(LoggerService);
  private readonly accountId = inject(ActivatedRoute).snapshot.paramMap.get('id') ?? '';

  readonly amount = signal('');
  readonly asOf = signal(localToday());
  protected readonly errorMessage = signal<string | null>(null);
  protected readonly isSaving = signal(false);
  protected readonly history = signal<Valuation[]>([]);

  constructor() {
    this.wealth
      .accountHistory(this.accountId)
      .then((valuations) => this.history.set(valuations.slice(-5).reverse()))
      .catch(() => this.logger.error('wealth.valuation.history_failed', {}));
  }

  async submit(): Promise<void> {
    const amount = normalizeAmount(this.amount());
    if (amount === null) {
      this.errorMessage.set(ERROR_MESSAGES['invalid_amount']);
      return;
    }
    this.isSaving.set(true);
    this.errorMessage.set(null);
    try {
      await this.wealth.recordValuation(this.accountId, amount, this.asOf());
      this.logger.info('wealth.valuation.recorded', {});
      await this.router.navigateByUrl('/patrimoine/comptes');
    } catch (error) {
      const code = error instanceof HttpErrorResponse ? (error.error?.code as string | undefined) : undefined;
      this.errorMessage.set((code && ERROR_MESSAGES[code]) ?? 'Enregistrement impossible, réessaie');
    } finally {
      this.isSaving.set(false);
    }
  }
}
```

`accounts.component.ts` :

```ts
import { Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';
import { LoggerService } from '../../core/logging/logger.service';
import { DiscreetModeService } from './discreet-mode.service';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { WealthService } from './wealth.service';
import { Account, ACCOUNT_KIND_LABELS, ACCOUNT_KINDS, AccountKind, Owner, OWNER_LABELS } from './wealth.models';

const OWNERS: readonly Owner[] = ['personal', 'company'];

@Component({
  selector: 'app-accounts',
  imports: [FormsModule, RouterLink, VaultSealedComponent, WealthAmountPipe],
  template: `
    <h2>Comptes</h2>
    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else {
      @for (group of groups(); track group.owner) {
        <h3>{{ group.label }}</h3>
        <ul class="accounts">
          @for (account of group.accounts; track account.id) {
            <li class="accounts__item">
              <a [routerLink]="['/patrimoine/comptes', account.id, 'releve']">
                <strong>{{ account.name }}</strong>
                <span>{{ kindLabels[account.kind] }}</span>
                <span>{{ account.latest_valuation?.amount ?? null | wealthAmount: account.currency : discreet.isDiscreet() }}</span>
                @if (account.is_stale) {
                  <span class="accounts__stale">À mettre à jour</span>
                }
              </a>
              <button type="button" (click)="archive(account.id)">
                {{ pendingArchiveId() === account.id ? 'Confirmer' : 'Archiver' }}
              </button>
            </li>
          }
        </ul>
      }

      <h3>Nouveau compte</h3>
      <form (ngSubmit)="create()" class="form">
        <input name="name" placeholder="Nom" [ngModel]="draftName()" (ngModelChange)="draftName.set($event)" required />
        <select name="kind" [ngModel]="draftKind()" (ngModelChange)="draftKind.set($event)">
          @for (kind of kinds; track kind) {
            <option [value]="kind">{{ kindLabels[kind] }}</option>
          }
        </select>
        <select name="owner" [ngModel]="draftOwner()" (ngModelChange)="draftOwner.set($event)">
          @for (owner of owners; track owner) {
            <option [value]="owner">{{ ownerLabels[owner] }}</option>
          }
        </select>
        <input name="currency" maxlength="3" [ngModel]="draftCurrency()" (ngModelChange)="draftCurrency.set($event.toUpperCase())" />
        <input name="notes" placeholder="Notes" [ngModel]="draftNotes()" (ngModelChange)="draftNotes.set($event)" />
        @if (errorMessage(); as message) {
          <p role="alert" class="form__error">{{ message }}</p>
        }
        <button type="submit">Créer</button>
      </form>
    }
  `,
  styles: `
    .accounts { list-style: none; padding: 0; }
    .accounts__item { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; padding: 0.5rem 0; border-bottom: 1px solid #222a31; }
    .accounts__item a { display: grid; gap: 0.15rem; color: inherit; text-decoration: none; }
    .accounts__stale { color: #f3c26b; font-size: 0.85rem; }
    .form { display: flex; flex-direction: column; gap: 0.6rem; max-width: 24rem; }
    .form input, .form select, button { min-height: 2.75rem; }
    .form__error { color: #f08a7e; }
  `,
})
export class AccountsComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly logger = inject(LoggerService);

  protected readonly kinds = ACCOUNT_KINDS;
  protected readonly kindLabels = ACCOUNT_KIND_LABELS;
  protected readonly owners = OWNERS;
  protected readonly ownerLabels = OWNER_LABELS;

  protected readonly accounts = signal<Account[]>([]);
  protected readonly pendingArchiveId = signal<string | null>(null);
  protected readonly errorMessage = signal<string | null>(null);
  readonly draftName = signal('');
  readonly draftKind = signal<AccountKind>('bank_account');
  readonly draftOwner = signal<Owner>('personal');
  readonly draftCurrency = signal('EUR');
  readonly draftNotes = signal('');

  protected readonly groups = computed(() =>
    OWNERS.map((owner) => ({
      owner,
      label: OWNER_LABELS[owner],
      accounts: this.accounts().filter((account) => account.owner === owner && !account.is_archived),
    })).filter((group) => group.accounts.length > 0),
  );

  constructor() {
    void this.reload();
  }

  async create(): Promise<void> {
    this.errorMessage.set(null);
    try {
      await this.wealth.createAccount({
        name: this.draftName(),
        kind: this.draftKind(),
        owner: this.draftOwner(),
        currency: this.draftCurrency(),
        notes: this.draftNotes().trim() === '' ? null : this.draftNotes(),
      });
      this.draftName.set('');
      this.draftNotes.set('');
      await this.reload();
    } catch (error) {
      const code = error instanceof HttpErrorResponse ? error.error?.code : undefined;
      this.errorMessage.set(
        code === 'unsupported_currency'
          ? 'Devise non suivie par la BCE : crée le compte en EUR'
          : code === 'empty_account_name'
            ? 'Donne un nom au compte'
            : 'Création impossible, réessaie',
      );
    }
  }

  async archive(id: string): Promise<void> {
    if (this.pendingArchiveId() !== id) {
      this.pendingArchiveId.set(id);
      return;
    }
    this.pendingArchiveId.set(null);
    try {
      await this.wealth.archiveAccount(id);
      await this.reload();
    } catch {
      this.logger.error('wealth.account.archive_failed', {});
    }
  }

  private async reload(): Promise<void> {
    try {
      this.accounts.set(await this.wealth.listAccounts());
    } catch {
      this.logger.error('wealth.accounts.load_failed', { isVaultSealed: this.wealth.vaultSealed() });
    }
  }
}
```

Dans `app.routes.ts`, ajouter après la route `patrimoine` :

```ts
      {
        path: 'patrimoine/comptes',
        loadComponent: () => import('./features/wealth/accounts.component').then((m) => m.AccountsComponent),
      },
      {
        path: 'patrimoine/comptes/:id/releve',
        loadComponent: () =>
          import('./features/wealth/valuation-form.component').then((m) => m.ValuationFormComponent),
      },
```

- [ ] **Step 4: Tests, lint, build**

Run: `cd frontend && npm test -- --watch=false --browsers=ChromeHeadless && npx ng lint && npm run build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/app
git commit -m "feat(pwa): add wealth accounts list and thumb-friendly valuation form

Refs #<issue W0.5 comptes>"
```

---

### Task 13: Déploiement Egide et runbook

**Files:**
- Modify: `deploy/compose.prod.yml`
- Modify: `.env.example`
- Modify: `.gitignore`
- Create: `deploy/wealth-vault-runbook.md`

**Interfaces:**
- Consumes: variables `EGIDE_URL`, `EGIDE_TOKEN_FILE` (Task 8), sous-commande `api rewrap-wealth-key`.

- [ ] **Step 1: Vérifier l'image Egide**

Run: `docker manifest inspect nubster/egide:0.1.0 >/dev/null && echo ok`
Expected: `ok`. Si l'image épinglée n'existe pas, s'arrêter et demander à Pierrick quelle version publier ou utiliser (ne jamais mettre `latest` en production).

- [ ] **Step 2: Service Egide et secret**

Dans `deploy/compose.prod.yml`, ajouter le service :

```yaml
  egide:
    image: nubster/egide:0.1.0
    restart: unless-stopped
    environment:
      EGIDE_ENV: production
      EGIDE_DATA_DIR: /var/lib/egide
      EGIDE_BIND_ADDRESS: 0.0.0.0:8200
      RUST_LOG: info
    volumes:
      - egide_data:/var/lib/egide
    networks: [data]
```

Dans le service `api`, ajouter :

```yaml
    environment:
      EGIDE_URL: http://egide:8200
      EGIDE_TOKEN_FILE: /run/secrets/egide_token
    secrets:
      - egide_token
```

(fusionner les deux clés `EGIDE_*` dans le bloc `environment` existant), et ajouter `egide` à la liste `depends_on` de `api` sans condition de santé (l'api démarre même si Egide est scellé).

En bas du fichier :

```yaml
secrets:
  egide_token:
    file: ./secrets/egide_token
```

et `egide_data:` dans `volumes`.

Dans `.gitignore`, ajouter :

```
deploy/secrets/
```

Dans `.env.example`, ajouter en commentaire :

```
# Wealth vault: the Egide service token lives in deploy/secrets/egide_token (never in this file).
```

- [ ] **Step 3: Runbook**

`deploy/wealth-vault-runbook.md` :

```markdown
# Coffre patrimoine (Egide) : exploitation

Egide garde la clé qui enveloppe la clé de données du module patrimoine. Tant qu'Egide est
scellé, Joel fonctionne mais les routes `/api/wealth/*` répondent `503 wealth_vault_sealed`.

## Mise en place (une seule fois)

1. Démarrer Egide : `docker compose -f deploy/compose.prod.yml up -d egide`
2. Initialiser : `docker compose -f deploy/compose.prod.yml exec egide egide operator init`.
   Ranger les parts Shamir et le token root hors du VPS (gestionnaire de mots de passe, une
   part par emplacement). Le token root n'est affiché qu'une fois.
3. Desceller : `docker compose -f deploy/compose.prod.yml exec egide egide operator unseal`,
   autant de fois que le seuil de parts l'exige.
4. Créer la clé Transit, avec le token root :
   `docker compose -f deploy/compose.prod.yml exec -e EGIDE_TOKEN=<root> egide sh -c 'curl -s -X POST http://localhost:8200/v1/transit/keys -H "Authorization: Bearer $EGIDE_TOKEN" -H "Content-Type: application/json" -d "{\"name\":\"joel-wealth\"}"'`
5. Créer le token de service de Joel, avec le token root :
   `... -d '{"service_name": "joel-api"}'` sur `POST /v1/auth/service-tokens`.
6. Écrire le token `egst_...` dans `deploy/secrets/egide_token` (droits `600`, jamais commité).
7. Redémarrer l'api : `docker compose -f deploy/compose.prod.yml up -d api`. Au premier
   démarrage, Joel génère la clé de données et enregistre sa version enveloppée.

## Après chaque redémarrage du VPS

Egide redémarre scellé. Lancer l'étape 3. Les routes patrimoine reviennent seules dans les
30 secondes, sans redémarrer Joel.

## Rotation de la clé

1. Faire tourner la clé maîtresse avec le token root :
   `POST /v1/transit/keys/joel-wealth/rotate`.
2. Ré-envelopper la clé de données :
   `docker compose -f deploy/compose.prod.yml run --rm api rewrap-wealth-key`.
   Les données chiffrées ne bougent pas.

## Limites connues (Egide 0.1.0)

- Pas de moteur de politiques : le token `joel-api` peut utiliser toutes les clés Transit et
  tous les secrets. Cette instance ne sert que Joel. À restreindre dès que les politiques
  existeront.
- Egide parle HTTP en clair sur le réseau Docker interne `data`, non routé.
- Un attaquant root sur le VPS pendant que Joel tourne peut lire la clé en mémoire. Le
  chiffrement protège les dumps, les sauvegardes et les disques, pas un hôte compromis.
```

- [ ] **Step 4: Valider la composition**

Run: `docker compose -f deploy/compose.prod.yml config --quiet && echo ok`
Expected: `ok` (créer un `deploy/secrets/egide_token` factice en local si la commande l'exige, puis le supprimer).

- [ ] **Step 5: Commit**

```bash
git add deploy/compose.prod.yml deploy/wealth-vault-runbook.md .env.example .gitignore
git commit -m "chore(deploy): add Egide service and wealth vault runbook

Refs #<issue W0.6 déploiement>"
```

- [ ] **Step 6: Vérification de bout en bout (avec Pierrick)**

Après déploiement sur le VPS et descellement :
1. Sur l'iPhone, en 4G, NetBird actif, ouvrir la PWA et se connecter par Face ID.
2. Créer un compte « Livret A » (Épargne, Perso, EUR) et un compte « Prêt maison » (Prêt).
3. Saisir « 22 950 » sur le Livret A et « -180 000 » sur le prêt, au pouce.
4. Le tableau de bord affiche `-157 050,00 €`, la tuile du cockpit aussi, le mode discret masque tout.
5. `docker compose exec postgres psql -U joel -c "SELECT name, amount FROM wealth_accounts JOIN wealth_valuations ON wealth_valuations.account_id = wealth_accounts.id"` ne montre que des octets illisibles.
6. Redémarrer Egide (`docker compose restart egide`) : le tableau de bord affiche « Le coffre patrimoine est scellé ». Desceller : il revient dans les 30 secondes.

Consigner le résultat dans la PR. Puis ouvrir la PR `feature/joel-wealth-foundation` vers `main` avec le lien vers la milestone, et attendre le GO de Pierrick pour le merge.
