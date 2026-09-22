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

/// One rule's execution record: not just "did it run" but what it was handed
/// and what it judged. A rule that ran over zero derived input and a rule that
/// ran cleanly over the whole corpus both report `rulesExecuted` unless the
/// population is tracked separately.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuleExecution {
    pub rule: String,
    /// What this rule was handed and what it judged, in a stated unit.
    ///
    /// This replaces `records_examined` and `RuleScope`, which were one number
    /// and a hand-set label for what it counted. The number held three
    /// different denominators across the rule set -- 1041 of them over a
    /// 315-record corpus for `field.quality` -- because the unit was named
    /// beside it rather than carried with it (`BUG-40`), and the label went
    /// wrong twice on its own (`BUG-81`, `BUG-83`). A unit that travels with
    /// its count cannot be read as a different unit's.
    ///
    /// `None` only when the rule was not enabled, where `status` says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<Population>,
    /// ADR-7: a rule a repository did not turn on is reported as deliberately
    /// skipped, never omitted -- "off" and "ran clean" must stay
    /// distinguishable in the report.
    pub status: RuleStatus,
    /// The records this rule reached a verdict about, for the report's
    /// `records_read_by_any_rule`. Not serialized: it is an input to one
    /// top-level number, and per-rule it would be a third denominator beside
    /// the two the population already carries.
    ///
    /// Empty for `RecordType`, `Path` and `Claim` units -- a declaration, a
    /// filename and an external assertion are not records read.
    #[serde(skip)]
    pub examined_records: Vec<PathBuf>,
}

/// The number of distinct records some rule reached a verdict about.
///
/// A union, never a sum: `field.quality` contributing 1041 slots contributes
/// at most one record each, and populations in different units must never be
/// added. It is the one number that makes "you get what you declare"
/// (`ADR-53`) safe to say -- a run whose declared rules could read no record
/// exits 0 and says so here, rather than the gate inferring a verdict from a
/// count that meant something different for every rule.
pub fn records_read_by_any_rule(executed: &[RuleExecution]) -> usize {
    executed
        .iter()
        .filter(|e| e.status == RuleStatus::Ran)
        .flat_map(|e| e.examined_records.iter())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

/// `records_read_by_any_rule`, restricted to a reported-on set.
///
/// Rules run over the whole corpus because a reference resolves against records
/// outside the scope (`BUG-67`). Counting those here would put this number on a
/// different denominator from `files_examined` in the same report.
pub fn records_read_by_any_rule_within(
    executed: &[RuleExecution],
    in_scope: &std::collections::HashSet<&std::path::Path>,
) -> usize {
    executed
        .iter()
        .filter(|e| e.status == RuleStatus::Ran)
        .flat_map(|e| e.examined_records.iter())
        .filter(|p| in_scope.contains(p.as_path()))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
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
    /// One other rule's execution. The unit of a rule that judges the run
    /// rather than the corpus.
    Rule,
}

/// What a rule decided about one candidate it was handed.
///
/// There is no "not in the population" answer, deliberately: the candidate list
/// *is* the population, built before the rule runs. A rule that could reduce its
/// own denominator is the hand-maintained count this type replaces.
///
/// The variants below `Examined` all count as unexamined on the wire, where they
/// collapse into `eligible - examined`. They are separate here because the
/// reason a rule judged nothing decides what an adopter should do about it, and
/// those actions are opposite: `Absent` means wait, `Unreadable` means fix the
/// records, `OutOfScope` means fix the configuration. Collapsing them in the
/// code rather than at the wire is what left `MILE-106` with no way to tell a
/// fresh corpus from a misconfigured one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The rule reached a verdict about this candidate.
    Examined,
    /// The declared slot or section is not there. Legitimate and common on a
    /// corpus that has not adopted the convention yet.
    Absent,
    /// Present, and the rule could not read it -- a header that did not parse.
    /// The corpus is malformed, not incomplete.
    Unreadable,
    /// The record does not match the shape the configuration declared, so no
    /// amount of editing this record makes the rule apply: a filename yielding
    /// no identifier under the declared `prefix` is the case `BUG-61` found,
    /// where `init` wrote a prefix matching nothing and five rules silently
    /// examined zero records.
    ///
    /// The one state whose subject is the configuration rather than the corpus,
    /// and the reason a configurable engine needs this distinction at all --
    /// a linter with compiled-in rules cannot reach it.
    OutOfScope,
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
    let mut examined = 0;
    let mut out_of_scope = 0;
    for candidate in &candidates {
        match body(candidate) {
            Outcome::Examined => examined += 1,
            Outcome::OutOfScope => out_of_scope += 1,
            Outcome::Absent | Outcome::Unreadable => {}
        }
    }
    Population::detailed(unit, eligible, examined, out_of_scope)
}

/// `census`, also reporting *which* records were judged.
///
/// `record_of` names the record a candidate belongs to, so a `Field` rule with
/// several slots per record still contributes that record once. The returned
/// list feeds `records_read_by_any_rule` and nothing else; it is derived from
/// the same pass that produces the population, so the two cannot disagree.
pub fn census_records<T>(
    unit: PopulationUnit,
    candidates: Vec<T>,
    record_of: impl Fn(&T) -> PathBuf,
    mut body: impl FnMut(&T) -> Outcome,
) -> (Population, Vec<PathBuf>) {
    let eligible = candidates.len();
    let mut examined_records = Vec::new();
    let mut examined = 0;
    let mut out_of_scope = 0;
    for candidate in &candidates {
        match body(candidate) {
            Outcome::Examined => {
                examined += 1;
                examined_records.push(record_of(candidate));
            }
            Outcome::OutOfScope => out_of_scope += 1,
            Outcome::Absent | Outcome::Unreadable => {}
        }
    }
    examined_records.sort();
    examined_records.dedup();
    (
        Population::detailed(unit, eligible, examined, out_of_scope),
        examined_records,
    )
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
///
/// ```compile_fail
/// use urzua_core::report::{Population, PopulationUnit};
/// let _ = Population { unit: PopulationUnit::Record, eligible: 2, examined: 5 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Population {
    unit: PopulationUnit,
    eligible: usize,
    examined: usize,
    /// Candidates the *configuration* does not reach, as distinct from ones the
    /// corpus has not written yet. Always serialized, including as `0`: a
    /// diagnostic that appears only when non-zero cannot be told from one
    /// nothing computed.
    out_of_scope: usize,
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

    /// How many candidates the configuration could not reach. Non-zero means
    /// the configuration does not match this corpus, which is a different
    /// problem from the corpus not having the content yet -- and the only one
    /// of the two an adopter fixes by editing config (`BUG-61`).
    pub fn out_of_scope(&self) -> usize {
        self.out_of_scope
    }

    pub fn of(unit: PopulationUnit, eligible: usize, examined: usize) -> Self {
        Population::detailed(unit, eligible, examined, 0)
    }

    /// `of`, naming how many candidates were beyond the configuration's reach.
    pub fn detailed(
        unit: PopulationUnit,
        eligible: usize,
        examined: usize,
        out_of_scope: usize,
    ) -> Self {
        // A real `assert!`, not `debug_assert!` (`BUG-113`): this runs once
        // per rule per invocation, not a hot-path cost worth trading away,
        // and a release build is exactly where this invariant silently
        // laundering into `eligible.max(examined)` below would be worst --
        // the shipped binary is what an adopter's CI actually runs.
        assert!(
            examined <= eligible,
            "{unit:?}: examined {examined} exceeds eligible {eligible}"
        );
        // Raise `eligible`, never lower `examined`. Clamping the other way
        // resolves a contradiction into `eligible == examined` -- "the rule
        // examined its whole population", the most reassuring wrong answer
        // available.
        Population {
            unit,
            eligible: eligible.max(examined),
            examined,
            out_of_scope,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    Ran,
    NotEnabled,
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
    /// How many records some rule actually reached a verdict about
    /// ([`records_read_by_any_rule`]). `files_examined` says what was read off
    /// disk; this says what was judged, and the gap between them is a run whose
    /// declared rules could not address the corpus (`BUG-61`). The verdict
    /// deliberately does not read it -- what gets checked is the adopter's call
    /// (`ADR-53`) -- but no reader takes `0` here for a clean corpus.
    pub records_read_by_any_rule: usize,
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
    pub population: Population,
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
    pub population: Population,
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

    /// `Population` implements no `Add`, so the compiler refuses the obvious
    /// mistake outright.
    ///
    /// ```compile_fail
    /// use urzua_core::report::{Population, PopulationUnit};
    /// let a = Population::of(PopulationUnit::Field, 1041, 1041);
    /// let b = Population::of(PopulationUnit::Record, 315, 315);
    /// let _ = a + b;
    /// ```
    #[test]
    fn a_total_across_units_is_a_union_of_records_never_a_sum_of_populations() {
        // 1041 field slots and 315 records are the same 315 records counted
        // twice in different units. Adding the columns is how `records_examined`
        // came to report a number larger than the corpus (`BUG-90`), and the
        // accessors still make it typable even though `Population` has no `Add`.
        // `records_read_by_any_rule` is the replacement, and it must stay a
        // union: one record judged by five rules contributes one, not five.
        let path = |p: &str| std::path::PathBuf::from(p);
        let executed = vec![
            RuleExecution {
                rule: "field.quality".to_string(),
                population: Some(Population::of(PopulationUnit::Field, 4, 4)),
                status: RuleStatus::Ran,
                examined_records: vec![path("a.md"), path("b.md")],
            },
            RuleExecution {
                rule: "identity.collision".to_string(),
                population: Some(Population::of(PopulationUnit::Record, 2, 2)),
                status: RuleStatus::Ran,
                examined_records: vec![path("a.md"), path("b.md")],
            },
            RuleExecution {
                rule: "type.no-declared-spec".to_string(),
                population: Some(Population::of(PopulationUnit::RecordType, 1, 1)),
                status: RuleStatus::Ran,
                examined_records: vec![],
            },
        ];

        let naive_sum: usize = executed
            .iter()
            .filter_map(|e| e.population.as_ref())
            .map(|p| p.examined())
            .sum();
        assert_eq!(naive_sum, 7, "the mistake this guards against");
        assert_eq!(
            records_read_by_any_rule(&executed),
            2,
            "two records, however many rules and units judged them"
        );
    }

    #[test]
    fn a_rule_that_did_not_run_contributes_no_records() {
        let executed = vec![RuleExecution {
            rule: "field.quality".to_string(),
            population: None,
            status: RuleStatus::NotEnabled,
            examined_records: vec![std::path::PathBuf::from("a.md")],
        }];
        assert_eq!(records_read_by_any_rule(&executed), 0);
    }

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
            records_read_by_any_rule: 0,
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
