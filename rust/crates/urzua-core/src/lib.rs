//! Record schema, parsing, and validation rules.
//!
//! Pure: no I/O, no filesystem, no process. Callers read files and hand in
//! content. Validation rules are data where possible, not code — a design
//! independently converged on more than once, after re-diverging when it
//! wasn't.
//!
//! Implements: SPEC-0001, RFC-0001

#![forbid(unsafe_code)]

pub mod config;
pub mod field_state;
pub mod header;
pub mod record;
pub mod report;
pub mod rules;
pub mod waiver;

/// The decision axis: where a record sits in its own lifecycle.
///
/// Values are profile-specific in RFC-0001; this is the ADR set. Parsing must
/// tolerate free-text annotation after the value — a rule written against an
/// assumed canonical format can be invalidated by real corpus variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

/// The realization axis: was this actually built?
///
/// Orthogonal to [`Status`] by design (RFC-0001 §3). Computed from grep-able
/// back-pointers, never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Embodiment {
    NotStarted,
    Specified,
    Implemented,
    Verified,
    /// Alarm state, reachable from any other state.
    DriftDetected,
    /// Terminal, for decisions that will never be built (e.g. "we're not doing X").
    Inactive,
}

/// Presence state of a field.
///
/// Blank, placeholder, and pending are three distinct states, not one. Collapsing
/// them is a common, repeated bug class in hand-written record linters —
/// each rediscovered and re-fixed independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldState {
    /// Field absent or empty.
    Blank,
    /// Field holds unedited template text (e.g. `name`, `TBD`, `—`).
    Placeholder,
    /// Field explicitly marks work not yet done.
    Pending,
    /// Field holds a real value.
    Present,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unrecognized status: {0}")]
    UnknownStatus(String),
    #[error("unrecognized embodiment: {0}")]
    UnknownEmbodiment(String),
}
