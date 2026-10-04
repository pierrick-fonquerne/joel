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
        std::str::from_utf8(&self.0).unwrap_or("???")
    }
}

impl FromStr for Currency {
    type Err = WealthError;

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
            assert_eq!(
                code.parse::<Currency>(),
                Err(WealthError::InvalidCurrency),
                "{code}"
            );
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
        for input in [
            "1,5", "1 234.56", "1e3", "12.345", "abc", "", ".", "1.", "+3",
        ] {
            assert_eq!(
                parse_amount(input),
                Err(WealthError::InvalidAmount),
                "{input}"
            );
        }
    }
}
