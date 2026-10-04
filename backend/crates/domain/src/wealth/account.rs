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
        match self {
            Self::Personal => "personal",
            Self::Company => "company",
        }
    }

    /// Parses a storage or API code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidOwner`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        match code {
            "personal" => Ok(Self::Personal),
            "company" => Ok(Self::Company),
            _ => Err(WealthError::InvalidOwner),
        }
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
        Self::Brokerage {
            envelope: BrokerageEnvelope::Pea,
        },
        Self::Brokerage {
            envelope: BrokerageEnvelope::Cto,
        },
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
        match self {
            Self::Brokerage {
                envelope: BrokerageEnvelope::Pea,
            } => "brokerage_pea",
            Self::Brokerage {
                envelope: BrokerageEnvelope::Cto,
            } => "brokerage_cto",
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

    /// Parses a storage or API code.
    ///
    /// # Errors
    /// Returns [`WealthError::InvalidAccountKind`] for an unknown code.
    pub fn from_code(code: &str) -> Result<Self, WealthError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.code() == code)
            .ok_or(WealthError::InvalidAccountKind)
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
            AccountKind::Brokerage {
                envelope: BrokerageEnvelope::Pea
            }
            .code(),
            "brokerage_pea"
        );
        assert_eq!(
            AccountKind::from_code("stocks"),
            Err(WealthError::InvalidAccountKind)
        );
    }

    #[test]
    fn owner_codes_roundtrip() {
        assert_eq!(Owner::from_code("personal").unwrap(), Owner::Personal);
        assert_eq!(
            Owner::from_code(Owner::Company.code()).unwrap(),
            Owner::Company
        );
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
            AccountKind::Brokerage {
                envelope: BrokerageEnvelope::Pea,
            },
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
