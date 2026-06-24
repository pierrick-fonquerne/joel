//! Pure text helpers for the e5 embedding family.

/// The query/passage prefix prepended before embedding, as required by e5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E5Prefix {
    /// Prefix for search queries.
    Query,
    /// Prefix for stored documents.
    Passage,
}

impl E5Prefix {
    /// Prepends the e5 prefix to `text`.
    #[must_use]
    pub fn apply(self, text: &str) -> String {
        match self {
            E5Prefix::Query => format!("query: {text}"),
            E5Prefix::Passage => format!("passage: {text}"),
        }
    }
}

/// Divides `vector` by its L2 norm; returns it unchanged if the norm is ~0.
#[must_use]
pub fn normalize_l2(vector: &[f32]) -> Vec<f32> {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return vector.to_vec();
    }
    vector.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn query_prefix_is_prepended() {
        assert_eq!(E5Prefix::Query.apply("chat"), "query: chat");
    }

    #[test]
    fn passage_prefix_is_prepended() {
        assert_eq!(E5Prefix::Passage.apply("chat"), "passage: chat");
    }

    #[test]
    fn normalized_vector_has_unit_norm() {
        let out = normalize_l2(&[3.0, 4.0]);
        let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm was {norm}");
    }

    #[test]
    fn zero_vector_is_returned_unchanged() {
        let out = normalize_l2(&[0.0, 0.0, 0.0]);
        assert_eq!(out, vec![0.0, 0.0, 0.0]);
    }
}
