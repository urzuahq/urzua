//! The output contract (SPEC-0002): `status`, `filesExamined`, `rulesExecuted`
//! (with per-rule input population -- `rulesExecuted` alone doesn't prove a
//! rule examined anything), `scope`, `blocking`, `findings`.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub message: String,
    /// The covering waiver record's id, if any (ADR-0011). A waived finding
    /// is still listed here -- never omitted -- only excluded from
    /// `blocking`/`status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waived: Option<String>,
}

/// One rule's execution record: not just "did it run" but "how many records
/// did it actually examine." A rule that ran over zero derived input and a
/// rule that ran cleanly over the whole corpus both report `rulesExecuted`
/// unless this is tracked separately.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuleExecution {
    pub rule: String,
    pub records_examined: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportStatus {
    Ok,
    FindingsPresent,
    /// A zero selection, or discovery/parsing could not run at all. Never
    /// reported as `Ok` -- a check that examined nothing has told you
    /// nothing about the corpus.
    NotRun,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeInfo {
    pub source: String,
    pub record_types: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckReport {
    pub status: ReportStatus,
    pub files_examined: usize,
    pub rules_executed: Vec<RuleExecution>,
    pub scope: ScopeInfo,
    pub blocking: bool,
    pub findings: Vec<Finding>,
}

impl CheckReport {
    /// SPEC-0001's exit codes: 0 clean, 1 blocking findings, 2 could not run.
    /// Collapsing these is how "0 errors" comes to mean "never executed."
    pub fn exit_code(&self) -> i32 {
        match self.status {
            ReportStatus::NotRun => 2,
            _ if self.blocking => 1,
            _ => 0,
        }
    }
}
