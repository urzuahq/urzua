//! AgDR import and export.
//!
//! Urzua owns its record format; AgDR compatibility is a boundary adapter, not
//! the native model (ADR-0002).
//!
//! Export that would drop fields **must warn**. Silently discarding data during
//! a format transform is precisely the failure class this project exists to
//! prevent, and shipping it here would be self-refuting.
//!
//! Implements: ADR-0002

#![forbid(unsafe_code)]

/// Fields adopted from AgDR into Urzua's core schema, because they are a
/// genuine improvement on RFC-0001 for agent-authored records: which agent,
/// running which model, triggered by what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProvenance {
    pub agent: Option<String>,
    pub model: Option<String>,
    pub trigger: Option<String>,
}

/// Fields with no AgDR equivalent, preserved on round-trip rather than dropped.
#[derive(Debug, Clone, Default)]
pub struct LossReport {
    pub dropped_fields: Vec<String>,
}

impl LossReport {
    pub fn is_lossless(&self) -> bool {
        self.dropped_fields.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed AgDR record: {0}")]
    Malformed(String),
    #[error("missing required AgDR field: {0}")]
    MissingField(String),
}
