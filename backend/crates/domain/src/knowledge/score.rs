//! Relevance score newtype. Never compared by equality.

/// A relevance score; larger means closer. Never compared with `==`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score(f32);

impl Score {
    /// Wraps a raw score.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// The raw score value.
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_its_value() {
        assert!((Score::new(0.5).value() - 0.5).abs() < f32::EPSILON);
    }
}
