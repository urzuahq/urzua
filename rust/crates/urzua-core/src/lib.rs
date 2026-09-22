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
pub mod fix;
pub mod graph;
pub mod header;
pub mod migrate;
pub mod new_record;
/// `SPEC-4`'s acceptance suite (`MILE-101`). Test-only: it exercises the
/// engine, it is not part of its surface.
#[cfg(test)]
mod property;
pub mod record;
pub mod report;
pub mod rules;
pub mod values;
pub mod waiver;

/// Presence state of a field.
///
/// Blank, placeholder, and pending are three distinct states, not one. Collapsing
/// them is a common, repeated bug class in hand-written record linters —
/// each rediscovered and re-fixed independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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
