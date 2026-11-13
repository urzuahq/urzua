//! The output contract (SPEC-0001/0002): `status`, `filesExamined`,
//! `rulesExecuted` (with per-rule input population -- `rulesExecuted` alone
//! doesn't prove a rule examined anything), `scope`, `blocking`, `findings`.
//! Every command's report implements `Report` (ADR-0046) -- `urzua-cli`'s
//! `emit()` is the actual print, since printing is I/O and this crate stays
//! pure (this module only defines the data shapes and `exit_code()`).

use std::path::PathBuf;
use std::process::ExitCode;

// Implements: ADR-87 (a finding at `info` is reported and counted, never blocking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Error,
    Warning,
    /// A finding at `level: info` (`ADR-87`): reported and counted, never
    /// trips `fail_on` and never sets `blocking`.
    Info,
}

impl FindingSeverity {
    /// This severity, raised to `floor` when it is lower.
    pub fn at_least(self, floor: FindingSeverity) -> FindingSeverity {
        match (floor, self) {
            (FindingSeverity::Error, _) => FindingSeverity::Error,
            (FindingSeverity::Warning, FindingSeverity::Info) => FindingSeverity::Warning,
            _ => self,
        }
    }

    /// Strictness, for choosing the higher of two: `Info` below `Warning`
    /// below `Error`.
    pub fn rank(self) -> u8 {
        match self {
            FindingSeverity::Info => 0,
            FindingSeverity::Warning => 1,
            FindingSeverity::Error => 2,
        }
    }
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
    /// The severity this finding's own field was declared at (`MILE-140`),
    /// for a rule that takes `field_levels`. `None` means the rule declared
    /// nothing for it and the gate applies the rule-level severity. Never
    /// serialized: the finding's `severity` already carries the outcome.
    #[serde(skip)]
    pub declared_severity: Option<FindingSeverity>,
    /// What to do about it (`ADR-73`): omitted when the finding's variant is
    /// declared exempt in the rule table, or when no action applies. Never
    /// `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<SuggestedAction>,
    /// What the rule knows that the declared action needs: the variant it is
    /// reporting and the target and value it computed. The rule table turns
    /// this into `suggested_action` (`rules::resolve_action`). Never
    /// serialized.
    #[serde(skip)]
    pub action_source: ActionSource,
}

impl Finding {
    /// A finding with no line, no covering waiver, no declared field
    /// severity and no resolved action: what a rule knows when it reports.
    /// `action_source` is required so that no finding is built without
    /// naming its variant (`ADR-73`); the rule table resolves
    /// `suggested_action` from it later.
    pub fn new(
        rule: impl Into<String>,
        severity: FindingSeverity,
        file: impl Into<PathBuf>,
        action_source: ActionSource,
        message: impl Into<String>,
    ) -> Self {
        Finding {
            rule: rule.into(),
            severity,
            file: file.into(),
            line: None,
            message: message.into(),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source,
        }
    }

    /// The finding sits on this line of its file.
    pub fn at(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// The finding sits on this line when one is known.
    pub fn at_opt(mut self, line: Option<usize>) -> Self {
        self.line = line;
        self
    }

    /// The severity the finding's own field was declared at (`MILE-140`).
    pub fn declared(mut self, severity: FindingSeverity) -> Self {
        self.declared_severity = Some(severity);
        self
    }

    /// `declared`, when the rule declared one.
    pub fn declared_opt(mut self, severity: Option<FindingSeverity>) -> Self {
        self.declared_severity = severity;
        self
    }
}

/// The closed vocabulary of `suggested_action.kind` (`ADR-73`). Closed per
/// release and extensible across releases: a consumer treats a kind it does
/// not know as no machine-actionable guidance and reads `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    EditField,
    EditSection,
    EditConfig,
    AddRecord,
    RunCommand,
}

impl ActionKind {
    pub const ALL: [ActionKind; 5] = [
        ActionKind::EditField,
        ActionKind::EditSection,
        ActionKind::EditConfig,
        ActionKind::AddRecord,
        ActionKind::RunCommand,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::EditField => "edit-field",
            ActionKind::EditSection => "edit-section",
            ActionKind::EditConfig => "edit-config",
            ActionKind::AddRecord => "add-record",
            ActionKind::RunCommand => "run-command",
        }
    }

    /// The `op` values the kind accepts. Empty means the kind carries none.
    pub fn ops(self) -> &'static [ActionOp] {
        match self {
            ActionKind::EditField => &[
                ActionOp::Set,
                ActionOp::Remove,
                ActionOp::Append,
                ActionOp::Rename,
            ],
            ActionKind::EditSection => &[ActionOp::Add, ActionOp::Set, ActionOp::Remove],
            ActionKind::EditConfig => &[ActionOp::Set, ActionOp::Remove, ActionOp::Append],
            ActionKind::AddRecord | ActionKind::RunCommand => &[],
        }
    }
}

/// What an edit does (`ADR-73`): one word each, so the same finding object
/// never means both "fill in" and "remove".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionOp {
    Set,
    Remove,
    Append,
    Rename,
    Add,
}

/// A computed `value` (`ADR-73`): a string, a list of strings or a boolean,
/// and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum ActionValue {
    Text(String),
    List(Vec<String>),
    Flag(bool),
}

/// The flat `target` key vocabulary (`ADR-73`). A key that was not computed
/// is omitted, never `null`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ActionTarget {
    /// Repository-root-relative. May differ from the finding's own file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    /// The declared spelling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Only where it differs from the finding's own line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Path segments, never a dotted string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<Vec<String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
    /// Concrete, `argv[0]` is `urzua`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argv: Option<Vec<String>>,
}

/// `Finding.suggested_action` (`ADR-73`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SuggestedAction {
    pub kind: ActionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<ActionOp>,
    pub target: ActionTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<ActionValue>,
}

/// What a rule hands the rule table about one finding: the variant it is
/// (the unit the table declares an action or an exemption for) and the target
/// and value it computed. The kind and `op` are not here: they are declared
/// once per variant in `RULE_TABLE`, not at each construction site.
///
/// The default has an empty variant, which no rule declares: it is for a
/// finding that no rule produced (a test double).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionSource {
    pub variant: &'static str,
    pub target: ActionTarget,
    pub value: Option<ActionValue>,
}

impl ActionSource {
    pub fn new(variant: &'static str) -> Self {
        ActionSource {
            variant,
            ..Default::default()
        }
    }

    /// The edit belongs on a file other than the finding's own.
    pub fn file(mut self, file: impl Into<PathBuf>) -> Self {
        self.target.file = Some(file.into());
        self
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.target.field = Some(field.into());
        self
    }

    pub fn section(mut self, section: impl Into<String>) -> Self {
        self.target.section = Some(section.into());
        self
    }

    pub fn line(mut self, line: usize) -> Self {
        self.target.line = Some(line);
        self
    }

    pub fn key_path<S: Into<String>>(mut self, path: impl IntoIterator<Item = S>) -> Self {
        self.target.key_path = Some(path.into_iter().map(Into::into).collect());
        self
    }

    pub fn record_type(mut self, record_type: impl Into<String>) -> Self {
        self.target.record_type = Some(record_type.into());
        self
    }

    pub fn argv<S: Into<String>>(mut self, argv: impl IntoIterator<Item = S>) -> Self {
        self.target.argv = Some(argv.into_iter().map(Into::into).collect());
        self
    }

    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.value = Some(ActionValue::Text(value.into()));
        self
    }

    pub fn list<S: Into<String>>(mut self, value: impl IntoIterator<Item = S>) -> Self {
        self.value = Some(ActionValue::List(
            value.into_iter().map(Into::into).collect(),
        ));
        self
    }
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

impl RuleExecution {
    /// A rule that ran, over this population, reaching a verdict about
    /// these records.
    pub fn ran(
        rule: impl Into<String>,
        population: Population,
        examined_records: Vec<PathBuf>,
    ) -> Self {
        RuleExecution {
            rule: rule.into(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        }
    }

    /// A rule that did not apply: it was handed nothing it could judge.
    pub fn not_applicable(rule: impl Into<String>) -> Self {
        Self::unrun(rule, RuleStatus::NotApplicable)
    }

    /// A rule the repository did not turn on (`ADR-7`).
    pub fn not_enabled(rule: impl Into<String>) -> Self {
        Self::unrun(rule, RuleStatus::NotEnabled)
    }

    fn unrun(rule: impl Into<String>, status: RuleStatus) -> Self {
        RuleExecution {
            rule: rule.into(),
            population: None,
            status,
            examined_records: Vec::new(),
        }
    }
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
    /// One decision ID cited in a record's prose.
    Citation,
    /// One `(record, section)` pair where the section is **declared by
    /// config** (`sections.required`), so a section the record lacks is still
    /// counted.
    Section,
    /// One other rule's execution. The unit of a rule that judges the run
    /// rather than the corpus.
    Rule,
    /// One tracked file under a declared published path, read as text. The
    /// file is not a record: the rule judges what it cites.
    File,
    /// One path in the changed set of a `--base` run.
    ChangedPath,
    /// One fetched pull-request head ref under `refs/urzua/pr` (`ADR-93`): zero
    /// means nothing was fetched, which a clean run must not be mistaken for.
    PullRef,
    /// One entry of a declared header field's list (`MILE-186`, `ADR-75`
    /// clause 3): an id a record names without citing it as a reference. The
    /// entry exists because the record declares it, so an uncited or malformed
    /// one is still counted.
    FieldEntry,
    /// The configuration file itself, for a rule that judges how it was read.
    ConfigFile,
    /// One entry of a committed log a rule reads (`override.unreviewed`).
    Entry,
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
    /// The rule declares `scope: changed` (`ADR-90`) and the candidate's record
    /// is not in the diff's changed set (or there is no diff): the slot is
    /// there, the rule chose not to judge it. Counted apart from `OutOfScope`
    /// so `config.scope-matches-nothing`, which reads a configuration that
    /// reaches nothing, does not warn about a rule that was told to wait for a
    /// diff.
    DeferredByScope,
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
    let mut unreadable = 0;
    let mut deferred_by_scope = 0;
    for candidate in &candidates {
        match body(candidate) {
            Outcome::Examined => examined += 1,
            Outcome::OutOfScope => out_of_scope += 1,
            Outcome::Unreadable => unreadable += 1,
            Outcome::DeferredByScope => deferred_by_scope += 1,
            Outcome::Absent => {}
        }
    }
    Population::build(
        unit,
        eligible,
        examined,
        (out_of_scope, unreadable, deferred_by_scope),
    )
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
    // A `BTreeSet`, not a `Vec` clone-and-sort-at-the-end: for a `Field`-unit
    // rule, a candidate is a slot, not a record, so a record with many
    // declared fields would otherwise contribute one clone per examined slot
    // (up to `eligible`, routinely far larger than the record count --
    // `BUG-40`) before the same set collapses out of a single sort pass
    // (`BUG-122`). Inserting into the set as candidates are examined does the
    // same collapse incrementally, one path at a time.
    let mut examined_records: std::collections::BTreeSet<PathBuf> =
        std::collections::BTreeSet::new();
    let population = census(unit, candidates, |candidate| {
        let outcome = body(candidate);
        if outcome == Outcome::Examined {
            examined_records.insert(record_of(candidate));
        }
        outcome
    });
    (population, examined_records.into_iter().collect())
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
    /// Present, but the header didn't parse (`Outcome::Unreadable`). Disclosed
    /// on its own so a rule's honesty about a malformed record never depends
    /// on a *different*, independently opt-in rule also being enabled to
    /// surface it (`ADR-53`, `BUG-125`, `ADR-63`).
    unreadable: usize,
    /// Candidates a rule that declares `scope: changed` (`ADR-90`) left
    /// unjudged because their record is not in the diff's changed set, or
    /// because there is no diff. Eligible and not examined, like every other
    /// unexamined candidate, but with its own cause: nothing about the
    /// configuration or the corpus is wrong. Serialized only when non-zero, so
    /// a rule that does not declare the option reports exactly what it did
    /// before; a consumer that does not know the field ignores it
    /// (`config.scope-matches-nothing` does not read it).
    #[serde(skip_serializing_if = "is_zero")]
    deferred_by_scope: usize,
    /// Lines a declared `ignore_marker` excused from a back-pointer rule
    /// (`ADR-80` amendment): a count of lines inside the examined candidates,
    /// not a kind of candidate. Serialized only when non-zero, so a
    /// configuration that declares no marker reports exactly what it did
    /// before.
    #[serde(skip_serializing_if = "is_zero")]
    excused_lines: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
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

    /// How many candidates were present but unreadable (a header that didn't
    /// parse). See the field's own doc comment for why this is never folded
    /// into `examined`'s shortfall silently.
    pub fn unreadable(&self) -> usize {
        self.unreadable
    }

    /// How many candidates a `scope: changed` rule did not judge because their
    /// record is not in the diff's changed set (`ADR-90`).
    pub fn deferred_by_scope(&self) -> usize {
        self.deferred_by_scope
    }

    /// How many lines a declared `ignore_marker` excused (`ADR-80` amendment).
    pub fn excused_lines(&self) -> usize {
        self.excused_lines
    }

    /// This population with the lines a declared `ignore_marker` excused.
    pub fn with_excused_lines(mut self, excused_lines: usize) -> Self {
        self.excused_lines = excused_lines;
        self
    }

    pub fn of(unit: PopulationUnit, eligible: usize, examined: usize) -> Self {
        Population::detailed(unit, eligible, examined, 0, 0)
    }

    /// `of`, naming every non-`Absent` reason a candidate went unexamined.
    pub fn detailed(
        unit: PopulationUnit,
        eligible: usize,
        examined: usize,
        out_of_scope: usize,
        unreadable: usize,
    ) -> Self {
        Population::build(unit, eligible, examined, (out_of_scope, unreadable, 0))
    }

    fn build(
        unit: PopulationUnit,
        eligible: usize,
        examined: usize,
        (out_of_scope, unreadable, deferred_by_scope): (usize, usize, usize),
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
            unreadable,
            deferred_by_scope,
            excused_lines: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    Ran,
    NotEnabled,
    /// Enabled, but the run cannot supply what the rule judges: a diff rule on a
    /// run with no `--base`. Distinct from `Ran` so a skipped gate is never read
    /// as a passed one, and from `NotEnabled` so the declaration is still seen.
    NotApplicable,
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
    /// Explicit paths on argv, read as given (`BUG-24`) -- distinct from
    /// `TrackedSweep` because a path git does not track is still examined.
    Argv,
    /// Only the records changed against a base ref (`MILE-123`); the ref is
    /// reported in [`ScopeInfo::base`].
    GitDiff,
    /// One record's proposed content read from stdin (`check --stdin-path`,
    /// `MILE-191`); the record need not exist on disk.
    Stdin,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeInfo {
    pub source: ScopeSource,
    /// The ref a `git-diff` run compared against, as the caller gave it;
    /// `null` for every other source -- a full sweep legitimately has none
    /// (SPEC-2).
    pub base: Option<String>,
    pub record_types: Vec<String>,
    /// Whether the configuration set `discovery.include_untracked` (`ADR-92`).
    /// Always present, so a consumer can tell the two corpora apart.
    pub include_untracked: bool,
    /// How many untracked records that option added to the run's files; `0`
    /// with the option off. Always present.
    pub untracked_included: usize,
    /// What a `git-diff` run examined and why; absent on every other source.
    #[serde(flatten)]
    pub diff: Option<DiffScope>,
}

/// The shape of a `git-diff` run's examined set (`MILE-123`, SPEC-2): the
/// changed records plus every record that depends on a change. Reported so a
/// run that examined nothing, a run that examined a few records and a run
/// that examined the whole corpus can be told apart.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffScope {
    /// Every path git reports as differing from the merge base, deletions
    /// included; empty exactly when nothing changed.
    pub changed_paths: Vec<String>,
    /// Records pulled in only because they refer to a changed or deleted
    /// record, or cite changed or deleted evidence.
    pub dependents: Vec<String>,
    /// Why every record was examined, when one was (a configuration change);
    /// `null` otherwise.
    pub whole_corpus_reason: Option<String>,
}

/// The date every comparison in a run was made against, and where it came
/// from (`ADR-97`, amendment of 2026-11-08). Present in every report of a
/// command that compares dates.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AsOf {
    /// `YYYY-MM-DD`, UTC.
    pub date: String,
    pub source: AsOfSourceKind,
}

/// `flag` when `--as-of` named the date, `clock` when the machine's UTC date
/// was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AsOfSourceKind {
    Flag,
    Clock,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckReport {
    pub status: ReportStatus,
    pub as_of: AsOf,
    pub files_examined: usize,
    /// How many records some rule actually reached a verdict about
    /// ([`records_read_by_any_rule`]). `files_examined` says what was read off
    /// disk; this says what was judged, and the gap between them is a run whose
    /// declared rules could not address the corpus (`BUG-61`). The verdict
    /// deliberately does not read it -- what gets checked is the adopter's call
    /// (`ADR-53`) -- but no reader takes `0` here for a clean corpus.
    pub records_read_by_any_rule: usize,
    pub rules_executed: Vec<RuleExecution>,
    /// The rule ids `--rules` restricted the run to, in the order given and
    /// without repeats; `null` on a run that was not restricted (`ADR-79`). A
    /// rule outside the selection is omitted from `rules_executed`, never
    /// listed as `not-enabled`, which would say the repository declined a
    /// policy it declared.
    pub rules_selected: Option<Vec<String>>,
    pub scope: ScopeInfo,
    pub blocking: bool,
    pub findings: Vec<Finding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
    /// The exit threshold in force (`fail_on` / `--fail-on`). Never
    /// serialized: stdout is the same whatever the threshold, only the exit
    /// code moves.
    #[serde(skip)]
    pub fail_on: crate::config::FailOn,
}

impl CheckReport {
    /// SPEC-0001's exit codes: 0 clean, 1 blocking findings, 2 could not run.
    /// Under `fail_on: warning` an unwaived warning also exits 1.
    /// Collapsing these is how "0 errors" comes to mean "never executed."
    pub fn exit_code(&self) -> i32 {
        match self.status {
            ReportStatus::NotRun => 2,
            _ if self.blocking => 1,
            _ if self.fail_on == crate::config::FailOn::Warning
                && self
                    .findings
                    .iter()
                    .any(|f| f.waived.is_none() && f.severity == FindingSeverity::Warning) =>
            {
                1
            }
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
    /// Structured fields a reader can branch on without parsing `message`
    /// (for example the offending `key` and its `nearest` valid key). Omitted
    /// from the output when a notice carries none. Each value is a string or,
    /// where the field is a set, a list of strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<std::collections::BTreeMap<String, DetailValue>>,
}

/// One value in a notice's `detail`: a string, or a list of strings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum DetailValue {
    Text(String),
    List(Vec<String>),
}

impl Notice {
    /// A notice with no structured `detail`.
    pub fn new(
        severity: NoticeSeverity,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Notice {
        Notice {
            severity,
            subject: subject.into(),
            message: message.into(),
            detail: None,
        }
    }
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
    /// Present when the config's `schema_version` is outside the window this
    /// build reads (`ADR-77`): the same facts `error` states, as data an agent
    /// can branch on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version_error: Option<crate::schema_version::SchemaVersionError>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl CouldNotRun {
    pub fn from(error: impl Into<Failure>) -> Self {
        let Failure {
            message,
            schema_version,
        } = error.into();
        CouldNotRun {
            status: ReportStatus::NotRun,
            error: message,
            schema_version_error: schema_version.map(|boxed| *boxed),
            notices: Vec::new(),
        }
    }
}

/// Why a command could not run: a message, and the structured form of the
/// cause when it has one. Every `String` error converts into it, so a command
/// that only has a sentence is unchanged; the config loader is the one source
/// of the structured part (`ADR-77`).
#[derive(Debug, Clone)]
pub struct Failure {
    pub message: String,
    /// Boxed so a `Result<_, Failure>` stays small on the success path.
    pub schema_version: Option<Box<crate::schema_version::SchemaVersionError>>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure {
            message,
            schema_version: None,
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Failure::from(message.to_string())
    }
}

impl From<Failure> for String {
    fn from(failure: Failure) -> Self {
        failure.message
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
    /// The resolved span of `path#scope` and its pin, when the argument names
    /// a scope (`ADR-74` point 7): the authoring route for a scoped pin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeExplanation>,
    /// The files a directory or glob argument covers and the pin over them
    /// (`BUG-176`), when the config accepts that kind of locator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageExplanation>,
    /// The records read to answer, as `check` reports a rule's population
    /// (`BUG-169`): an empty answer from an unread corpus is not the same as
    /// an empty answer from a read one.
    pub population: Population,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

/// What `urzua explain <directory-or-glob>` resolved (`BUG-176`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageExplanation {
    /// `directory` or `glob`.
    pub kind: String,
    /// The tracked files the locator covers, sorted: what the pin hashes.
    pub files: Vec<String>,
    /// The hash over the sorted (path, file content hash) list.
    pub sha256: String,
    /// The locator to paste into `Realized-by`; absent when the config does not
    /// let this kind carry a pin (`pinnable`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
}

/// What `urzua explain <path>#<scope>` resolved.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeExplanation {
    pub scope: String,
    /// The resolver that found the symbol; absent for a line range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver: Option<String>,
    pub start: usize,
    pub end: usize,
    /// The pinned bytes: the span's lines, normalised as hashed.
    pub text: String,
    pub sha256: String,
    /// The locator to paste into `Realized-by`.
    pub pin: String,
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
    /// See [`ExplainReport::population`].
    pub population: Population,
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

/// `urzua roadmap`'s report (`ADR-82`): the computed order of work, read-only
/// like `GraphReport`. The roadmap's own notes carry what could not be read;
/// the exit code is 0 whenever it could be computed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoadmapReport {
    pub status: ReportStatus,
    pub roadmap: crate::rules::Roadmap,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for RoadmapReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// One header field of an exported record, in file order. A list entry, not a
/// map entry: a repeated key is a defect `check` reports, and a map would
/// hide it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportedField {
    pub name: String,
    pub value: String,
    pub line: usize,
}

/// One record in `urzua export --format json` (SPEC-1, "Output contract").
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportedRecord {
    /// The configured type name (e.g. `milestone`).
    #[serde(rename = "type")]
    pub record_type: String,
    pub path: PathBuf,
    /// The display identifier from the filename (e.g. `MILE-115`); `null` when
    /// the filename carries none under the type's prefix.
    pub id: Option<String>,
    /// The `Stable-Id` header value; `null` when the record has none.
    pub stable_id: Option<String>,
    /// `false` when the header could not be parsed, so an empty `header` means
    /// "unreadable", not "no fields".
    pub header_readable: bool,
    pub header: Vec<ExportedField>,
    /// The file's complete text, unmodified, header included. No record type
    /// declares sections yet (`MILE-98`), so this is the one body field.
    pub raw_text: String,
    /// The record's sections, for a type that declares `sections`; `null` (not
    /// absent) for a type that declares none, so "no declaration" differs from
    /// "declared, none present" (`[]`).
    pub sections: Option<Vec<ExportedSectionEntry>>,
    /// Text between the header and the first section; `null` as `sections`.
    pub preamble: Option<String>,
}

/// One section in `urzua export --format json` (SPEC-1, "Export contract").
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportedSectionEntry {
    /// The declared section name; an alias resolves to the name it satisfies.
    pub name: String,
    /// The heading text as written.
    pub heading: String,
    /// 1-based file line of the heading.
    pub line: usize,
    pub end_line: usize,
    /// The lines after the heading through `end_line`.
    pub content: String,
}

/// How many records `export` emitted for one configured type; a type with
/// none is listed with `0`, so "empty" and "not configured" differ.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportTypeCount {
    #[serde(rename = "type")]
    pub record_type: String,
    pub dir: String,
    pub records: usize,
}

/// `urzua export --format json`'s report (SPEC-1): read-only, `Ok`-only
/// status axis like `GraphReport`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportReport {
    pub status: ReportStatus,
    pub record_count: usize,
    pub types: Vec<ExportTypeCount>,
    pub records: Vec<ExportedRecord>,
    /// The computed roadmap (`ADR-82`), present only when `roadmap.pairs` is
    /// declared: a repository that has not opted in sees no new key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roadmap: Option<crate::rules::Roadmap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

impl Report for ExportReport {
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
    /// `null` when the type's closed field set does not declare `Stable-Id`, so
    /// none was written (`BUG-182`).
    pub stable_id: Option<String>,
    /// What the numbering consulted beyond the working tree (`ADR-93` clause 6).
    pub pull_refs: PullRefsReport,
    /// Every number numbering passed over and why, one entry per number and
    /// reason (`ADR-93`, Amendment of 2026-11-07). Always present; empty when
    /// nothing was skipped.
    pub skipped: Vec<SkippedReason>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

/// How a `urzua waive` run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WaiveStatus {
    /// It ran: a preview, or every draft written.
    Ok,
    /// An `--apply` wrote some drafts and then failed (exit 1), as `fix` does.
    PartialFailure,
    /// It could not run, or an `--apply` failed before writing any draft (exit 2).
    NotRun,
}

/// `urzua waive --from-report`'s report (`ADR-91` point 12, `SPEC-25`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WaiveReport {
    pub status: WaiveStatus,
    /// `dry-run` or `apply`.
    pub mode: &'static str,
    /// The value each draft's `Reason` holds.
    pub pending_value: String,
    pub pending_source: crate::waive::PendingSource,
    pub drafts: Vec<WaiveDraft>,
    pub skipped: Vec<crate::waive::Skipped>,
    pub unmatched_filters: Vec<crate::waive::UnmatchedFilter>,
    /// Why an `--apply` stopped before writing every draft (`status` is then
    /// `partial-failure`, or `not-run` when none was written); `drafts[].written`
    /// says which were written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One waiver the command drafts (or wrote), for one rule in one file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WaiveDraft {
    /// The record's path: the number it would take now, in a dry run.
    pub path: PathBuf,
    /// `null` in a dry run: the id is minted when the record is written, and
    /// stays `null` for a draft that was not written.
    pub stable_id: Option<String>,
    pub rule: String,
    pub scope: String,
    pub findings: WaiveFindings,
    pub written: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WaiveFindings {
    pub count: usize,
    pub lines: Vec<String>,
}

impl Report for WaiveReport {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    /// 0 when it ran, 1 after a partial write, 2 when nothing could be written.
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(match self.status {
            WaiveStatus::Ok => 0,
            WaiveStatus::PartialFailure => 1,
            WaiveStatus::NotRun => 2,
        })
    }
}

/// One reason a number was skipped: `open-pr <ref>` (held by a ref under
/// `refs/urzua/pr`), `worktree <path>` (held in a sibling worktree) or `local`
/// (held by a local branch or remote-tracking ref).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SkippedReason {
    pub number: u32,
    pub reason: String,
}

/// The pull-request refs and sibling worktrees `urzua new` consulted
/// (`ADR-93` clause 6, `SPEC-12`). Always present, so a run that did not fetch
/// says why instead of being silent.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PullRefsReport {
    /// Whether this run fetched pull-request head refs.
    pub fetched: bool,
    /// Why it did not: `disabled` (`new.check_open_prs` is off), `offline` (the
    /// remote is missing or the fetch failed) or `timeout`; `null` when it did.
    pub reason: Option<String>,
    /// The remote fetched from; `null` when the option is off.
    pub remote: Option<String>,
    /// How many refs under `refs/urzua/pr` were read, fetched now or earlier.
    pub refs: usize,
    /// How many other worktrees of this repository were read.
    pub worktrees_scanned: usize,
    /// The numbers numbering skipped, with the refs or worktrees holding each.
    pub skipped_numbers: Vec<SkippedNumber>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkippedNumber {
    pub number: u32,
    pub held_by: Vec<String>,
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
    pub as_of: AsOf,
    pub population: Population,
    pub repairs: Vec<crate::fix::Repair>,
    /// Stale evidence pins, listed (or, after `--apply`, re-pinned) by
    /// `urzua fix --rebaseline` (`MILE-181`); absent otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rebaselines: Vec<crate::fix::Rebaseline>,
    /// Scoped pins that do not resolve and so cannot be re-pinned, each with
    /// its reason (`ADR-74`); absent when every pin resolves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_pins: Vec<crate::fix::FailedPin>,
    pub failed: Vec<FixFailure>,
    /// Who the written rows name and on what footing (`ADR-89` section 5);
    /// absent when nothing was applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<AttestationReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

/// The identity `fix --apply` wrote under (`ADR-89` section 5).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AttestationReport {
    pub name: String,
    pub source: crate::fix::IdentitySource,
    pub actor: crate::fix::ActorKind,
    /// The `--by` a verified `gh` login replaced, when they differ.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    /// The login `gh` verified, beside an `asserted` name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_login: Option<String>,
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

/// `urzua index`'s report (`MILE-152`): each declared index artifact, and
/// whether regenerating it from the records changes it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexReport {
    pub status: IndexStatus,
    pub indexes: Vec<IndexResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndexStatus {
    /// No index is declared, or the declaration could not be read: nothing ran.
    NotRun,
    /// At least one artifact could not be regenerated or written.
    PartialFailure,
    /// Every artifact already matches what regeneration produces.
    Ok,
    /// Detect mode: at least one artifact differs; nothing was written.
    RegenerationAvailable,
    /// Apply mode wrote at least one artifact, and none failed.
    Applied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndexOutcome {
    /// Already what regeneration produces; nothing to do.
    Current,
    /// Differs; detect mode left it alone.
    Stale,
    /// Differed; apply mode rewrote it.
    Written,
    /// Did not exist; apply mode created it.
    Created,
    /// Missing in detect mode; apply mode would create it.
    Missing,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexResult {
    pub path: PathBuf,
    pub outcome: IndexOutcome,
    /// Records the regenerated index lists.
    pub entries: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Report for IndexReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        match self.status {
            IndexStatus::NotRun => ExitCode::from(2),
            IndexStatus::PartialFailure => ExitCode::from(1),
            IndexStatus::Ok | IndexStatus::RegenerationAvailable | IndexStatus::Applied => {
                ExitCode::from(0)
            }
        }
    }
}

/// `urzua refs refresh`'s report (`MILE-143`): the snapshot of issue and pull
/// request numbers it wrote. A refresh that could not fetch every page writes
/// nothing and is a `CouldNotRun`, never a partial snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RefsRefreshReport {
    pub status: ReportStatus,
    pub snapshot: PathBuf,
    pub repository: String,
    pub generated_at: String,
    pub outcome: RefsRefreshOutcome,
    pub issues: usize,
    pub pulls: usize,
    /// The highest issue or pull request number recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefsRefreshOutcome {
    /// There was no snapshot file; one was written.
    Created,
    /// The snapshot file changed.
    Written,
    /// The file already held exactly this content.
    Current,
}

impl Report for RefsRefreshReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
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

    /// `ADR-90`: a candidate a `scope: changed` rule did not judge has its own
    /// counter, apart from `out_of_scope` (which `config.scope-matches-nothing`
    /// reads) and from `Absent`.
    #[test]
    fn a_deferred_candidate_is_counted_apart_from_out_of_scope_and_absent() {
        let outcomes = [
            Outcome::Examined,
            Outcome::DeferredByScope,
            Outcome::DeferredByScope,
            Outcome::Absent,
            Outcome::OutOfScope,
            Outcome::Unreadable,
        ];
        let p = census(PopulationUnit::Field, outcomes.to_vec(), |o| *o);
        assert_eq!(
            (
                p.eligible(),
                p.examined(),
                p.out_of_scope(),
                p.unreadable(),
                p.deferred_by_scope()
            ),
            (6, 1, 1, 1, 2)
        );
    }

    /// Additive on the wire: absent when zero, so a rule that does not declare
    /// the option reports byte for byte what it did before.
    #[test]
    fn deferred_by_scope_is_serialized_only_when_it_is_non_zero() {
        let none = census(PopulationUnit::Field, vec![Outcome::Examined], |o| *o);
        let json = serde_json::to_value(none).unwrap();
        assert!(json.get("deferred_by_scope").is_none(), "{json}");
        assert_eq!(json["out_of_scope"], 0, "out_of_scope stays always-present");
        let some = census(PopulationUnit::Field, vec![Outcome::DeferredByScope], |o| {
            *o
        });
        let json = serde_json::to_value(some).unwrap();
        assert_eq!(json["deferred_by_scope"], 1, "{json}");
        assert_eq!(json["eligible"], 1);
        assert_eq!(json["examined"], 0);
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
            detail: None,
        }
    }

    /// The invariant this whole design exists to protect: a `Notice` -- of
    /// any severity the type allows -- never moves the exit code either way.
    /// A successful report stays successful with notices attached...
    #[test]
    fn notices_never_move_a_successful_exit_code() {
        let report = CheckReport {
            as_of: AsOf {
                date: "2026-11-08".to_string(),
                source: AsOfSourceKind::Clock,
            },
            status: ReportStatus::Ok,
            files_examined: 1,
            records_read_by_any_rule: 0,
            rules_executed: vec![],
            rules_selected: None,
            scope: ScopeInfo {
                source: ScopeSource::TrackedSweep,
                base: None,
                record_types: vec![],
                include_untracked: false,
                untracked_included: 0,
                diff: None,
            },
            blocking: false,
            findings: vec![],
            notices: vec![warning_notice()],
            fail_on: Default::default(),
        };
        assert_eq!(Report::exit_code(&report), ExitCode::from(0));
    }

    fn report_with_warning(fail_on: crate::config::FailOn, waived: Option<&str>) -> CheckReport {
        CheckReport {
            as_of: AsOf {
                date: "2026-11-08".to_string(),
                source: AsOfSourceKind::Clock,
            },
            status: ReportStatus::FindingsPresent,
            files_examined: 1,
            records_read_by_any_rule: 1,
            rules_executed: vec![],
            rules_selected: None,
            scope: ScopeInfo {
                source: ScopeSource::TrackedSweep,
                base: None,
                record_types: vec![],
                include_untracked: false,
                untracked_included: 0,
                diff: None,
            },
            blocking: false,
            findings: vec![Finding {
                rule: "field.pending".to_string(),
                severity: FindingSeverity::Warning,
                file: PathBuf::from("docs/bugs/BUG-1.md"),
                line: None,
                message: "pending".to_string(),
                waived: waived.map(str::to_string),
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::default(),
            }],
            notices: vec![],
            fail_on,
        }
    }

    fn finding_with_action(action: Option<SuggestedAction>) -> serde_json::Value {
        serde_json::to_value(Finding {
            rule: "field.quality".to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from("docs/bugs/BUG-1.md"),
            line: None,
            message: "blank".to_string(),
            waived: None,
            declared_severity: None,
            suggested_action: action,
            action_source: ActionSource::default(),
        })
        .unwrap()
    }

    #[test]
    fn a_finding_without_an_action_has_no_suggested_action_key() {
        let json = finding_with_action(None);
        assert!(json.get("suggested_action").is_none(), "{json}");
    }

    #[test]
    fn an_action_serializes_its_kind_op_and_target_and_omits_what_was_not_computed() {
        let json = finding_with_action(Some(SuggestedAction {
            kind: ActionKind::EditField,
            op: Some(ActionOp::Set),
            target: ActionTarget {
                file: Some(PathBuf::from("docs/bugs/BUG-1.md")),
                field: Some("Regression-test".to_string()),
                ..Default::default()
            },
            value: None,
        }));
        assert_eq!(
            json["suggested_action"],
            serde_json::json!({
                "kind": "edit-field",
                "op": "set",
                "target": { "file": "docs/bugs/BUG-1.md", "field": "Regression-test" }
            })
        );
        // An uncomputed key is omitted, never null (`line: null` on the finding
        // itself is the shipped contract and not the action's).
        assert!(
            !json["suggested_action"].to_string().contains("null"),
            "{json}"
        );
    }

    #[test]
    fn a_value_is_a_string_a_list_of_strings_or_a_boolean() {
        let with = |value| {
            finding_with_action(Some(SuggestedAction {
                kind: ActionKind::EditConfig,
                op: Some(ActionOp::Set),
                target: ActionTarget {
                    file: Some(PathBuf::from(".urzua/config.yaml")),
                    key_path: Some(vec![
                        "record_types".into(),
                        "adr.v2".into(),
                        "known_fields".into(),
                    ]),
                    ..Default::default()
                },
                value: Some(value),
            }))["suggested_action"]
                .clone()
        };
        assert_eq!(with(ActionValue::Text("x".into()))["value"], "x");
        assert_eq!(
            with(ActionValue::List(vec![]))["value"],
            serde_json::json!([])
        );
        assert_eq!(with(ActionValue::Flag(true))["value"], true);
        // A key path is segments, never a dotted string, so a dotted name survives.
        assert_eq!(
            with(ActionValue::Flag(true))["target"]["key_path"],
            serde_json::json!(["record_types", "adr.v2", "known_fields"])
        );
    }

    #[test]
    fn a_kind_with_no_ops_serializes_without_one_and_a_command_carries_argv() {
        let json = finding_with_action(Some(SuggestedAction {
            kind: ActionKind::RunCommand,
            op: None,
            target: ActionTarget {
                argv: Some(vec!["urzua".into(), "index".into(), "--apply".into()]),
                ..Default::default()
            },
            value: None,
        }));
        assert_eq!(
            json["suggested_action"],
            serde_json::json!({ "kind": "run-command", "target": { "argv": ["urzua", "index", "--apply"] } })
        );
    }

    #[test]
    fn the_vocabulary_is_the_five_kinds_of_adr_73() {
        let names: Vec<&str> = ActionKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "edit-field",
                "edit-section",
                "edit-config",
                "add-record",
                "run-command"
            ]
        );
        for kind in ActionKind::ALL {
            let serialized = serde_json::to_value(kind).unwrap();
            assert_eq!(serialized, kind.as_str());
        }
    }

    #[test]
    fn a_warning_exits_0_by_default_and_1_when_fail_on_is_warning() {
        use crate::config::FailOn;
        let default = report_with_warning(FailOn::Error, None);
        assert_eq!(CheckReport::exit_code(&default), 0);
        let strict = report_with_warning(FailOn::Warning, None);
        assert_eq!(CheckReport::exit_code(&strict), 1);
    }

    #[test]
    fn an_info_finding_never_trips_fail_on_and_serializes_as_info() {
        use crate::config::FailOn;
        for fail_on in [FailOn::Error, FailOn::Warning] {
            let mut r = report_with_warning(fail_on, None);
            r.findings[0].severity = FindingSeverity::Info;
            assert_eq!(CheckReport::exit_code(&r), 0, "{fail_on:?}");
        }
        assert_eq!(serde_json::to_value(FindingSeverity::Info).unwrap(), "info");
    }

    #[test]
    fn a_waived_warning_never_fails_under_fail_on_warning() {
        use crate::config::FailOn;
        let waived = report_with_warning(FailOn::Warning, Some("WAIVER-1"));
        assert_eq!(CheckReport::exit_code(&waived), 0);
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
