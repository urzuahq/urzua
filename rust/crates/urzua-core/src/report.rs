//! The output contract (SPEC-0001/0002): `status`, `filesExamined`,
//! `rulesExecuted` (with per-rule input population -- `rulesExecuted` alone
//! doesn't prove a rule examined anything), `scope`, `blocking`, `findings`.
//! Every command's report implements `Report` (ADR-0046) -- `urzua-cli`'s
//! `emit()` is the actual print, since printing is I/O and this crate stays
//! pure (this module only defines the data shapes and `exit_code()`).

use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: FindingSeverity,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
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

/// `Info`/`Warning` only, deliberately -- no `Error` variant. A `Notice`
/// exists specifically so a non-fatal observation can never contradict the
/// exit code (ADR-0046); an `Error`-shaped `Notice` would be a way to write
/// that contradiction, so the type makes it unrepresentable instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoticeSeverity {
    Info,
    Warning,
}

/// A non-fatal, non-file-scoped observation about an otherwise-successful
/// run (e.g. an explicit `--by` diverging from the identity actually used).
/// Never affects a report's `exit_code()` -- that's `NoticeSeverity`'s job to
/// guarantee structurally. `subject` is populated from a `const &str` at
/// each call site, matching `Finding.rule`'s existing convention.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notice {
    pub severity: NoticeSeverity,
    pub subject: String,
    pub message: String,
}

/// Every command's report implements this (ADR-0046): `urzua-cli`'s
/// `emit()` is the shared print step, and an exit code that `notices()` can
/// never move.
pub trait Report: serde::Serialize {
    fn notices(&self) -> &[Notice];
    fn exit_code(&self) -> ExitCode;
}

impl Report for CheckReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(CheckReport::exit_code(self) as u8)
    }
}

/// The shared shape for every fatal "could not run at all" path -- collapses
/// what were three different patterns (`report_could_not_run`,
/// `report_fix_could_not_run`, and several bare `eprintln!`s with nothing on
/// stdout) onto one. `error` carries the actual fatal message directly, not
/// as a `Notice` -- a `Notice` exists to never affect the exit code, and this
/// message is the reason for it, so stretching `Notice` to cover it would be
/// the wrong fit (the same reasoning that keeps the panic hook outside
/// `Notice` too).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CouldNotRun {
    pub status: ReportStatus,
    pub error: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl CouldNotRun {
    pub fn from(error: impl Into<String>) -> Self {
        CouldNotRun {
            status: ReportStatus::NotRun,
            error: error.into(),
            notices: Vec::new(),
        }
    }
}

impl Report for CouldNotRun {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(2)
    }
}

/// `urzua explain`'s report (SPEC-0013): a read-only query, so its status
/// axis is `Ok | NotRun` only -- no `blocking`/`FindingsPresent` concept
/// exists for a query that doesn't validate anything.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainReport {
    pub status: ReportStatus,
    pub path: String,
    pub governing_records: Vec<crate::graph::GoverningRecord>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for ExplainReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// `urzua graph`'s report (SPEC-0013) -- same `Ok`-only status axis as
/// `ExplainReport`, for the same reason.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphReport {
    pub status: ReportStatus,
    pub edges: Vec<crate::graph::GraphEdge>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for GraphReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// `urzua new`'s report (SPEC-0012).
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewReport {
    pub status: ReportStatus,
    pub path: PathBuf,
    pub display_number: u32,
    pub stable_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for NewReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// `urzua fix`'s report (SPEC-0008): `status` already carries the
/// not-run/partial-failure/ok/repairs-available distinction `run_fix`
/// computed today -- `exit_code()` just reads it back.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FixReport {
    pub status: FixStatus,
    pub records_examined: usize,
    pub repairs: Vec<crate::fix::Repair>,
    pub failed: Vec<FixFailure>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixStatus {
    NotRun,
    PartialFailure,
    Ok,
    RepairsAvailable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FixFailure {
    pub record: PathBuf,
    pub error: String,
}

impl Report for FixReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        match self.status {
            FixStatus::NotRun => ExitCode::from(2),
            FixStatus::PartialFailure => ExitCode::from(1),
            FixStatus::Ok | FixStatus::RepairsAvailable => ExitCode::from(0),
        }
    }
}

/// `urzua migrate schema --report`'s report (SPEC-0014) -- read-only preview,
/// same `Ok`-only status axis as `ExplainReport`/`GraphReport`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrateSchemaReport {
    pub status: ReportStatus,
    pub field: String,
    pub records_examined: usize,
    pub would_fail: Vec<crate::migrate::SchemaReportEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for MigrateSchemaReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning_notice() -> Notice {
        Notice {
            severity: NoticeSeverity::Warning,
            subject: "test".to_string(),
            message: "a non-fatal observation".to_string(),
        }
    }

    /// The invariant this whole design exists to protect: a `Notice` -- of
    /// any severity the type allows -- never moves the exit code either way.
    /// A successful report stays successful with notices attached...
    #[test]
    fn notices_never_move_a_successful_exit_code() {
        let report = CheckReport {
            status: ReportStatus::Ok,
            files_examined: 1,
            rules_executed: vec![],
            scope: ScopeInfo {
                source: "test".to_string(),
                record_types: vec![],
            },
            blocking: false,
            findings: vec![],
            notices: vec![warning_notice()],
        };
        assert_eq!(Report::exit_code(&report), ExitCode::from(0));
    }

    /// ...and a not-run report stays not-run, whether or not notices are
    /// attached -- `notices` is orthogonal to `status`/`blocking` in both
    /// directions.
    #[test]
    fn notices_never_move_a_not_run_exit_code() {
        let without_notices = CouldNotRun::from("could not run");
        let with_notices = CouldNotRun {
            notices: vec![warning_notice()],
            ..CouldNotRun::from("could not run")
        };
        assert_eq!(
            Report::exit_code(&without_notices),
            Report::exit_code(&with_notices)
        );
        assert_eq!(Report::exit_code(&with_notices), ExitCode::from(2));
    }
}
