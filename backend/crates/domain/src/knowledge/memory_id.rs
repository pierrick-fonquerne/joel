//! Stable identifier for a memory, provided by the caller.

use uuid::Uuid;

/// Caller-provided stable identifier for a memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryId(Uuid);

impl MemoryId {
    /// Generates a fresh time-ordered identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing UUID.
    #[must_use]
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// The wrapped UUID.
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_uuid() {
        let id = MemoryId::new();
        assert_eq!(MemoryId::from_uuid(id.as_uuid()), id);
    }
}
