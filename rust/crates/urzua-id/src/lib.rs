//! Stable record identifiers and display numbers.
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

/// Human-facing sequential number, e.g. `42` rendered as `0042`.
///
/// Presentation only. Assigned at merge time, may change, and nothing may
/// depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayNumber(pub u32);

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

impl std::fmt::Display for DisplayNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed stable id: {0}")]
    Malformed(String),
    /// Two records claiming one identity. Unrecoverable corruption in a system
    /// whose value is being a trustworthy record — never auto-resolve.
    #[error("duplicate stable id {0} claimed by {1} and {2}")]
    Duplicate(String, String, String),
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

    #[test]
    fn display_number_pads_to_four_digits() {
        assert_eq!(DisplayNumber(7).to_string(), "0007");
        assert_eq!(DisplayNumber(1234).to_string(), "1234");
    }
}
