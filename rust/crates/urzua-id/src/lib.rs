//! Stable record identifiers and display numbers.
//!
//! Authors never choose an identifier (ADR-0003). A record gets a stable,
//! time-ordered, collision-free ID at creation with no coordination; the
//! human-facing sequential display number is assigned at merge time. All
//! cross-references resolve against the stable ID, which is what makes
//! renumbering safe.
//!
//! The exact encoding (ULID vs. UUIDv7 vs. a short typable prefix) is
//! deliberately still open — ADR-0003 committed to the *separation*, not the
//! encoding. Keep that choice behind [`StableId`] so it stays swappable.
//!
//! Implements: ADR-0003

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
