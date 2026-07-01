//! Pure mappings between domain recall types and `EidosDB` query types.

use std::collections::BTreeMap;

use domain::knowledge::{KnowledgeError, MemoryId, RecallFilter};
use eidosdb_query::{FieldValue, Filter, Payload, Value};
use time::OffsetDateTime;

/// Reserved payload key for the memory identifier.
pub const KEY_MEMORY_ID: &str = "memory_id";
/// Reserved payload key for the epoch-seconds timestamp.
pub const KEY_OCCURRED_AT: &str = "occurred_at";
/// Reserved payload key for the theme tag.
pub const KEY_THEME: &str = "theme";
/// Reserved payload key for the serialized business metadata.
pub const KEY_PAYLOAD_JSON: &str = "payload_json";

/// Translates a [`RecallFilter`] into an `EidosDB` [`Filter`], or `None` if the filter is empty.
#[must_use]
pub fn recall_filter_to_filter(filter: &RecallFilter) -> Option<Filter> {
    let mut clauses = Vec::new();
    if let Some(theme) = &filter.theme {
        clauses.push(Filter::Eq(KEY_THEME.into(), Value::Text(theme.clone())));
    }
    if let Some(since) = filter.since {
        clauses.push(Filter::Gte(
            KEY_OCCURRED_AT.into(),
            Value::Integer(since.unix_timestamp()),
        ));
    }
    if let Some(until) = filter.until {
        clauses.push(Filter::Lte(
            KEY_OCCURRED_AT.into(),
            Value::Integer(until.unix_timestamp()),
        ));
    }
    match clauses.len() {
        0 => None,
        1 => clauses.into_iter().next(),
        _ => Some(Filter::And(clauses)),
    }
}

/// Builds the reserved-key [`Payload`] for an upsert.
///
/// The payload holds four reserved keys: `memory_id`, `occurred_at`,
/// optionally `theme`, and `payload_json` (the business metadata serialized
/// as a JSON string).
///
/// # Errors
///
/// Returns [`KnowledgeError::Config`] if JSON serialization fails or if the
/// payload construction is rejected by [`Payload::new`].
pub fn build_payload(
    memory_id: MemoryId,
    occurred_at: OffsetDateTime,
    theme: Option<&str>,
    business: &serde_json::Value,
) -> Result<Payload, KnowledgeError> {
    let mut fields = BTreeMap::new();
    fields.insert(
        KEY_MEMORY_ID.to_owned(),
        FieldValue::Scalar(Value::Text(memory_id.to_string())),
    );
    fields.insert(
        KEY_OCCURRED_AT.to_owned(),
        FieldValue::Scalar(Value::Integer(occurred_at.unix_timestamp())),
    );
    if let Some(t) = theme {
        fields.insert(
            KEY_THEME.to_owned(),
            FieldValue::Scalar(Value::Text(t.to_owned())),
        );
    }
    let json =
        serde_json::to_string(business).map_err(|e| KnowledgeError::Config(e.to_string()))?;
    fields.insert(
        KEY_PAYLOAD_JSON.to_owned(),
        FieldValue::Scalar(Value::Text(json)),
    );
    Payload::new(fields).map_err(|e| KnowledgeError::Config(format!("{e:?}")))
}

/// Recovers the business metadata stored under [`KEY_PAYLOAD_JSON`].
///
/// Returns [`serde_json::Value::Null`] if the key is absent or the stored
/// value cannot be deserialized as JSON.
#[must_use]
pub fn payload_to_json(payload: &Payload) -> serde_json::Value {
    payload
        .get(KEY_PAYLOAD_JSON)
        .and_then(|field| match field {
            FieldValue::Scalar(Value::Text(text)) => serde_json::from_str(text).ok(),
            _ => None,
        })
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn empty_filter_maps_to_none() {
        assert!(recall_filter_to_filter(&RecallFilter::default()).is_none());
    }

    #[test]
    fn theme_and_window_combine_with_and() {
        let filter = RecallFilter {
            theme: Some("press".into()),
            since: Some(OffsetDateTime::from_unix_timestamp(100).unwrap()),
            until: Some(OffsetDateTime::from_unix_timestamp(200).unwrap()),
        };
        match recall_filter_to_filter(&filter) {
            Some(Filter::And(clauses)) => assert_eq!(clauses.len(), 3),
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn business_payload_round_trips_through_json() {
        let id = MemoryId::new();
        let business = serde_json::json!({ "title": "Hello", "url": "https://x" });
        let payload = build_payload(
            id,
            OffsetDateTime::from_unix_timestamp(10).unwrap(),
            Some("press"),
            &business,
        )
        .expect("payload");
        assert_eq!(payload_to_json(&payload), business);
    }
}
