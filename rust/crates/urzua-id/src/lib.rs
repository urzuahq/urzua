//! Stable record identifiers.
//!
//! Authors never choose an identifier (ADR-0003). A record gets a stable,
//! time-ordered, collision-free ID at creation with no coordination; the
//! human-facing sequential display number is assigned at merge time. All
//! cross-references resolve against the stable ID, which is what makes
//! renumbering safe.
//!
//! The encoding is ULID (ADR-0021) — time-ordered, lexicographically
//! sortable, collision-free without coordination. Kept behind [`StableId`]
//! so it stays swappable if that decision is ever revisited.
//!
//! Implements: ADR-0003, ADR-0021

#![forbid(unsafe_code)]

/// Immutable identity of a record. Never reused, never reassigned.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    /// A fresh, collision-free identifier. No coordination needed — this is
    /// the whole point (ADR-0003): twenty concurrent branches each call this
    /// and none of them collide.
    pub fn generate() -> Self {
        StableId(ulid::Ulid::generate().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_generated_ids_never_collide() {
        let a = StableId::generate();
        let b = StableId::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn later_generated_ids_sort_lexicographically_after_earlier_ones() {
        let a = StableId::generate();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = StableId::generate();
        assert!(a.as_str() < b.as_str());
    }
}
