//! Client of the European Central Bank daily reference rates feed.

use std::str::FromStr;

use domain::wealth::Currency;
use rust_decimal::Decimal;
use time::Date;
use time::macros::format_description;

/// Reference rates of the last 90 business days.
pub const ECB_LAST_90_DAYS_URL: &str =
    "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml";

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
    let malformed = |what: &str| EcbError::Malformed(what.to_owned());
    let document =
        roxmltree::Document::parse(xml).map_err(|error| EcbError::Malformed(error.to_string()))?;
    let date_format = format_description!("[year]-[month]-[day]");
    let mut rates = Vec::new();
    for day in document
        .descendants()
        .filter(|node| node.has_tag_name("Cube") && node.has_attribute("time"))
    {
        let on = Date::parse(day.attribute("time").unwrap_or_default(), date_format)
            .map_err(|_| malformed("time"))?;
        for quote in day.children().filter(|node| node.has_tag_name("Cube")) {
            let currency = quote
                .attribute("currency")
                .ok_or_else(|| malformed("currency"))?;
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

/// Downloads and parses the last 90 days of reference rates.
///
/// # Errors
/// [`EcbError::Transport`] or [`EcbError::Malformed`].
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
            on: date!(2026 - 10 - 02),
            units_per_eur: Decimal::new(10_850, 4),
        }));
        assert!(
            rates
                .iter()
                .any(|r| r.currency.as_str() == "CHF" && r.on == date!(2026 - 10 - 02))
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(parse_feed("not xml"), Err(EcbError::Malformed(_))));
        let bad_rate = SAMPLE.replace("1.0850", "abc");
        assert!(matches!(parse_feed(&bad_rate), Err(EcbError::Malformed(_))));
    }
}
