//! The logical corpus a memory belongs to.

/// A logical collection of memories backing one storage-backend collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corpus {
    /// Press review documents.
    Press,
    /// Agent memories.
    AgentMemory,
}

impl Corpus {
    /// The storage backend collection name backing this corpus.
    #[must_use]
    pub fn collection(self) -> &'static str {
        match self {
            Self::Press => "press",
            Self::AgentMemory => "memory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_corpus_to_its_collection() {
        assert_eq!(Corpus::Press.collection(), "press");
        assert_eq!(Corpus::AgentMemory.collection(), "memory");
    }
}
