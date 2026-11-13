use super::*;

/// The spec that documents every rule this build ships: its generated rule
/// table lists each one, and its action table lists each variant's action.
pub const RULE_SPEC: &str = "SPEC-2";

/// Where a rule looks, which fixes what its population is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Loaded records: a header, a filename or a body.
    Records,
    /// The configuration alone.
    Config,
    /// Files that are not records: claim files, published files, source files,
    /// index artifacts, tracked paths.
    Files,
    /// The paths a `--base` run changed.
    Diff,
    /// The run itself: other rules' executions and findings, waiver records.
    Run,
}

/// What a rule's verdict on a record depends on (`ADR-90`). It decides whether
/// the rule may list the `scope` option, which defers every record a `--base`
/// diff did not change: that is sound only where the unchanged records cannot
/// hold a defect that the changed ones cause, or hide one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judges {
    /// A record-level rule whose verdict on a record depends on that record
    /// alone, with its findings on that record (`Anchor::Record` or
    /// `Anchor::RecordOrDeclaredFile`). May list `scope`.
    OneRecord,
    /// A record-level rule whose verdict on a record reads other records (a
    /// reference to resolve, a collision, a reciprocal pair, a sequence), so
    /// deferring the unchanged ones would hide a defect a change caused. Does
    /// not list `scope`.
    CrossRecord,
    /// A record-level rule that already judges only the changed set
    /// (`status.transition-allowed`, `revision-log.version-bump-required`):
    /// `scope` would do nothing. Does not list `scope`.
    ChangedSet,
    /// A record-level rule that reads other records as context and reports
    /// only advisory findings anchored on a record the diff changed, one per
    /// changed anchor (`ADR-90` amendment, 2026-11-07). May list `scope`.
    AnchorScoped,
    /// Not a record-level rule: it judges the configuration, files, changed
    /// paths, the run itself, or a directory. Does not list `scope`.
    NotRecordLevel,
}

impl Scope {
    /// The name `urzua rules` and `urzua explain --rule` print.
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Records => "records",
            Scope::Config => "config",
            Scope::Files => "files",
            Scope::Diff => "diff",
            Scope::Run => "run",
        }
    }
}

impl Judges {
    /// The name `urzua rules` prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Judges::OneRecord => "one-record",
            Judges::CrossRecord => "cross-record",
            Judges::AnchorScoped => "anchor-scoped",
            Judges::ChangedSet => "changed-set",
            Judges::NotRecordLevel => "not-record-level",
        }
    }
}

/// How a rule meets a `header_shape: none` type (`ADR-50`, `MILE-119`): such a
/// type has nowhere for a header to be, so a rule that reads one classifies the
/// type's records `OutOfScope`. A rule that cannot read a header says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderNone {
    OutOfScope,
    Exempt(&'static str),
}

/// A per-type declaration that a rule reads and that an adopted corpus starts
/// without. Named by the config key, which is what an adopter writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declaration {
    RequiredFields,
    KnownFields,
    PointerFields,
    /// `revision_log.version_bump` (`BUG-51`).
    VersionBump,
    /// `statuses.blocked` or `statuses.progress` (`MILE-144`).
    BlockedOrProgress,
    /// `statuses.delivery.items` (`ADR-96`).
    DeliveryItems,
}

impl Declaration {
    /// The record-type config key that carries the declaration.
    pub fn key(self) -> &'static str {
        match self {
            Declaration::RequiredFields => "required_fields",
            Declaration::KnownFields => "known_fields",
            Declaration::PointerFields => "pointer_fields",
            Declaration::VersionBump => "revision_log.version_bump",
            Declaration::BlockedOrProgress => "statuses.blocked or statuses.progress",
            Declaration::DeliveryItems => "statuses.delivery.items",
        }
    }
}

/// A rule's second population: its execution and findings, or `None` when the
/// rule has nothing to judge in that unit on this run.
pub type CompanionFn = fn(&RuleCtx<'_>) -> Option<(RuleExecution, Vec<Finding>)>;

/// One rule, with every property of it that anything outside the rule needs.
/// The single place a rule is registered: the shipped rule set, `urzua rules`,
/// `SPEC-2`'s rule table, config validation and `init`'s proposal are all
/// derived from `RULE_TABLE`, so a property cannot live in a second list.
pub struct RuleDef {
    pub id: &'static str,
    /// Adopter-facing; `SPEC-2`'s own rule table is generated from it.
    pub description: &'static str,
    pub scope: Scope,
    /// The file a finding of this rule sits on, which decides whether a path
    /// scope or `--base` keeps it.
    pub anchor: Anchor,
    /// What the rule's verdict on a record depends on, which decides whether it
    /// may list the `scope` option (`ADR-90`).
    pub judges: Judges,
    /// Resolves through the identity index, which is built from filename type
    /// prefixes (`ADR-36`), so a corpus with no prefix gives it nothing to
    /// examine (`BUG-61`).
    pub needs_identity_index: bool,
    /// Enabling it with no `relation_pairs` declared is a config load error
    /// (`RFC-47`).
    pub requires_relation_pairs: bool,
    /// Enabling it with no `relation_pairs` entry declaring `acyclic: true` is
    /// a config load error: it reads only acyclic pairs (`RFC-48`).
    pub requires_acyclic_pair: bool,
    /// Enabling it with no top-level `override_log` declared is a config load
    /// error: it would read no log (`ADR-78`, `MILE-188`).
    pub requires_override_log: bool,
    /// Enabling it with no top-level `authority` declared is a config load
    /// error: it would judge no gate (`ADR-89` section 4, `MILE-200`).
    pub requires_authority: bool,
    /// The per-type declaration the rule reads, without which it examines
    /// nothing. `urzua init` writes none of these (`ADR-33`), so it does not
    /// propose such a rule: it would report `ran` over zero records
    /// (`BUG-73`). `None` for a rule that reads no per-type declaration.
    pub reads_declaration: Option<Declaration>,
    pub header_none: HeaderNone,
    /// The `RuleSetting` options the rule accepts, by name. Declaring one on any
    /// other rule is a load-time error: a silently ignored option is a check
    /// that never fires.
    pub options: &'static [&'static str],
    /// The unit of the rule's population (`RuleExecution::population`): what
    /// one candidate it is handed is. Declared here so `urzua rules` and
    /// `urzua explain --rule` can state it without running the rule; a test
    /// checks it against the unit a run reports.
    pub population: PopulationUnit,
    /// What each finding variant the rule can emit offers, or why it offers
    /// nothing (`ADR-73`). Every variant is listed; a variant listed nowhere
    /// fails the coverage tests.
    pub actions: &'static [VariantAction],
    /// When in a run the rule executes.
    pub phase: Phase,
    /// Judges the run's inputs; never called for a rule the repository has not
    /// enabled, and its findings are given the declared severity by the runner.
    pub run: fn(&RuleCtx<'_>) -> (RuleExecution, Vec<Finding>),
    /// A second population the rule judges in a unit of its own, reported as a
    /// second `rules_executed` entry under the same rule id (`MILE-186`): a
    /// rule has one population, and a second unit is a second execution, not a
    /// second denominator under one name. `None` from the function means the
    /// rule has nothing to judge in that unit on this run (its option is
    /// unset). Gated and given severities exactly as `run` is.
    pub companion: Option<(PopulationUnit, CompanionFn)>,
}

/// What a rule's findings sit on. A scoped `check` (a path argument or
/// `--base`) keeps or drops a finding by this, never by rule id: a rule that
/// reports on something other than a record would otherwise be silently
/// dropped from a narrowed run (`BUG-67`, `BUG-86`, `BUG-121`).
#[derive(Clone, Copy)]
pub enum Anchor {
    /// A loaded record. A path scope keeps it under a requested prefix; a diff
    /// run keeps it when the record was examined.
    Record,
    /// The configuration file. Outside every scope, so every scope keeps it.
    Config,
    /// A file the rule reads that no record-type `dir` covers (a claim file, a
    /// source file, an index artifact, a changed path). Every scope keeps it:
    /// the run, not a record, is what it is about.
    Outside,
    /// A tracked path that is not a loaded record, judged by path. A path scope
    /// applies its prefixes; a diff run keeps it when the path changed.
    ChangedPath,
    /// A type's directory. A path scope applies its prefixes; a diff run keeps
    /// it when any changed path lies inside.
    Directory,
    /// A record, or a file the rule's own options declare (returned by the
    /// function, `None` when the rule is not enabled): a finding on that file
    /// is no record's and belongs to every scope.
    RecordOrDeclaredFile(fn(&Config) -> Option<PathBuf>),
}

/// When a rule runs. A rule that judges other rules' executions, or the
/// findings left after scoping, can only run after those exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Main,
    /// After every `Main` rule: it reads their executions.
    AfterRules,
    /// After scope filtering: it reads the findings the run will report.
    AfterScoping,
}

/// Every rule this build ships, in the order `check` runs them and lists them
/// in `rules_executed`.
pub const RULE_TABLE: &[RuleDef] = &[
    RuleDef {
        id: RULE_HEADER_REQUIRED_FIELDS,
        description: "Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("header-region-missing", ActionDecl::Exempt("the header region is neither a field nor a section, so no action kind addresses it")),
            VariantAction::new("duplicate-key", ActionDecl::FIELD_REMOVE),
            VariantAction::new("field-missing", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            header_required_fields(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_POINTER_RESOLUTION,
        description: "A type's config-declared `pointer_fields` \u{222a} `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted. A reference to a record whose file exists on disk but is not tracked by git is told apart from one that exists nowhere (`BUG-13`): a different finding variant, naming the file, because discovery reads only tracked and staged files; the same holds for `relation.supersession-reciprocity`, `relation.pair-reciprocity` and `sequence.unresolved-blocker`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("pointer-unresolved", ActionDecl::FIELD_SET),
            VariantAction::new("pointer-untracked", ActionDecl::Exempt("the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            pointer_resolution(c.records, c.config, c.index, c.untracked_records)
        },
    },
    RuleDef {
        id: RULE_POINTER_TARGET_STATUS,
        description: "A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["not_in", "fields"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("pointer-target-status", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: PointerTargetOpts = c.config.rule_opts(RULE_POINTER_TARGET_STATUS);
            pointer_target_status(c.records, c.config, &opts.not_in, opts.fields.as_deref(), c.index)
        },
    },
    RuleDef {
        id: RULE_FIELD_QUALITY,
        description: "Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `template_placeholders` option (`BUG-22`), off unless declared `true`, reads a field whose value is identical after trimming to its type's template text for that field (`.urzua/templates/<type>.md`) as a placeholder, and stops applying the compiled-in placeholder tokens; a type with no template is judged by its declared `pending` vocabulary alone, and declaring the option when no type has a template is disclosed as a warning notice.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::RequiredFields),
        header_none: HeaderNone::OutOfScope,
        options: &["field_levels", "template_placeholders"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("field-quality", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            field_quality(c.records, c.config, c.templates)
        },
    },
    RuleDef {
        id: RULE_FIELD_PENDING,
        description: "A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `pending_until_required` option (`MILE-169`), off unless declared `true`, treats a pending value as legitimate until the record reaches a status the type's `statuses.require` ties the field to: a deferred slot stays eligible and is counted unexamined, and a field no `require` entry names is judged as before.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::RequiredFields),
        header_none: HeaderNone::OutOfScope,
        options: &["pending_until_required", "field_levels"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("field-pending", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            field_pending(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CLAIM_STATUS_AGREEMENT,
        description: "A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared. A claim is a closing verb followed by a record reference in prose (`Fixes BUG-60`), which is a deprecated convenience: a claim written without the verb is not seen. The optional `closes` option (`ADR-85`) names a frontmatter key in which a claim file lists, on one line, the records it closes (`closes: [BUG-60, BUG-61]`, a single id, or `[]` for none), and that data is checked instead of a sentence: a listed id that resolves to no record is `claim.id-unresolved`; a listed record not at a terminal status its own type declares (the type's `statuses` success and failure, not `closed_statuses`) is an error naming the record, and offers an `edit-field` on its `Status`; a value that is a block list, a list continued over several lines or not a list of record ids is `claim.field-malformed`, never guessed at. Only the frontmatter block is read for it and every other key is ignored, so a release tool's own keys are untouched. The optional `strict` option (default `false`, needs `closes`) makes a claim file that has a frontmatter block and lacks the key a finding (`claim.field-missing`); `closes: []` declares that it closes nothing. A file with no frontmatter block is not a fragment: counted out of scope in the population and named in a notice, never given a finding; the exception is a file whose `---` fence is never closed or has a blank line before it, which the release tool refuses to read, and which under `strict` is `claim.field-malformed`. With `closes` declared the closing-verb reading still runs over the body, and `check` prints how many claims it found, unless the optional `verb_claims` option (`ADR-88`, default `true`, needs `closes`) is `false`: then no prose is read for a claim, only the field is, and `check` states that the reading is off (and, without `strict`, that a fragment lacking the key carries no claim). Without `closes` nothing changes.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is claims, not records"),
        options: &[
            "claim_paths",
            "closed_statuses",
            "closes",
            "strict",
            "verb_claims"
        ],
        population: PopulationUnit::Claim,
        actions: &[
            VariantAction::new("claim-status-disagrees", ActionDecl::Exempt("the rule catches a claim that may be false, so the right fix is often the claim file, a non-record file with no action kind yet, and editing the record Status would be the mechanical answer and possibly the wrong one")),
            VariantAction::new("claim-closes-open", ActionDecl::FIELD_SET),
            VariantAction::new("claim-id-unresolved", ActionDecl::Exempt("the id names no record, so there is no record to edit; the fix is to correct the id in the claim file, a non-record file with no action kind yet, or to file the record")),
            VariantAction::new("claim-field-malformed", ActionDecl::Exempt("the edit is to the claim file, a non-record file with no action kind yet")),
            VariantAction::new("claim-field-missing", ActionDecl::Exempt("the edit is to the claim file, a non-record file with no action kind yet, and which records the change closes is the author's to say")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: ClaimOpts = c.config.rule_opts(RULE_CLAIM_STATUS_AGREEMENT);
            claim_status_agreement(c.claims, &opts, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_FILENAME_TITLE_CONSISTENCY,
        description: "The filename's display number and the H1's display number agree (ADR-3). The H1's id is read the way a filename's is (`BUG-32`): with a declared `identity` pattern, the leading token must be the pattern's `prefix` and a number (`SPEC-3`); with none, a leading `PREFIX-N` or a bare `N`. A title that does not match, such as `# 2026-09-16 Notes` or `# v1.2 Release`, is reported as carrying no record id and never read as a number, and a number too large to compare is reported as such.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads the filename and H1 only"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("title-mismatch", ActionDecl::Exempt("the fix is a rename or an H1 edit, and no action kind addresses either yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            filename_title_consistency(c.records, c.full_text)
        },
    },
    RuleDef {
        id: RULE_RELATION_SUPERSESSION_RECIPROCITY,
        description: "If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("supersession-unresolved", ActionDecl::FIELD_SET),
            VariantAction::new("supersession-untracked", ActionDecl::Exempt("the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration")),
            VariantAction::new("supersession-unreciprocated", ActionDecl::FIELD_APPEND),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| supersession_reciprocity(c.records, c.config, c.index, c.untracked_records),
    },
    RuleDef {
        id: RULE_RELATION_PAIR_RECIPROCITY,
        description: "For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. A pair that declares `types` is carried by those types only, and a claim to a type outside the list is excluded as out of scope (`MILE-155`). A pair that declares `provisional: {statuses, level}` reports an unechoed claim from a claimant at one of those statuses at the provisional level (default `warn`), and any other claimant at the rule's level (`MILE-154`). The optional `provisional.acknowledged_by` names a field on the peer that settles a provisional claim by naming the claimant (`MILE-163`); it is not read for any other claimant. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: true,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("pair-unresolved", ActionDecl::FIELD_SET),
            VariantAction::new("pair-untracked", ActionDecl::Exempt("the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration")),
            VariantAction::new("pair-unreciprocated", ActionDecl::FIELD_APPEND),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            relation_pair_reciprocity(c.records, c.config, c.index, c.untracked_records)
        },
    },
    RuleDef {
        id: RULE_RELATION_CROSS_TYPE_DISALLOWED,
        description: "A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: true,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("cross-type-claim", ActionDecl::FIELD_REMOVE),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            relation_cross_type_disallowed(c.records, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_SEQUENCE_CYCLE,
        description: "A cycle in a declared `relation_pairs` entry that sets `acyclic: true` (`RFC-48`, `MILE-27`): following the written claims of either field, in the direction each declares, returns to the record it started at. Every record on a cycle is reported with the cycle's path, including a record that names itself and a cycle that crosses record types. The pairs `roadmap.pairs` names are read as one graph (`ADR-82`, `MILE-192`), so a loop that uses an edge of each of two declared pairs is reported on every member, and the message names the pairs it crosses; any other acyclic pair, and every pair when `roadmap.pairs` is absent, is read on its own. A claim whose target does not resolve is `sequence.unresolved-blocker`'s. Needs an acyclic pair; enabling it with none declared is a load error.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: true,
        requires_acyclic_pair: true,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("sequence-cycle", ActionDecl::Exempt("breaking a cycle means choosing which dependency to drop, a decision the tool cannot compute")),
            VariantAction::new("sequence-cycle-cross-pair", ActionDecl::Exempt("breaking a loop that crosses declared roadmap pairs means choosing which dependency, in which pair, to drop, a decision the tool cannot compute")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| sequence_cycle(c.records, c.config, c.index),
    },
    RuleDef {
        id: RULE_SEQUENCE_UNRESOLVED_BLOCKER,
        description: "A value in either field of a declared `relation_pairs` entry that sets `acyclic: true` and names no record (`RFC-48`, `MILE-27`): a reference that resolves to no discovered record, or an entry that is not, as a whole, one record reference (`MILE-4 MILE-5` is prose). A claim in a field that is one of the type's `pointer_fields` is left to `pointer.resolution` and `header.pointer-field-clean` when those are enabled. A fact that is not yet a record is a record of its own, not a prose value (`ADR-76`). Needs an acyclic pair; enabling it with none declared is a load error.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: true,
        requires_acyclic_pair: true,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("blocker-unresolved", ActionDecl::FIELD_SET),
            VariantAction::new("blocker-untracked", ActionDecl::Exempt("the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration")),
            VariantAction::new("blocker-not-a-reference", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| sequence_unresolved_blocker(c.records, c.config, c.index, c.untracked_records),
    },
    RuleDef {
        id: RULE_SEQUENCE_STATUS_AGREEMENT,
        description: "A record's written `Status` contradicts the derived `blocked` value (`MILE-144`, `RFC-48`, `ADR-83`). A record is blocked when any record it depends on, over the pairs `roadmap.pairs` declares and in either field, is not in a status its own type declares `success`; a dependency in a `failure` status, with no `Status`, or whose type declares no `success` blocks. Reports a blocked record that states a status its type lists in `statuses.progress` (`Done` included when listed; `Planned` is not progress), and a status the type lists in `statuses.blocked` while nothing blocks the record. Opt-in per type: only a type that declares `progress` or `blocked` is examined, and enabling the rule with no `roadmap.pairs`, or with no type declaring either, is a config load error. A value that cannot be computed is not guessed: a record in a `blocked` or a `progress` status that would be reported unblocked is counted unreadable when it names a dependency that resolves to nothing or to an unreadable record, or when a record that may write an inverse claim cannot be read, whether or not the type writes a `blocked` status at all (`ADR-83`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: true,
        requires_acyclic_pair: true,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::BlockedOrProgress),
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("status-blocked-unblocked", ActionDecl::FIELD_SET),
            VariantAction::new("status-blocked-derived", ActionDecl::FIELD_SET),
            VariantAction::new("status-blocked-capped", ActionDecl::Exempt("an info notice, not a defect: the record's `partial_status` comes from the dependency cap (every Done means item resolves, only an unfinished dependency holds it back), so there is nothing to repair")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            with_locator_env(c, |env| {
                sequence_status_agreement(c.records, c.config, c.index, c.back_pointed, env)
            })
        },
    },
    RuleDef {
        id: RULE_RELATION_TARGET_STATUS_UNDECLARED,
        description: "A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("target-status-undeclared", ActionDecl::CONFIG_APPEND),
            VariantAction::new("target-status-undeclared-declare-set", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| relation_target_status_undeclared(c.records, c.config, c.config_path, c.index),
    },
    RuleDef {
        id: RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
        description: "Every revision-log entry carries a `change_class` (ADR-14) from the type's declared vocabulary, by default `substantive` or `structural`; missing or unclassified is blocking, and the finding lists the accepted classes. A type may declare the vocabulary (`classes`) and the header names of its class and date columns (`class_column`, `date_column`) in `revision_log`; undeclared, the class is the last cell and the date the first (`ADR-94`). A log whose header lacks a declared column, or repeats it, is one finding on the record (`revision-log-column-unreadable`) and none of its rows is judged. A record of a type that declares `revision_log: {required: true}` with no revision-log section is a finding too (`BUG-50`); a type that does not declare it, or declares `false`, is not held to a log and its records without one stay unexamined. The log is the section a type's `revision_log` identity (default the line `**Revision log**`) opens, found as that line and never by a line that mentions the text (`MILE-167`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body text only"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("change-class-missing", ActionDecl::SECTION_SET),
            VariantAction::new("revision-log-missing", ActionDecl::SECTION_ADD),
            VariantAction::new("revision-log-column-unreadable", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| revision_log_change_class(c.records, c.config, c.full_text),
    },
    RuleDef {
        id: RULE_REVISION_LOG_VERSION_BUMP_REQUIRED,
        description: "A changed record that adds a revision-log row of a class its type declares must also change its version (`BUG-51`, `SPEC-18`: a substantive or structural change is a `Version` bump plus a revision-log entry). A type declares it as `revision_log: {version_bump: {field: Version, classes: [substantive, structural]}}`: a field name and the classes that require a bump (each one in the type's declared `classes` when it declares them), and no type has it by default. Judged against the merge base's copy of the record, so it reports `not-applicable` on a run with no `--base`, never clean; a record new in the diff, including one renamed in it, has nothing to bump from and stays unexamined. A row counts as added when its cells appear in the current log and not in the merge base's. A record whose log has a declared column missing or repeated, now or at the merge base, is reported unreadable in the rule's population, never clean and never read as an empty log (`ADR-94`). Enabling the rule when no type declares a bump is a config load error.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::ChangedSet,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::VersionBump),
        header_none: HeaderNone::Exempt("reads a declared version field and body text, only on a --base run"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("version-bump", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| revision_log_version_bump(c.records, c.config, c.full_text, c.changed_paths, c.base_text),
    },
    RuleDef {
        id: RULE_EMBODIMENT_CONSISTENCY,
        description: "A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (the tier order of the `embodiment` declaration, by default `test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to the declared drift state unconditionally (RFC-5 tier 1, ADR-18/ADR-32). A record with no `Realized-by` field is judged on the back-pointers that cite it (`MILE-149`); one stating a state the declaration marks `claims_built` that has neither a `Realized-by` field nor a citing back-pointer is reported as a claim with no evidence, computed to the declared no-evidence state (`MILE-161`), and one stating any other value is not judged and stays unexamined. Requires full git history — a shallow checkout makes this rule silently unable to detect anything. Three opt-in refinements, all off by default (`MILE-129`): a locator written `path@sha256:<64 hex>` drifts when the file's content hash differs and not otherwise, in place of the git-history heuristic; the `drift_grace_days` option holds back reporting drift whose locator last changed fewer than that many days ago, and a record in that window agrees with either its tier or a stated drift state (the window never contradicts a record that already says so, and `urzua fix` leaves it alone); `check` discloses what the window holds back in one info notice, `embodiment-drift-in-grace`, for the records the rule would report once the window ends (a record of a type with no `Embodiment`, one stating a sticky state, or one already stating the drift state is not counted), with the count, the earliest date they leave the window and their paths; the optional `drift_audit_field` option names a declared date field (needs `drift_grace_days`): a record carrying a `YYYY-MM-DD` date there has its window counted from that last-audit date instead of from the cited file's last change, and a malformed date is reported and falls back to the file (`MILE-166`); the `claim_max_age_days` option warns when the `Realized-by` line was last touched more than that many days ago. The optional `sticky_states` option lists `Embodiment` values a human sets that no evidence can compute, such as `Deprecated` and `Inactive`: a record stating one is examined and never reported as drift or as an old claim, `urzua fix` leaves it alone, and a listed value that evidence does compute (a state of the `embodiment` declaration) is a load error because it would hide real drift (`MILE-148`). Source back-pointers (see `embodiment.back-pointer-resolution`) are evidence too: an `Implements:` citation is code tier and a `Verifies:` citation in a declared test file is test tier, equal to a `test:` locator (`MILE-146`). A record with `Embodiment` but no `Realized-by` field is judged on back-pointer evidence alone when one cites it, and a record neither cited nor carrying `Realized-by` is not judged (`MILE-149`); a finding names the evidence it computed from.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["drift_grace_days", "drift_audit_field", "claim_max_age_days", "sticky_states"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("embodiment-drift", ActionDecl::RUN_COMMAND),
            VariantAction::new("embodiment-unevidenced", ActionDecl::FIELD_SET),
            VariantAction::new("last-audit-date-invalid", ActionDecl::FIELD_SET),
            VariantAction::new("claim-too-old", ActionDecl::Exempt("the remedy is re-verifying the claim, which may need no edit, and touching the field to reset the age without checking would defeat the rule")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            embodiment_consistency(
                c.records,
                c.config,
                c.drifted,
                c.fresh,
                c.back_pointed,
                c.claim_ages,
            )
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_CLAIM_EXPIRED,
        description: "A claim expires after a period declared per record type, status and evidence tier (`ADR-97`, `MILE-129`). Opt-in: a configuration that does not declare the rule runs nothing new and loads and reports as before. `periods` is a list of `{type, status?, tier?, days, spread_days?, level?}`; the most specific entry wins (type, status and tier outrank type and status, which outrank type alone), two entries of equal specificity that can match one record are a load error, and a record of a listed type that no entry matches is eligible and not examined, and an entry whose type holds claims (a record with a locator) and covers none of them is reported by `config.scope-matches-nothing`, while an entry for a type with no claim yet is not. The `tier` is the one the record's locators compute to under the `embodiment` declaration, so a milestone or bug that states no `Embodiment` is judged too; a record whose status is in the type's `claim_retained`, or that states a `sticky_states` value, is not. A claim's age is whole UTC days from its anchor: the date in the declared `anchor_field` (`YYYY-MM-DD`, written only by `urzua reverify`; a malformed or future date is reported and ignored), otherwise the date of the commit that last touched the first evidence line: the `Realized-by` line, or for a record with no value there the first Done-means item evidence clause (`ADR-97`, amendment of 2026-11-08). A record is judged when its `Realized-by` field or its Done-means item evidence clauses give any evidence; one with neither is not examined (\"no locator field and no Done-means item evidence clause\"). A record with no valid anchor date has its first period lengthened by a deterministic offset of 0 to `spread_days` days, the first 8 bytes of the SHA-256 of its `Stable-Id` as a big-endian number modulo `spread_days + 1`, so a corpus rewritten on one day does not expire on one day; the offset ends with the first valid anchor date and a record with no `Stable-Id` gets 0. A finding is reported at the entry's `level`, held to the rule's own, and offers `urzua reverify --ids <record>` (a review report, not a write). `require_pin` (evidence categories that must be pinned before a renewal), `drift` (a map from an evidence category to `block`, `acknowledge` or `ignore`: whether drift of a locator of that category since the claim's line was last touched blocks a renewal, lets the reviewer's `--reason` renew it with the drift written into the revision row, or is left out of renewal; a category not named is `block`, and an unknown category or policy is a load error) and `max_batch` (most records one `reverify --apply` names) are read by that command (`ADR-97`, amendment of 2026-11-09). Drift and expiry are independent: both report on one record, `drift_grace_days` does not delay expiry, and a waiver suppresses the finding without renewing the claim (a waiver with no `Expires` is governed by the `permanent` declaration, `ADR-86`). Declaring this rule together with `claim_max_age_days` on `embodiment.consistency` is a load error.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["anchor_field", "require_pin", "drift", "max_batch", "periods", "scope"],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("claim-expired", ActionDecl::RUN_COMMAND),
            VariantAction::new("claim-anchor-invalid", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            embodiment_claim_expired(
                c.records,
                c.config,
                changed_only(c.config, RULE_EMBODIMENT_CLAIM_EXPIRED, c.changed_paths).as_ref(),
                c.back_pointed,
                &ClaimInputs {
                    now_unix: c.now_unix,
                    line_times: c.claim_line_times,
                    seeds: c.claim_seeds,
                },
            )
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED,
        description: "On a `--base` run, a changed record whose `anchor_field` (declared by `embodiment.claim-expired`) is newer than in its merge-base copy and which gained no `Re-verified` revision row dated that day, or whose `anchor_field` the merge-base copy carried and the branch removed or left malformed, not a date (`ADR-97` section 6); the finding says \"was removed\" for a removed field, \"is empty\" for one left blank and \"is malformed\" for one that is not a date. A `Re-verified` row counts only in the shape `urzua reverify` writes (attested by, identity and actor, an `Evidence:` list, a `Reason:`); text that merely mentions a re-verification is not one. The rule also reports a drift-acknowledging `Re-verified` row the branch added that names a locator which had not drifted at the merge base (`category:locator`, or `category:ID` for a cited record, read from the merge base's copy of the record, its claim lines dated as of the merge base) or whose `Evidence:` list is not the record's current locators or lists a pin hash the record does not hold, or which is dated outside the branch (later than the evaluation date, more than one UTC day before the UTC author day of the branch's first commit (the merge base's when the branch has no commit yet), or after the later of the UTC author day and committer day of the commit that added the row, both settable like the typed row date; today for a row not yet committed), so a row typed by hand that does not describe the real drift is a finding at the pull request and a row the tool wrote passes. Prevention is not claimed, detection is: a date in a text file can be typed, and only `urzua reverify` writes the row that says who renewed the claim and what the tool checked. A reordered `Evidence:` list and a changed attestor are not seen, and a plain `check` without `--base` trusts a well-formed acknowledging row (a row dated after the evaluation date clears nothing in any run): the PR-time check is the `--base` run that CI makes. It cannot see a record that never carried the field, nor one new in the diff (no merge-base copy), and not-applicable on a run with no `--base`. Needs `embodiment.claim-expired` with an `anchor_field`; opt-in; its level is the repository's.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges a changed record against its merge-base copy, only on a --base run"),
        options: &["scope"],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("claim-renewal-unattested", ActionDecl::Exempt("the remedy is a judgement: undo the hand-typed date, or have someone re-verify the claim with `urzua reverify`, which writes the row")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            embodiment_claim_renewal_unattested(
                c.records,
                c.config,
                c.full_text,
                c.base_text,
                c.changed_paths,
                &BaseReads {
                    drift: c.base_drift,
                    now_unix: c.now_unix,
                    added_row_days: c.added_row_days,
                },
                changed_only(
                    c.config,
                    RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED,
                    c.changed_paths,
                )
                .as_ref(),
            )
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS,
        description: "A record whose `Status` is not an accepted status states an `Embodiment` the `embodiment` declaration marks `claims_built` (`MILE-138`): a decision not yet made cannot have been built. The accepted statuses are the type's declared `statuses.success`; a type declaring none uses the rule's `accepted_statuses` option, and with neither the type is not examined. Any other state, including the drift state and a sticky one, is not flagged, and a record missing either field is left to the header rules. Opt-in; the population counts the records the rule could judge.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["accepted_statuses"],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("embodiment-state-not-accepted", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| embodiment_requires_accepted_status(c.records, c.config),
    },
    RuleDef {
        id: RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        description: "A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids, each optionally followed by a note, as in `Implements: ADR-1 (why), ADR-2`) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. The marker is read at the start of a comment line or in a trailing comment after code (`//`, `#`, `--`, `/*`, `<!--` after whitespace); the marker is case-sensitive; reading continues past a note to the next comma-separated id and stops at a segment that does not open with one; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. The optional `test_paths` option (path prefixes, each inside a `back_pointer_paths` entry, else a load error, or globs such as `**/*.test.*` matched among the scanned files; a glob that cannot compile is a load error and one matching no scanned file is a finding) declares the test class: a `Verifies: ADR-12` comment in a file under it is test-tier evidence for the record it cites, and is checked for resolution like an `Implements:` one; a `Verifies:` comment elsewhere is not read (`MILE-146`). The optional `ignore_marker` option (`ADR-80`, the boundary rule's option reused) declares text that excuses the whole line it is on, shared with `embodiment.back-pointer-decision-accepted` and the evidence collector (two different values are a load error; a marker empty or inside `Implements:` or `Verifies:` is refused): a back-pointer on a line carrying it is neither reported here nor counted as evidence, the count of excused lines is disclosed in the population's `excused_lines` and in an `info` notice (also from `fix` and `reverify`), and a marker on a line with no back-pointer this rule reads, such as a `Verifies:` line outside `test_paths`, is itself a finding, reported once. Undeclared, nothing changes.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is source files, not records"),
        options: &["back_pointer_paths", "test_paths", "ignore_marker"],
        population: PopulationUnit::File,
        actions: &[
            VariantAction::new("back-pointer-unresolved", ActionDecl::Exempt("the edit is to a source file comment, and no action kind addresses a non-record file yet")),
            VariantAction::new("back-pointer-uncovered-path", ActionDecl::CONFIG_SET),
            VariantAction::new("back-pointer-uncovered-test-path", ActionDecl::CONFIG_SET),
            VariantAction::new("back-pointer-marker-unused", ActionDecl::Exempt("the edit removes a marker from a source file comment, and no action kind addresses a non-record file yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
                let (files, paths, tests): (&[_], &[String], &[String]) =
                    match c.back_pointer_sources {
                        Some(s) => (&s.files, &s.back_pointer_paths, &s.test_paths),
                        None => (&[], &[], &[]),
                    };
                embodiment_back_pointer_resolution(
                    files,
                    paths,
                    tests,
                    c.config,
                    c.config_path,
                    c.index,
                )
            },
    },
    RuleDef {
        id: RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
        description: "A source back-pointer (`Implements: ADR-12`, read exactly as `embodiment.back-pointer-resolution` reads it) citing a record whose `Status` is not one at which implementation is permitted is an error: code that says it implements a decision is landing ahead of that decision (`MILE-133`). Permitted means the cited type's declared `statuses.implementable` (a terminal `success` status is not assumed to permit it); a type that declares none uses the rule's `accepted_statuses`, and with neither the status cannot be judged, which is reported rather than passed. Needs `back_pointer_paths`; `accepted_statuses` is optional. On a `--base` run only back-pointers in files the diff changed are judged (file granularity). A citation of no record is `embodiment.back-pointer-resolution`'s, not this rule's. A declared path covering no tracked file is itself a finding. Mechanical fixes carry no back-pointer and are outside it. It checks the cited record's status, never whether the code matches the decision. The optional `ignore_marker` option (`ADR-80`) declares the text that excuses the whole line it is on, the one marker shared with `embodiment.back-pointer-resolution`, so a back-pointer there is not judged; excused lines are counted in the population's `excused_lines`, and an unused marker is reported by the resolution rule while it is enabled, else here. Undeclared, nothing changes.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is source files, not records"),
        options: &["back_pointer_paths", "accepted_statuses", "ignore_marker"],
        population: PopulationUnit::File,
        actions: &[
            VariantAction::new("decision-not-implementable", ActionDecl::Exempt("the edit is to a source file comment or to the cited record, and no action kind addresses a non-record file yet")),
            VariantAction::new("decision-unjudgeable", ActionDecl::Exempt("the edit is to a type or rule declaration that depends on which statuses permit implementation, a policy choice with no single target")),
            VariantAction::new("decision-uncovered-path", ActionDecl::CONFIG_SET),
            VariantAction::new("decision-marker-unused", ActionDecl::Exempt("the edit removes a marker from a source file comment, and no action kind addresses a non-record file yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
                let (files, paths): (&[_], &[String]) = match c.decision_gate_sources {
                    Some(s) => (&s.files, &s.back_pointer_paths),
                    None => (&[], &[]),
                };
                let fallback = c.config
                    .rule_opts::<DecisionAcceptedOpts>(RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED)
                    .accepted_statuses;
                embodiment_back_pointer_decision_accepted(
                    files,
                    paths,
                    &fallback,
                    c.changed_paths,
                    c.config,
                    c.config_path,
                    c.index,
                )
            },
    },
    RuleDef {
        id: RULE_CODE_DANGLING_BUG_REFERENCE,
        description: "A source comment that cites a record id (a declared type prefix, a hyphen and a number, standing alone: `BUG-12`, `ADR-3`) which resolves to no record is an error, the mirror of `embodiment.locator-exists` (`MILE-108`, `RFC-41`): a locator names a file and this names a record from the file's side. Only existence is judged, never currency, so a citation of a superseded or retired record is not reported; ids are compared numerically. Opt-in, and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs) and `extensions` (file extensions without the dot, such as `rs` or `sh`; both are declared, and the engine compiles in no language): only a tracked file under a path with one of those extensions is read. A comment is read as `embodiment.back-pointer-resolution` reads one by default: a leader (`//`, `///`, `//!`, `#`, `--`, `;`, `/*`, `*`, `<!--`) at the start of a line, or `//`, `#`, `--`, `/*`, `<!--` after whitespace following code, and every id in the rest of the comment counts. The optional `code_comment_prefixes` option declares the leaders per extension instead (a map such as `rs: [\"//\"]`, `sh: [\"#\"]`): a leader opening a line or following whitespace starts a comment, the longest leader at a position wins, an extension with no entry reads the built-in set, and a key that is not one of `extensions`, an empty list, or a blank, whitespace-holding or repeated leader is a load error. The engine reads no syntax beyond a leader, so an id in a string literal or a doc-comment code example after a leader is read, and the declared `ignore_marker` excuses it. An id run into a longer token (`ADR-12-title.md`) is a name, a word holding `://` is a URL, neither is a citation, and prose outside a comment is not read. A citation `embodiment.back-pointer-resolution` judges (an `Implements:` back-pointer, or a `Verifies:` one in its test class) is left to that rule while it is enabled over the file, so one line is not reported twice. The optional `external_ids` option (`citation.resolution`'s) lists ids that share a record prefix but are not records (`RFC-4122`, or a range written `PREFIX-LOW..HIGH`) and are not judged. The optional `ignore_marker` option (`ADR-80`, the boundary rule's option reused, read by this rule alone) declares text that excuses the whole line it is on: the line's citations are neither reported nor counted, the count is disclosed in the population's `excused_lines` and in an `info` notice, and a marker on a line that cites no record is itself a finding (a marker of fewer than three characters, with no letter or digit, or that is or ends a record id is refused at load). A declared path covering no tracked file with a declared extension is itself a finding, and a non-text file is counted as unreadable, never as clean. The population is the source files read, so a run over none is distinguishable from a clean one. Undeclared, nothing is read and nothing changes.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is source files, not records"),
        options: &["back_pointer_paths", "extensions", "code_comment_prefixes", "external_ids", "ignore_marker"],
        population: PopulationUnit::File,
        actions: &[
            VariantAction::new("dangling-reference", ActionDecl::Exempt("the edit is to a source file comment, and no action kind addresses a non-record file yet")),
            VariantAction::new("dangling-reference-uncovered-path", ActionDecl::CONFIG_SET),
            VariantAction::new("dangling-reference-marker-unused", ActionDecl::Exempt("the edit removes a marker from a source file comment, and no action kind addresses a non-record file yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
                let (files, paths, extensions): (&[_], &[String], Vec<String>) =
                    match c.code_citation_sources {
                        Some(s) => (
                            &s.files,
                            &s.back_pointer_paths,
                            c.config
                                .rules
                                .get(RULE_CODE_DANGLING_BUG_REFERENCE)
                                .and_then(|r| r.extensions.clone())
                                .unwrap_or_default(),
                        ),
                        None => (&[], &[], Vec::new()),
                    };
                code_dangling_bug_reference(
                    files,
                    paths,
                    &extensions,
                    c.config,
                    c.config_path,
                    c.index,
                )
            },
    },
    RuleDef {
        id: RULE_EMBODIMENT_LOCATOR_EXISTS,
        description: "A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, is empty, does not parse, or carries a scope (`#symbol` or `#L<a>-L<b>`) that does not resolve in the file, a scope without a pin, or a commit anchor (not yet supported). Tracked alone passes a staged deletion; on disk alone passes a gitignored file. A locator that resolves outside the repository (absolute, `..` or a link out) is never probed on disk, so it reads as not in the working tree, and a Done-means item that carries a valid waiver is not examined (the waiver holds while its evidence does not resolve). A locator naming a file that exists on disk but is not tracked by git is a separate variant from one naming nothing (`BUG-13`), because the repair is to track the file. A comma separates `Realized-by` entries only when a category prefix (`spec:`, `code:`, `test:`) follows it; any other comma is part of the path (`BUG-129`), and a path that itself contains `,<category>:` is read as two entries. The `locator_kinds` option (`file`, `directory`, `glob`; default `[file]`) declares which kinds a locator may be: a directory or glob is legal when it matches at least one tracked file present on disk, and one of a kind not declared is reported naming the option. The `pinnable` option (default `[file]`, each kind also in `locator_kinds`) declares which kinds may carry a `@sha256:` pin (`BUG-176`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["locator_kinds", "pinnable"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("locator-empty", ActionDecl::FIELD_SET),
            VariantAction::new("locator-missing", ActionDecl::FIELD_SET),
            VariantAction::new("locator-untracked", ActionDecl::Exempt("the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration")),
            VariantAction::new("locator-malformed", ActionDecl::FIELD_SET),
            VariantAction::new("locator-scope-unresolved", ActionDecl::FIELD_SET),
            VariantAction::new("locator-scope-unpinned", ActionDecl::FIELD_SET),
            VariantAction::new("locator-commit-anchor", ActionDecl::FIELD_SET),
            VariantAction::new("locator-kind-undeclared", ActionDecl::FIELD_SET),
            VariantAction::new("locator-kind-unpinnable", ActionDecl::FIELD_SET),
            // A locator in a Done-means item clause (`ADR-96`), whichever of
            // the defects above it has: the repair is an edit of the item.
            VariantAction::new("item-locator-defect", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            // Tracked *and* on disk: tracked alone passes a staged deletion,
            // since discovery unions `ls-files` with the cached diff, and on
            // disk alone passes a gitignored file that then fails in CI.
            let tracked: std::collections::HashSet<&std::path::Path> =
                c.tracked_paths.iter().map(|p| p.as_path()).collect();
            let present = |p: &str| {
                let path = std::path::Path::new(p);
                if !tracked.contains(path) {
                    return false;
                }
                // `is_file()` folds every error into `false`, so an unreadable
                // path would be reported as missing. This rule is declared
                // `error` in repositories that enable it, and a blocking
                // finding the tool cannot substantiate is worse than silence --
                // so only a definite absence counts.
                (c.may_exist)(path)
            };
            // Real, but not tracked: the repair is `git add`, not a new locator.
            let untracked_on_disk = |p: &str| {
                let path = std::path::Path::new(p);
                !tracked.contains(path) && (c.may_exist)(path)
            };
            embodiment_locator_exists_on(
                c.records,
                c.config,
                c.tracked_paths,
                &present,
                c.read_text,
                &untracked_on_disk,
                c.run.today,
            )
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
        description: "The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18). The optional `scope: changed` option (`ADR-90`) makes it a nudge for the moment someone touches a record that shares a locator: a locator is reported only when a record a `--base` diff adds or changes cites it, with the finding on that changed record and every citer (changed or not) named in the message; none on a plain `check`, and the slots it did not judge are counted in the population's `deferred_by_scope`, with one `info` notice saying why.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::AnchorScoped,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["scope"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("locator-promotion-candidate", ActionDecl::Exempt("an advisory nudge that is never auto-promoted")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            embodiment_locator_promotion_candidate(
                c.records,
                c.config,
                changed_only(
                    c.config,
                    RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
                    c.changed_paths,
                )
                .as_ref(),
            )
        },
    },
    RuleDef {
        id: RULE_HEADER_FIELD_SET_CONSISTENCY,
        description: "Every header field belongs to its type's `required_fields` \u{222a} `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding. The optional `extension_prefixes` option (`MILE-141`) maps a record type to its own prefix, used for that type in place of `x-`; a type it does not name keeps the rule-level behaviour. A prefix is non-empty, holds no whitespace and does not start a field the type declares.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::KnownFields),
        header_none: HeaderNone::OutOfScope,
        options: &["allow_extension_prefix", "extension_prefixes"],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("undeclared-field", ActionDecl::FIELD_REMOVE),
            VariantAction::new("undeclared-field-case", ActionDecl::FIELD_RENAME),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            header_field_set_consistency(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_FIELD_UNTRIMMED_VALUE,
        description: "A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("untrimmed-value", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            field_untrimmed_value(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_FIELD_LEADING_RESERVED_INDICATOR,
        description: "A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("leading-reserved-indicator", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| field_leading_reserved_indicator(c.records, c.config),
    },
    RuleDef {
        id: RULE_HEADER_FIELD_CASE_MISMATCH,
        description: "A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("field-case-mismatch", ActionDecl::FIELD_RENAME),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            header_field_case_mismatch(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_HEADER_CASE_VARIANT_KEY,
        description: "One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("case-variant-key", ActionDecl::Exempt("the rule reads the keys a record wrote, not a declared vocabulary, so it cannot tell the declared spelling from the stray one and a remove could delete the declared key")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            header_case_variant_key(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_NARRATIVE_FIELD_STALE,
        description: "A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["terminal_statuses"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("narrative-field-stale", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: NarrativeStaleOpts = c.config.rule_opts(RULE_NARRATIVE_FIELD_STALE);
            narrative_field_stale(c.records, c.config, &opts.terminal_statuses, c.index)
        },
    },
    RuleDef {
        id: RULE_TYPE_DIR_MATCHES_NOTHING,
        description: "A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. Also reports a type\u{2019}s `exclude_files` entry that names no tracked file directly in its `dir` (`MILE-150`), so a stale exclusion is visible. Reports each `extra_dirs` entry that does not exist (`MILE-130`). Reports each status a type's `sections.exempt_statuses` lists that none of the type's records holds (`MILE-7`), so an exemption that sets nothing aside is visible rather than silent.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("exclude-files-matches-nothing", ActionDecl::CONFIG_SET),
            VariantAction::new("extra-dir-missing", ActionDecl::CONFIG_SET),
            VariantAction::new("dir-missing", ActionDecl::CONFIG_SET),
            VariantAction::new("exempt-status-matches-nothing", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let mut matched: HashMap<String, usize> = HashMap::new();
            for r in c.records {
                *matched.entry(r.record_type.clone()).or_insert(0) += 1;
            }
            let dir_exists = |d: &str| (c.dir_exists)(d);
            let (execution, mut findings) = type_dir_matches_nothing(
                c.config,
                c.config_path,
                &matched,
                &dir_exists,
                c.live_paths,
            );
            findings.extend(section_exemptions_matching_nothing(
                c.records,
                c.config,
                c.config_path,
            ));
            (execution, findings)
        },
    },
    RuleDef {
        id: RULE_IDENTITY_COLLISION,
        description: "Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads the filename identifier only"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("identifier-collision", ActionDecl::Exempt("the fix is a renumber, and no action kind addresses a rename yet (`urzua migrate ids` exists)")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            identity_collision(c.records, c.collisions)
        },
    },
    RuleDef {
        id: RULE_IDENTITY_OPEN_PR_COLLISION,
        description: "A record a `--base` diff adds shares its type and number with a file on a fetched pull-request head ref under a different `Stable-Id` (`ADR-93`, `MILE-89`): two branches filed the same number from one `main`, and each looks valid alone. The refs are the ones `urzua new` fetches into `refs/urzua/pr/<segment>/*` with `new.check_open_prs`; `check` never fetches, so with none present the rule notices how to fetch them and its population counts zero refs, never a clean run over nothing. A head whose file for that id carries the record's own `Stable-Id` is the same record and is not a collision, so a pull request is not flagged against itself nor against its own merged copy. Plain git cannot tell an open pull request from a closed one, so a closed, unmerged head can still hold a number a new record legitimately took: the finding is a warning, which still gates under `fail_on: warning`, and the path out is a waiver whose `Reason` names that pull request, or renaming the file and its H1 to the next free number (the one `urzua new` would take; `urzua migrate ids` cannot renumber). Judges the changed set: `not-applicable` on a run with no `--base`. Opt-in.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::ChangedSet,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads the filename identifier and the Stable-Id, only on a --base run"),
        options: &[],
        population: PopulationUnit::PullRef,
        actions: &[
            VariantAction::new("open-pr-collision", ActionDecl::Exempt("the fix is to rename the file and its H1 to the next free number, or a waiver naming a closed pull request; neither is a field edit")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| open_pr_collision(c.records, c.added_paths, c.pull_ref_claims),
    },
    RuleDef {
        id: RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
        description: "A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise.",
        scope: Scope::Files,
        anchor: Anchor::ChangedPath,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges tracked paths against the declared directories"),
        options: &[],
        population: PopulationUnit::Path,
        actions: &[
            VariantAction::new("record-outside-declared-dir", ActionDecl::Exempt("the fix is a move, and no action kind addresses a move yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
                // A staged deletion is going away, not a real ownership
                // question to raise (`BUG-71`/`BUG-82`'s shape, one level up:
                // those fixed a staged deletion being wrongly judged *inside*
                // its dir; this is the same file wrongly judged *outside* one
                // it's leaving).
                type_record_outside_declared_dir(c.config, c.live_paths)
            },
    },
    RuleDef {
        id: RULE_TYPE_NO_DECLARED_SPEC,
        description: "Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("no-declared-spec", ActionDecl::Exempt("a signal to review, never a mandate")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            type_no_declared_spec(c.config, c.config_path)
        },
    },
    RuleDef {
        id: RULE_HEADER_POINTER_FIELD_CLEAN,
        description: "A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::PointerFields),
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("pointer-field-not-clean", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            header_pointer_field_clean(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CONFIG_COMPANION_DECLARATION_MISSING,
        description: "A type's `companion_fields` lists pairs of relation roles, for example `[embodiment_state, embodiment_locator]`, whose fields it must declare together (`BUG-41`). A type declaring one field of a pair in `required_fields`/`known_fields` without the other is reported, as is a pair whose fields it declares neither of: a rule reading both, such as `embodiment.consistency`, cannot judge a record that lacks one. The fields are the ones the type's `relation_fields` names for the roles; no pair is known to the engine.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("companion-field-missing", ActionDecl::CONFIG_APPEND),
            VariantAction::new("companion-pair-undeclared", ActionDecl::CONFIG_APPEND),
            VariantAction::new("companion-field-missing-declare-set", ActionDecl::CONFIG_SET),
            VariantAction::new("companion-pair-undeclared-declare-set", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_companion_declaration_missing(c.config, c.config_path, c.records),
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_DECLARATION_MISSING,
        description: "A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("pointer-declaration-missing", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_pointer_declaration_missing(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("pointer-field-not-known", ActionDecl::CONFIG_APPEND),
            VariantAction::new("pointer-field-not-known-declare-set", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            config_pointer_field_not_known(c.config, c.config_path, c.records)
        },
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
        description: "A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("pointer-narrative-overlap", ActionDecl::Exempt("a field named in both lists could belong in either, and which list should keep it is the author's decision, not a computable repair")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_pointer_narrative_overlap(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
        description: "A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("header-none-field", ActionDecl::CONFIG_REMOVE),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_header_none_has_no_required_fields(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`), including the `date` and `author` fields `urzua new` fills (`MILE-176`), must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("relation-field-not-known", ActionDecl::CONFIG_APPEND),
            VariantAction::new("relation-field-not-known-declare-set", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            config_relation_field_not_known(c.config, c.config_path, c.records)
        },
    },
    RuleDef {
        id: RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
        description: "A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s \"declared, not voted\" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("known-fields-declaration-missing", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_known_fields_declaration_missing(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_SCHEMA_VERSION_OUTDATED,
        description: "The configuration file is at the previous `schema_version` and was read by upgrading it in memory, so no rule saw the old shape (`ADR-77`, `SPEC-3`'s \"Supported schema versions\"). One finding on the config file naming the version found, the current version and the command that rewrites the file, `urzua migrate config`, plus the keys the upgrade dropped. A config at the current version is examined and clean. Opt-in like every rule, and each upgrade step enables it, so a config upgraded once is reported by the next release that moves the version again; under `fail_on: warning` it fails a strict repository until the file is migrated.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::ConfigFile,
        actions: &[
            VariantAction::new("schema-version-outdated", ActionDecl::RUN_COMMAND),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| config_schema_version_outdated(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_FILENAME_IDENTITY_PATTERN,
        description: "A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined, and omits a file the type declares in `exclude_files` (`MILE-150`).",
        scope: Scope::Records,
        anchor: Anchor::Directory,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads the filename only"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("identity-pattern-none-match", ActionDecl::Exempt("the fix is a rename, and no action kind addresses a rename yet")),
            VariantAction::new("identity-pattern-mismatch", ActionDecl::Exempt("the fix is a rename, and no action kind addresses a rename yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            filename_identity_pattern(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_UNDECLARED_VALUE,
        description: "A record's `Status` is not one of the values its type declares in `statuses.values` (`MILE-132`). A type declaring no `statuses` is not examined, and a record with no `Status` is left to `header.required-fields`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("status-undeclared-value", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_undeclared_value(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_REQUIRED_FIELD_MISSING,
        description: "A field the type's `statuses.require` ties to the record's current `Status` is blank, unedited template text, or (when the type declares `statuses.pending`) one of that declared placeholder vocabulary's whole values such as `Unknown` or `none recorded`, or a value matching one of its `{pattern: ...}` entries for a placeholder family (`MILE-132`, `MILE-153`, `MILE-162`), for example `Deciders` on an `Accepted` record. A type declaring no `pending` keeps the old reading, so a value that only contains the word is still a real value. A type declaring no `require` entries is not examined. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["field_levels"],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("status-required-field-missing", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_required_field_missing(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
        description: "A field the type's `statuses.forbid` bars at the record's current `Status` carries a value (`MILE-132`), for example an acceptance date on a record that is not accepted. A type declaring no `forbid` entries is not examined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("status-forbidden-field-present", ActionDecl::FIELD_REMOVE),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_forbidden_field_present(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_TRANSITION_ALLOWED,
        description: "A changed record's `Status` moved along an edge its type's `statuses.transitions` does not list, comparing the status the merge base committed with the one in the change under check, or a record new in the change starts at a status outside `statuses.initial` (`MILE-144`, `ADR-72`). Needs `--base`: a whole-repo run reports it not-applicable, not a pass. A type declaring no `transitions` is not examined, a record the diff did not change is eligible and not examined, and a record whose committed status cannot be read or is no longer declared is counted unreadable. A path the merge base lacks reads as new; a base copy that exists but cannot be read (not UTF-8, a missing object) is not new: it is counted unreadable and a warning notice names the error, so a read failure cannot skip the transition check. A record added at a path the merge base lacks is matched by `Stable-Id` and the same record id (type and number) to a record the diff deleted, so a slug-renamed record is judged against the status it had before and not called new (`BUG-191`), while a renumbered one, or a new record that only copies a deleted record's `Stable-Id`, is new; one with no `Stable-Id` or no match reads as new, and one matching several deleted records is counted unreadable. A repository whose ADRs and RFCs enter at `Proposed` or `Draft` and are accepted in a later change declares that as `initial`, so a record filed already accepted is reported; this repository does so.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::ChangedSet,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("transition-not-allowed", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_transition_allowed(c.records, c.config, c.committed_statuses)
        },
    },
    RuleDef {
        id: RULE_STATUS_UNREACHABLE,
        description: "A status a type declares in `statuses.values` that no path of its `statuses.transitions` reaches from `statuses.initial` (`MILE-144`, `ADR-72`): no record can legitimately hold it. Judges the declaration, not a record, so it needs no `--base`. A type declaring no `transitions` is not examined. Enable it at `warn` for the informational reading.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        population: PopulationUnit::RecordType,
        actions: &[
            VariantAction::new("status-unreachable", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_unreachable(c.config, c.config_path)
        },
    },
    RuleDef {
        id: RULE_STATUS_DELIVERY_AGREEMENT,
        description: "A record's `Status` agrees with its delivery evidence, in both directions (`MILE-183`): the tier computed from its `Realized-by` locators and source back-pointers (drift ignored) is one of the type's `statuses.delivery.delivered_tiers` while the `Status` is one of its `not_done` statuses, so a built record still reads as unbuilt; and a `success` status whose evidence computes to one of its `unevidenced_tiers`, so a record claims to be done with nothing behind it. The mapping is the type's own declaration (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`); the rule knows no status or type by name. `urzua fix --apply` writes `set_status` for the first direction; the second has no single correct value and is repaired by adding evidence. `unevidenced_tiers` may be empty, and the type then never reports the second direction. `back_pointer_evidence: false` (default true) makes a source or test back-pointer that cites the record not count as evidence, so only the type's evidence field is read (`ADR-84`). The rule applies to any type that declares `delivery`, whatever the type is called. A type declaring no `delivery` is not examined. The population counts the records of types that declare it.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("delivery-not-done", ActionDecl::RUN_COMMAND),
            VariantAction::new("delivery-without-evidence", ActionDecl::Exempt("no single correct value exists and `urzua fix` writes nothing for it; the repair is adding evidence, which may be a locator or a source back-pointer")),
            VariantAction::new("delivery-partial-not-started", ActionDecl::RUN_COMMAND),
            VariantAction::new("delivery-partial-blocked", ActionDecl::Exempt("the record is derived-blocked with an unresolved item, and `urzua fix` writes no `progress` `partial_status` to it because `sequence.status-agreement` would refuse it; the repair is finishing the open dependency")),
            // `DoneWithUnresolvedItems` (`ADR-96`): a guaranteed error. Its
            // severity is a floor no setting lowers and no rule-level waiver
            // matches. It offers two actions, the reopen first and then the
            // evidence; a finding carries one `suggested_action` (`ADR-73`), so
            // it carries the reopen, or the evidence edit when the type's `transitions` do not allow the reopen, and `urzua rules` lists both.
            VariantAction::new("done-with-unresolved-items", ActionDecl::FIELD_SET)
                .floored(FindingSeverity::Error)
                .unwaivable(),
            VariantAction::new("done-with-unresolved-items-evidence", ActionDecl::SECTION_SET)
                .floored(FindingSeverity::Error)
                .unwaivable(),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            with_locator_env(c, |env| status_delivery_agreement(c.records, c.config, c.back_pointed, env))
        },
    },
    RuleDef {
        id: RULE_STATUS_DONE_ITEMS_SHAPE,
        description: "A Done-means section that does not follow the item grammar of `ADR-96` section 1, on a type that declares `statuses.delivery.items`: a prose-only section (no list item), a list marker indented less than the item indent, a clause on a line that belongs to no item, clauses on the first line and on final lines at once, a waiver clause before the evidence clause, an evidence clause that is not entirely locators (backticks or quotes before the first `category:`, whitespace inside a locator, no locator at all), a leading `[x]` or `[ ]` checkbox, and a waiver clause with an unknown category, an empty or missing reason, a missing, malformed or unneeded cite or date. A clause that does not parse is a finding, never skipped, and its item reads as unresolved. A marker word with no clause separator before it is plain text. A `Kind` value outside the type's declared `delivery.items.kind` vocabulary is a finding naming the allowed kinds, and the record is then judged as one with no kind. An item whose text matches the type's declared `delivery.items.pending` vocabulary is an unfilled placeholder: a finding on the item's line, and the item never resolves and cannot be waived. A type declaring no `delivery.items` is not examined. The population counts the records of types that declare it.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::DeliveryItems),
        header_none: HeaderNone::Exempt("judges only a type that declares `statuses.delivery.items`, which needs a `Status` header a `header_shape: none` type does not have"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("done-items-shape", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            status_done_items_shape(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_WAIVED_ITEM_STALE,
        description: "A waived Done-means item whose waiver has outlived its reason (`ADR-96` section 3), on a type that declares `statuses.delivery.items`: the item also carries an evidence clause that now resolves (remove the waiver), the record its waiver cites does not exist, a `deferred-to` (a category declared `expires: required`) whose cited record is finished, or an expiry date that has come (from that date on). Every waived item is also listed by category in the report's notices, never omitted. A dated `WAIVER` record can waive this rule like any warning. A type declaring no `delivery.items` is not examined. The population counts the records of types that declare it.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: true,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: Some(Declaration::DeliveryItems),
        header_none: HeaderNone::Exempt("judges only a type that declares `statuses.delivery.items`, which needs a `Status` header a `header_shape: none` type does not have"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("waived-item-stale", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            with_locator_env(c, |env| status_waived_item_stale(c.records, c.config, c.index, env, c.run.today))
        },
    },
    RuleDef {
        id: RULE_DATE_FIELD_VALID,
        description: "A value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date, or is well-formed but not a real calendar date such as `2026-02-30`, is an error (`MILE-137`). A blank, placeholder or pending value is not a date value and is left to `field.quality` and `status.required-field-missing`. Needs `date_fields`. The population counts the date values examined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["date_fields"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("date-invalid", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            date_field_valid(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_DATE_FIELD_ORDER,
        description: "Within one record, a date field that is later than the field `date_order` says must not be later is an error: `Date <= Accepted-on` reports a record accepted before it was written (`MILE-157`). Equal dates pass. A value that is blank, a placeholder, pending or not a real `YYYY-MM-DD` date is not compared and is left to `date.field-valid`, `field.quality` and `status.required-field-missing`, which own those states. Applies to every record type that declares both fields of an entry. Needs `date_order`. The population counts one slot per record and entry; a record missing either date is eligible and unexamined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["date_order"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("date-order", ActionDecl::FIELD_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            date_field_order(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CITATION_RESOLUTION,
        description: "A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in. The optional `external_ids` option (`MILE-156`) lists ids that share a record prefix but are not records, each an exact id (`RFC-4122`) or an inclusive numeric range written `PREFIX-LOW..HIGH` under a declared prefix; a matching citation is counted as eligible and unexamined, never resolved or reported, and not as out of scope, so `config.scope-matches-nothing` does not read a declared id as a misconfiguration. An entry under no declared prefix is a config error. Its population counts the citations, so a run over none is distinguishable from a clean one, and external citations are visible as eligible and unexamined. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that cites an id as it was is not reported; it is `false` by default (`MILE-156`). The optional `non_reference_field` option (`BUG-190`, `ADR-75`) names a header field in which a record lists, as comma-separated exact ids (`ADR-72`; no ranges, compared by value), the ids it names without citing as a reference: a citation of a listed id in that record only is counted as eligible and unexamined, neither resolved nor reported, with `external_ids` taking precedence. A listed id the record never cites in the prose the rule reads, and an entry that is not an exact id of a declared prefix, are findings on the record naming the field and its line. With the option set, those declared entries are judged in a population of their own (`MILE-186`, `ADR-75` clause 3): a second `rules_executed` entry for this rule whose unit is `field-entry`, with one candidate per entry a record declares (valid, uncited and malformed alike, all examined, none out of scope), and zero eligible when no record declares any; the uncited and malformed findings are counted there and not in the citation population. The field must be listed in a record type's `known_fields` and must not be a pointer, narrative or relation-role field, or the config does not load. `check` prints one aggregated Info notice naming declared ids that now match a real record; a notice never changes the exit code.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::CrossRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["external_ids", "non_reference_field", "skip_revision_log"],
        population: PopulationUnit::Citation,
        actions: &[
            VariantAction::new("citation-unresolved", ActionDecl::Exempt("the fix edits prose located by line, and no action kind addresses a line in prose yet")),
            VariantAction::new("non-reference-malformed", ActionDecl::FIELD_SET),
            VariantAction::new("non-reference-uncited", ActionDecl::FIELD_REMOVE),
        ],
        phase: Phase::Main,
        companion: Some((
            PopulationUnit::FieldEntry,
            |c| citation_non_reference(c.records, c.config, c.full_text),
        )),
        run: |c| {
            citation_resolution(c.records, c.config, c.full_text, c.index)
        },
    },
    RuleDef {
        id: RULE_CITATION_PATH_EXISTS,
        description: "A repository path cited in backticks inside a `reference_sections` section (a `##` heading, matched exactly) exists in the working tree, tracked and on disk (`MILE-143`). A cited path is a code span with no whitespace, containing a `/` and ending in a file name with an extension or in `/`; a URL, a host-led path (`example.com/x`), an absolute or `~` path, a glob or placeholder (`*?<>{}$|`) and anything else is not a path and is not read. A trailing `:line` or `#anchor` is dropped, and `./` or `../` resolves from the record's directory. Fenced code blocks are not read. Needs `reference_sections`; a record without such a section is out of scope, and the population counts the paths cited. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that names a path as it was is not reported. It is `false` by default. Offline and deterministic. Cited issue and pull request numbers are `citation.issue-exists`'s.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["reference_sections", "skip_revision_log"],
        population: PopulationUnit::Citation,
        actions: &[
            VariantAction::new("path-missing", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            // A file is present when tracked and on disk (as for
            // `embodiment.locator-exists`); a directory when a tracked file
            // sits below it.
            let tracked: std::collections::HashSet<&std::path::Path> =
                c.tracked_paths.iter().map(|p| p.as_path()).collect();
            let directories: std::collections::HashSet<&std::path::Path> = c
                .tracked_paths
                .iter()
                .flat_map(|p| p.ancestors().skip(1))
                .filter(|a| !a.as_os_str().is_empty())
                .collect();
            let present = |p: &str| {
                let path = std::path::Path::new(p);
                if directories.contains(path) {
                    return true;
                }
                // Only a definite absence counts, as for `embodiment.locator-exists`.
                tracked.contains(path) && (c.may_exist)(path)
            };
            citation_path_exists(c.records, c.config, c.full_text, &present)
        },
    },
    RuleDef {
        id: RULE_CITATION_ISSUE_EXISTS,
        description: "An issue or pull request cited inside a `reference_sections` section (a `##` heading, matched exactly) is in the committed snapshot of the repository's numbers (`MILE-143`). `issue_refs` declares the `snapshot` file (relative to the repository root), the `repository` (`owner/name`) it describes and the citation `forms` read: `number` is `#123` (outside inline code, not `owner/repo#123`, not an anchor link `](#123)`), `url` is a `https://github.com/owner/name/issues/123` or `/pull/123` link to that repository; a form not declared is not read. A number is judged against the snapshot's issues and pull requests together, since GitHub shares one number space and redirects either URL. Offline and deterministic: `check` never reaches the network, and only `urzua refs refresh` writes the snapshot. A snapshot that is absent, unreadable, of another schema or of another repository is one finding on the snapshot path, and every citation is counted as eligible and unexamined: the rule never passes for want of its input. Each finding names the date the snapshot was generated, and `check` prints a notice with that date and the snapshot's size, so staleness is visible; the notice is printed only when this rule is part of the run (not under a `--rules` selection that leaves it out). The optional `skip_revision_log` option behaves as for `citation.path-exists`. The optional `issue_refs.fields` names header fields (as a type declares them, such as `Found-in`) whose values are read for the same forms, each finding naming the field and its line; a name no record type declares is refused at load, and a record without the field is not examined. Body prose outside `reference_sections` is deliberately not readable. The population's unit is the citation.",
        scope: Scope::Records,
        judges: Judges::OneRecord,
        anchor: Anchor::RecordOrDeclaredFile(|config| {
            config
                .rules
                .get(RULE_CITATION_ISSUE_EXISTS)
                .filter(|s| s.level != crate::config::RuleLevel::Off)
                .and_then(|s| s.issue_refs.as_ref())
                .map(|r| PathBuf::from(&r.snapshot))
        }),
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["reference_sections", "skip_revision_log", "issue_refs"],
        population: PopulationUnit::Citation,
        actions: &[
            VariantAction::new("issue-missing-in-section", ActionDecl::SECTION_SET),
            VariantAction::new("issue-missing-in-field", ActionDecl::FIELD_SET),
            VariantAction::new("snapshot-unusable", ActionDecl::Exempt("regenerating the snapshot with `urzua refs refresh` needs the network and a token, so the command is named in the message and not offered as an action")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            citation_issue_exists(c.records, c.config, c.full_text, c.refs_snapshot)
        },
    },
    RuleDef {
        id: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        description: "Declared boundaries (`boundaries`: path globs with a `kind` of `published`, `vendored` or `generated`, a required `reason` and a `severity`; `published_paths` is read as one published boundary with no reason required, and is not deprecated) and what follows from each kind (`RFC-11`/`ADR-9`/`ADR-80`, `MILE-125`). A record identifier (any identifier carrying a configured type prefix, resolvable or not, in prose as well as in a marker) inside a `published` boundary is a violation, because it publishes a pointer no external reader can resolve; a back-pointer (`Implements:` or `Verifies:`) inside a `generated` boundary is reported, and a plain record identifier there is not; a `vendored` boundary is declared and prohibits nothing. A boundary's `severity` is the level of its findings and the rule's `level` only enables the rule. A boundary's `exclude` globs remove paths from its violation check only, and the declared `ignore_marker` excuses the one line it is on. An `exclude` entry, a boundary path or an `ignore_marker` use that matches or suppresses nothing is itself a finding, never a clean pass. The violation matcher is wide and the evidence collector's stays narrow: a back-pointer inside a `published` boundary is not realization evidence. An identifier split across lines is not detected.",
        scope: Scope::Files,
        anchor: Anchor::Record,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is published files, not records"),
        options: &["published_paths", "boundaries", "ignore_marker"],
        population: PopulationUnit::File,
        actions: &[
            VariantAction::new("published-path-reference", ActionDecl::Exempt("the edit is to a published non-record file, and no action kind addresses a non-record file yet")),
            VariantAction::new("published-path-uncovered", ActionDecl::CONFIG_SET),
            VariantAction::new("generated-back-pointer", ActionDecl::Exempt("the edit is to a generated non-record file, which is overwritten, and no action kind addresses a non-record file yet")),
            VariantAction::new("boundary-uncovered", ActionDecl::CONFIG_SET),
            VariantAction::new("boundary-exclude-unused", ActionDecl::CONFIG_REMOVE),
            VariantAction::new("boundary-marker-unused", ActionDecl::Exempt("the edit removes a marker from a non-record file, and no action kind addresses a non-record file yet")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            published_path_reference(
                c.published_files,
                c.boundaries,
                c.ignore_marker,
                c.config,
                c.config_path,
            )
        },
    },
    RuleDef {
        id: RULE_SECTION_REQUIRED,
        description: "Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`), under its `name` or one of its declared `aliases` (`MILE-165`; `check` discloses each alias in use as an info notice), or, for a `y-statement` declaring `preamble: true` and a `marker`, in the text before the first `##`. A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted eligible and unexamined, not asked for the sections, and not out of scope, so `config.scope-matches-nothing` does not read a declared exemption as a misconfiguration (`BUG-187`). A required section may declare `level` and `under` to be a `###` (or deeper) heading nested under a named `##`, which needs the type's `sections.depth` to reach it (`MILE-98`). Presence only: it never judges what a section says.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body sections; the header only for exempt_statuses, which a header-less type declaring them is counted out of scope for (unit-tested)"),
        options: &[],
        population: PopulationUnit::Section,
        actions: &[
            VariantAction::new("section-missing", ActionDecl::SECTION_ADD),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            section_required(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_SECTION_SHAPE,
        description: "A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A section found under an alias is checked too, and a `preamble: true` Y-statement is judged from the text before the first `##`. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body sections, as section.required does"),
        options: &[],
        population: PopulationUnit::Section,
        actions: &[
            VariantAction::new("section-shape", ActionDecl::SECTION_SET),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            section_shape(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_SECTION_DUPLICATE,
        description: "A heading appears once among its siblings (`MILE-104`): the same heading text twice at one level under one parent is an error naming both lines, so two `## Open questions` in one record are caught while the same `### Pros` under two different options are not. Headings are compared as exact text, whitespace trimmed and case-sensitive (`## Context` and `## context` are different). Opt-in per type through `sections.unique_headings: true`; a type that does not declare it is out of scope, and a rule enabled with no declaring type is reported by `config.scope-matches-nothing`. The sections are read down to the type's `sections.depth`, and a heading inside a fenced code block is not one. A record's `exempt_statuses` do not set it aside: a duplicated section is a broken document whatever was decided. The population's unit is the record.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body sections only; the header is read for its extent"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("section-duplicate", ActionDecl::Exempt("merging or renaming one of two identical headings is an author's judgment about which text survives")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            section_duplicate(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_SECTION_LEVEL_SKIPPED,
        description: "A heading level is not skipped (`MILE-104`): a section nested more than one level deeper than the section above it, or deeper than `##` with no section above it (an `###` before any `##`), is an error. Defined against the type's `sections.depth` (`MILE-98`), which says where the section tree starts and how far it reads: a heading deeper than the depth is body text and is never read as a skip. Opt-in per type through `sections.no_skipped_levels: true`, which needs a `depth` of at least 2 (at depth 1 only `##` is a section and no level can be skipped; the config does not load otherwise). The population's unit is the record.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body sections only; the header is read for its extent"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("section-level-skipped", ActionDecl::Exempt("the heading to add or the level to lower is an author's judgment about the document's shape")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            section_level_skipped(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_SECTION_ORDER,
        description: "A record's sections follow the order its type's template gives them (`MILE-104`). Opt-in per type through `sections.order: {from: template}`, which reads the headings of `.urzua/templates/<type>.md` down to the type's `sections.depth`. Order is judged among siblings sharing a parent: a section the template places before one the record already showed is an error on that section's line, naming both. A section the template does not name is ignored and a template section the record lacks is not reported (`section.required` owns presence). Order matching is by heading text alone and ignores `sections.required` aliases. A declared order whose template is missing or holds no sections reads nothing: each record of the type is an error saying so and is counted eligible and unexamined, so a deleted template fails a run rather than switching the rule off. The population's unit is the record.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("reads body sections only; the header is read for its extent"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("section-order", ActionDecl::Exempt("moving a section is an author's edit; the template, not the tool, says where it belongs")),
            VariantAction::new("section-order-unreadable", ActionDecl::Exempt("restoring or filling the template is an author's edit; the tool does not invent a type's template")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            section_order(c.records, c.config, c.full_text, c.template_texts)
        },
    },
    RuleDef {
        id: RULE_INDEX_FRESHNESS,
        description: "A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}`, `{title}` and declared-field placeholders such as `{Status}`) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A `format` may span several lines (written with `\\n`), reading one entry over that many non-blank lines; it needs `{id}` or `{file}` (a link list that names no id is identified by its file); `{title}` is the H1 without its leading number or the record's own id (`MILE-152`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); `check` only compares, and `urzua index` regenerates the artifacts from the records. The population counts the artifacts compared, so a run over none is distinguishable from a clean one.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is index artifacts, not records"),
        options: &["indexes"],
        population: PopulationUnit::File,
        actions: &[
            VariantAction::new("index-stale", ActionDecl::RUN_COMMAND),
            VariantAction::new("index-unregenerable", ActionDecl::Exempt("`urzua index --apply` cannot regenerate this artifact (not text, a missing marker or an unparsable format), so the artifact or the declaration is edited by hand")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            index_freshness(c.records, c.full_text, c.index_artifacts)
        },
    },
    RuleDef {
        id: RULE_DIFF_SOURCE_REQUIRES_RECORD,
        description: "A `--base` diff changes a path under a declared `source_paths` entry (a prefix or a glob) but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`, `MILE-158`). A changed path matching `exclude_paths` is not source and stays in the population unexamined, so a diff touching only such files (tests beside the code) is not a trigger. One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed. Declaring `decision_types` (each record type that holds decisions, with the statuses at which one counts as decided) turns it into the decision gate (`MILE-133`): the diff must change a decision record at a decided status, a record whose `link_fields` entry (a pointer or narrative field such as `Implements`) names one, or a record of a `fix_types` type (a mechanical fix needs a bug record and no decision); a changed decision still at an undecided status does not release it and is named in the finding. A deleted record satisfies nothing. `fix_types` and `link_fields` need `decision_types`, and every type, status and field a declaration names must exist, or the config does not load. Off unless declared: without `decision_types`, any governed record releases the rule.",
        scope: Scope::Diff,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is changed paths, and only on a --base run"),
        options: &["source_paths", "exclude_paths", "decision_types", "fix_types", "link_fields"],
        population: PopulationUnit::ChangedPath,
        actions: &[
            VariantAction::new("source-without-record", ActionDecl::ADD_RECORD),
            VariantAction::new("source-without-decision", ActionDecl::ADD_RECORD),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: SourceRequiresRecordOpts =
                c.config.rule_opts(RULE_DIFF_SOURCE_REQUIRES_RECORD);
            source_requires_record(c.changed_paths, &opts, c.config, c.records, c.index)
        },
    },
    RuleDef {
        id: RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        description: "A `--base` diff changes both a governed record and a path under a declared `application_paths` entry, a prefix or a glob (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean.",
        scope: Scope::Diff,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is changed paths, and only on a --base run"),
        options: &["application_paths", "exclude_paths"],
        population: PopulationUnit::ChangedPath,
        actions: &[
            VariantAction::new("source-and-records-mixed", ActionDecl::Exempt("process advice, no file edit")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: SourceAndRecordsMixedOpts =
                c.config.rule_opts(RULE_DIFF_SOURCE_AND_RECORDS_MIXED);
            source_and_records_mixed(
                c.changed_paths,
                &opts.application_paths,
                &opts.exclude_paths,
                c.config,
            )
        },
    },
    RuleDef {
        id: RULE_DIFF_DECIDER_REQUIRED,
        description: "A `--base` diff moves a record to a status a gate of the top-level `authority` table names (`gates: [{type, to, by}]`, for example an ADR to `Accepted`), or changes a path a gate names (`gates: [{paths, by}]`), and the pull request's author, or a login whose approval is current for the head commit, is not in the gate's decider group (`ADR-89` section 4, `MILE-200`). The identities arrive as input and are never fetched (`check` does not use the network): `urzua check --actors FILE` reads a JSON file the gate action writes from the host API, `{\"schema\": 1, \"pr_author\": LOGIN, \"approvers\": [LOGIN, ...]}`, where an approval given before a later push is not listed. Which of the two counts is the table's `evidence` (`pr-author`, `pr-approver`, any of; `pr-author` unless declared, because a host bars an author from approving their own pull request and a single maintainer could then never satisfy `pr-approver`). A move is read against the merge base's copy of the record, the status-transition mechanism: a record new in the diff, and one whose base copy cannot be read, count as a move to the status they hold, and a record edited in place at a status it already held does not. One finding per gated record or path, naming the gate, the group and what the file says. **This rule proves authorship, not who decided.** With a single maintainer every pull request an agent opens under the maintainer's token is authored by a decider, so the rule passes and cannot tell an agent from the maintainer; it gives a recorded trail and, in a repository with a second human or a distinct agent identity, a check that means something. A move or change with no `--actors` file is reported as `info` on a local run (a pre-commit hook cannot know) and as an error at the rule's level under the option `require_evidence: true` (default `false`), so a missing input is never read as a pass. Which revision's table is read is the `authority.decider_table` option (`ADR-89`, amendment of 2026-11-09): `head` reads the checkout's table, as before, and is what an undeclared key means; `base` reads the groups, logins, gates and evidence from the configuration file at the merge base, so a pull request cannot add its author to a group or remove a gate in the change it asks to have approved; `both` judges each gated change under both tables. A base with no configuration or no `authority` is not read as no gates: the head table is judged instead, with an `info` notice. An unreadable base is a warning notice and the head table is used. Under `base` and `both` the rule's level and options and `fail_on` are the stricter of the base's and the head's, so a pull request cannot switch the rule off or lower it. Whether a waiver may silence the rule is the `authority.waivable` option (`ADR-89`, amendment of 2026-11-09, gate waivers): `any` (undeclared) lets any active waiver count, `base` lets only a waiver the base revision holds unchanged count (one the pull request adds or edits is ignored, with an `info` notice), `never` lets none count; the stricter of the base's and the head's applies, and the options are merged the same way (`require_evidence` is on when either side sets it). Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Opt-in, and needs the top-level `authority` declaration. The population counts changed paths.",
        scope: Scope::Diff,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: true,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is changed paths, and only on a --base run"),
        options: &["require_evidence"],
        population: PopulationUnit::ChangedPath,
        actions: &[
            VariantAction::new("gated-move-without-decider", ActionDecl::Exempt("accepting a record is a decider's act; no edit to the record supplies the pull request's author or approval")),
            VariantAction::new("gated-deletion-without-decider", ActionDecl::Exempt("deleting a gated record is a decider's act; no edit to the repository supplies the pull request's author or approval")),
            VariantAction::new("gated-path-without-decider", ActionDecl::Exempt("changing a protected path is a decider's act; no edit to the file supplies the pull request's author or approval")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts: DeciderRequiredOpts = c.config.rule_opts(RULE_DIFF_DECIDER_REQUIRED);
            decider_required(
                c.changed_paths,
                c.records,
                c.config,
                &BaseHistory {
                    committed: c.committed_statuses,
                    deleted: c.deleted_records,
                },
                c.actors,
                c.base_authority,
                &opts,
            )
        },
    },
    RuleDef {
        id: RULE_OVERRIDE_UNREVIEWED,
        description: "An editor-hook override that no one has reviewed is reported, on the committed override log the repository declares as `override_log` (`ADR-78`, `MILE-188`). The log is a Markdown table of `Date`, `Name`, `Record`, `Field`, `Rule`, `Reason` and `Reviewed-by`; the hook appends a row, with `Reviewed-by` blank, whenever a named override lets a blocked edit through, and the entry is reviewed once a person names themselves in `Reviewed-by` on its row in a later change; a blank cell, a placeholder (`TBD`, `-`) and a declared pending value are not a review. The finding names the override, the record, the rule and the reason, so it surfaces in the pull request that carries the log change. A row that is not an entry is a finding; a declared log that does not exist, or is not text, is reported and counted unreadable, never read as an empty log. Opt-in; needs the top-level `override_log` declaration. The population counts log entries.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: true,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("population is override-log entries, not records"),
        options: &[],
        population: PopulationUnit::Entry,
        actions: &[
            VariantAction::new("override-unreviewed", ActionDecl::Exempt("reviewing an override is a person's act, recorded in the log's `Reviewed-by` cell; no field of a record holds it")),
            VariantAction::new("override-log-missing", ActionDecl::Exempt("the log is not a record: creating it writes a file, which no action kind addresses (the hook creates it on the first override)")),
            VariantAction::new("override-log-malformed", ActionDecl::Exempt("the log is not a record: a malformed row is repaired by editing the log file, which no action kind addresses")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| override_unreviewed(c.override_log, c.config),
    },
    RuleDef {
        id: RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH,
        description: "A `Realized-by` locator names a file under `back_pointer_paths`, where an `Implements:` or `Verifies:` comment would say the same thing and is read automatically (`ADR-78` item 4, `MILE-189`, `RFC-7` section 5: prefer the lighter mechanism). A warning nudging the evidence into that comment, never an error (a locator whose file already carries the comment for the record is told to drop the locator, not to add the comment, and a record of a type declaring `statuses.delivery.back_pointer_evidence: false`, which counts no comment as evidence, is not nudged), because the two mechanisms are not interchangeable: only a `code:` locator on a file under `back_pointer_paths`, or a `test:` locator on a file under the optional `test_paths` (the only files whose `Verifies:` comment counts), has a comment of the same tier. A `spec:` locator, a `test:` locator outside `test_paths` and any locator with a `#scope` or an `@sha256` pin stay `Realized-by` and are not flagged. It offers no edit action, because the comment must exist before the locator goes (an exemption with a reason, as for the back-pointer rules). Opt-in, and needs `back_pointer_paths`; `test_paths` is optional and, as for the back-pointer rules, each entry must lie inside a `back_pointer_paths` entry. The population counts the declared `Realized-by` slots. The optional `scope: changed` option (`ADR-90`, `MILE-203`) makes it a nudge for the moment someone touches a record: it then judges only the records a `--base` diff adds or changes (not their dependents), none on a plain `check`, and the slots it did not judge are counted in the population's `deferred_by_scope`, with one `info` notice saying why.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        judges: Judges::OneRecord,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::OutOfScope,
        options: &["back_pointer_paths", "test_paths", "scope"],
        population: PopulationUnit::Field,
        actions: &[
            VariantAction::new("locator-on-back-pointer-path", ActionDecl::Exempt("the repair is a comment in a source file, and no action kind addresses a non-record file yet; dropping the locator first would delete the evidence before the comment exists")),
            VariantAction::new("locator-on-back-pointer-path-sole", ActionDecl::Exempt("the repair is a comment in a source file, and no action kind addresses a non-record file yet; dropping the only locator first would leave the record with no evidence")),
        ],
        phase: Phase::Main,
        companion: None,
        run: |c| {
            let opts = c
                .config
                .enabled_rule_opts::<LocatorMechanismOpts>(RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH)
                .unwrap_or_default();
            embodiment_locator_on_back_pointer_path(
                c.records,
                c.config,
                &opts.back_pointer_paths,
                &opts.test_paths,
                changed_only(
                    c.config,
                    RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH,
                    c.changed_paths,
                )
                .as_ref(),
                c.read_text,
            )
        },
    },
    RuleDef {
        id: RULE_CONFIG_SCOPE_MATCHES_NOTHING,
        description: "Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`).",
        scope: Scope::Run,
        anchor: Anchor::Config,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges other rules' populations"),
        options: &[],
        population: PopulationUnit::Rule,
        actions: &[
            VariantAction::new("scope-matches-nothing", ActionDecl::CONFIG_SET),
        ],
        phase: Phase::AfterRules,
        companion: None,
        run: |c| config_scope_matches_nothing(c.run.executions, c.config_path),
    },
    RuleDef {
        id: RULE_WAIVER_MATCHES_NOTHING,
        description: "A waiver that does nothing is reported on the waiver record, naming the rule and scope it claimed (`MILE-168`). Four cases: the waiver is expired (its `Expires` has passed or is not a `YYYY-MM-DD` date, so it already covers nothing); it is not in effect (it lacks a field the suppression type's `effective_requires` names, or holds a blank, placeholder or pending value there; or, when the type declares `permanent`, it has no `Expires` and its rule has no `permanent` entry, its category is absent or outside the entry's vocabulary, or a category that requires a citation has none that resolves; either way it suppresses nothing and its finding is reported again, naming the unmet requirement); it names a rule this build does not ship; or it is active, its rule ran, and no finding of that rule falls in its scope. A waiver matching a finding is not reported, and an expired waiver is reported once, as expired; an ineffective one is reported once, as ineffective. The unmatched case is judged only on a whole-corpus run in which the target rule ran: under a path scope or `--base`, or with the target rule off, an absent finding proves nothing, so the waiver is counted as eligible and unexamined (never out of scope). The expired and unknown-rule cases need only the waiver record to have been examined. Opt-in. The population counts waiver records.",
        scope: Scope::Run,
        anchor: Anchor::Record,
        judges: Judges::NotRecordLevel,
        needs_identity_index: false,
        requires_relation_pairs: false,
        requires_acyclic_pair: false,
        requires_override_log: false,
        requires_authority: false,
        reads_declaration: None,
        header_none: HeaderNone::Exempt("judges waiver records against the run's findings, not a configured header"),
        options: &[],
        population: PopulationUnit::Record,
        actions: &[
            VariantAction::new("waiver-expired", ActionDecl::FIELD_SET),
            VariantAction::new("waiver-ineffective", ActionDecl::FIELD_SET),
            VariantAction::new("waiver-unknown-rule", ActionDecl::FIELD_SET),
            VariantAction::new("waiver-matches-no-finding", ActionDecl::FIELD_SET),
        ],
        phase: Phase::AfterScoping,
        companion: None,
        run: |c| waiver_matches_nothing(
            c.run.waivers,
            c.run.findings,
            c.run.executions,
            c.run.whole_corpus,
            |p| (c.run.reached)(p),
            |rule| {
                c.config
                    .rules
                    .get(rule)
                    .is_some_and(|s| s.scope.is_some())
            },
            c.run.today,
        ),
    },
];

const fn ids<const N: usize>(table: &[RuleDef]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = table[i].id;
        i += 1;
    }
    out
}

/// Every rule this build ships, by id. One canonical name per rule: the config
/// is validated against this list, so a typo is a load-time error naming the
/// valid set rather than a check that silently never runs. Derived from
/// `RULE_TABLE`.
pub const ALL_RULES: &[&str] = &ids::<{ RULE_TABLE.len() }>(RULE_TABLE);

/// Whether `rule` accepts the option named `option`.
pub fn rule_accepts_option(rule: &str, option: &str) -> bool {
    rule_def(rule).is_some_and(|d| d.options.contains(&option))
}

/// The table entry for a rule id.
pub fn rule_def(id: &str) -> Option<&'static RuleDef> {
    RULE_TABLE.iter().find(|r| r.id == id)
}

/// Ids of the rules whose `RuleDef` satisfies `pred`.
pub fn rule_ids_where(pred: impl Fn(&RuleDef) -> bool) -> impl Iterator<Item = &'static str> {
    RULE_TABLE.iter().filter(move |r| pred(r)).map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_has_a_unique_id_and_a_description() {
        let mut seen = std::collections::HashSet::new();
        for def in RULE_TABLE {
            assert!(seen.insert(def.id), "{} is in the table twice", def.id);
            assert!(
                !def.description.trim().is_empty(),
                "{} has an empty description",
                def.id
            );
        }
        assert_eq!(ALL_RULES.len(), RULE_TABLE.len());
    }

    /// A rule that does not look at records has no header to read, so the
    /// table must say so; a record-scoped rule is exempt only with a reason.
    #[test]
    fn a_rule_outside_the_records_is_exempt_from_the_header_none_check() {
        for def in RULE_TABLE {
            if def.scope != Scope::Records {
                assert!(
                    matches!(def.header_none, HeaderNone::Exempt(_)),
                    "{} does not look at records, so it must carry a header_none exemption",
                    def.id
                );
            }
            if let HeaderNone::Exempt(reason) = def.header_none {
                assert!(!reason.trim().is_empty(), "{} has an empty reason", def.id);
            }
        }
    }

    /// A rule id constant with no table entry is a rule that was declared and
    /// never registered: it would not load from a config, and no check would
    /// run it.
    #[test]
    fn every_rule_id_constant_has_a_table_entry() {
        let source = include_str!("catalog.rs");
        let mut declared = 0;
        for line in source.lines() {
            let Some(rest) = line.strip_prefix("pub const RULE_") else {
                continue;
            };
            if !rest.contains(": &str") {
                continue;
            }
            declared += 1;
            let name = format!("RULE_{}", rest.split(':').next().unwrap());
            let literal_on_this_line = rest.split('"').nth(1);
            // A constant whose value wraps onto the next line is read from it.
            let id = match literal_on_this_line {
                Some(id) => id.to_string(),
                None => {
                    let next = source
                        .lines()
                        .skip_while(|l| *l != line)
                        .nth(1)
                        .unwrap_or_default();
                    next.split('"').nth(1).unwrap_or_default().to_string()
                }
            };
            assert!(
                RULE_TABLE.iter().any(|d| d.id == id),
                "{name} (\"{id}\") has no entry in RULE_TABLE"
            );
        }
        assert_eq!(declared, RULE_TABLE.len());
    }

    /// A rule that judges the configuration reports on the configuration; one
    /// that reads files outside the corpus is anchored there. Declared once, in
    /// the entry, which is what a scoped run reads.
    #[test]
    fn the_rules_that_read_a_per_type_declaration_name_it() {
        let reads: Vec<(&str, Declaration)> = RULE_TABLE
            .iter()
            .filter_map(|d| d.reads_declaration.map(|r| (d.id, r)))
            .collect();
        assert_eq!(
            reads,
            vec![
                (RULE_FIELD_QUALITY, Declaration::RequiredFields),
                (RULE_FIELD_PENDING, Declaration::RequiredFields),
                (
                    RULE_SEQUENCE_STATUS_AGREEMENT,
                    Declaration::BlockedOrProgress
                ),
                (
                    RULE_REVISION_LOG_VERSION_BUMP_REQUIRED,
                    Declaration::VersionBump
                ),
                (RULE_HEADER_FIELD_SET_CONSISTENCY, Declaration::KnownFields),
                (RULE_HEADER_POINTER_FIELD_CLEAN, Declaration::PointerFields),
                (RULE_STATUS_DONE_ITEMS_SHAPE, Declaration::DeliveryItems),
                (RULE_STATUS_WAIVED_ITEM_STALE, Declaration::DeliveryItems),
            ]
        );
    }

    /// `ADR-90`: a rule is marked one-record, cross-record, changed-set or not
    /// record-level, and the mark agrees with where the rule looks and what its
    /// findings sit on.
    #[test]
    fn a_rule_judgement_agrees_with_its_scope_and_anchor() {
        for def in RULE_TABLE {
            let record_anchored =
                matches!(def.anchor, Anchor::Record | Anchor::RecordOrDeclaredFile(_));
            if def.scope == Scope::Records && record_anchored {
                assert!(
                    matches!(
                        def.judges,
                        Judges::OneRecord
                            | Judges::AnchorScoped
                            | Judges::CrossRecord
                            | Judges::ChangedSet
                    ),
                    "{} is record-level, so it is one-record, cross-record or changed-set",
                    def.id
                );
            } else {
                assert_eq!(
                    def.judges,
                    Judges::NotRecordLevel,
                    "{} is not anchored to a record in the records scope",
                    def.id
                );
            }
        }
        let judged = |j: Judges| {
            RULE_TABLE
                .iter()
                .filter(|d| d.judges == j)
                .map(|d| d.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            judged(Judges::ChangedSet),
            vec![
                RULE_REVISION_LOG_VERSION_BUMP_REQUIRED,
                RULE_IDENTITY_OPEN_PR_COLLISION,
                RULE_STATUS_TRANSITION_ALLOWED
            ]
        );
        for id in [
            RULE_IDENTITY_COLLISION,
            RULE_POINTER_RESOLUTION,
            RULE_CITATION_RESOLUTION,
            RULE_RELATION_PAIR_RECIPROCITY,
            RULE_SEQUENCE_CYCLE,
        ] {
            assert!(
                judged(Judges::CrossRecord).contains(&id),
                "{id} reads other records, so it is cross-record"
            );
        }
        assert!(judged(Judges::OneRecord).contains(&RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH));
        assert_eq!(
            judged(Judges::AnchorScoped),
            vec![RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE]
        );
    }

    /// `ADR-90` and its 2026-11-07 amendment: `scope` appears in `options` only
    /// for a per-record rule or an anchor-scoped rule, and the load-time check
    /// (`rule_accepts_option`) reads the same list.
    #[test]
    fn scope_is_listed_only_by_one_record_and_anchor_scoped_rules() {
        let mut listing = Vec::new();
        for def in RULE_TABLE {
            let lists = def.options.contains(&"scope");
            assert_eq!(rule_accepts_option(def.id, "scope"), lists, "{}", def.id);
            if lists {
                assert!(
                    matches!(def.judges, Judges::OneRecord | Judges::AnchorScoped),
                    "{} lists `scope` but is neither per-record nor anchor-scoped",
                    def.id
                );
                listing.push(def.id);
            }
        }
        assert_eq!(
            listing,
            vec![
                RULE_EMBODIMENT_CLAIM_EXPIRED,
                RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED,
                RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
                RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH
            ]
        );
    }

    #[test]
    fn a_rule_anchor_agrees_with_its_scope() {
        for def in RULE_TABLE {
            match def.scope {
                Scope::Config => assert!(
                    matches!(def.anchor, Anchor::Config),
                    "{} judges the configuration, so its findings sit on it",
                    def.id
                ),
                Scope::Diff => assert!(
                    matches!(def.anchor, Anchor::Outside),
                    "{} judges changed paths, which are no record",
                    def.id
                ),
                Scope::Records | Scope::Files | Scope::Run => {}
            }
        }
    }
}
