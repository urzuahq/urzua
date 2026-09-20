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
    /// What this rule was handed and what it judged, in a stated unit
    /// (`BUG-40`). `records_examined` holds three different denominators
    /// across the rule set -- 1023 of them over a 309-record corpus for
    /// `field.quality` -- because the unit was never named. `None` while a
    /// rule is not yet converted; `records_examined` is authoritative until
    /// every rule carries a population.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<Population>,
    /// What `records_examined` counted. A rule that reads the configuration
    /// counts declarations, not records, and the two must not be added
    /// together: a config-scoped count satisfying "some rule examined
    /// something" let `check` report `Ok` having read no record at all
    /// (BUG-81).
    #[serde(default)]
    pub scope: RuleScope,
    /// ADR-7: a rule a repository did not turn on is reported as deliberately
    /// skipped, never omitted -- "off" and "ran clean" must stay
    /// distinguishable in the report.
    pub status: RuleStatus,
}

/// What a rule's population is counted in. A number without its unit is how
/// `records_examined` came to mean records for one rule, configuration
/// declarations for another and `(record, field)` pairs for a third, all under
/// one name (`BUG-40`, `BUG-81`, `BUG-83`, `BUG-90`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopulationUnit {
    /// One entry in `config.record_types`.
    RecordType,
    /// One tracked record-shaped path, judged without opening the file.
    Path,
    /// One loaded record; the verdict is about the file as a whole.
    Record,
    /// One `(record, field-name)` pair where the name is **declared by
    /// config** -- never "a field that happens to be present". The slot
    /// exists because the configuration says so, which is what lets an
    /// absent or unreadable one still be counted.
    Field,
    /// One closure assertion drawn from a declared `claim_paths` tree.
    Claim,
}

/// What a rule decided about one candidate it was handed.
///
/// There is no "not in the population" answer, deliberately: the candidate list
/// *is* the population, built before the rule runs. A rule that could reduce
/// its own denominator is the hand-maintained count this type replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The rule reached a verdict about this candidate.
    Examined,
    /// The candidate was in the population and the rule did not judge it --
    /// the declared slot is absent, or the header it needed did not parse.
    /// `eligible > examined` is this, and it is the signal `MILE-106` reads.
    NotExamined,
}

/// Run `body` over a population and report what it judged.
///
/// `eligible` is the candidate list's length, so it cannot drift from the
/// rule's own skips -- the list and the loop are one thing. Hand-writing a
/// parallel filter beside the loop is how `records_examined` came to hold
/// three different denominators (`BUG-40`).
pub fn census<T>(
    unit: PopulationUnit,
    candidates: Vec<T>,
    mut body: impl FnMut(&T) -> Outcome,
) -> Population {
    let eligible = candidates.len();
    let examined = candidates
        .iter()
        .filter(|c| body(c) == Outcome::Examined)
        .count();
    Population::of(unit, eligible, examined)
}

/// A rule's input population: what it was eligible to examine, and what it
/// judged. `eligible > examined` is the signal, not an accounting slip -- it
/// means the rule had something in front of it that it did not reach.
///
/// No public constructor. `of` is the only way in and it upholds
/// `examined <= eligible`, so the invariant cannot be written wrong rather
/// than being checked for afterwards.
///
/// The fields are **private**. Public fields would let any caller write
/// `Population { eligible: 2, examined: 5, .. }` and attach it to a
/// `RuleExecution`, which is the same bypass as having no constructor at all --
/// the invariant is only structural if there is no second way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Population {
    unit: PopulationUnit,
    eligible: usize,
    examined: usize,
}

impl Population {
    pub fn unit(&self) -> PopulationUnit {
        self.unit
    }

    pub fn eligible(&self) -> usize {
        self.eligible
    }

    pub fn examined(&self) -> usize {
        self.examined
    }

    pub fn of(unit: PopulationUnit, eligible: usize, examined: usize) -> Self {
        debug_assert!(
            examined <= eligible,
            "{unit:?}: examined {examined} exceeds eligible {eligible}"
        );
        Population {
            unit,
            eligible,
            examined: examined.min(eligible),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    Ran,
    NotEnabled,
}

/// Whether a rule's examined count refers to corpus records or to
/// configuration entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleScope {
    #[default]
    Records,
    Config,
    /// Tracked path names, examined without opening a file. Neither a corpus
    /// record nor a configuration entry: counting it as a record let a rule
    /// that never parsed anything certify the corpus (BUG-83).
    Paths,
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

/// How the examined set was selected. A declared contract value (SPEC-0002),
/// not a rendering of whichever internal type happened to produce it -- a
/// `Debug` rendering makes an ordinary rename a silent contract change
/// (BUG-0025). Lives here rather than in `urzua-io` because the value is part
/// of the output contract, and `urzua-core` stays free of I/O (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeSource {
    TrackedSweep,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeInfo {
    pub source: ScopeSource,
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
    /// Detect mode found repairs, none applied yet.
    RepairsAvailable,
    /// Apply mode wrote one or more repairs, no failures -- distinct from
    /// `RepairsAvailable` so a completed apply doesn't read as still-pending.
    Applied,
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
            FixStatus::Ok | FixStatus::RepairsAvailable | FixStatus::Applied => ExitCode::from(0),
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

    #[test]
    #[should_panic(expected = "exceeds eligible")]
    fn a_population_cannot_examine_more_than_it_was_eligible_for() {
        // Upheld by construction rather than checked after the fact: BUG-90
        // was a count exceeding the size of the corpus that nothing in the
        // report could contradict. Every test run and every dev build trips
        // the assertion; release clamps rather than emitting a number that
        // contradicts itself.
        let _ = Population::of(PopulationUnit::Record, 2, 5);
    }

    /// Compile-time, not runtime: with private fields there is no expression
    /// that writes a contradictory `Population`, so this property is checked by
    /// the module boundary rather than by a test that could be deleted.
    ///
    /// ```compile_fail
    /// use urzua_core::report::{Population, PopulationUnit};
    /// let _ = Population { unit: PopulationUnit::Record, eligible: 2, examined: 5 };
    /// ```
    #[test]
    fn a_population_carries_the_unit_its_numbers_are_in() {
        let types = Population::of(PopulationUnit::RecordType, 6, 6);
        let records = Population::of(PopulationUnit::Record, 309, 309);
        assert_ne!(
            types.unit(),
            records.unit(),
            "six declarations and 309 records must not read as the same quantity"
        );
    }

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
                source: ScopeSource::TrackedSweep,
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
