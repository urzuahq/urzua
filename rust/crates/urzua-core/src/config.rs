//! Minimal `.urzua/config.yaml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::document_model::DocumentModel;
use crate::header::HeaderShape;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The schema version this build writes (ADR-0012). It reads this version and
/// the one before it (`ADR-77`, `crate::schema_version`); a config declaring
/// any other value is a structured parse-time error, not a silent best-effort
/// read.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    /// A `BTreeMap`, not a `HashMap`: iterated by several rules and by
    /// discovery, and `HashMap` iteration order is randomized per process,
    /// which several call sites' own results must not depend on.
    pub record_types: BTreeMap<String, RecordTypeConfig>,
    /// Every rule a repository has turned on, and at what level. Absent or
    /// empty means no rule runs: governance is declared, never inherited
    /// (ADR-53). A `BTreeMap` for the same reason as `record_types`: `parse`
    /// below reports the first unrecognized name it finds, and that should
    /// not depend on `HashMap`'s randomized order.
    #[serde(default)]
    pub rules: BTreeMap<String, RuleSetting>,
    /// Declared inverse relation-field pairs (`RFC-47`, `MILE-122`); see
    /// `RelationPair`. Absent means none declared.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_pairs: Vec<RelationPair>,
    /// The roadmap's declaration (`ADR-82`, `ADR-83`): the sequencing pairs the
    /// roadmap command and the derived `blocked` value both read, and how the
    /// roadmap groups its records.
    #[serde(default, skip_serializing_if = "RoadmapDecl::is_empty")]
    pub roadmap: RoadmapDecl,
    /// The lowest finding severity that makes `urzua check` exit 1. Absent
    /// means `error`, so a repository that does not declare it is unchanged.
    #[serde(default, skip_serializing_if = "FailOn::is_default")]
    pub fail_on: FailOn,
    /// The `Embodiment` vocabulary (`MILE-144`); absent means the built-in one.
    #[serde(
        default,
        skip_serializing_if = "crate::embodiment_model::EmbodimentDecl::is_default"
    )]
    pub embodiment: crate::embodiment_model::EmbodimentDecl,
    /// Declared symbol resolvers for scoped evidence pins (`ADR-74`,
    /// `SPEC-3`); absent means none, so only line ranges are available.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locator_resolvers: Vec<crate::locator::LocatorResolver>,
    /// Set when the file was at the previous schema version and was upgraded
    /// in memory (`ADR-77`). Not part of the declared configuration: it is
    /// never read from or written to a file, so an upgraded config and the
    /// same config written at the current version are equal.
    #[serde(skip)]
    pub upgraded_from: Option<crate::schema_version::UpgradeInfo>,
    /// The committed log of editor-hook overrides (`ADR-78`, `MILE-188`): a
    /// repository-relative path. The hook appends to it and
    /// `override.unreviewed` reads it. Absent means no log is declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_log: Option<String>,
    /// Which errors the editor hook blocks on (`ADR-78` amendment, `MILE-195`,
    /// `SPEC-3`): `introduced` (the default, and what an absent key means) or
    /// `all`. The engine does not read it; the hook does, from the
    /// committed file, and any other value is a load error here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_on: Option<String>,
    /// Who an attestation names (`ADR-89` section 5, `MILE-201`, `SPEC-3`):
    /// where `fix --apply` and `fix --rebaseline --apply` take the attesting
    /// identity from. Absent means `gh-verified` without `require_verified`,
    /// today's behaviour.
    #[serde(default, skip_serializing_if = "AttestationDecl::is_default")]
    pub attestation: AttestationDecl,
    /// How `urzua new` numbers a record (`ADR-93`, `MILE-89`, `SPEC-3`): whether
    /// it fetches pull-request head refs to skip the numbers open pull requests
    /// took. Absent means it does not, and touches no remote.
    #[serde(default, skip_serializing_if = "NewDecl::is_default")]
    pub new: NewDecl,
    /// Which files discovery reads beyond the tracked ones (`ADR-92`,
    /// `MILE-206`, `SPEC-3`). Absent means the tracked and staged files only.
    #[serde(default, skip_serializing_if = "DiscoveryDecl::is_default")]
    pub discovery: DiscoveryDecl,
    /// The agent harness a repository installs (`ADR-89`, `MILE-198`): its
    /// adapters, the agent-instructions file and which guardrails are on.
    /// Absent installs and enables nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<crate::harness::HarnessDecl>,
    /// Who may accept a record and who may change a protected path (`ADR-89`
    /// section 4, `MILE-200`, `SPEC-3`): named decider groups, the host evidence
    /// that may name one, and the gates by type and status and by path. Read by
    /// the acceptance and protected-path guardrails and by
    /// `diff.decider-required`. Absent declares no gate; a key present but
    /// empty, or malformed, is a load error.
    #[serde(
        default,
        deserialize_with = "keep_authority",
        skip_serializing_if = "Option::is_none"
    )]
    pub authority: Option<crate::authority::AuthorityDecl>,
}

/// The `new` key (`ADR-93`, `SPEC-3`). Structured (`ADR-64`): booleans, numbers
/// and a list of ref patterns, no string mini-language beyond a git refspec source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewDecl {
    /// Fetch pull-request head refs with plain git before numbering and skip the
    /// numbers found there. Default `false`: `new` touches no remote.
    #[serde(default)]
    pub check_open_prs: bool,
    /// The ref patterns to fetch, each with exactly one `*` standing for the pull
    /// request number. Default GitHub's layout.
    #[serde(default = "NewDecl::default_patterns")]
    pub pull_ref_patterns: Vec<String>,
    /// The remote to fetch from. Default `origin`.
    #[serde(default = "NewDecl::default_remote")]
    pub remote: String,
    /// Seconds before a hung fetch is killed. Default 30.
    #[serde(default = "NewDecl::default_fetch_timeout")]
    pub fetch_timeout_seconds: u32,
    /// Seconds a second `new` waits for the numbering lock. Default 10.
    #[serde(default = "NewDecl::default_lock_timeout")]
    pub lock_timeout_seconds: u32,
}

impl NewDecl {
    fn default_patterns() -> Vec<String> {
        vec!["refs/pull/*/head".to_string()]
    }
    fn default_remote() -> String {
        "origin".to_string()
    }
    fn default_fetch_timeout() -> u32 {
        30
    }
    fn default_lock_timeout() -> u32 {
        10
    }
    pub fn is_default(&self) -> bool {
        *self == NewDecl::default()
    }
}

impl Default for NewDecl {
    fn default() -> Self {
        NewDecl {
            check_open_prs: false,
            pull_ref_patterns: Self::default_patterns(),
            remote: Self::default_remote(),
            fetch_timeout_seconds: Self::default_fetch_timeout(),
            lock_timeout_seconds: Self::default_lock_timeout(),
        }
    }
}

/// The `discovery` key (`ADR-92`, `SPEC-3`): a repository's choice to govern
/// the record files it has written and not yet staged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDecl {
    /// Include untracked, non-ignored files directly under a declared record
    /// directory (`ADR-92` clause 1). Default `false`: a scratch file in a
    /// working tree is not part of the corpus (`ADR-56`).
    #[serde(default)]
    pub include_untracked: bool,
}

impl DiscoveryDecl {
    pub fn is_default(&self) -> bool {
        *self == DiscoveryDecl::default()
    }
}

/// The `attestation` key (`ADR-89` section 5, `SPEC-3`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttestationDecl {
    #[serde(default)]
    pub identity: AttestationIdentity,
    #[serde(default)]
    pub require_verified: bool,
}

impl AttestationDecl {
    pub fn is_default(&self) -> bool {
        *self == AttestationDecl::default()
    }
}

/// Where the attesting identity comes from: a verified `gh` login that wins
/// over `--by` (`gh-verified`, the default), or `--by` as given (`asserted`,
/// which reverses `RFC-2`'s verified-beats-typed order and is opt-in).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttestationIdentity {
    #[default]
    GhVerified,
    Asserted,
}

/// The values `block_on` accepts (`SPEC-3`).
pub const BLOCK_ON_VALUES: [&str; 2] = ["introduced", "all"];

/// `roadmap` (`SPEC-3`, `ADR-82`): which `relation_pairs` are dependencies, and
/// how `urzua roadmap` groups its records. A pair can be `acyclic` without
/// being a dependency: a containment pair (parent and children) is checked for
/// cycles but a child does not wait on its parent finishing. Absent means no
/// roadmap, which the command states instead of printing an empty one.
/// The derived `blocked` value reads exactly these pairs, so `check` and the
/// roadmap cannot disagree about what is blocked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoadmapDecl {
    /// Names of `relation_pairs` entries that declare `acyclic: true`.
    #[serde(default)]
    pub pairs: Vec<String>,
    /// Header field names the roadmap groups by, outermost first. The engine
    /// names none; each must be declared by some record type.
    #[serde(default)]
    pub group_by: Vec<String>,
    /// Names of `record_types` entries that are work (`ADR-82`, Amendment of
    /// 2026-11-05): the records the roadmap orders. A record of any other type
    /// still gates its dependents but is not in waves, `ready`, the critical
    /// path, `records` or `groups`. Absent means every type is work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_types: Option<Vec<String>>,
}

impl RoadmapDecl {
    fn is_empty(&self) -> bool {
        self.pairs.is_empty() && self.group_by.is_empty() && self.work_types.is_none()
    }
}

/// The exit-code threshold (`SPEC-3`, `fail_on`). Only the exit code reads
/// it: the report's `blocking` and `status` keep meaning "an error".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    #[default]
    Error,
    Warning,
}

impl FailOn {
    fn is_default(&self) -> bool {
        *self == FailOn::Error
    }
}

/// What a `Realized-by` locator may name (`BUG-176`): one tracked file, a
/// directory (every tracked file below it) or a glob (every tracked file it
/// matches). Which kinds a repository accepts, and which of those may carry a
/// `@sha256:` pin, are the `locator_kinds` and `pinnable` options of
/// `embodiment.locator-exists`; both default to `[file]`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LocatorKind {
    File,
    Directory,
    Glob,
}

impl std::fmt::Display for LocatorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LocatorKind::File => "file",
            LocatorKind::Directory => "directory",
            LocatorKind::Glob => "glob",
        })
    }
}

/// The value of the `scope` rule option (`ADR-90`): the rule judges only the
/// records a `--base` diff adds or changes. `changed` is the one value; there
/// is no `scope: all`, because omitting the option is that. Any other value is
/// refused at load, naming `changed`. `scope` is a scalar, which the
/// list-option reader (`option_field`) never sees, so the value is validated
/// here and the rule's acceptance of the option at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleScope {
    Changed,
}

impl<'de> Deserialize<'de> for RuleScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = yaml_serde::Value::deserialize(deserializer)?;
        match value.as_str() {
            Some("changed") => Ok(RuleScope::Changed),
            _ => {
                let shown = match &value {
                    yaml_serde::Value::String(s) => format!("'{s}'"),
                    other => yaml_serde::to_string(other)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "?".to_string()),
                };
                Err(serde::de::Error::custom(format!(
                    "rule option 'scope' is {shown}; its only value is 'changed' (omit the option to judge every record)"
                )))
            }
        }
    }
}

/// A rule's options as a typed value, built from its `RuleSetting` in one
/// place (`crate::rules::options`). Implemented per rule that reads options.
pub trait RuleOpts: Sized {
    fn from_setting(setting: &RuleSetting) -> Self;
}

impl Config {
    /// The typed options of `rule` when the repository declared it and it is
    /// not `off`; `None` otherwise. The one place that decides a rule is "on"
    /// for a caller that gathers a rule's inputs (`check` reading claim files),
    /// so it cannot disagree with the rule about whether it runs.
    pub fn enabled_rule_opts<T: RuleOpts>(&self, rule: &str) -> Option<T> {
        self.rules
            .get(rule)
            .filter(|s| s.level != RuleLevel::Off)
            .map(T::from_setting)
    }

    /// The typed options of `rule` as declared, whatever its level, and the
    /// options of an undeclared rule when it is not (every option absent). For
    /// a rule's own run, which the runner only starts for a rule that is on.
    pub fn rule_opts<T: RuleOpts>(&self, rule: &str) -> T {
        match self.rules.get(rule) {
            Some(setting) => T::from_setting(setting),
            None => T::from_setting(&RuleSetting::default()),
        }
    }

    /// The record type whose records suppress findings: the one declaring
    /// `role: suppression`, else the type named `waiver` (the fallback for a
    /// config that declares no role). Load refuses two declaring types.
    pub fn suppression_type(&self) -> &str {
        self.record_types
            .iter()
            .find(|(_, t)| t.role == Some(TypeRole::Suppression))
            .map(|(name, _)| name.as_str())
            .unwrap_or(DEFAULT_SUPPRESSION_TYPE)
    }

    /// Every declared record type's name, in `record_types`' own (sorted)
    /// order.
    pub fn sorted_type_names(&self) -> Vec<&String> {
        self.record_types.keys().collect()
    }
}

/// What a repository declared about one rule. Written either as a bare level
/// (`field.quality: error`) or as a table when a rule takes options.
///
/// This struct is the one definition of the option set: the table form is read
/// straight into it (`deny_unknown_fields`, so an unknown key is refused), and
/// the bare-level shorthand is the only thing the hand-written `Deserialize`
/// below adds. A new option is a field here plus, for a list-valued one, an
/// entry in `RULE_OPTIONS`; `option_table_tests` fails when the two disagree.
/// Rules read their options through a typed `RuleOpts` value
/// (`Config::rule_opts`), never by re-reading these fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct RuleSetting {
    pub level: RuleLevel,
    /// `changed`: the rule judges only the records a `--base` diff adds or
    /// changes, none on a plain `check` (`ADR-90`). Only meaningful to a rule
    /// whose verdict on a record depends on that record alone; declaring it on
    /// any other rule is a load-time error. Omitted means every record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<RuleScope>,
    /// Statuses a reference's target may not be in. Only meaningful to
    /// `pointer.target-status`; declaring it on any other rule is a load-time
    /// error, because a silently-ignored option is a check that never fires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_in: Option<Vec<String>>,
    /// Path prefixes to scan for claims about records. Only meaningful to
    /// `claim.status-agreement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_paths: Option<Vec<String>>,
    /// Statuses that make a record legitimately claimable as closed. Declared,
    /// never inferred: a built-in list is how a rule silently stops applying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_statuses: Option<Vec<String>>,
    /// The frontmatter key in which a claim file lists, on one line, the
    /// records it closes (`ADR-85`). Declared, never known: the engine names
    /// no field. Only meaningful to `claim.status-agreement`; optional, and
    /// declaring none leaves the rule exactly as it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closes: Option<String>,
    /// When `true`, a claim file with frontmatter but without the `closes` key
    /// is a finding (`ADR-85`). Only meaningful to `claim.status-agreement`,
    /// needs `closes`, and `false` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// `false` turns the closing-verb reading of `claim.status-agreement` off,
    /// so only the `closes` field is a claim (`ADR-88`). Only meaningful to that
    /// rule, needs `closes`, and `true` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verb_claims: Option<bool>,
    /// `true` makes a gated move or change with no `--actors` file an error
    /// where a local run reports it as `info` (`ADR-89` section 4, `MILE-200`).
    /// Only meaningful to `diff.decider-required`, and `false` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_evidence: Option<bool>,
    /// Statuses that mean a referenced record's work is over. Only meaningful
    /// to `narrative-field.stale`. Declared per repository rather than compiled
    /// in: a built-in table keyed on this repository's own type names is how the
    /// rule went inert for every other corpus (BUG-59).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_statuses: Option<Vec<String>>,
    /// Declared pointer/narrative field names a rule checks, instead of all of
    /// them. Only meaningful to `pointer.target-status`, and optional: omitted
    /// means every declared field. Lets a repository judge a dependency field
    /// (`Implements`) apart from a lineage field (`Derives-from`) whose
    /// Superseded target is permanently true history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Path prefixes that publish beyond the repository (`RFC-11` section 1).
    /// Only meaningful to `boundary.published-path-reference`. The rule's own
    /// `level` is the boundary's severity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_paths: Option<Vec<String>>,
    /// Declared boundaries (`ADR-80`, `MILE-125`): path globs with a kind, a
    /// required reason and a severity. Only meaningful to
    /// `boundary.published-path-reference`, where a boundary's severity is the
    /// level of its findings and the rule's `level` only enables the rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<Vec<Boundary>>,
    /// The text that, on a line inside a boundary, excuses that line alone
    /// (`ADR-80`). Declared, never compiled in. Meaningful to
    /// `boundary.published-path-reference` and, for a back-pointer on the line,
    /// to `embodiment.back-pointer-resolution` and
    /// `embodiment.back-pointer-decision-accepted` (`ADR-80` amendment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_marker: Option<String>,
    /// The generated index artifacts `index.freshness` compares against the
    /// records (`MILE-139`). Only meaningful to that rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<IndexSpec>>,
    /// Path prefixes or globs of source code that must change with a governed
    /// record. Only meaningful to `diff.source-requires-record` (`MILE-124`,
    /// `MILE-158`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_paths: Option<Vec<String>>,
    /// Path prefixes of application source. Only meaningful to
    /// `diff.source-and-records-mixed` (`MILE-142`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_paths: Option<Vec<String>>,
    /// Path prefixes or globs removed from `application_paths` or
    /// `source_paths` (tests, fixtures). Only meaningful to the two diff rules
    /// (`MILE-142`, `MILE-158`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_paths: Option<Vec<String>>,
    /// The decision gate (`MILE-133`): each record type that holds decisions,
    /// with the statuses at which one counts as decided. Declaring it turns
    /// `diff.source-requires-record` from "some governed record changed" into
    /// "a decision at a decided status changed, or a record linking one
    /// (`link_fields`), or a fix record (`fix_types`)". Only meaningful to
    /// `diff.source-requires-record`; omitted means the rule asks only for
    /// any governed record, and `fix_types` and `link_fields` are then refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_types: Option<BTreeMap<String, Vec<String>>>,
    /// Record types whose records exempt a change from the decision gate
    /// (`MILE-133`), such as `bug`: a mechanical fix needs a bug record and no
    /// decision. Only meaningful to `diff.source-requires-record`, and only
    /// with `decision_types`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_types: Option<Vec<String>>,
    /// Pointer or narrative fields (such as a milestone's `Implements`) through
    /// which a changed record links a decision for the decision gate
    /// (`MILE-133`): a changed record whose field names a decision at a
    /// decided status satisfies it. Only meaningful to
    /// `diff.source-requires-record`, and only with `decision_types`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_fields: Option<Vec<String>>,
    /// Admit header keys beginning `x-` (case-insensitive) as extension keys
    /// (`MILE-141`). Only meaningful to `header.field-set-consistency`, where
    /// omitted means `false`: every undeclared key is a finding. A declared
    /// `true` exempts prefixed keys from that rule and nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_extension_prefix: Option<bool>,
    /// Per-type extension prefix (`MILE-141`): a record type name mapped to the
    /// prefix its extension header keys begin with, in place of the fixed `x-`.
    /// Only meaningful to `header.field-set-consistency`. A type named here
    /// admits keys beginning with its own prefix (any case) and no others; a
    /// type left out falls back to `allow_extension_prefix`. Each prefix is
    /// non-empty, holds no whitespace and is not the start of a field the type
    /// declares, and each type must be a declared type with `known_fields`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension_prefixes: Option<BTreeMap<String, String>>,
    /// Treat a pending value as legitimate until the record reaches a status
    /// that requires the field (`MILE-169`). Only meaningful to `field.pending`,
    /// where omitted means `false`: a pending value is reported at every status.
    /// A field no `statuses.require` entry names is unaffected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_until_required: Option<bool>,
    /// Read a required field whose value is identical (after trimming) to the
    /// type's template text for that field as a Placeholder (`BUG-22`). Only
    /// meaningful to `field.quality`, where omitted means `false`: the
    /// compiled-in placeholder tokens apply as before. Declared `true`, the
    /// template (`.urzua/templates/<type>.md`) is the placeholder vocabulary
    /// and the compiled-in tokens no longer apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_placeholders: Option<bool>,
    /// Path prefixes of the source files scanned for `Implements:` comment
    /// back-pointers (`MILE-128`). Only meaningful to
    /// `embodiment.back-pointer-resolution`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_pointer_paths: Option<Vec<String>>,
    /// Path prefixes, each inside a `back_pointer_paths` entry, whose files are
    /// the test class: a `Verifies:` comment there is test-tier evidence for
    /// the record it cites (`MILE-146`). Only meaningful to
    /// `embodiment.back-pointer-resolution`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_paths: Option<Vec<String>>,
    /// Days a locator change must have aged before it counts as drift
    /// (`MILE-129`). Only meaningful to `embodiment.consistency`; omitted
    /// means no grace window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_grace_days: Option<u32>,
    /// A declared field holding the date a record's evidence was last audited
    /// (`YYYY-MM-DD`). Where present, the `drift_grace_days` window counts from
    /// it instead of from the cited file's last change (`MILE-166`). Only
    /// meaningful to `embodiment.consistency`, and only with `drift_grace_days`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_audit_field: Option<String>,
    /// Days after which an `Embodiment` claim last verified longer ago than
    /// this draws a warning (`MILE-129`). Only meaningful to
    /// `embodiment.consistency`; omitted means no age limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_max_age_days: Option<u32>,
    /// A declared date field holding the day a claim was last re-verified
    /// (`YYYY-MM-DD`), written only by `urzua reverify` (`ADR-97`). Only
    /// meaningful to `embodiment.claim-expired`; optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_field: Option<String>,
    /// The evidence categories whose locators must be pinned before a claim is
    /// renewed (`ADR-97`), as the `embodiment` declaration names them. Only
    /// meaningful to `embodiment.claim-expired`; omitted means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_pin: Option<Vec<String>>,
    /// The most records one `urzua reverify --apply` may name (`ADR-97`). Only
    /// meaningful to `embodiment.claim-expired`; omitted means 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_batch: Option<u32>,
    /// The expiry period per record type, status and evidence tier
    /// (`ADR-97`, `ADR-64`). Only meaningful to `embodiment.claim-expired`,
    /// which needs at least one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periods: Option<Vec<ClaimPeriod>>,
    /// `Embodiment` values a human sets that no evidence can compute
    /// (`Deprecated`, `Inactive`): a record stating one is not reported as
    /// drift (`MILE-148`). Only meaningful to `embodiment.consistency`;
    /// omitted means every stated value must match the computed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky_states: Option<Vec<String>>,
    /// The kinds of locator a `Realized-by` may name: `file`, `directory`,
    /// `glob` (`BUG-176`). Only meaningful to `embodiment.locator-exists`;
    /// omitted means `[file]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator_kinds: Option<Vec<LocatorKind>>,
    /// The kinds of locator that may carry a `@sha256:` pin (`BUG-176`); each
    /// must also be in `locator_kinds`. Only meaningful to
    /// `embodiment.locator-exists`; omitted means `[file]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinnable: Option<Vec<LocatorKind>>,
    /// Header fields whose value must be a real `YYYY-MM-DD` calendar date
    /// (`MILE-137`). Only meaningful to `date.field-valid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_fields: Option<Vec<String>>,
    /// Date orderings `{earlier, later}` (`MILE-157`): the `earlier` field's
    /// date may not be later than the `later` field's. Only meaningful to
    /// `date.field-order`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_order: Option<Vec<DateOrder>>,
    /// Severity per field, within one rule (`MILE-140`): a field named here
    /// reports at its own level (`warn` or `error`), every other field at the
    /// rule's `level`. Only meaningful to the rules that report per field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_levels: Option<BTreeMap<String, RuleLevel>>,
    /// Statuses that count as accepted for a record whose type declares no
    /// `statuses.success`, or, for the back-pointer gate, no
    /// `statuses.implementable` (`MILE-133`). Only meaningful to
    /// `embodiment.back-pointer-decision-accepted` and
    /// `embodiment.requires-accepted-status`; optional, because a type that declares its
    /// success statuses needs no fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_statuses: Option<Vec<String>>,
    /// Prefixed ids that are not records of this repository (`MILE-156`): an
    /// exact id (`RFC-4122`) or an inclusive numeric range (`RFC-4000..4999`)
    /// under a declared record prefix, for a public standard number sharing a
    /// record prefix. Only meaningful to `citation.resolution`; optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ids: Option<Vec<String>>,
    /// The header field in which a record lists, as comma-separated exact ids,
    /// the ids it names without citing as a reference (`BUG-190`, `ADR-75`).
    /// Must be a known field that is not a pointer, narrative or relation-role
    /// field. Only meaningful to `citation.resolution`; optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_reference_field: Option<String>,
    /// The `##` headings whose backtick-quoted paths `citation.path-exists`
    /// reads (`MILE-143`), matched exactly after trimming. Only meaningful to
    /// that rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_sections: Option<Vec<String>>,
    /// When `true`, `citation.path-exists` and `citation.resolution` do not
    /// read each record's revision log, the table under the type's declared
    /// `revision_log` identity (`MILE-167`, `MILE-156`): append-only history
    /// names a path or an id as it was. Only meaningful to those rules; optional, and `false` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_revision_log: Option<bool>,
    /// The committed snapshot of issue and pull request numbers
    /// `citation.issue-exists` checks citations against, the repository it
    /// describes and the citation forms it reads (`MILE-143`). Only meaningful
    /// to that rule, and read by `urzua refs refresh`, the one command that
    /// writes the snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_refs: Option<IssueRefs>,
}

/// One `periods` entry of `embodiment.claim-expired` (`ADR-97`): a claim of a
/// record of `type` (optionally in one of `status`, optionally computing to
/// `tier`) expires `days` after its anchor. `spread_days` lengthens the first
/// period of a record with no recorded audit date by a deterministic offset of
/// `0..=spread_days` days; `level` is the entry's own, below the rule's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimPeriod {
    #[serde(rename = "type")]
    pub record_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<RuleLevel>,
}

/// `citation.issue-exists`'s structured option (`MILE-143`, `ADR-64`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRefs {
    /// The snapshot file, relative to the repository root.
    pub snapshot: String,
    /// The GitHub repository the snapshot describes, `owner/name`.
    pub repository: String,
    /// The citation forms read: `number` is `#123`, `url` is a full link to an
    /// issue or pull request of `repository`. A form not listed is not read.
    pub forms: Vec<RefForm>,
    /// Header fields whose values are also read for citations, named as the
    /// type declares them (`Found-in`), so a number in a field is judged as one
    /// in a reference section is. Absent by default: only `reference_sections`
    /// are read. A name no record type declares is refused at load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefForm {
    Number,
    Url,
}

/// One `date_order` entry (`MILE-157`, `ADR-64`): two header field names, the
/// `earlier` date of which may not be later than the `later` one's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DateOrder {
    pub earlier: String,
    pub later: String,
}

/// One generated index artifact a repository commits: a file listing the
/// records of one type, one entry per line, in a declared line format. The
/// engine regenerates nothing and writes nothing; it compares the committed
/// entries with the records (`MILE-139`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexSpec {
    /// The artifact, relative to the repository root.
    pub path: String,
    /// The declared record type whose records the artifact lists.
    #[serde(rename = "type")]
    pub record_type: String,
    /// One entry's line, with `{id}` (required), `{file}` and `{title}`
    /// placeholders, e.g. `- [{id}]({file}): {title}`.
    pub format: String,
    /// Marker lines delimiting the generated list inside a larger file (a
    /// README). Both or neither: with neither, every non-blank line of the
    /// file is an entry.
    #[serde(default)]
    pub begin: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

/// What a declared boundary is for (`ADR-80`, `RFC-11` section 1). The prohibitions
/// follow from the kind and from nothing else: a record identifier inside a
/// `published` boundary, a back-pointer inside a `generated` one, none for
/// `vendored`. Any other value is refused at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoundaryKind {
    Published,
    Vendored,
    Generated,
}

impl BoundaryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BoundaryKind::Published => "published",
            BoundaryKind::Vendored => "vendored",
            BoundaryKind::Generated => "generated",
        }
    }
}

/// The level of every finding one boundary produces (`ADR-80`): `error` or
/// `warn`, never `off` (a boundary that reports nothing is not declared).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoundarySeverity {
    Error,
    Warn,
    /// `ADR-87` point 10: a declared severity may be `info` too.
    Info,
}

/// One declared boundary (`ADR-80`, `MILE-125`): path globs with a kind, a
/// required reason and a severity. Structured config, never a string
/// mini-language (`ADR-64`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    /// Path prefixes or globs (`MILE-158`) the boundary covers.
    pub paths: Vec<String>,
    pub kind: BoundaryKind,
    /// Why the paths are declared. Specification, not decoration
    /// (`RFC-11` section 4): required and non-blank.
    pub reason: String,
    pub severity: BoundarySeverity,
    /// Paths removed from this boundary for the violation check only, the way
    /// a fixture directory is excused. Evidence ignores it (`ADR-80`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
}

// Implements: ADR-87 (the `info` rule level)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuleLevel {
    #[default]
    Off,
    /// `ADR-87`: findings are reported and counted but never trip `fail_on`.
    Info,
    Warn,
    Error,
}

impl RuleLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleLevel::Off => "off",
            RuleLevel::Info => "info",
            RuleLevel::Warn => "warn",
            RuleLevel::Error => "error",
        }
    }

    /// The severity this level gives a finding, or `None` for `off`, which
    /// runs nothing. Exhaustive on purpose: a new level cannot reach a
    /// severity without being decided here (`ADR-87` point 10).
    pub fn severity(self) -> Option<crate::report::FindingSeverity> {
        use crate::report::FindingSeverity;
        match self {
            RuleLevel::Off => None,
            RuleLevel::Info => Some(FindingSeverity::Info),
            RuleLevel::Warn => Some(FindingSeverity::Warning),
            RuleLevel::Error => Some(FindingSeverity::Error),
        }
    }
}

impl BoundarySeverity {
    pub fn severity(self) -> crate::report::FindingSeverity {
        use crate::report::FindingSeverity;
        match self {
            BoundarySeverity::Info => FindingSeverity::Info,
            BoundarySeverity::Warn => FindingSeverity::Warning,
            BoundarySeverity::Error => FindingSeverity::Error,
        }
    }
}

impl Serialize for RuleLevel {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RuleLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "off" => Ok(RuleLevel::Off),
            "info" => Ok(RuleLevel::Info),
            "warn" => Ok(RuleLevel::Warn),
            "error" => Ok(RuleLevel::Error),
            other => Err(serde::de::Error::custom(format!(
                "unrecognized rule level '{other}' -- expected \"off\", \"info\", \"warn\", or \"error\""
            ))),
        }
    }
}

impl Serialize for RuleSetting {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        RuleSetting::serialize(self, s)
    }
}

impl<'de> Deserialize<'de> for RuleSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Dispatched on the parsed value rather than through `#[serde(untagged)]`.
        // Untagged discards each variant's error, so every malformed setting --
        // a bad level, a bad level in table form, an unknown key -- collapsed
        // into "data did not match any variant of untagged enum Either", and the
        // messages naming the valid levels and keys were unreachable (BUG-46).
        let value = yaml_serde::Value::deserialize(deserializer)?;
        if let Some(map) = value.as_mapping() {
            // A present key with no value (`fields:` or `fields: null`) would
            // deserialize to `None`, the same as an omitted key, and silently
            // restore the rule's default scope. Reject it instead.
            for (key, v) in map {
                if v.is_null() {
                    return Err(serde::de::Error::custom(format!(
                        "rule option '{}' is present with no value; give it a list or remove the key",
                        key.as_str().unwrap_or("?")
                    )));
                }
            }
            RuleSetting::deserialize(value).map_err(serde::de::Error::custom)
        } else {
            let level = RuleLevel::deserialize(value).map_err(serde::de::Error::custom)?;
            Ok(RuleSetting {
                level,
                ..RuleSetting::default()
            })
        }
    }
}

impl RecordTypeConfig {
    /// Every directory this type's records live in: `dir` first (where
    /// `urzua new` writes), then `extra_dirs`. One identity namespace spans
    /// all of them (`MILE-130`).
    pub fn all_dirs(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dir.as_str()).chain(self.extra_dirs.iter().map(String::as_str))
    }

    /// Whether `parent` (a path's parent directory) is one of this type's
    /// directories. A directory, never its subtree (`RFC-35`).
    pub fn owns_dir(&self, parent: Option<&std::path::Path>) -> bool {
        self.all_dirs()
            .any(|d| parent == Some(std::path::Path::new(d)))
    }

    /// Whether `file_name` is declared a non-record file of this type
    /// (`exclude_files`): it is not loaded and no rule examines it.
    pub fn excludes(&self, file_name: &str) -> bool {
        self.exclude_files.iter().any(|f| f == file_name)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTypeConfig {
    pub dir: String,
    /// Further directories holding this type's records (`MILE-130`): one
    /// identity namespace across `dir` and each of these. `urzua new` writes
    /// to `dir` but numbers past every directory; ids, pointers, collisions and
    /// `filename.identity-pattern` see them as one population. Each is a
    /// directory like `dir` (not its subtree, `RFC-35`); no two types may
    /// own the same directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_dirs: Vec<String>,
    /// The raw `identity` declaration (RFC-33 layer 1, MILE-98), kept as the
    /// YAML value so the typed parse of every other key is unchanged;
    /// `parse_with_document_models` validates it afterwards. A key present
    /// with no value is kept as `Some(Null)`, not read as absent.
    #[serde(
        default,
        deserialize_with = "keep_raw",
        skip_serializing_if = "Option::is_none"
    )]
    pub identity: Option<yaml_serde::Value>,
    /// The raw `fields` declaration; see `identity`.
    #[serde(
        default,
        deserialize_with = "keep_raw",
        skip_serializing_if = "Option::is_none"
    )]
    pub fields: Option<yaml_serde::Value>,
    /// The raw `sections` declaration; see `identity`.
    #[serde(
        default,
        deserialize_with = "keep_raw",
        skip_serializing_if = "Option::is_none"
    )]
    pub sections: Option<yaml_serde::Value>,
    /// The raw `statuses` declaration (MILE-132): the type's status
    /// vocabulary, which values are terminal, the fields a status requires or
    /// forbids, and the type's own "pending" values. Kept as the YAML value
    /// like `identity`; `status_model::validate` checks it at load.
    #[serde(
        default,
        deserialize_with = "keep_raw",
        skip_serializing_if = "Option::is_none"
    )]
    pub statuses: Option<yaml_serde::Value>,
    #[serde(default)]
    pub required_fields: Vec<String>,
    /// RFC-0010: the header shape is declared per profile, never sniffed.
    /// Omitted means `yaml-frontmatter`, the one shape with a reader
    /// (`blockquote` and `bold-list` were removed, MILE-85).
    #[serde(default)]
    pub header_shape: HeaderShape,
    /// The filename/ID prefix `urzua new` emits and `check` recognizes for
    /// this type (e.g. `MILE` for a `milestone` type, producing
    /// `MILE-7-slug.md`). Omitted means the type name itself, upper-cased --
    /// the long-standing default, so an existing config needs no change.
    /// Decouples the type's own name (used for required-fields lookup, the
    /// `urzua new <type>` argument) from its filename prefix, the same way
    /// `dir` already decouples the type name from its directory name.
    #[serde(default)]
    pub prefix: Option<String>,
    /// File names directly in `dir` that are not this type's records (an index
    /// `README.md`, a `_template.md`). A named file is never loaded as a
    /// record, so it is in no rule's population and not in the type's file
    /// census; an unnamed stray file is still reported by
    /// `filename.identity-pattern`. Bare file names only, matched exactly
    /// (MILE-150): a path or a glob would be a second selection language.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_files: Vec<String>,
    /// The additional fields a type's records may carry beyond
    /// `required_fields` (MILE-0081) -- e.g. `Embodiment`/`Realized-by` for
    /// `adr`. Declared, not voted, same principle as `required_fields`: a
    /// majority-rule inference would just ratify whatever's already in the
    /// corpus. Omitted means this type's field set isn't checked at all --
    /// additive, and deliberately left undeclared for `spec` until MILE-0074
    /// decides its canonical fields, rather than guessed at here.
    #[serde(default)]
    pub known_fields: Option<Vec<String>>,
    /// Clean, comma-separated reference fields (MILE-0090/ADR-0044) --
    /// existence-checked by `pointer_resolution`, enforced clean by
    /// `header_pointer_field_clean`. Declared, not voted, same "undeclared
    /// means skip, no implicit default" principle as `known_fields`: this
    /// tool has no real external adopters yet whose behavior a hardcoded
    /// fallback would need to preserve. (Note, 2026-11-05: that holds through
    /// Phase 0 and expires at the Phase 1 switchover, MILE-102, when RFC-49's
    /// compatibility contract takes over; MILE-118.) Every field named here
    /// must also appear in this type's `required_fields`/`known_fields`
    /// (`config.pointer-field-not-known`) and must not also appear in
    /// `narrative_fields` (`config.pointer-narrative-overlap`).
    #[serde(default)]
    pub pointer_fields: Option<Vec<String>>,
    /// Prose-tolerant reference fields with an optional embedded reference
    /// (MILE-0090/ADR-0044) -- resolved the same as `pointer_fields` but
    /// never subject to the clean-format check, and separately checked for
    /// the target's terminal status (`narrative_field_stale`). A type
    /// declaring either `pointer_fields` or `narrative_fields` must declare
    /// both explicitly, even as an empty array
    /// (`config.pointer-declaration-missing`) -- omitting one is not the
    /// same as deliberately declaring zero fields of that kind.
    #[serde(default)]
    pub narrative_fields: Option<Vec<String>>,
    /// Which spec documents this type's schema as a coherent feature area
    /// (MILE-0077/ADR-0041 -- "does this type need a spec" stays editorial,
    /// never inferred). Omitted means none currently covers this type, which
    /// `type.no-declared-spec` surfaces every run.
    ///
    /// Omission is not a way to opt out: there is no
    /// value meaning "decided, none needed" (`ADR-0051`), so the signal
    /// cannot be answered -- only silenced per-repo once rule severity is
    /// configurable (MILE-0080).
    #[serde(default)]
    pub spec: Option<String>,
    /// The field name that plays each fixed relation/status role for this
    /// type (`RFC-42`/`ADR-61`). Every role defaults to its own literal
    /// (`RelationRole::default_field_name`) when omitted, so an existing
    /// config needs no change. A declared name not also present in this
    /// type's `required_fields`/`known_fields` is reported by
    /// `config.relation-field-not-known`, mirroring
    /// `config.pointer-field-not-known`.
    #[serde(default)]
    pub relation_fields: Option<RelationFields>,
    /// Pairs of relation roles (`RelationRole::config_key`) whose fields a
    /// record must carry together (`BUG-41`): `config.companion-declaration-missing`
    /// reports a type declaring one field of a pair without the other. Each
    /// entry is exactly two distinct role keys; a role whose field the type does not declare is
    /// reported by `config.companion-declaration-missing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_fields: Option<Vec<[String; 2]>>,
    /// How this type's revision-log section is found (`MILE-167`): the
    /// `marker` line (default `**Revision log**`) or a `heading`. Omitted means
    /// the default marker, so an existing config needs no change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_log: Option<RevisionLogDecl>,
    /// What this type is for, beyond its fields (`BUG-74`). The one role
    /// today is `suppression`: its records suppress findings (`SPEC-10`). At
    /// most one type declares it. Omitted by every type means the type named
    /// `waiver` is the suppression type (`DEFAULT_SUPPRESSION_TYPE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<TypeRole>,
    /// The fields a suppression must carry before it takes effect
    /// (`BUG-177`), on the type declaring `role: suppression`. A suppression
    /// whose field is absent, blank, a placeholder or pending (the type's
    /// own `statuses.pending` vocabulary) suppresses nothing and is reported
    /// by `waiver.matches-nothing`. Omitted means today's behaviour: only
    /// `Rule` and `Scope` are needed. Every entry must be a field the type
    /// declares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_requires: Option<Vec<String>>,
    /// Where a suppression may be permanent, and on which reasons
    /// (`ADR-86`), on the type declaring `role: suppression`. Once declared,
    /// a suppression without `Expires` takes effect only for a rule listed
    /// here, with a category from that entry's vocabulary. Omitted means a
    /// suppression without `Expires` is allowed for any rule, as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent: Option<Vec<PermanentDecl>>,
}

// Implements: ADR-86 (the declared vocabulary of permanent suppressions)
/// One rule a suppression may be permanent for (`RecordTypeConfig::permanent`).
/// The engine names no category, rule or field: the vocabulary is the
/// repository's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PermanentDecl {
    /// A rule id this build ships.
    pub rule: String,
    /// The suppression field that holds the category.
    pub reason_field: String,
    /// The categories that allow permanence for this rule. Non-empty and
    /// duplicate-free.
    pub categories: Vec<PermanentCategory>,
    /// The pointer field that holds a citation, for a category that requires
    /// one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite_field: Option<String>,
}

/// A category in a `permanent` entry's vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PermanentCategory {
    pub name: String,
    /// `required` means a suppression in this category must cite a record
    /// that resolves. Omitted means no citation is needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<CiteRequirement>,
}

/// Whether a category needs a citation (`PermanentCategory::cite`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CiteRequirement {
    Required,
}

/// A role a record type can declare (`RecordTypeConfig::role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeRole {
    /// Records of this type suppress the findings they name (`SPEC-10`).
    Suppression,
}

/// The type name read as the suppression type when no type declares
/// `role: suppression`, so a config written before the declaration existed
/// keeps its suppression.
pub const DEFAULT_SUPPRESSION_TYPE: &str = "waiver";

/// The declared identity of a type's revision-log section (`MILE-167`):
/// exactly one of `marker` (a whole line, blockquote marks aside, such as
/// `**Revision log**`) or `heading` (the text of a markdown heading of any
/// level, such as `Revision log`).
///
/// `required` (`MILE-159`) says whether `urzua fix --apply` needs the section:
/// `true` (the default) refuses a write that has no log to record itself in,
/// `false` writes the field without a log row. It may stand alone, leaving the
/// identity at its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionLogDecl {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Which revision-log change classes require the record's version field to
    /// change (`BUG-51`). Absent for every type unless declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_bump: Option<VersionBumpDecl>,
    /// The class vocabulary of the log's class column (`ADR-94`). Absent: the
    /// engine default, `DEFAULT_REVISION_LOG_CLASSES`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The header cell that names the class column. Absent: the last cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_column: Option<String>,
    /// The header cell that names the date column. Absent: the first cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_column: Option<String>,
    /// The class `urzua fix` writes. Absent: `DEFAULT_REVISION_LOG_TOOL_CLASS`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_class: Option<String>,
}

/// A type's version-bump requirement: a revision-log row of one of `classes`
/// added to a changed record means the record's `field` must differ from the
/// merge base's. Read by `revision-log.version-bump-required`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VersionBumpDecl {
    /// The header field that carries the version.
    pub field: String,
    /// The change classes (the revision log's Class column, without emphasis)
    /// that require a bump.
    pub classes: Vec<String>,
    /// The version `urzua new` writes into `field` on a new record. Unset, the
    /// field starts blank for a human, as every field `new` cannot fill does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,
}

/// The default marker line: what every repository used before the identity
/// was declared.
pub const DEFAULT_REVISION_LOG_MARKER: &str = "**Revision log**";

/// The default class vocabulary of a revision log, held in the engine so a type
/// that declares no `classes` behaves as every type did before `ADR-94`. This
/// pair and `DEFAULT_REVISION_LOG_TOOL_CLASS` are the only class names the
/// engine knows.
pub const DEFAULT_REVISION_LOG_CLASSES: [&str; 2] = ["substantive", "structural"];

/// The class `urzua fix` writes when a type declares no `tool_class`.
pub const DEFAULT_REVISION_LOG_TOOL_CLASS: &str = "structural";

/// A cell or declared name with emphasis marks and whitespace stripped: how a
/// revision-log class, header cell and declared name are compared (`ADR-94`).
pub fn strip_emphasis(s: &str) -> &str {
    s.trim().trim_matches('*').trim()
}

/// A type's revision-log vocabulary and columns, resolved from its
/// `revision_log` declaration with the engine defaults filled in (`ADR-94`).
/// Every name is already stripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionLogShape {
    /// The accepted classes.
    pub classes: Vec<String>,
    /// The header cell naming the class column; `None` reads the last cell.
    pub class_column: Option<String>,
    /// The header cell naming the date column; `None` reads the first cell.
    pub date_column: Option<String>,
    /// The class `urzua fix` writes.
    pub tool_class: String,
}

impl Default for RevisionLogShape {
    fn default() -> Self {
        RevisionLogShape {
            classes: DEFAULT_REVISION_LOG_CLASSES
                .iter()
                .map(|c| c.to_string())
                .collect(),
            class_column: None,
            date_column: None,
            tool_class: DEFAULT_REVISION_LOG_TOOL_CLASS.to_string(),
        }
    }
}

/// A resolved revision-log identity: how to recognise the line that opens the
/// section. A line matches only when it is that line, not when it mentions the
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionLogId {
    Marker(String),
    Heading(String),
}

impl Default for RevisionLogId {
    fn default() -> Self {
        RevisionLogId::Marker(DEFAULT_REVISION_LOG_MARKER.to_string())
    }
}

impl RevisionLogId {
    /// The text that opens the section: the marker line or the heading text.
    pub fn name(&self) -> &str {
        match self {
            RevisionLogId::Marker(text) | RevisionLogId::Heading(text) => text.trim(),
        }
    }

    /// The identity in words, for a message that names what was looked for.
    pub fn describe(&self) -> String {
        match self {
            RevisionLogId::Marker(m) => format!("the line `{}`", m.trim()),
            RevisionLogId::Heading(h) => format!("a heading `{}`", h.trim()),
        }
    }

    /// Whether `line` opens the section: a marker line equals the marker, a
    /// heading line is `#`..`######`, a space, then the heading text. Leading
    /// blockquote marks and surrounding whitespace are ignored.
    pub fn opens_at(&self, line: &str) -> bool {
        let s = line
            .trim_start_matches(|c: char| c == '>' || c.is_whitespace())
            .trim_end();
        match self {
            RevisionLogId::Marker(m) => s == m.trim(),
            RevisionLogId::Heading(h) => {
                let hashes = s.chars().take_while(|c| *c == '#').count();
                if !(1..=6).contains(&hashes) {
                    return false;
                }
                let rest = &s[hashes..];
                rest.starts_with(char::is_whitespace) && rest.trim() == h.trim()
            }
        }
    }
}

impl RecordTypeConfig {
    /// The revision-log identity this type declares, or the default marker.
    pub fn revision_log_id(&self) -> RevisionLogId {
        match &self.revision_log {
            Some(RevisionLogDecl {
                heading: Some(h), ..
            }) => RevisionLogId::Heading(h.clone()),
            Some(RevisionLogDecl {
                marker: Some(m), ..
            }) => RevisionLogId::Marker(m.clone()),
            _ => RevisionLogId::default(),
        }
    }

    /// The revision-log vocabulary and columns this type declares, with the
    /// engine defaults for what it leaves out (`ADR-94`).
    pub fn revision_log_shape(&self) -> RevisionLogShape {
        let default = RevisionLogShape::default();
        let Some(decl) = &self.revision_log else {
            return default;
        };
        let name = |v: &Option<String>| v.as_deref().map(|v| strip_emphasis(v).to_string());
        RevisionLogShape {
            classes: decl.classes.as_ref().map_or(default.classes, |c| {
                c.iter().map(|c| strip_emphasis(c).to_string()).collect()
            }),
            class_column: name(&decl.class_column),
            date_column: name(&decl.date_column),
            tool_class: name(&decl.tool_class).unwrap_or(default.tool_class),
        }
    }

    /// The version-bump requirement this type declares, if any (`BUG-51`).
    pub fn version_bump(&self) -> Option<&VersionBumpDecl> {
        self.revision_log
            .as_ref()
            .and_then(|d| d.version_bump.as_ref())
    }

    /// Whether this type declares `revision_log: {required: true}`
    /// (`BUG-50`): the opt-in under which a record with no log is a finding.
    /// A type that does not declare it, or declares `false`, reports none.
    pub fn revision_log_declared_required(&self) -> bool {
        self.revision_log
            .as_ref()
            .and_then(|d| d.required)
            .unwrap_or(false)
    }

    /// Whether `urzua fix --apply` needs this type's revision-log section
    /// (`MILE-159`): `true` unless declared `required: false`.
    pub fn revision_log_required(&self) -> bool {
        self.revision_log
            .as_ref()
            .and_then(|d| d.required)
            .unwrap_or(true)
    }
}

impl Config {
    /// The revision-log identity for a record of `record_type` (the default
    /// marker for a type the config does not declare).
    pub fn revision_log_id(&self, record_type: &str) -> RevisionLogId {
        self.record_types
            .get(record_type)
            .map(RecordTypeConfig::revision_log_id)
            .unwrap_or_default()
    }

    /// The revision-log shape for a record of `record_type` (the defaults for a
    /// type the config does not declare).
    pub fn revision_log_shape(&self, record_type: &str) -> RevisionLogShape {
        self.record_types
            .get(record_type)
            .map(RecordTypeConfig::revision_log_shape)
            .unwrap_or_default()
    }

    /// Whether a record of `record_type` with no revision log is a finding
    /// (`BUG-50`): only a type declaring `revision_log: {required: true}`.
    pub fn revision_log_declared_required(&self, record_type: &str) -> bool {
        self.record_types
            .get(record_type)
            .is_some_and(RecordTypeConfig::revision_log_declared_required)
    }

    /// Whether `fix --apply` needs a revision-log section for a record of
    /// `record_type` (`MILE-159`); required for a type the config omits.
    pub fn revision_log_required(&self, record_type: &str) -> bool {
        self.record_types
            .get(record_type)
            .is_none_or(RecordTypeConfig::revision_log_required)
    }
}

/// Keeps a present key's value as written, null included, so a half-written
/// declaration is refused rather than read as absent.
fn keep_raw<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<yaml_serde::Value>, D::Error> {
    yaml_serde::Value::deserialize(d).map(Some)
}

/// Reads the `authority` key. A present key is always `Some` (a null is refused
/// by `AuthorityDecl` itself), so a half-written table is an error and never
/// reads as absent.
fn keep_authority<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<crate::authority::AuthorityDecl>, D::Error> {
    crate::authority::AuthorityDecl::deserialize(d).map(Some)
}

/// A fixed, small role set (`RFC-42`) -- not adopter-extensible like
/// `pointer_fields`, because each role has role-specific behavior elsewhere
/// in the engine (e.g. `embodiment_state`/`embodiment_locator` are read as a
/// pair by `ADR-60`'s gating).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationRole {
    Status,
    EmbodimentState,
    EmbodimentLocator,
    Supersession,
}

impl RelationRole {
    /// Every role, so a validation loop can't silently miss one added later
    /// -- the compiler catches an unmatched variant in `default_field_name`
    /// and `RelationFields::get` below, but not a hand-copied array of role
    /// names, which is exactly what this constant replaces.
    pub const ALL: [RelationRole; 4] = [
        RelationRole::Status,
        RelationRole::EmbodimentState,
        RelationRole::EmbodimentLocator,
        RelationRole::Supersession,
    ];

    /// The pre-`RFC-42` literal this role reads when a type declares no
    /// override.
    pub fn default_field_name(self) -> &'static str {
        match self {
            RelationRole::Status => "Status",
            RelationRole::EmbodimentState => "Embodiment",
            RelationRole::EmbodimentLocator => "Realized-by",
            RelationRole::Supersession => "Supersedes / Superseded-by",
        }
    }

    /// The role declared under `key` in `relation_fields` or `companion_fields`.
    pub fn from_config_key(key: &str) -> Option<RelationRole> {
        RelationRole::ALL
            .into_iter()
            .find(|role| role.config_key() == key)
    }

    /// The config key this role is declared under in `relation_fields`
    /// (`RelationFields`'s own field name).
    pub fn config_key(self) -> &'static str {
        match self {
            RelationRole::Status => "status",
            RelationRole::EmbodimentState => "embodiment_state",
            RelationRole::EmbodimentLocator => "embodiment_locator",
            RelationRole::Supersession => "supersession",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RelationFields {
    pub status: Option<String>,
    pub embodiment_state: Option<String>,
    pub embodiment_locator: Option<String>,
    pub supersession: Option<String>,
    /// The field `urzua new` fills with today's date (`MILE-176`). Not a
    /// relation role: it only names a field, so it is not in `RelationRole`.
    pub date: Option<String>,
    /// The field `urzua new` fills with the author (`MILE-176`).
    pub author: Option<String>,
    /// The field `urzua new` fills with the record's title, for a required field
    /// that restates what the record is about (a spec's `Subject`, `MILE-74`).
    /// Unset, no field takes the title.
    pub title: Option<String>,
}

impl RelationFields {
    /// Every field name this type declares, with the `relation_fields` key it
    /// was declared under: the four relation roles, then `date` and `author`
    /// (`MILE-176`).
    pub fn declared(&self) -> Vec<(&'static str, &str)> {
        RelationRole::ALL
            .into_iter()
            .filter_map(|role| self.get(role).map(|field| (role.config_key(), field)))
            .chain(self.date.as_deref().map(|f| ("date", f)))
            .chain(self.author.as_deref().map(|f| ("author", f)))
            .chain(self.title.as_deref().map(|f| ("title", f)))
            .collect()
    }

    /// The declared override for `role`, or `None` if this type didn't
    /// declare one -- the single match the compiler checks against
    /// `RelationRole::ALL`, in place of a hand-enumerated tuple list.
    pub fn get(&self, role: RelationRole) -> Option<&str> {
        match role {
            RelationRole::Status => self.status.as_deref(),
            RelationRole::EmbodimentState => self.embodiment_state.as_deref(),
            RelationRole::EmbodimentLocator => self.embodiment_locator.as_deref(),
            RelationRole::Supersession => self.supersession.as_deref(),
        }
    }
}

impl RecordTypeConfig {
    /// `ADR-50`: a `none`-shaped type has nowhere for a header field to be at
    /// all, the gate every `config.header-none-has-no-*` rule shares.
    ///
    /// A declared `fields: {from: none}` is the same statement (RFC-33: the
    /// shape enum is one case of `fields.from`), so it gates identically.
    pub fn has_no_header(&self) -> bool {
        self.header_shape == HeaderShape::None
            || self
                .fields
                .as_ref()
                .and_then(|f| f.get("from"))
                .and_then(yaml_serde::Value::as_str)
                == Some("none")
    }

    /// Every field this type declares, `required_fields` **or** `known_fields`
    /// (`BUG-106`): reading either alone silently drops the type that requires
    /// a field instead of merely permitting it.
    /// A `BTreeSet`, not a `HashSet`: this feeds `field_slots` (`rules.rs`)
    /// for `field.untrimmed-value`/`header.field-case-mismatch`'s candidate
    /// order, and `HashSet` iteration order is randomized per process --
    /// unsorted, two runs of the same unchanged corpus could report those
    /// rules' findings in a different relative order.
    pub fn declared_fields(&self) -> std::collections::BTreeSet<crate::values::FieldName> {
        self.required_fields
            .iter()
            .chain(self.known_fields.iter().flatten())
            .map(|s| crate::values::FieldName::new(s.as_str()))
            .collect()
    }

    /// The field `urzua new` fills with today's date, declared under
    /// `relation_fields.date` or defaulting to `Date` (`MILE-176`).
    pub fn date_field(&self) -> &str {
        self.relation_fields
            .as_ref()
            .and_then(|r| r.date.as_deref())
            .unwrap_or("Date")
    }

    /// The field `urzua new` fills with the author, declared under
    /// `relation_fields.author` or defaulting to `Author` (`MILE-176`).
    pub fn author_field(&self) -> &str {
        self.relation_fields
            .as_ref()
            .and_then(|r| r.author.as_deref())
            .unwrap_or("Author")
    }

    /// The field `urzua new` fills with the record's title, when the type
    /// declares one under `relation_fields.title` (`MILE-74`).
    pub fn title_field(&self) -> Option<&str> {
        self.relation_fields
            .as_ref()
            .and_then(|r| r.title.as_deref())
    }

    /// The field and starting value `urzua new` writes for the version, when the
    /// type declares `revision_log.version_bump.initial` (`MILE-74`).
    pub fn initial_version(&self) -> Option<(&str, &str)> {
        let bump = self.version_bump()?;
        Some((bump.field.as_str(), bump.initial.as_deref()?))
    }

    /// The field name this type uses for `role`, declared or defaulted
    /// (`RFC-42`/`ADR-61`). The default is today's pre-`RFC-42` literal, so
    /// an existing config's behavior is unchanged until it opts into a
    /// different name.
    pub fn relation_field(&self, role: RelationRole) -> &str {
        self.relation_fields
            .as_ref()
            .and_then(|r| r.get(role))
            .unwrap_or_else(|| role.default_field_name())
    }
}

/// The load error for a header shape that was removed (MILE-85): it names the
/// form and the one to use, rather than the generic "unrecognized" message.
pub(crate) fn removed_header_shape_message(shape: &str) -> String {
    format!(
        "header shape '{shape}' is no longer supported (removed in MILE-85); rewrite the type's records as YAML frontmatter and declare `yaml-frontmatter` (or `none` for a type with no header)"
    )
}

impl<'de> Deserialize<'de> for HeaderShape {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            removed @ ("blockquote" | "bold-list") => Err(serde::de::Error::custom(
                removed_header_shape_message(removed),
            )),
            "yaml-frontmatter" => Ok(HeaderShape::YamlFrontmatter),
            "none" => Ok(HeaderShape::None),
            other => Err(serde::de::Error::custom(format!(
                "unrecognized header_shape '{other}' -- expected \"yaml-frontmatter\" or \"none\""
            ))),
        }
    }
}

impl Serialize for HeaderShape {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            HeaderShape::YamlFrontmatter => "yaml-frontmatter",
            HeaderShape::None => "none",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not parse config: {0}")]
    Parse(#[from] yaml_serde::Error),
    #[error("{0}")]
    SchemaVersionOutOfWindow(crate::schema_version::SchemaVersionError),
    #[error("could not upgrade the schema_version {from} config: {reason}")]
    UpgradeFailed { from: u32, reason: String },
    #[error("unknown rule '{found}' under `rules:` -- this build ships: {}", known.join(", "))]
    UnknownRule { found: String, known: Vec<String> },
    #[error("rule '{rule}' does not take the option '{option}'")]
    OptionNotApplicable { rule: String, option: String },
    #[error("rule '{rule}' option 'fields' names '{field}', which is not a declared pointer_fields or narrative_fields entry of any record type")]
    UndeclaredRuleField { rule: String, field: String },
    #[error("rule '{rule}' option 'field_levels' {reason}")]
    InvalidFieldLevels { rule: String, reason: String },
    #[error("rule '{rule}' decision gate (`decision_types`, `fix_types`, `link_fields`) {reason}")]
    InvalidDecisionGate { rule: String, reason: String },
    #[error("rule '{rule}' option 'extension_prefixes' entry '{record_type}' {reason}")]
    InvalidExtensionPrefix {
        rule: String,
        record_type: String,
        reason: String,
    },
    #[error("rule '{rule}' requires the option '{option}' -- without it the rule {consequence}")]
    OptionRequired {
        rule: String,
        option: String,
        consequence: &'static str,
    },
    #[error("rule '{rule}' option 'test_paths' entry '{entry}' is not inside any 'back_pointer_paths' entry, so no file under it is scanned and a `Verifies:` comment there would count for nothing")]
    TestPathOutsideBackPointerPaths { rule: String, entry: String },
    #[error("rule '{rule}' option 'sticky_states' names '{state}', which is a state evidence computes; a sticky state is one no evidence can reproduce, and naming a computed one would hide real drift")]
    StickyStateIsComputed { rule: String, state: String },
    #[error("rule '{rule}' option 'pinnable' names '{kind}', which 'locator_kinds' does not declare; a locator of a kind the rule rejects can never be pinned (declared kinds: {declared})")]
    PinnableKindNotDeclared {
        rule: String,
        kind: String,
        declared: String,
    },
    #[error("rule '{rule}' option 'drift_audit_field' {reason}")]
    InvalidDriftAuditField { rule: String, reason: String },
    #[error("rule '{rule}' option '{option}' {reason}")]
    InvalidClaimExpiry {
        rule: String,
        option: &'static str,
        reason: String,
    },
    #[error("embodiment: {reason}")]
    InvalidEmbodiment { reason: String },
    #[error("locator_resolvers: {reason}")]
    InvalidLocatorResolvers { reason: String },
    #[error("record type '{record_type}' companion_fields: {reason}")]
    InvalidCompanionFields { record_type: String, reason: String },
    #[error("relation_pairs: {reason}")]
    InvalidRelationPair { reason: String },
    #[error("roadmap: {reason}")]
    InvalidRoadmap { reason: String },
    #[error("record type '{record_type}' declares prefix '{prefix}', which {reason}")]
    InvalidPrefix {
        record_type: String,
        prefix: String,
        reason: &'static str,
    },
    #[error("record types '{first}' and '{second}' both use the prefix '{prefix}' -- `urzua new` would mint colliding identities")]
    DuplicatePrefix {
        prefix: String,
        first: String,
        second: String,
    },
    #[error("record type '{record_type}' declares dir '{dir}', which names no directory below the repository root -- the type would examine nothing")]
    InvalidDir { record_type: String, dir: String },
    #[error("record type '{record_type}' declares dir '{dir}', which is not in canonical form -- write '{canonical}'")]
    NonCanonicalDir {
        record_type: String,
        dir: String,
        canonical: String,
    },
    #[error("record type '{record_type}' declares directory '{dir}' more than once -- list each directory once")]
    DuplicateDir { record_type: String, dir: String },
    #[error("record types '{first}' and '{second}' both own directory '{dir}' -- a directory holds one type's records")]
    SharedDir {
        dir: String,
        first: String,
        second: String,
    },
    #[error("record type '{record_type}' declares exclude_files entry '{entry}', which is not a bare file name -- it would exclude nothing")]
    InvalidExcludedFile { record_type: String, entry: String },
    #[error("record types '{first}' and '{second}' both declare `role: {role}` -- one type suppresses findings")]
    DuplicateRole {
        role: &'static str,
        first: String,
        second: String,
    },
    #[error("record type '{record_type}' declares effective_requires {reason}")]
    InvalidEffectiveRequires { record_type: String, reason: String },
    #[error("record type '{record_type}' declares permanent {reason}")]
    InvalidPermanent { record_type: String, reason: String },
    #[error("record type '{record_type}' declares revision_log {reason}")]
    InvalidRevisionLog { record_type: String, reason: String },
    #[error("rule '{rule}' option '{option}' entry '{entry}' {reason}")]
    InvalidPublishedPath {
        rule: String,
        option: &'static str,
        entry: String,
        reason: &'static str,
    },
    #[error("rule '{rule}' option '{option}' {reason}")]
    InvalidBoundary {
        rule: String,
        option: &'static str,
        reason: String,
    },
    #[error("rule '{rule}' option 'indexes' entry '{path}' {reason}")]
    InvalidIndex {
        rule: String,
        path: String,
        reason: String,
    },
    #[error("rule '{rule}' option 'issue_refs' {reason}")]
    InvalidIssueRefs { rule: String, reason: String },
    #[error("rule '{rule}' option 'date_order' entry '{entry}' {reason}")]
    InvalidDateOrder {
        rule: String,
        entry: String,
        reason: String,
    },
    #[error("rule '{rule}' option 'date_fields' entry '{entry}' {reason}")]
    InvalidDateFields {
        rule: String,
        entry: String,
        reason: String,
    },
    #[error("rule '{rule}' option 'non_reference_field' {reason}")]
    InvalidNonReferenceField { rule: String, reason: String },
    #[error("rule '{rule}' option '{option}' {reason}")]
    InvalidClaimField {
        rule: String,
        option: &'static str,
        reason: String,
    },
    #[error("rule '{rule}' option 'external_ids' entry '{entry}' {reason}")]
    InvalidExternalId {
        rule: String,
        entry: String,
        reason: String,
    },
    #[error("record type '{record_type}' declares an invalid `{key}`: {reason}")]
    InvalidDocumentModel {
        record_type: String,
        key: &'static str,
        reason: String,
    },
}

/// Why a revision-log declaration's identity (`marker` or `heading`) cannot be
/// used, or `None`.
fn revision_log_identity_problem(decl: &RevisionLogDecl) -> Option<&'static str> {
    match (&decl.marker, &decl.heading) {
        (None, None)
            if decl.required.is_some()
                || decl.version_bump.is_some()
                || decl.classes.is_some()
                || decl.class_column.is_some()
                || decl.date_column.is_some()
                || decl.tool_class.is_some() =>
        {
            None
        }
        (None, None) => Some("neither `marker` nor `heading` -- name one"),
        (Some(_), Some(_)) => Some("both `marker` and `heading` -- name exactly one"),
        (Some(t), None) | (None, Some(t)) if t.trim().is_empty() => {
            Some("a blank identity -- it would match no line")
        }
        (Some(t), None) | (None, Some(t)) if t.contains('\n') => {
            Some("an identity spanning lines -- it would match no line")
        }
        _ => None,
    }
}

/// Why a type's `revision_log` declaration cannot be used, or `None`: the
/// identity (`MILE-167`), the version bump (`BUG-51`) and the vocabulary,
/// columns and tool class (`ADR-94`).
fn revision_log_problem(decl: &RevisionLogDecl) -> Option<String> {
    let identity = revision_log_identity_problem(decl);
    if let Some(reason) = identity {
        return Some(reason.to_string());
    }
    if let Some(bump) = &decl.version_bump {
        if bump.field.trim().is_empty() {
            return Some("a `version_bump` with a blank `field` -- it names no field".into());
        } else if bump.classes.is_empty() {
            return Some("a `version_bump` with no `classes` -- it would require nothing".into());
        } else if bump.classes.iter().any(|c| c.trim().is_empty()) {
            return Some("a `version_bump` with a blank class -- it would match no row".into());
        } else if bump.initial.as_deref().is_some_and(|v| v.trim().is_empty()) {
            return Some(
                "a `version_bump` with a blank `initial` -- it would start every record blank"
                    .into(),
            );
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    if let Some(classes) = &decl.classes {
        if classes.is_empty() {
            return Some("an empty `classes` -- it would accept no row".into());
        }
        for class in classes {
            let class = strip_emphasis(class);
            if class.is_empty() {
                return Some("a blank class in `classes` -- it would match no row".into());
            }
            if seen.contains(&class) {
                return Some(format!(
                    "`classes` naming `{class}` twice (names are compared after stripping emphasis and whitespace)"
                ));
            }
            seen.push(class);
        }
        if let Some(bump) = &decl.version_bump {
            if let Some(out) = bump
                .classes
                .iter()
                .find(|c| !seen.contains(&strip_emphasis(c)))
            {
                return Some(format!(
                    "a `version_bump` class `{}` that is not in `classes`",
                    strip_emphasis(out)
                ));
            }
        }
    }
    for (key, value) in [
        ("class_column", &decl.class_column),
        ("date_column", &decl.date_column),
        ("tool_class", &decl.tool_class),
    ] {
        if value
            .as_deref()
            .is_some_and(|v| strip_emphasis(v).is_empty())
        {
            return Some(format!("a blank `{key}` -- it names nothing"));
        }
    }
    if let (Some(class), Some(date)) = (&decl.class_column, &decl.date_column) {
        if strip_emphasis(class) == strip_emphasis(date) {
            return Some(format!(
                "`class_column` and `date_column` both naming `{}` -- they must be different headers",
                strip_emphasis(class)
            ));
        }
    }
    if decl.classes.is_some() {
        match decl.tool_class.as_deref().map(strip_emphasis) {
            Some(tool) if !seen.contains(&tool) => {
                return Some(format!("a `tool_class` `{tool}` that is not in `classes`"));
            }
            None if !seen.contains(&DEFAULT_REVISION_LOG_TOOL_CLASS) => {
                return Some(format!(
                    "`classes` without `{DEFAULT_REVISION_LOG_TOOL_CLASS}` and no `tool_class` -- name the class `urzua fix` writes"
                ));
            }
            _ => {}
        }
    }
    None
}

/// Why a declared `prefix` cannot head a recognized `PREFIX-N-slug.md`
/// filename, or `None` when it can. Reuses the filename grammar
/// (`parse_record_filename`): hyphen-separated segments, each non-empty
/// upper-case/digits and not all digits (an all-digit segment would be read
/// as the record number).
fn prefix_problem(prefix: &str) -> Option<&'static str> {
    if prefix.is_empty() {
        return Some("is empty");
    }
    for seg in prefix.split('-') {
        if seg.is_empty() {
            return Some(
                "has an empty hyphen-separated segment (leading, trailing or doubled hyphen)",
            );
        }
        if crate::new_record::is_digit_segment(seg) {
            return Some("has an all-digit segment, which would be read as the record number");
        }
        if !crate::new_record::is_prefix_segment(seg) {
            return Some(
                "must be upper-case ASCII letters and digits in hyphen-separated segments",
            );
        }
    }
    None
}

/// True when `dir` has no real path component (`""`, `"."`, `"./"`): it
/// resolves to the repository root, which no record type can govern.
fn dir_names_nothing(dir: &str) -> bool {
    !std::path::Path::new(dir.trim())
        .components()
        .any(|c| matches!(c, std::path::Component::Normal(_)))
        && !dir.trim().starts_with('/')
        && dir.split('/').all(|p| p.trim() == "" || p == ".")
}

/// True when `dir` is absolute or has a `..` component: it would point
/// `urzua new` outside the repository, where discovery never looks.
fn dir_escapes_repo(dir: &str) -> bool {
    let path = std::path::Path::new(dir.trim());
    dir.trim().starts_with('/')
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

/// The error for one option of an `embodiment.claim-expired` declaration.
fn claim_expiry_error(rule: &str, option: &'static str, reason: String) -> ConfigError {
    ConfigError::InvalidClaimExpiry {
        rule: rule.to_string(),
        option,
        reason,
    }
}

/// One `periods` entry of an `embodiment.claim-expired` declaration: a declared
/// type, a positive period, a real level, status list and tier, a type that
/// carries the locator field, and (with an anchor) a type that declares it.
fn claim_period_problem(
    bad: &dyn Fn(&'static str, String) -> ConfigError,
    setting: &RuleSetting,
    config: &Config,
    n: usize,
    entry: &ClaimPeriod,
) -> Result<(), ConfigError> {
    let tiers: Vec<&str> = config.embodiment.evidence_states().collect();
    let at = format!("entry {n} (type '{}')", entry.record_type);
    let Some(type_config) = config.record_types.get(&entry.record_type) else {
        return Err(bad(
            "periods",
            format!("{at} names a record type that is not declared"),
        ));
    };
    if entry.days == 0 {
        return Err(bad(
            "periods",
            format!("{at} has days 0; a period is 1 day or more"),
        ));
    }
    if entry.level == Some(RuleLevel::Off) {
        return Err(bad(
            "periods",
            format!("{at} has the level 'off'; use 'info', 'warn' or 'error' (an entry is removed to switch it off)"),
        ));
    }
    if let Some(statuses) = &entry.status {
        if statuses.is_empty() {
            return Err(bad("periods", format!("{at} has an empty status list")));
        }
        let declared = type_config.status_decl().map(|d| d.values);
        for status in statuses {
            if !declared.as_ref().is_some_and(|v| v.contains(status)) {
                return Err(bad(
                    "periods",
                    format!("{at} names the status '{status}', which the type does not declare in statuses.values"),
                ));
            }
        }
    }
    if let Some(tier) = &entry.tier {
        if !tiers.contains(&tier.as_str()) {
            return Err(bad(
                "periods",
                format!(
                    "{at} names the tier '{tier}', which is not a state of the embodiment declaration ({})",
                    tiers.join(", ")
                ),
            ));
        }
    }
    let locator_field = type_config.relation_field(RelationRole::EmbodimentLocator);
    let declares = |field: &str| {
        type_config
            .declared_fields()
            .iter()
            .any(|f| f.as_str() == field)
    };
    if !declares(locator_field) {
        return Err(bad(
            "periods",
            format!("{at} names a type that declares no '{locator_field}' field, so it has no claim to expire"),
        ));
    }
    if let Some(anchor) = setting.anchor_field.as_deref() {
        if !declares(anchor.trim()) {
            return Err(bad(
                "anchor_field",
                format!("names '{anchor}', which {at} does not declare (required_fields or known_fields)"),
            ));
        }
    }
    Ok(())
}

/// No two `periods` entries of one type are equally specific and able to match
/// the same record.
fn claim_periods_overlap(
    bad: &dyn Fn(&'static str, String) -> ConfigError,
    periods: &[ClaimPeriod],
) -> Result<(), ConfigError> {
    for (i, a) in periods.iter().enumerate() {
        for b in &periods[i + 1..] {
            let rank =
                |p: &ClaimPeriod| usize::from(p.status.is_some()) + usize::from(p.tier.is_some());
            let statuses_overlap = match (&a.status, &b.status) {
                (Some(x), Some(y)) => x.iter().any(|s| y.contains(s)),
                _ => true,
            };
            let tiers_overlap = match (&a.tier, &b.tier) {
                (Some(x), Some(y)) => x == y,
                _ => true,
            };
            if a.record_type == b.record_type
                && rank(a) == rank(b)
                && statuses_overlap
                && tiers_overlap
            {
                return Err(bad(
                "periods",
                format!(
                    "has two entries for type '{}' of equal specificity that can match the same record; narrow one with a status or a tier",
                    a.record_type
                ),
            ));
            }
        }
    }
    Ok(())
}

/// Why an `embodiment.claim-expired` declaration cannot work (`ADR-97`): every
/// entry, the pin categories, the batch size and the anchor are checked at load
/// so a declaration that matches nothing or reads nothing is refused, never
/// silently ignored.
fn claim_expiry_problems(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    let bad = |option: &'static str, reason: String| claim_expiry_error(name, option, reason);
    if config
        .rules
        .get(crate::rules::RULE_EMBODIMENT_CONSISTENCY)
        .is_some_and(|s| s.level != RuleLevel::Off && s.claim_max_age_days.is_some())
    {
        return Err(bad(
            "periods",
            format!(
                "is declared together with {}'s claim_max_age_days: both would report the same claim twice, so declare one",
                crate::rules::RULE_EMBODIMENT_CONSISTENCY
            ),
        ));
    }
    let periods = setting.periods.as_deref().unwrap_or_default();
    if periods.is_empty() {
        return Err(bad(
            "periods",
            "is missing or empty: the rule would expire nothing; give at least one entry"
                .to_string(),
        ));
    }
    for (n, entry) in periods.iter().enumerate() {
        claim_period_problem(&bad, setting, config, n, entry)?;
    }
    if setting
        .anchor_field
        .as_deref()
        .is_some_and(|a| a.trim().is_empty())
    {
        return Err(bad("anchor_field", "is blank".to_string()));
    }
    claim_periods_overlap(&bad, periods)?;
    let categories: Vec<&str> = config
        .embodiment
        .tiers
        .iter()
        .map(|t| t.evidence.as_str())
        .collect();
    for category in setting.require_pin.iter().flatten() {
        if !categories.contains(&category.trim()) {
            return Err(bad(
                "require_pin",
                format!(
                    "names the category '{category}', which the embodiment declaration does not use ({})",
                    categories.join(", ")
                ),
            ));
        }
    }
    if setting.max_batch == Some(0) {
        return Err(bad(
            "max_batch",
            "is 0; a batch is 1 record or more".to_string(),
        ));
    }
    Ok(())
}

/// Whether no record type needs the fallback status lists: each type either
/// declares `statuses`, or declares no status field a rule could read (a
/// suppression type with no `Status`). Such a type is never judged by a list.
fn no_type_needs_the_status_fallback(config: &Config) -> bool {
    !config.record_types.is_empty()
        && config.record_types.values().all(|t| {
            t.status_decl().is_some()
                || !t.declared_fields().contains(&crate::values::FieldName::new(
                    t.relation_field(RelationRole::Status),
                ))
        })
}

/// One list-valued option's name and what happens if a rule that accepts it is
/// declared without it. Which rules accept an option is the rule table's
/// (`RuleDef::options`), so adding an option-taking rule is a table entry plus
/// an entry here for a new option, not a second list of owners.
struct RuleOption {
    name: &'static str,
    consequence_if_missing: &'static str,
    /// An optional option may be omitted, but a declared one is still
    /// validated (non-empty, no blanks) so it cannot be a silent no-op.
    optional: bool,
    /// Needed only while some record type declares a status field but no
    /// `statuses`: a type that declares them is judged by its own success and
    /// failure sets, so this list is a fallback that is never read once no
    /// type lacks them (`BUG-178`). A declared list is still validated.
    fallback_for_undeclared_statuses: bool,
}

const RULE_OPTIONS: &[RuleOption] = &[
    RuleOption {
        name: "not_in",
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "claim_paths",
        consequence_if_missing: "scans nothing and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "closed_statuses",
        consequence_if_missing: "treats every claim as a violation, including correct ones",
        optional: false,
        fallback_for_undeclared_statuses: true,
    },
    RuleOption {
        name: "terminal_statuses",
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
        fallback_for_undeclared_statuses: true,
    },
    // Optional on its own: a rule declares `boundaries`, `published_paths` or
    // both, and a rule declaring neither is refused by the boundary check below.
    RuleOption {
        name: "published_paths",
        consequence_if_missing: "declares no boundary and can never report",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "source_paths",
        consequence_if_missing: "declares no source and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "application_paths",
        consequence_if_missing: "declares no application source and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "exclude_paths",
        consequence_if_missing: "excludes nothing from the source",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "back_pointer_paths",
        consequence_if_missing: "scans no source file and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "test_paths",
        consequence_if_missing: "reads no `Verifies:` comment as test evidence",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "sticky_states",
        consequence_if_missing: "holds every stated Embodiment to the computed value",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "locator_kinds",
        consequence_if_missing: "accepts only a file locator",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "pinnable",
        consequence_if_missing: "lets only a file locator carry a pin",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "date_fields",
        consequence_if_missing: "names no date field and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "date_order",
        consequence_if_missing: "names no date ordering and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "accepted_statuses",
        consequence_if_missing: "leaves a type with no declared success statuses (implementable statuses for the back-pointer gate) unjudged",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "reference_sections",
        consequence_if_missing: "reads no section and can never report",
        optional: false,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "external_ids",
        consequence_if_missing: "resolves every prefixed id as a record, including external standard numbers",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
    RuleOption {
        name: "fields",
        consequence_if_missing: "checks every declared field",
        optional: true,
        fallback_for_undeclared_statuses: false,
    },
];

/// A list-valued option's declared entries, by name, read from the setting's
/// own serialized form so no second per-field table exists: an option the
/// setting does not carry is `None`, one it carries is its entries. A
/// structured entry (`ADR-64`, `date_order`'s `{earlier, later}`) is presented
/// as its string values, so the shared "non-empty, no blank value" check reads
/// it too.
fn option_field(name: &str, setting: &RuleSetting) -> Option<Vec<String>> {
    let yaml_serde::Value::Mapping(map) = yaml_serde::to_value(setting).ok()? else {
        return None;
    };
    let yaml_serde::Value::Sequence(entries) = map.get(name)? else {
        return None;
    };
    Some(
        entries
            .iter()
            .flat_map(|entry| match entry {
                yaml_serde::Value::String(s) => vec![s.clone()],
                yaml_serde::Value::Mapping(m) => m
                    .values()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            })
            .collect(),
    )
}

/// Rules that cannot be declared without their options. Adopt mode skips these
/// rather than proposing a declaration that will not load (BUG-44).
pub fn rule_requires_options(rule: &str) -> bool {
    // The companion of the expiry rule is refused without the expiry rule
    // (`ADR-97`), which is itself refused without `periods`.
    rule == crate::rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED
        || crate::rules::rule_def(rule).is_some_and(|d| {
            d.options.iter().any(|o| {
                *o == "indexes" || *o == "periods" || RULE_OPTIONS.iter().any(|r| r.name == *o)
            })
        })
}

/// Why a decision-gate declaration (`MILE-133`) cannot work, or `None` when it
/// can. Each declaration names records or fields the configuration must have,
/// because one that names nothing real would leave the gate silently open.
fn decision_gate_problem(setting: &RuleSetting, config: &Config) -> Option<String> {
    let Some(decisions) = &setting.decision_types else {
        let orphan = if setting.fix_types.is_some() {
            Some("fix_types")
        } else if setting.link_fields.is_some() {
            Some("link_fields")
        } else {
            None
        };
        return orphan.map(|o| {
            format!("declares '{o}' without 'decision_types': there is no decision to exempt a change from or to link, so it would never apply")
        });
    };
    if decisions.is_empty() {
        return Some(
            "declares an empty 'decision_types'; name a type or omit the option".to_string(),
        );
    }
    for (type_name, statuses) in decisions {
        let Some(record_type) = config.record_types.get(type_name) else {
            return Some(format!(
                "'decision_types' names '{type_name}', which is not a declared record type"
            ));
        };
        if statuses.is_empty() || statuses.iter().any(|s| s.trim().is_empty()) {
            return Some(format!(
                "'decision_types' gives '{type_name}' an empty or blank status list; name the statuses at which one counts as decided"
            ));
        }
        if let Some(decl) = record_type.status_decl() {
            if let Some(unknown) = statuses.iter().find(|s| !decl.values.contains(s)) {
                return Some(format!(
                    "'decision_types' names status '{unknown}' for '{type_name}', which that type's `statuses.values` does not list, so it could never match"
                ));
            }
        }
    }
    for type_name in setting.fix_types.iter().flatten() {
        if !config.record_types.contains_key(type_name) {
            return Some(format!(
                "'fix_types' names '{type_name}', which is not a declared record type"
            ));
        }
        if decisions.contains_key(type_name) {
            return Some(format!(
                "names '{type_name}' in both 'decision_types' and 'fix_types'; a record cannot be both a decision and a fix"
            ));
        }
    }
    if setting.fix_types.as_ref().is_some_and(Vec::is_empty) {
        return Some("declares an empty 'fix_types'; name a type or omit the option".to_string());
    }
    if setting.link_fields.as_ref().is_some_and(Vec::is_empty) {
        return Some(
            "declares an empty 'link_fields'; name a field or omit the option".to_string(),
        );
    }
    for field in setting.link_fields.iter().flatten() {
        let relational = config.record_types.values().any(|t| {
            t.pointer_fields.iter().flatten().any(|f| f == field)
                || t.narrative_fields.iter().flatten().any(|f| f == field)
        });
        if !relational {
            return Some(format!(
                "'link_fields' names '{field}', which is not a declared pointer_fields or narrative_fields entry of any record type"
            ));
        }
    }
    None
}

/// Why an `issue_refs` declaration cannot work, or `None` when it can.
fn issue_refs_problem(refs: &IssueRefs) -> Option<String> {
    let snapshot = refs.snapshot.trim();
    if snapshot.is_empty() {
        return Some("has an empty snapshot path".to_string());
    }
    if let Some(reason) = published_path_problem(snapshot) {
        return Some(format!("snapshot path {reason}"));
    }
    if snapshot.ends_with('/') {
        return Some("snapshot path names a directory, not a file".to_string());
    }
    let mut parts = refs.repository.split('/');
    let valid = |p: &str| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    match (parts.next(), parts.next(), parts.next()) {
        (Some(o), Some(n), None) if valid(o) && valid(n) => {}
        _ => {
            return Some(format!(
                "repository '{}' is not of the form owner/name",
                refs.repository
            ))
        }
    }
    if refs.forms.is_empty() {
        return Some("declares no forms; give 'number', 'url' or both".to_string());
    }
    if let Some(fields) = &refs.fields {
        if fields.is_empty() || fields.iter().any(|f| f.trim().is_empty()) {
            return Some("declares an empty or blank 'fields' entry".to_string());
        }
        let mut names: Vec<&String> = fields.iter().collect();
        names.sort();
        names.dedup();
        if names.len() != fields.len() {
            return Some("names a field twice in 'fields'".to_string());
        }
    }
    let mut seen = refs.forms.clone();
    seen.sort();
    seen.dedup();
    if seen.len() != refs.forms.len() {
        return Some("declares a form twice".to_string());
    }
    None
}

/// Why an `indexes` entry cannot be compared, or `None` when it can. Every
/// reason is a config error: an entry that cannot be matched would be a rule
/// that never reports.
fn index_problem(spec: &IndexSpec, config: &Config) -> Option<String> {
    let path = spec.path.trim();
    if path.is_empty() {
        return Some("has an empty path".to_string());
    }
    if let Some(reason) = published_path_problem(path) {
        return Some(format!("path {reason}"));
    }
    if !config.record_types.contains_key(&spec.record_type) {
        return Some(format!(
            "names the type '{}', which is not a declared record type",
            spec.record_type
        ));
    }
    match crate::rules::parse_index_format(&spec.format) {
        Err(reason) => return Some(format!("has an invalid format: {reason}")),
        Ok(lines) => {
            // A `{Field}` placeholder must name a field the type declares: a
            // typo would otherwise fill nothing and compare nothing (MILE-164).
            let declared = config.record_types[&spec.record_type].declared_fields();
            for segment in lines.iter().flatten() {
                if let crate::rules::FormatSegment::Field(name) = segment {
                    if !declared.contains(&crate::values::FieldName::new(name.as_str())) {
                        return Some(format!(
                            "has an invalid format: the placeholder '{{{name}}}' is neither {{id}}, {{file}}, {{title}} nor a declared field (required_fields or known_fields) of the type '{}'",
                            spec.record_type
                        ));
                    }
                }
            }
        }
    }
    match (&spec.begin, &spec.end) {
        (None, None) => None,
        (Some(b), Some(e)) if !b.trim().is_empty() && !e.trim().is_empty() => {
            if b.trim() == e.trim() {
                Some("declares the same begin and end marker".to_string())
            } else {
                None
            }
        }
        (Some(_), Some(_)) => Some("declares a blank marker".to_string()),
        _ => Some("declares only one of begin and end; give both or neither".to_string()),
    }
}

/// A `published_paths` entry in the one form the rule compares: whitespace
/// trimmed, any leading `./` and trailing `/` removed. Entries are path
/// prefixes, never globs, so a `*` is kept literally.
pub fn normalize_published_path(entry: &str) -> String {
    let mut s = entry.trim();
    loop {
        let before = s;
        s = s.strip_prefix("./").unwrap_or(s);
        s = s.strip_suffix('/').unwrap_or(s);
        if s == before {
            break;
        }
    }
    s.to_string()
}

/// Why a `published_paths` entry cannot name a path inside the repository.
pub(crate) fn published_path_problem(entry: &str) -> Option<&'static str> {
    let trimmed = entry.trim();
    if trimmed.contains('\\') {
        return Some("contains a backslash; write the path with forward slashes");
    }
    if trimmed.starts_with('/') {
        return Some("is absolute; write a path relative to the repository root");
    }
    if trimmed.split('/').any(|c| c == "..") {
        return Some("contains a `..` component; a published path must stay inside the repository");
    }
    None
}

/// The `schema_version` a config declares, read on its own: the probe skips
/// every other key without building a value for it, so a config that reads
/// today (an unquoted integer beyond `u64` read as a string, say) is read
/// exactly as it was, and only a config that needs the upgrade is parsed whole.
fn declared_version(content: &str) -> Result<Option<u64>, ConfigError> {
    #[derive(Deserialize)]
    struct VersionProbe {
        #[serde(default)]
        schema_version: Option<yaml_serde::Value>,
    }
    let probe: VersionProbe = yaml_serde::from_str(content)?;
    Ok(probe.schema_version.and_then(|v| v.as_u64()))
}

/// Reads the typed config, upgrading it first when it is at the previous
/// schema version and rejecting it when it is outside the window (`ADR-77`).
/// A config that declares no integer `schema_version` falls through to the
/// typed parse, whose error names the field.
fn read_in_window(content: &str) -> Result<Config, ConfigError> {
    let Some(found) = declared_version(content)? else {
        return Ok(yaml_serde::from_str(content)?);
    };
    if found == u64::from(CURRENT_SCHEMA_VERSION) {
        return Ok(yaml_serde::from_str(content)?);
    }
    if found != u64::from(crate::schema_version::OLDEST_SUPPORTED_SCHEMA_VERSION) {
        return Err(ConfigError::SchemaVersionOutOfWindow(
            crate::schema_version::SchemaVersionError::new(found),
        ));
    }
    let from = crate::schema_version::OLDEST_SUPPORTED_SCHEMA_VERSION;
    let doc: yaml_serde::Value = yaml_serde::from_str(content)?;
    let (upgraded, changes) = crate::schema_version::upgrade(doc)
        .map_err(|reason| ConfigError::UpgradeFailed { from, reason })?;
    let mut config: Config = yaml_serde::from_value(upgraded)?;
    config.upgraded_from = Some(crate::schema_version::UpgradeInfo { from, changes });
    Ok(config)
}

/// The one spelling of a repository-relative directory: surrounding
/// whitespace, `.` and empty components, and a trailing `/` removed.
fn canonical_dir(dir: &str) -> String {
    dir.trim()
        .split('/')
        .filter(|c| !c.is_empty() && *c != ".")
        .collect::<Vec<_>>()
        .join("/")
}

pub fn parse(content: &str) -> Result<Config, ConfigError> {
    parse_with_document_models(content).map(|(config, _)| config)
}

/// Why a back-pointer `ignore_marker` is too generic to excuse one line, or `None`.
/// The marker excuses its whole line, so text that sits on every back-pointer line
/// would excuse them all: under three non-whitespace characters, no letter or digit
/// (comment leaders and punctuation such as `//`, `#`), or part of a declared record
/// id head (`ADR-` for a type whose prefix is `ADR`).
fn generic_marker_problem(config: &Config, marker: &str) -> Option<String> {
    if marker.chars().filter(|c| !c.is_whitespace()).count() < 3 {
        return Some(format!(
            "'{marker}' is shorter than 3 non-whitespace characters and would match almost every back-pointer line; declare a specific text"
        ));
    }
    if !marker.chars().any(char::is_alphanumeric) {
        return Some(format!(
            "'{marker}' is only comment leaders or punctuation and would match every back-pointer line; declare text with letters or digits"
        ));
    }
    crate::rules::declared_prefixes(config)
        .into_iter()
        .map(|prefix| format!("{prefix}-"))
        .find(|head| head.contains(marker))
        .map(|head| {
            format!("'{marker}' is part of the record id head '{head}' and would match every citation of that type; declare text that appears in no id")
        })
}

/// The declared `ignore_marker` (`ADR-80`): only the boundary rule and the two
/// back-pointer rules take one; it is one non-blank line with no padding; on a
/// back-pointer rule it is not part of a back-pointer keyword (it would sit in
/// every citation and excuse them all); and the two back-pointer rules, which
/// share one marker with the evidence collector, may not declare different ones.
fn validate_ignore_markers(config: &Config) -> Result<(), ConfigError> {
    let bad = |rule: &str, reason: String| ConfigError::InvalidBoundary {
        rule: rule.to_string(),
        option: "ignore_marker",
        reason,
    };
    let keywords = [
        crate::rules::MARKER_IMPLEMENTS,
        crate::rules::MARKER_VERIFIES,
    ];
    let back_pointer_rules = [
        crate::rules::RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        crate::rules::RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
    ];
    let mut declared: Vec<(&str, &str)> = Vec::new();
    for (name, setting) in &config.rules {
        let Some(marker) = setting.ignore_marker.as_deref() else {
            continue;
        };
        if !crate::rules::rule_accepts_option(name, "ignore_marker") {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "ignore_marker".to_string(),
            });
        }
        if setting.level == RuleLevel::Off {
            continue;
        }
        if marker.trim().is_empty() || marker.trim() != marker || marker.contains('\n') {
            return Err(bad(
                name,
                "must be non-blank text on one line with no surrounding whitespace".to_string(),
            ));
        }
        if back_pointer_rules.contains(&name.as_str()) {
            if let Some(reason) = generic_marker_problem(config, marker) {
                return Err(bad(name, reason));
            }
            if let Some(k) = keywords.iter().find(|k| k.contains(marker)) {
                return Err(bad(
                    name,
                    format!("'{marker}' is part of the back-pointer keyword '{k}' and would excuse every back-pointer; declare text that appears in no keyword"),
                ));
            }
            declared.push((name, marker));
        }
    }
    if let [(a, x), (b, y)] = declared.as_slice() {
        if x != y {
            return Err(bad(
                a,
                format!("is '{x}' but '{b}' declares '{y}'; the two back-pointer rules share one marker, declare the same text on both or on one"),
            ));
        }
    }
    Ok(())
}

/// `parse`, plus each record type's declared document model (RFC-33 layer 1,
/// MILE-98). The declaration is validated here, so every command that loads a
/// config rejects a bad one. The raw declarations ride on `RecordTypeConfig`
/// through the one typed parse, so a config reads, and errors, exactly as it
/// did before the declaration existed; the validated models are returned
/// beside `Config`.
pub fn parse_with_document_models(
    content: &str,
) -> Result<(Config, BTreeMap<String, DocumentModel>), ConfigError> {
    let mut config = read_in_window(content)?;
    let models = crate::document_model::validate_declarations(&config.record_types)?;
    config
        .embodiment
        .validate()
        .map_err(|reason| ConfigError::InvalidEmbodiment { reason })?;
    crate::locator::validate_resolvers(&config.locator_resolvers)
        .map_err(|reason| ConfigError::InvalidLocatorResolvers { reason })?;
    let status_decls = crate::status_model::validate(&config.record_types, &config.embodiment)?;
    crate::status_model::validate_items_rule(&config.record_types, &status_decls, &config.rules)?;
    validate_relation_pairs(&config)?;
    validate_roadmap(&config)?;
    validate_companion_fields(&config)?;
    validate_record_types(&config)?;
    validate_rule_names(&config)?;
    validate_index_options(&config)?;
    validate_issue_refs_options(&config)?;
    validate_rule_settings(&config)?;
    normalize_rule_paths(&mut config)?;
    validate_boundaries(&mut config)?;
    validate_external_ids(&config)?;
    validate_claim_fields(&config)?;
    validate_non_reference_fields(&config)?;
    validate_test_paths(&config)?;
    validate_rule_fields(&config)?;
    Ok((config, models))
}

/// Record types: the values that reach filenames and discovery (directories,
/// prefixes, `requires`, `permanent`, excluded files, the revision-log declaration).
fn validate_record_types(config: &Config) -> Result<(), ConfigError> {
    // Values that reach filenames and discovery: validated here, once, so
    // every command (and `doctor`, which loads through this) agrees.
    let mut seen_prefix: std::collections::BTreeMap<String, &String> = Default::default();
    let mut seen_dir: std::collections::BTreeMap<std::path::PathBuf, &String> = Default::default();
    let mut suppression_type: Option<&String> = None;
    for (name, rt) in &config.record_types {
        validate_type_dirs(name, rt, &mut seen_dir)?;
        validate_effective_requires(name, rt)?;
        if let Some(entries) = &rt.permanent {
            if let Some(reason) = permanent_problem(rt, entries) {
                return Err(ConfigError::InvalidPermanent {
                    record_type: name.clone(),
                    reason,
                });
            }
        }
        if rt.role == Some(TypeRole::Suppression) {
            if let Some(first) = suppression_type.replace(name) {
                return Err(ConfigError::DuplicateRole {
                    role: "suppression",
                    first: first.clone(),
                    second: name.clone(),
                });
            }
        }
        for entry in &rt.exclude_files {
            let bare = !entry.is_empty()
                && entry != "."
                && entry != ".."
                && !entry.contains(['/', '\\', '*', '?', '[']);
            if !bare {
                return Err(ConfigError::InvalidExcludedFile {
                    record_type: name.clone(),
                    entry: entry.clone(),
                });
            }
        }
        if let Some(decl) = &rt.revision_log {
            if let Some(reason) = revision_log_problem(decl) {
                return Err(ConfigError::InvalidRevisionLog {
                    record_type: name.clone(),
                    reason,
                });
            }
        }
        // The effective prefix, not only a declared one: a type named
        // `my_type` with no `prefix` defaults to `MY_TYPE`, which no filename
        // can carry, so `new` would mint records the numbering scan ignores.
        let effective = rt
            .prefix
            .clone()
            .unwrap_or_else(|| name.to_ascii_uppercase());
        if let Some(reason) = prefix_problem(&effective) {
            return Err(ConfigError::InvalidPrefix {
                record_type: name.clone(),
                prefix: effective,
                reason,
            });
        }
        if let Some(first) = seen_prefix.insert(effective.clone(), name) {
            return Err(ConfigError::DuplicatePrefix {
                prefix: effective,
                first: first.clone(),
                second: name.clone(),
            });
        }
    }
    Ok(())
}

/// A record type's directories: real, inside the repository, in canonical
/// spelling, listed once and not owned by another type.
fn validate_type_dirs<'a>(
    name: &'a String,
    rt: &RecordTypeConfig,
    seen_dir: &mut std::collections::BTreeMap<std::path::PathBuf, &'a String>,
) -> Result<(), ConfigError> {
    let mut own_dirs: std::collections::BTreeSet<std::path::PathBuf> = Default::default();
    for dir in rt.all_dirs() {
        if dir_names_nothing(dir) || dir_escapes_repo(dir) {
            return Err(ConfigError::InvalidDir {
                record_type: name.clone(),
                dir: dir.to_string(),
            });
        }
        // Discovery compares paths component by component, so a spelling
        // with a `.` component, an empty one or a trailing slash names the
        // directory without ever matching it (`BUG-168`).
        let canonical = canonical_dir(dir);
        if canonical != *dir {
            return Err(ConfigError::NonCanonicalDir {
                record_type: name.clone(),
                dir: dir.to_string(),
                canonical,
            });
        }
        let path: std::path::PathBuf = std::path::Path::new(&canonical).components().collect();
        if !own_dirs.insert(path.clone()) {
            return Err(ConfigError::DuplicateDir {
                record_type: name.clone(),
                dir: dir.to_string(),
            });
        }
        if let Some(first) = seen_dir.insert(path, name) {
            return Err(ConfigError::SharedDir {
                dir: dir.to_string(),
                first: first.clone(),
                second: name.clone(),
            });
        }
    }
    Ok(())
}

/// A suppression type's `requires`: only on that role, non-empty, each field
/// declared and named once.
fn validate_effective_requires(name: &str, rt: &RecordTypeConfig) -> Result<(), ConfigError> {
    if let Some(requires) = &rt.effective_requires {
        let reason = if rt.role != Some(TypeRole::Suppression) {
            Some("but not `role: suppression` -- only the suppression type has suppressions that take effect".to_string())
        } else if requires.is_empty() {
            Some("as an empty list -- it would require nothing; omit it".to_string())
        } else {
            requires.iter().enumerate().find_map(|(i, f)| {
                if f.trim().is_empty() {
                    Some("with a blank field name".to_string())
                } else if requires[..i].contains(f) {
                    Some(format!("naming '{f}' twice"))
                } else if !rt
                    .declared_fields()
                    .iter()
                    .any(|d| d.as_str() == f.as_str())
                {
                    Some(format!(
                        "naming '{f}', which is in neither `required_fields` nor `known_fields`"
                    ))
                } else {
                    None
                }
            })
        };
        if let Some(reason) = reason {
            return Err(ConfigError::InvalidEffectiveRequires {
                record_type: name.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Every rule name under `rules:` is one this build ships.
fn validate_rule_names(config: &Config) -> Result<(), ConfigError> {
    // A misspelled rule name would otherwise be a check that silently never
    // runs, which is the failure direction this project treats as the worse
    // one (ADR-53).
    for name in config.rules.keys() {
        if !crate::rules::ALL_RULES.contains(&name.as_str()) {
            let mut known: Vec<String> = crate::rules::ALL_RULES
                .iter()
                .map(|s| s.to_string())
                .collect();
            known.sort();
            return Err(ConfigError::UnknownRule {
                found: name.clone(),
                known,
            });
        }
    }
    Ok(())
}

/// The `indexes` option: owner rule only, declared when the rule runs, each spec valid.
fn validate_index_options(config: &Config) -> Result<(), ConfigError> {
    // `indexes` is the one structured option, so it is validated apart from
    // the list-of-strings table below.
    for (name, setting) in &config.rules {
        let owner = crate::rules::rule_accepts_option(name, "indexes");
        if setting.indexes.is_some() && !owner {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "indexes".to_string(),
            });
        }
        if !owner || setting.level == RuleLevel::Off {
            continue;
        }
        let Some(indexes) = setting.indexes.as_ref().filter(|v| !v.is_empty()) else {
            return Err(ConfigError::OptionRequired {
                rule: name.clone(),
                option: "indexes".to_string(),
                consequence: "compares no artifact and can never report",
            });
        };
        for spec in indexes {
            if let Some(reason) = index_problem(spec, config) {
                return Err(ConfigError::InvalidIndex {
                    rule: name.clone(),
                    path: spec.path.clone(),
                    reason,
                });
            }
        }
    }
    Ok(())
}

/// The `issue_refs` option: owner rule only, declared when the rule runs, each field known.
fn validate_issue_refs_options(config: &Config) -> Result<(), ConfigError> {
    // `issue_refs` is structured too (`MILE-143`).
    for (name, setting) in &config.rules {
        let owner = crate::rules::rule_accepts_option(name, "issue_refs");
        if setting.issue_refs.is_some() && !owner {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "issue_refs".to_string(),
            });
        }
        if !owner || setting.level == RuleLevel::Off {
            continue;
        }
        let Some(refs) = setting.issue_refs.as_ref() else {
            return Err(ConfigError::OptionRequired {
                rule: name.clone(),
                option: "issue_refs".to_string(),
                consequence: "names no snapshot and can never report",
            });
        };
        if let Some(reason) = issue_refs_problem(refs) {
            return Err(ConfigError::InvalidIssueRefs {
                rule: name.clone(),
                reason,
            });
        }
        // A field no type declares reads nothing: refuse it rather than pass.
        for field in refs.fields.iter().flatten() {
            let known = config.record_types.values().any(|t| {
                t.declared_fields()
                    .contains(&crate::values::FieldName::from(field.as_str()))
            });
            if !known {
                return Err(ConfigError::InvalidIssueRefs {
                    rule: name.clone(),
                    reason: format!("'fields' names '{field}', which no record type declares"),
                });
            }
        }
    }
    Ok(())
}

/// The per-rule options: acceptance by the rule, the value checks of each option,
/// then the options a running rule must declare.
fn validate_rule_settings(config: &Config) -> Result<(), ConfigError> {
    for (name, setting) in &config.rules {
        check_rule_option_applicability(name, setting, config)?;
        check_rule_date_options(name, setting, config)?;
        check_rule_field_levels(name, setting, config)?;
        check_rule_decision_gate(name, setting, config)?;
        check_rule_pinnable_kinds(name, setting)?;
        check_rule_sticky_states(name, setting, config)?;
        check_rule_drift_audit_field(name, setting, config)?;
        check_rule_expiry_options(name, setting, config)?;
        if setting.level == RuleLevel::Off {
            continue;
        }
        check_rule_required_options(name, setting, config)?;
    }
    Ok(())
}

/// Which options a rule accepts, and the value checks of the plain ones.
fn check_rule_option_applicability(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    for opt in RULE_OPTIONS {
        let present = option_field(opt.name, setting).is_some();
        if present && !crate::rules::rule_accepts_option(name, opt.name) {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: opt.name.to_string(),
            });
        }
    }
    // `scope` is a scalar, which `option_field` does not read, so its
    // acceptance is checked here (`ADR-90`). The value was validated when
    // the setting was read.
    if setting.scope.is_some() && !crate::rules::rule_accepts_option(name, "scope") {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "scope".to_string(),
        });
    }
    if setting.non_reference_field.is_some()
        && !crate::rules::rule_accepts_option(name, "non_reference_field")
    {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "non_reference_field".to_string(),
        });
    }
    for (option, present) in [
        ("closes", setting.closes.is_some()),
        ("strict", setting.strict.is_some()),
        ("verb_claims", setting.verb_claims.is_some()),
        ("require_evidence", setting.require_evidence.is_some()),
    ] {
        if present && !crate::rules::rule_accepts_option(name, option) {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: option.to_string(),
            });
        }
    }
    if setting.skip_revision_log.is_some()
        && !crate::rules::rule_accepts_option(name, "skip_revision_log")
    {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "skip_revision_log".to_string(),
        });
    }
    if setting.allow_extension_prefix.is_some()
        && !crate::rules::rule_accepts_option(name, "allow_extension_prefix")
    {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "allow_extension_prefix".to_string(),
        });
    }
    if let Some(prefixes) = &setting.extension_prefixes {
        if !crate::rules::rule_accepts_option(name, "extension_prefixes") {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: "extension_prefixes".to_string(),
            });
        }
        for (record_type, prefix) in prefixes {
            if let Some(reason) = extension_prefix_problem(config, record_type, prefix) {
                return Err(ConfigError::InvalidExtensionPrefix {
                    rule: name.to_string(),
                    record_type: record_type.clone(),
                    reason,
                });
            }
        }
    }

    if setting.pending_until_required.is_some()
        && !crate::rules::rule_accepts_option(name, "pending_until_required")
    {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "pending_until_required".to_string(),
        });
    }

    if setting.template_placeholders.is_some()
        && !crate::rules::rule_accepts_option(name, "template_placeholders")
    {
        return Err(ConfigError::OptionNotApplicable {
            rule: name.to_string(),
            option: "template_placeholders".to_string(),
        });
    }
    Ok(())
}

/// `date_fields` and `date_order` name declared fields, for a rule that runs.
fn check_rule_date_options(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    if let (Some(fields), true) = (&setting.date_fields, setting.level != RuleLevel::Off) {
        for entry in fields {
            let bad = |reason: &str| ConfigError::InvalidDateFields {
                rule: name.to_string(),
                entry: entry.clone(),
                reason: reason.to_string(),
            };
            let field = entry.trim();
            if field.is_empty() {
                return Err(bad("names a blank field"));
            }
            let declared = config
                .record_types
                .values()
                .any(|t| t.declared_fields().iter().any(|f| f.as_str() == field));
            if !declared {
                return Err(bad(
                    "names a field that is not declared (required_fields or known_fields) by any record type, so it could never be examined",
                ));
            }
        }
    }
    if let (Some(orders), true) = (&setting.date_order, setting.level != RuleLevel::Off) {
        for entry in orders {
            let bad = |reason: String| ConfigError::InvalidDateOrder {
                rule: name.to_string(),
                entry: format!("{{earlier: {}, later: {}}}", entry.earlier, entry.later),
                reason,
            };
            let (earlier, later) = (entry.earlier.trim(), entry.later.trim());
            if earlier.is_empty() || later.is_empty() {
                return Err(bad("names a blank field".into()));
            }
            if earlier == later {
                return Err(bad("orders a field against itself".into()));
            }
            for field in [earlier, later] {
                let declared = config
                    .record_types
                    .values()
                    .any(|t| t.declared_fields().iter().any(|f| f.as_str() == field));
                if !declared {
                    return Err(bad(format!(
                        "names '{field}', which is not a declared field (required_fields or known_fields) of any record type"
                    )));
                }
            }
            let both = config.record_types.values().any(|t| {
                let d = t.declared_fields();
                d.iter().any(|f| f.as_str() == earlier) && d.iter().any(|f| f.as_str() == later)
            });
            if !both {
                return Err(bad(format!(
                    "names '{earlier}' and '{later}', but no one record type declares both, so it could never compare"
                )));
            }
        }
    }
    Ok(())
}

/// `field_levels`: accepted by the rule, non-empty, no `off`, declared fields.
fn check_rule_field_levels(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    if let Some(levels) = &setting.field_levels {
        if !crate::rules::rule_accepts_option(name, "field_levels") {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: "field_levels".to_string(),
            });
        }
        if levels.is_empty() {
            return Err(ConfigError::InvalidFieldLevels {
                rule: name.to_string(),
                reason: "is empty; name a field or remove the option".to_string(),
            });
        }
        for (field, level) in levels {
            if *level == RuleLevel::Off {
                return Err(ConfigError::InvalidFieldLevels {
                    rule: name.to_string(),
                    reason: format!(
                        "gives '{field}' the level 'off'; use 'info', 'warn' or 'error' (a field is switched off by not declaring it)"
                    ),
                });
            }
            let declared = config
                .record_types
                .values()
                .any(|t| t.declared_fields().iter().any(|f| f.as_str() == field));
            if !declared {
                return Err(ConfigError::InvalidFieldLevels {
                    rule: name.to_string(),
                    reason: format!(
                        "names '{field}', which is not a declared field (required_fields or known_fields) of any record type"
                    ),
                });
            }
        }
    }
    Ok(())
}

/// The decision-gate options: accepted by the rule and a coherent gate.
fn check_rule_decision_gate(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    for (option, present) in [
        ("decision_types", setting.decision_types.is_some()),
        ("fix_types", setting.fix_types.is_some()),
        ("link_fields", setting.link_fields.is_some()),
    ] {
        if present && !crate::rules::rule_accepts_option(name, option) {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: option.to_string(),
            });
        }
    }
    if setting.level != RuleLevel::Off {
        if let Some(reason) = decision_gate_problem(setting, config) {
            return Err(ConfigError::InvalidDecisionGate {
                rule: name.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// `pinnable` only narrows `locator_kinds`.
fn check_rule_pinnable_kinds(name: &str, setting: &RuleSetting) -> Result<(), ConfigError> {
    // A pin on a kind the rule rejects is a locator that is always
    // reported, so `pinnable` may only narrow `locator_kinds` (`BUG-176`).
    if setting.level != RuleLevel::Off {
        let declared = setting
            .locator_kinds
            .clone()
            .unwrap_or_else(|| vec![LocatorKind::File]);
        for kind in setting.pinnable.iter().flatten() {
            if !declared.contains(kind) {
                return Err(ConfigError::PinnableKindNotDeclared {
                    rule: name.to_string(),
                    kind: kind.to_string(),
                    declared: declared
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
        }
    }
    Ok(())
}

/// A sticky state is never one the evidence computes.
fn check_rule_sticky_states(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    // A sticky state equal to a computed one would hide real drift: that
    // state is exactly what the evidence can reproduce.
    if setting.level != RuleLevel::Off {
        for state in setting.sticky_states.iter().flatten() {
            if config.embodiment.is_computed(state.trim()) {
                return Err(ConfigError::StickyStateIsComputed {
                    rule: name.to_string(),
                    state: state.clone(),
                });
            }
        }
    }
    Ok(())
}

/// `drift_audit_field` needs a window and a declared field.
fn check_rule_drift_audit_field(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    // The audit field only moves where the grace window counts from, so it
    // is meaningless without a window, and a field no type declares would
    // never be present (`MILE-166`).
    if setting.level != RuleLevel::Off {
        if let Some(field) = setting.drift_audit_field.as_deref() {
            let field = field.trim();
            let reason = if !crate::rules::rule_accepts_option(name, "drift_audit_field") {
                None
            } else if setting.drift_grace_days.is_none() {
                Some(
                    "needs 'drift_grace_days': it only changes where that window counts from"
                        .to_string(),
                )
            } else if field.is_empty() {
                Some("is blank".to_string())
            } else if !config
                .record_types
                .values()
                .any(|t| t.declared_fields().iter().any(|f| f.as_str() == field))
            {
                Some(format!(
                    "names '{field}', which is not a declared field (required_fields or known_fields) of any record type"
                ))
            } else {
                None
            };
            if let Some(reason) = reason {
                return Err(ConfigError::InvalidDriftAuditField {
                    rule: name.to_string(),
                    reason,
                });
            }
        }
    }
    Ok(())
}

/// The drift and claim-expiry options, checked together once every record type is known.
fn check_rule_expiry_options(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    for (option, present) in [
        ("drift_grace_days", setting.drift_grace_days.is_some()),
        ("drift_audit_field", setting.drift_audit_field.is_some()),
        ("claim_max_age_days", setting.claim_max_age_days.is_some()),
    ] {
        if present && !crate::rules::rule_accepts_option(name, option) {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: option.to_string(),
            });
        }
    }

    // `ADR-97`: the expiry rule's own options are checked together, after
    // the embodiment declaration and every record type are known.
    for (option, present) in [
        ("anchor_field", setting.anchor_field.is_some()),
        ("require_pin", setting.require_pin.is_some()),
        ("max_batch", setting.max_batch.is_some()),
        ("periods", setting.periods.is_some()),
    ] {
        if present && !crate::rules::rule_accepts_option(name, option) {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.to_string(),
                option: option.to_string(),
            });
        }
    }
    if setting.level != RuleLevel::Off
        && name == crate::rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED
    {
        let expiry = config
            .rules
            .get(crate::rules::RULE_EMBODIMENT_CLAIM_EXPIRED)
            .filter(|s| s.level != RuleLevel::Off);
        if expiry.and_then(|s| s.anchor_field.as_deref()).is_none() {
            return Err(ConfigError::InvalidClaimExpiry {
                rule: name.to_string(),
                option: "level",
                reason: format!(
                    "is declared without {} and its anchor_field: the rule compares the anchor date and reads it from there",
                    crate::rules::RULE_EMBODIMENT_CLAIM_EXPIRED
                ),
            });
        }
    }
    if setting.level != RuleLevel::Off && name == crate::rules::RULE_EMBODIMENT_CLAIM_EXPIRED {
        claim_expiry_problems(name, setting, config)?;
    }
    Ok(())
}

/// An option-taking rule given no option does not fall back to a sane
/// default -- it inverts or goes inert, and both report success (`BUG-44`).
/// `claim.status-agreement` with an empty `closed_statuses` treats every claim
/// as a violation; with no `claim_paths` it scans nothing forever. The caller
/// skips a declined rule, which never runs, so `gated` never reads its options
/// -- requiring them anyway would mean a rule could not be turned off without
/// supplying values it will not use.
fn check_rule_required_options(
    name: &str,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {
    // A list of blanks is an empty list wearing a value.
    let declared = |v: Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());
    for opt in RULE_OPTIONS
        .iter()
        .filter(|o| crate::rules::rule_accepts_option(name, o.name))
    {
        // Non-empty *and* no blank entries, not merely present: an empty
        // list is the harm each consequence describes, and a list of
        // blanks is the same list wearing a value.
        let value = option_field(opt.name, setting);
        let ok = if opt.optional
            || (opt.fallback_for_undeclared_statuses && no_type_needs_the_status_fallback(config))
        {
            value.is_none_or(declared)
        } else {
            value.is_some_and(declared)
        };
        if !ok {
            return Err(ConfigError::OptionRequired {
                rule: name.to_string(),
                option: opt.name.to_string(),
                consequence: opt.consequence_if_missing,
            });
        }
    }
    Ok(())
}

/// Published paths: refused when they cannot name a path in the repository, otherwise
/// stored in their normalised form.
fn normalize_rule_paths(config: &mut Config) -> Result<(), ConfigError> {
    // Published paths: reject what cannot name a path inside the repository,
    // then store the normalised form so the rule, its messages and its
    // findings all use the same spelling.
    for (name, setting) in config.rules.iter_mut() {
        let level = setting.level;
        let path_options: [(&'static str, &mut Option<Vec<String>>); 6] = [
            ("published_paths", &mut setting.published_paths),
            ("source_paths", &mut setting.source_paths),
            ("application_paths", &mut setting.application_paths),
            ("exclude_paths", &mut setting.exclude_paths),
            ("back_pointer_paths", &mut setting.back_pointer_paths),
            ("test_paths", &mut setting.test_paths),
        ];
        for (option, paths) in path_options {
            let Some(paths) = paths.as_mut() else {
                continue;
            };
            for entry in paths.iter_mut() {
                let normalised = normalize_published_path(entry);
                if level != RuleLevel::Off {
                    // The diff rules' options take globs (`MILE-158`): one that
                    // cannot compile would match nothing, which reads as clean.
                    let diff_option = matches!(
                        option,
                        "source_paths"
                            | "application_paths"
                            | "exclude_paths"
                            | "test_paths"
                            | "published_paths"
                    );
                    if diff_option
                        && crate::rules::is_path_glob(&normalised)
                        && crate::rules::path_glob_regex(&normalised).is_err()
                    {
                        return Err(ConfigError::InvalidPublishedPath {
                            rule: name.clone(),
                            option,
                            entry: entry.clone(),
                            reason: "is not a valid glob (an unclosed or empty `[` class)",
                        });
                    }
                    if let Some(reason) = published_path_problem(&normalised) {
                        return Err(ConfigError::InvalidPublishedPath {
                            rule: name.clone(),
                            option,
                            entry: entry.clone(),
                            reason,
                        });
                    }
                }
                *entry = normalised;
            }
        }
    }
    Ok(())
}

/// Declared boundaries: structure, then the one spelling of each path.
fn validate_boundaries(config: &mut Config) -> Result<(), ConfigError> {
    // Declared boundaries (`ADR-80`): structure, then the one spelling of each
    // path. A boundary that cannot be matched would be a rule that never
    // reports, so every reason is a load error.
    validate_ignore_markers(config)?;
    for (name, setting) in config.rules.iter_mut() {
        let owner = crate::rules::rule_accepts_option(name, "boundaries");
        if setting.boundaries.is_some() && !owner {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "boundaries".to_string(),
            });
        }
        if !owner || setting.level == RuleLevel::Off {
            continue;
        }
        let none_declared = setting.boundaries.as_ref().is_none_or(Vec::is_empty)
            && setting.published_paths.as_ref().is_none_or(Vec::is_empty);
        if none_declared {
            return Err(ConfigError::OptionRequired {
                rule: name.clone(),
                option: "boundaries".to_string(),
                consequence: "declares no boundary (give `boundaries` or `published_paths`) and can never report",
            });
        }
        let bad = |option: &'static str, reason: String| ConfigError::InvalidBoundary {
            rule: name.clone(),
            option,
            reason,
        };
        for (idx, boundary) in setting.boundaries.iter_mut().flatten().enumerate() {
            let at = format!("entry {idx}");
            if boundary.reason.trim().is_empty() {
                return Err(bad(
                    "boundaries",
                    format!("{at} has a blank reason; say why these paths are declared"),
                ));
            }
            if boundary.paths.is_empty() {
                return Err(bad("boundaries", format!("{at} declares no paths")));
            }
            if boundary.exclude.as_ref().is_some_and(Vec::is_empty) {
                return Err(bad(
                    "boundaries",
                    format!("{at} declares an empty exclude; give entries or remove the key"),
                ));
            }
            let lists = [
                ("paths", Some(&mut boundary.paths)),
                ("exclude", boundary.exclude.as_mut()),
            ];
            for (list, entries) in lists {
                for entry in entries.into_iter().flatten() {
                    let normalised = normalize_published_path(entry);
                    if normalised.is_empty() {
                        return Err(bad(
                            "boundaries",
                            format!(
                                "{at} {list} entry '{entry}' is blank or names the repository root"
                            ),
                        ));
                    }
                    if crate::rules::is_path_glob(&normalised)
                        && crate::rules::path_glob_regex(&normalised).is_err()
                    {
                        return Err(bad(
                            "boundaries",
                            format!("{at} {list} entry '{entry}' is not a valid glob (an unclosed or empty `[` class)"),
                        ));
                    }
                    if let Some(reason) = published_path_problem(&normalised) {
                        return Err(bad(
                            "boundaries",
                            format!("{at} {list} entry '{entry}' {reason}"),
                        ));
                    }
                    *entry = normalised;
                }
            }
        }
    }
    Ok(())
}

/// `external_ids` entries parse and sit under a declared record prefix.
fn validate_external_ids(config: &Config) -> Result<(), ConfigError> {
    // `external_ids` entries must parse and sit under a declared record prefix:
    // an entry that can never match a citation is a rule option that is a
    // silent no-op.
    let prefixes = crate::rules::declared_prefixes(config);
    for (name, setting) in &config.rules {
        if setting.level == RuleLevel::Off {
            continue;
        }
        for entry in setting.external_ids.iter().flatten() {
            if let Err(reason) = crate::rules::parse_external_id(entry, &prefixes) {
                return Err(ConfigError::InvalidExternalId {
                    rule: name.clone(),
                    entry: entry.clone(),
                    reason,
                });
            }
        }
    }
    Ok(())
}

/// `closes`, `strict` and `verb_claims`: a key the one-line reader can find, and the
/// options that need it.
fn validate_claim_fields(config: &Config) -> Result<(), ConfigError> {
    // `closes` names a frontmatter key, so it must be one the one-line reader
    // can find; `strict` judges the presence of that key, so it needs one. An
    // option that could never apply is refused, not ignored (`ADR-85`).
    for (name, setting) in &config.rules {
        if setting.level == RuleLevel::Off {
            continue;
        }
        let bad = |option: &'static str, reason: &str| ConfigError::InvalidClaimField {
            rule: name.clone(),
            option,
            reason: reason.to_string(),
        };
        if let Some(key) = setting.closes.as_deref() {
            if key.is_empty()
                || key
                    .chars()
                    .any(|c| c.is_whitespace() || c == ':' || c == '#')
            {
                return Err(bad(
                    "closes",
                    "must be one frontmatter key: not empty, with no whitespace, ':' or '#'",
                ));
            }
        }
        if setting.strict.is_some() && setting.closes.is_none() {
            return Err(bad(
                "strict",
                "needs `closes`: it judges whether a claim file carries that key",
            ));
        }
        if setting.verb_claims.is_some() && setting.closes.is_none() {
            return Err(bad(
                "verb_claims",
                "needs `closes`: it only has meaning beside the structured field, and with no field turning the closing-verb reading off would leave the rule no claim to read",
            ));
        }
    }
    Ok(())
}

/// `non_reference_field` names a declared field no rule resolves or stale-checks.
fn validate_non_reference_fields(config: &Config) -> Result<(), ConfigError> {
    // `non_reference_field` must name a field some type declares (the header is
    // a closed structure) that no rule resolves or stale-checks: a pointer,
    // narrative or relation-role field would read the ids as references, the
    // opposite of the declaration (`ADR-75`).
    for (name, setting) in &config.rules {
        if setting.level == RuleLevel::Off {
            continue;
        }
        let Some(field) = setting.non_reference_field.as_deref() else {
            continue;
        };
        let bad = |reason: String| ConfigError::InvalidNonReferenceField {
            rule: name.clone(),
            reason,
        };
        if field.trim().is_empty() || field.trim() != field {
            return Err(bad("is empty or has surrounding whitespace".to_string()));
        }
        let known = config.record_types.values().any(|t| {
            t.declared_fields()
                .contains(&crate::values::FieldName::from(field))
        });
        if !known {
            return Err(bad(format!(
                "names '{field}', which no record type lists in `known_fields` (or `required_fields`)"
            )));
        }
        for (type_name, t) in &config.record_types {
            let role = if t.pointer_fields.iter().flatten().any(|f| f == field) {
                Some("pointer_fields")
            } else if t.narrative_fields.iter().flatten().any(|f| f == field) {
                Some("narrative_fields")
            } else if RelationRole::ALL
                .into_iter()
                .any(|r| t.relation_field(r) == field)
            {
                Some("a relation-role field")
            } else {
                None
            };
            if let Some(role) = role {
                return Err(bad(format!(
                    "names '{field}', which record type '{type_name}' declares in {role}; those rules would resolve the ids as references"
                )));
            }
        }
    }
    Ok(())
}

/// A test path lies inside a back-pointer path, or is a glob.
fn validate_test_paths(config: &Config) -> Result<(), ConfigError> {
    // A test path outside every back-pointer path is never scanned, so a
    // `Verifies:` there would be a silent no-op.
    for (name, setting) in &config.rules {
        if setting.level == RuleLevel::Off {
            continue;
        }
        let scanned = setting.back_pointer_paths.as_deref().unwrap_or_default();
        for entry in setting.test_paths.iter().flatten() {
            // A glob names files wherever they sit; the rule reads it only
            // under a scanned path, and reports one matching none.
            if crate::rules::is_path_glob(entry) {
                continue;
            }
            let covered = scanned
                .iter()
                .any(|b| crate::rules::path_is_published(std::path::Path::new(entry), b));
            if !covered {
                return Err(ConfigError::TestPathOutsideBackPointerPaths {
                    rule: name.clone(),
                    entry: entry.clone(),
                });
            }
        }
    }
    Ok(())
}

/// `fields` names a declared pointer or narrative field.
fn validate_rule_fields(config: &Config) -> Result<(), ConfigError> {
    // `fields` must name something the repository declared: a name matching no
    // declared pointer/narrative field excludes nothing and checks nothing,
    // which reads as a clean run.
    for (name, setting) in &config.rules {
        if setting.level == RuleLevel::Off {
            continue;
        }
        for field in setting.fields.iter().flatten() {
            let known = config.record_types.values().any(|t| {
                t.pointer_fields
                    .iter()
                    .chain(t.narrative_fields.iter())
                    .flatten()
                    .any(|f| f == field)
            });
            if !known {
                return Err(ConfigError::UndeclaredRuleField {
                    rule: name.clone(),
                    field: field.clone(),
                });
            }
        }
    }
    Ok(())
}

/// What a relation pair requires of its two sides (`RFC-47`). `required` and
/// `none` only: the declared rule level already is the repository's severity
/// (`MILE-80`), so a per-pair `advisory` would have nothing to set. It is
/// refused at load rather than read as `required`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PairReciprocity {
    /// A claim on one side must be echoed on the other.
    Required,
    /// Direction is declared; reciprocity is not checked.
    None,
}

fn default_cross_type() -> bool {
    true
}

/// One declared pair of inverse relation fields (`RFC-47`, `MILE-122`): a
/// claim `A.forward = B` is echoed by `B.inverse = A`, and the reverse.
/// The two names are global to the repository: every type that declares a
/// name carries that side, so a pair cannot spell itself differently per type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelationPair {
    pub name: String,
    pub forward: String,
    pub inverse: String,
    pub reciprocity: PairReciprocity,
    /// May the two ends be different record types. Defaults to `true`.
    #[serde(default = "default_cross_type")]
    pub cross_type: bool,
    /// Optional split by the claimant's status (`MILE-154`): a claim from a
    /// record at one of these statuses that is not settled by an echo is held
    /// to the provisional level instead of the rule's.
    #[serde(default)]
    pub provisional: Option<ProvisionalClaim>,
    /// The record types that carry this pair (`MILE-155`). Omitted means
    /// global, as before: every type declaring a side's field carries it.
    /// Declared, a record of another type is not the pair's business, and a
    /// claim to a record of another type is out of scope rather than an
    /// unechoed claim, because that type cannot hold the inverse field.
    #[serde(default)]
    pub types: Option<Vec<String>>,
    /// The pair is a sequencing relation that must not loop (`RFC-48`,
    /// `MILE-27`): `sequence.cycle` reports a cycle through its claims and
    /// `sequence.unresolved-blocker` a claim that names no record. Defaults to
    /// `false`; it applies to any pair, not only a dependency one.
    #[serde(default)]
    pub acyclic: bool,
}

/// Which claimants make a pair's claim provisional, and how loudly an
/// unsettled provisional claim is reported (`MILE-154`, `MILE-127`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionalClaim {
    /// Claimant statuses (its `Status` role field, exact match) at which the
    /// claim is not yet an assertion, such as `Draft` or `Proposed`.
    pub statuses: Vec<String>,
    /// The level an unsettled provisional claim is reported at. Defaults to
    /// `warn`; the rule's own level is the full severity.
    #[serde(default = "default_provisional_level")]
    pub level: ProvisionalLevel,
    /// A field on the peer record that acknowledges a pending claim by naming
    /// the claimant (`MILE-163`), so a provisional claim is settled by the
    /// peer's own declaration as well as by the inverse relation. Read only for
    /// a provisional claim; a claim from any other status still needs the echo.
    #[serde(default)]
    pub acknowledged_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProvisionalLevel {
    Warn,
    Error,
}

fn default_provisional_level() -> ProvisionalLevel {
    ProvisionalLevel::Warn
}

/// Load-time checks for `companion_fields` (`BUG-41`): a pair that cannot run
/// reads as a clean corpus, so every misdeclaration is refused.
/// Why a per-type extension prefix cannot be used, if it cannot (`MILE-141`).
/// The type must be declared with `known_fields` (the rule examines no other,
/// so a prefix for one would be a silent no-op), and the prefix must be
/// non-empty, hold no whitespace and not be the start of a field the type
/// declares, case-insensitively, since the rule matches prefixes that way.
fn extension_prefix_problem(config: &Config, record_type: &str, prefix: &str) -> Option<String> {
    let Some(t) = config.record_types.get(record_type) else {
        return Some(format!(
            "names '{record_type}', which is not a declared record type"
        ));
    };
    if t.known_fields.is_none() {
        return Some(format!(
            "names '{record_type}', which declares no `known_fields`, so the rule never examines its keys and the prefix would read nothing"
        ));
    }
    if prefix.is_empty() {
        return Some("has an empty prefix; it must be non-empty".to_string());
    }
    if prefix.chars().any(char::is_whitespace) {
        return Some(format!("has the prefix '{prefix}', which holds whitespace"));
    }
    let lower = prefix.to_lowercase();
    let clash = t
        .declared_fields()
        .into_iter()
        .find(|f| f.as_str().to_lowercase().starts_with(&lower))?;
    Some(format!(
        "has the prefix '{prefix}', which collides with the declared field '{clash}'"
    ))
}

fn validate_companion_fields(config: &Config) -> Result<(), ConfigError> {
    for (name, rt) in &config.record_types {
        let Some(pairs) = &rt.companion_fields else {
            continue;
        };
        let bad = |reason: String| {
            Err(ConfigError::InvalidCompanionFields {
                record_type: name.clone(),
                reason,
            })
        };
        for [a, b] in pairs {
            if a == b {
                return bad(format!("pair [{a}, {b}] names one role twice"));
            }
            for key in [a, b] {
                if RelationRole::from_config_key(key).is_none() {
                    return bad(format!(
                        "'{key}' is not a relation role (one of status, embodiment_state, embodiment_locator, supersession)"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Load-time checks for `roadmap` (`ADR-82`): `pairs` names declared, acyclic
/// pairs, each once, `group_by` names fields some record type declares, each
/// once, and `work_types` names record types, each once. A declaration that can never apply reads as a clean roadmap, so
/// it is refused (`ADR-7`, `ADR-64` clause 5).
fn validate_roadmap(config: &Config) -> Result<(), ConfigError> {
    let bad = |reason: String| Err(ConfigError::InvalidRoadmap { reason });
    let mut named = std::collections::BTreeSet::new();
    for name in &config.roadmap.pairs {
        if !named.insert(name.as_str()) {
            return bad(format!("pairs lists '{name}' twice"));
        }
        match config.relation_pairs.iter().find(|p| p.name == *name) {
            None => {
                return bad(format!(
                    "pairs names '{name}', which is not a relation_pairs entry"
                ))
            }
            Some(p) if !p.acyclic => {
                return bad(format!(
                    "pairs names '{name}', which does not declare `acyclic: true`; a dependency relation must be checked for cycles"
                ))
            }
            Some(_) => {}
        }
    }
    let mut grouped = std::collections::BTreeSet::new();
    for field in &config.roadmap.group_by {
        if field.trim().is_empty() || *field != field.trim() {
            return bad(format!(
                "group_by field '{field}' is blank or has surrounding whitespace"
            ));
        }
        if !grouped.insert(field.as_str()) {
            return bad(format!("group_by lists '{field}' twice"));
        }
        if !config.record_types.values().any(|t| {
            t.declared_fields()
                .contains(&crate::values::FieldName::new(field))
        }) {
            return bad(format!(
                "group_by names '{field}', which is not in the required_fields or known_fields of any record type, so no record could carry it"
            ));
        }
    }
    if let Some(work_types) = &config.roadmap.work_types {
        if work_types.is_empty() {
            return bad(
                "work_types is an empty list, which declares that nothing is work; omit the key to treat every type as work".to_string(),
            );
        }
        let mut listed = std::collections::BTreeSet::new();
        for name in work_types {
            if name.trim().is_empty() || *name != name.trim() {
                return bad(format!(
                    "work_types name '{name}' is blank or has surrounding whitespace"
                ));
            }
            if !listed.insert(name.as_str()) {
                return bad(format!("work_types lists '{name}' twice"));
            }
            if !config.record_types.contains_key(name) {
                return bad(format!(
                    "work_types names '{name}', which is not a record_types entry"
                ));
            }
        }
    }
    Ok(())
}

/// Load-time checks for `relation_pairs`: every misdeclaration is refused,
/// because a pair that cannot run reads as a clean corpus.
fn validate_relation_pairs(config: &Config) -> Result<(), ConfigError> {
    check_pair_fields(config)?;
    check_pair_types(config)?;
    check_pair_provisional(config)?;
    check_pair_rule_requirements(config)?;
    check_declared_keys(config)?;
    check_rules_need_declarations(config)
}

/// The load error for a misdeclared relation pair or the keys checked beside it.
fn invalid_relation_pair(reason: String) -> Result<(), ConfigError> {
    Err(ConfigError::InvalidRelationPair { reason })
}

/// Every pair is named once, and each side is a distinct declared field.
fn check_pair_fields(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    let declared: std::collections::BTreeSet<&str> = config
        .record_types
        .values()
        .flat_map(|t| {
            t.required_fields
                .iter()
                .chain(t.known_fields.iter().flatten())
        })
        .map(String::as_str)
        .collect();
    let mut names = std::collections::BTreeSet::new();
    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    for pair in &config.relation_pairs {
        let label = pair.name.as_str();
        if pair.name.trim().is_empty() || pair.name != pair.name.trim() {
            return bad(format!(
                "pair name '{label}' is blank or has surrounding whitespace"
            ));
        }
        if !names.insert(label) {
            return bad(format!("pair '{label}' is declared twice"));
        }
        for (side, field) in [("forward", &pair.forward), ("inverse", &pair.inverse)] {
            if field.trim().is_empty() || *field != field.trim() {
                return bad(format!(
                    "pair '{label}' {side} field '{field}' is blank or has surrounding whitespace"
                ));
            }
            if !declared.contains(field.as_str()) {
                return bad(format!(
                    "pair '{label}' {side} field '{field}' is not in the required_fields or known_fields of any record type, so the pair could never examine a record"
                ));
            }
            if let Some(other) = fields.insert(field.as_str(), label) {
                return bad(format!(
                    "field '{field}' is used by pair '{other}' and pair '{label}' (or twice by one pair); each field belongs to exactly one side of one pair"
                ));
            }
        }
    }
    Ok(())
}

/// A pair scoped to types names configured types that carry one of its fields.
fn check_pair_types(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    // A pair scoped to types that do not exist, or that carry neither of its
    // fields, would examine nothing (ADR-7).
    for pair in &config.relation_pairs {
        let Some(types) = &pair.types else { continue };
        let label = pair.name.as_str();
        if types.is_empty() {
            return bad(format!(
                "pair '{label}' types must list at least one record type"
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for name in types {
            if !seen.insert(name.as_str()) {
                return bad(format!("pair '{label}' types lists '{name}' twice"));
            }
            let Some(rt) = config.record_types.get(name) else {
                return bad(format!(
                    "pair '{label}' types names '{name}', which is not a configured record type"
                ));
            };
            let declares = |f: &str| {
                rt.required_fields
                    .iter()
                    .chain(rt.known_fields.iter().flatten())
                    .any(|d| d == f)
            };
            if !declares(&pair.forward) && !declares(&pair.inverse) {
                return bad(format!(
                    "pair '{label}' types names '{name}', which declares neither '{}' nor '{}', so it could never carry the pair",
                    pair.forward, pair.inverse
                ));
            }
        }
    }
    Ok(())
}

/// A provisional split names statuses some type declares, and an acknowledgement field that can be read.
fn check_pair_provisional(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    // A provisional split naming a status no type declares never applies,
    // which reads as a clean run (ADR-7).
    for pair in &config.relation_pairs {
        let Some(prov) = &pair.provisional else {
            continue;
        };
        let label = pair.name.as_str();
        if prov.statuses.is_empty() {
            return bad(format!(
                "pair '{label}' provisional.statuses must list at least one status"
            ));
        }
        if let Some(ack) = &prov.acknowledged_by {
            if ack.trim().is_empty() || *ack != ack.trim() {
                return bad(format!(
                    "pair '{label}' provisional.acknowledged_by '{ack}' is blank or has surrounding whitespace"
                ));
            }
            if config
                .relation_pairs
                .iter()
                .any(|p| p.forward == *ack || p.inverse == *ack)
            {
                return bad(format!(
                    "pair '{label}' provisional.acknowledged_by '{ack}' is a relation field of a pair; it names a different, unpaired field"
                ));
            }
            let carried = config.record_types.iter().any(|(name, rt)| {
                let carries = match &pair.types {
                    Some(types) => types.contains(name),
                    None => true,
                };
                carries
                    && rt
                        .required_fields
                        .iter()
                        .chain(rt.known_fields.iter().flatten())
                        .any(|d| d == ack)
            });
            if !carried {
                return bad(format!(
                    "pair '{label}' provisional.acknowledged_by '{ack}' is not in the required_fields or known_fields of any record type the pair carries, so it could never be read"
                ));
            }
        }
        let declared: std::collections::BTreeSet<String> = config
            .record_types
            .values()
            .filter_map(|t| t.status_decl())
            .flat_map(|d| d.values)
            .collect();
        let mut seen = std::collections::BTreeSet::new();
        for status in &prov.statuses {
            if status.trim().is_empty() || *status != status.trim() {
                return bad(format!(
                    "pair '{label}' provisional status '{status}' is blank or has surrounding whitespace"
                ));
            }
            if !seen.insert(status.as_str()) {
                return bad(format!(
                    "pair '{label}' provisional.statuses lists '{status}' twice"
                ));
            }
            if !declared.contains(status) {
                return bad(format!(
                    "pair '{label}' provisional status '{status}' is not in the `statuses.values` of any record type, so it could never apply"
                ));
            }
        }
    }
    Ok(())
}

/// A rule that needs relation pairs finds the pairs it examines.
fn check_pair_rule_requirements(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    // A sequencing rule with no acyclic pair to read examines nothing (ADR-7).
    if !config.relation_pairs.iter().any(|p| p.acyclic) {
        for rule in crate::rules::rule_ids_where(|d| d.requires_acyclic_pair) {
            if config
                .rules
                .get(rule)
                .is_some_and(|s| s.level != RuleLevel::Off)
            {
                return bad(format!(
                    "rule '{rule}' is enabled but no relation_pairs entry declares `acyclic: true`, so it would examine nothing"
                ));
            }
        }
    }
    // A pair rule with nothing to pair is a check that ran on nothing.
    if config.relation_pairs.is_empty() {
        for rule in crate::rules::rule_ids_where(|d| d.requires_relation_pairs) {
            if config
                .rules
                .get(rule)
                .is_some_and(|s| s.level != RuleLevel::Off)
            {
                return bad(format!(
                    "rule '{rule}' is enabled but no relation_pairs are declared, so it would examine nothing"
                ));
            }
        }
    }
    // A status-agreement rule with no pair to read, or no type declaring
    // `statuses.blocked` or `statuses.progress`, examines nothing (`MILE-144`,
    // ADR-7).
    if config
        .rules
        .get(crate::rules::RULE_SEQUENCE_STATUS_AGREEMENT)
        .is_some_and(|s| s.level != RuleLevel::Off)
    {
        if config.roadmap.pairs.is_empty() {
            return bad(format!(
                "rule '{}' is enabled but roadmap.pairs declares no pair, so there is no dependency to derive blocked from",
                crate::rules::RULE_SEQUENCE_STATUS_AGREEMENT
            ));
        }
        if !config.record_types.values().any(|t| {
            t.status_decl()
                .is_some_and(|d| !d.blocked.is_empty() || !d.progress.is_empty())
        }) {
            return bad(format!(
                "rule '{}' is enabled but no record type declares statuses.blocked or statuses.progress, so it would examine nothing",
                crate::rules::RULE_SEQUENCE_STATUS_AGREEMENT
            ));
        }
    }
    Ok(())
}

/// The top-level keys read beside the pairs: `override_log`, `block_on`, `new`,
/// `attestation`, `harness` and `authority`.
fn check_declared_keys(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    // The override log is declared, never defaulted: a path that names nothing or
    // leaves the repository could be neither written nor read (`MILE-188`).
    if let Some(path) = &config.override_log {
        if dir_names_nothing(path) || dir_escapes_repo(path) || path.trim().ends_with('/') {
            return bad(format!(
                "override_log '{path}' is not a repository-relative file path"
            ));
        }
    }
    // The hook reads `block_on` from the file; a value it would not recognise
    // is refused here too, never defaulted (`MILE-195`).
    if let Some(value) = &config.block_on {
        if !BLOCK_ON_VALUES.contains(&value.as_str()) {
            return bad(format!(
                "block_on '{value}' is not one of {}",
                BLOCK_ON_VALUES.join(", ")
            ));
        }
    }
    // The `new` key is read by a command that must not guess (`ADR-93`): a
    // pattern that is not a refspec source with one `*`, or a remote git would read
    // as an option, is refused here.
    if config.new.pull_ref_patterns.is_empty() {
        return bad(
            "new.pull_ref_patterns is empty; list at least one pattern or omit the key".to_string(),
        );
    }
    for pattern in &config.new.pull_ref_patterns {
        if !pattern.starts_with("refs/")
            || pattern.matches('*').count() != 1
            || pattern.contains(char::is_whitespace)
            || pattern.contains("..")
        {
            return bad(format!(
                "new.pull_ref_patterns entry '{pattern}' must start with refs/ and contain exactly one * (the pull request number), for example refs/pull/*/head"
            ));
        }
    }
    if config.new.remote.trim().is_empty()
        || config.new.remote.starts_with('-')
        || config.new.remote.contains(char::is_whitespace)
    {
        return bad(format!(
            "new.remote '{}' is not a remote name",
            config.new.remote
        ));
    }
    if config.new.fetch_timeout_seconds == 0 {
        return bad("new.fetch_timeout_seconds must be at least 1".to_string());
    }
    if config.new.lock_timeout_seconds == 0 {
        return bad("new.lock_timeout_seconds must be at least 1".to_string());
    }
    // `asserted` takes `--by` as given, so there is no verification to require
    // (`ADR-89` section 5).
    if config.attestation.require_verified
        && config.attestation.identity == AttestationIdentity::Asserted
    {
        return bad(
            "attestation.require_verified: true cannot be combined with identity: asserted, which takes --by as given and verifies nothing"
                .to_string(),
        );
    }
    // A gate with nothing to enforce is not left to read as protection
    // (`ADR-89` section 3).
    if let Some(harness) = &config.harness {
        if let Some(message) = crate::harness::authority_problem(harness, config.authority.as_ref())
        {
            return bad(message);
        }
    }
    // Every gate names a declared group, type and status (`ADR-89` section 4).
    if let Some(problem) = config.authority.as_ref().and_then(|a| a.problem_in(config)) {
        return bad(problem);
    }
    Ok(())
}

/// A rule enabled without the declaration it reads would examine nothing.
fn check_rules_need_declarations(config: &Config) -> Result<(), ConfigError> {
    let bad = invalid_relation_pair;
    // `diff.decider-required` reads the table: enabled without one it would
    // examine nothing and read as protection (`MILE-200`).
    for rule in crate::rules::rule_ids_where(|d| d.requires_authority) {
        if config.authority.is_none()
            && config
                .rules
                .get(rule)
                .is_some_and(|s| s.level != RuleLevel::Off)
        {
            return bad(format!(
                "rule '{rule}' is enabled but no top-level `authority` is declared, so it would judge no gate"
            ));
        }
    }
    for rule in crate::rules::rule_ids_where(|d| d.requires_override_log) {
        if config.override_log.is_none()
            && config
                .rules
                .get(rule)
                .is_some_and(|s| s.level != RuleLevel::Off)
        {
            return bad(format!(
                "rule '{rule}' is enabled but no `override_log` is declared, so it would read no log"
            ));
        }
    }
    // A version-bump rule with no type declaring a bump is a check that ran on
    // nothing (`BUG-51`).
    if config
        .rules
        .get(crate::rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED)
        .is_some_and(|s| s.level != RuleLevel::Off)
        && !config
            .record_types
            .values()
            .any(|t| t.version_bump().is_some())
    {
        return bad(format!(
            "rule '{}' is enabled but no record type declares revision_log.version_bump, so it would examine nothing",
            crate::rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED
        ));
    }
    Ok(())
}

// Implements: ADR-86 (load-time validation of `permanent`)
/// Why a type's `permanent` declaration is refused at load (`ADR-86`), or
/// `None` when it is well formed. A declaration that names a rule this build
/// lacks, a field the type does not declare or a vocabulary that cannot match
/// would otherwise be a governance setting that silently governs nothing.
fn permanent_problem(rt: &RecordTypeConfig, entries: &[PermanentDecl]) -> Option<String> {
    if rt.role != Some(TypeRole::Suppression) {
        return Some(
            "but not `role: suppression` -- only the suppression type has permanent suppressions"
                .to_string(),
        );
    }
    if entries.is_empty() {
        return Some("as an empty list -- it would allow nothing; omit it".to_string());
    }
    let declared = rt.declared_fields();
    let declares = |f: &str| declared.iter().any(|d| d.as_str() == f);
    for (i, entry) in entries.iter().enumerate() {
        let rule = &entry.rule;
        if !crate::rules::ALL_RULES.contains(&rule.as_str()) {
            return Some(format!("for rule '{rule}', which this build does not ship"));
        }
        if entries[..i].iter().any(|e| e.rule == *rule) {
            return Some(format!("for rule '{rule}' twice"));
        }
        if !declares(&entry.reason_field) {
            return Some(format!(
                "for rule '{rule}' with reason_field '{}', which is in neither `required_fields` nor `known_fields`",
                entry.reason_field
            ));
        }
        if entry.categories.is_empty() {
            return Some(format!(
                "for rule '{rule}' with an empty `categories` list -- it would allow nothing"
            ));
        }
        for (j, category) in entry.categories.iter().enumerate() {
            if category.name.trim().is_empty() {
                return Some(format!("for rule '{rule}' with a blank category name"));
            }
            if entry.categories[..j]
                .iter()
                .any(|c| c.name == category.name)
            {
                return Some(format!(
                    "for rule '{rule}' naming category '{}' twice",
                    category.name
                ));
            }
        }
        match &entry.cite_field {
            Some(cite) => {
                if !declares(cite) {
                    return Some(format!(
                        "for rule '{rule}' with cite_field '{cite}', which is in neither `required_fields` nor `known_fields`"
                    ));
                }
                if !rt.pointer_fields.iter().flatten().any(|f| f == cite) {
                    return Some(format!(
                        "for rule '{rule}' with cite_field '{cite}', which is not one of the type's `pointer_fields`"
                    ));
                }
            }
            None => {
                if let Some(category) = entry.categories.iter().find(|c| c.cite.is_some()) {
                    return Some(format!(
                        "for rule '{rule}' with category '{}' requiring a citation, but no `cite_field`",
                        category.name
                    ));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_cfg(type_body: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n{type_body}"
        )
    }

    #[test]
    fn declared_fields_none_is_a_type_with_no_header() {
        let (c, _) = parse_with_document_models(&model_cfg("    fields: {from: none}\n")).unwrap();
        assert!(c.record_types["adr"].has_no_header());
        let (c, _) =
            parse_with_document_models(&model_cfg("    fields: {from: prefix-lines}\n")).unwrap();
        assert!(!c.record_types["adr"].has_no_header());
    }

    #[test]
    fn a_config_without_a_document_model_loads_unchanged() {
        let (c, models) = parse_with_document_models(&model_cfg("")).unwrap();
        assert!(models.is_empty());
        assert_eq!(c.record_types["adr"].required_fields, ["Status"]);
        assert_eq!(
            parse(&model_cfg("")).unwrap().record_types["adr"].dir,
            "docs/adr"
        );
    }

    #[test]
    fn a_declared_document_model_loads_beside_the_ordinary_keys() {
        let body = "    header_shape: yaml-frontmatter\n    identity: {from: filename, pattern: '^(?P<prefix>ADR)-(?P<number>\\d+)-(?P<slug>.+)$'}\n    fields: {from: yaml-frontmatter}\n    sections: {from: headings, depth: 2, items: true}\n";
        let (c, models) = parse_with_document_models(&model_cfg(body)).unwrap();
        assert_eq!(
            c.record_types["adr"].header_shape,
            HeaderShape::YamlFrontmatter
        );
        assert_eq!(c.record_types["adr"].required_fields, ["Status"]);
        assert_eq!(models["adr"].sections.as_ref().unwrap().depth, 2);
        assert_eq!(
            models["adr"].fields.as_ref().unwrap().from,
            crate::document_model::FieldsSource::YamlFrontmatter
        );
    }

    #[test]
    fn parse_accepts_a_declared_model_instead_of_rejecting_the_keys() {
        parse(&model_cfg("    sections: {from: headings}\n")).unwrap();
    }

    #[test]
    fn an_invalid_declaration_fails_the_load_naming_the_type_and_key() {
        let e = parse(&model_cfg("    sections: {from: headings, depth: 9}\n")).unwrap_err();
        assert!(matches!(e, ConfigError::InvalidDocumentModel { .. }), "{e}");
        let m = e.to_string();
        assert!(
            m.contains("adr") && m.contains("sections") && m.contains("depth"),
            "{m}"
        );
    }

    #[test]
    fn other_unknown_keys_are_still_refused_beside_a_declaration() {
        let body = "    sections: {from: headings}\n    bogus: 1\n";
        let e = parse(&model_cfg(body)).unwrap_err();
        assert!(e.to_string().contains("bogus"), "{e}");
        let e = parse(&model_cfg("    bogus: 1\n")).unwrap_err();
        assert!(e.to_string().contains("bogus"), "{e}");
    }

    #[test]
    fn an_unquoted_integer_beyond_u64_loads_as_a_string_with_and_without_a_declaration() {
        for decl in ["", "    sections: {from: headings}\n"] {
            let yaml = format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [18446744073709551616]\n{decl}"
            );
            let c = parse(&yaml).unwrap_or_else(|e| panic!("{decl:?}: {e}"));
            assert_eq!(
                c.record_types["adr"].required_fields,
                ["18446744073709551616"]
            );
        }
    }

    #[test]
    fn plain_scalars_typed_parsing_accepts_still_load_beside_a_declaration() {
        for (key, scalar) in [
            ("required_fields", "[2024]"),
            ("required_fields", "[1.0]"),
            ("known_fields", "[2024]"),
            ("pointer_fields", "[1.0]"),
        ] {
            let yaml = format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    {key}: {scalar}\n    sections: {{from: headings}}\n"
            );
            let c = parse(&yaml).unwrap_or_else(|e| panic!("{key}: {scalar}: {e}"));
            assert!(c.record_types["adr"].sections.is_some());
        }
        let c = parse("schema_version: 2\nrecord_types:\n  adr:\n    dir: 2024\n    sections: {from: headings}\n").unwrap();
        assert_eq!(c.record_types["adr"].dir, "2024");
    }

    #[test]
    fn a_typed_error_keeps_its_line_and_column_beside_a_declaration() {
        let yaml = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    sections: {from: headings}\n    required_fields: {a: b}\n";
        let m = parse(yaml).unwrap_err().to_string();
        assert!(m.contains("line 6"), "{m}");
    }

    #[test]
    fn a_bad_schema_version_is_reported_before_an_invalid_declaration() {
        let yaml = "schema_version: 99\nrecord_types:\n  adr:\n    dir: docs/adr\n    sections: {from: headings, depth: 9}\n";
        let e = parse(yaml).unwrap_err();
        assert!(
            matches!(&e, ConfigError::SchemaVersionOutOfWindow(v) if v.found == 99),
            "{e}"
        );
    }

    #[test]
    fn a_declaration_outside_a_record_type_is_not_accepted() {
        let yaml = "schema_version: 2\nsections: {from: headings}\nrecord_types:\n  adr:\n    dir: docs/adr\n";
        assert!(parse(yaml).is_err());
    }

    /// A hand-written `Serialize` that disagrees with its `Deserialize` twin
    /// would write configs this build cannot read back, and nothing else would
    /// notice.
    #[test]
    fn header_shape_survives_a_serialize_deserialize_round_trip() {
        for shape in [HeaderShape::YamlFrontmatter, HeaderShape::None] {
            let s = yaml_serde::to_string(&shape).unwrap();
            assert_eq!(shape, yaml_serde::from_str::<HeaderShape>(&s).unwrap());
        }
    }

    /// BUG-106: three independent computations of this union existed;
    /// `known_fields` alone must not drop a field only in `required_fields`.
    #[test]
    fn declared_fields_unions_required_and_known() {
        let t = RecordTypeConfig {
            exclude_files: Vec::new(),
            dir: "docs/adr".to_string(),
            extra_dirs: Vec::new(),
            identity: None,
            fields: None,
            sections: None,
            statuses: None,
            required_fields: vec!["Status".to_string()],
            header_shape: HeaderShape::default(),
            prefix: None,
            known_fields: Some(vec!["Embodiment".to_string()]),
            pointer_fields: None,
            narrative_fields: None,
            spec: None,
            relation_fields: None,
            companion_fields: None,
            revision_log: None,
            role: None,
            effective_requires: None,
            permanent: None,
        };
        let declared = t.declared_fields();
        assert!(declared.contains("Status"));
        assert!(declared.contains("Embodiment"));
        assert_eq!(declared.len(), 2);
    }

    /// BUG-178: a type that declares `statuses` is judged by its own success and
    /// failure sets, so the fallback lists are required only while some type
    /// declares a status field and no `statuses`.
    #[test]
    fn the_fallback_status_lists_are_optional_once_no_type_needs_them_observed_failing() {
        let rules = "rules:\n  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n  narrative-field.stale: warn\n";
        let declared = "    statuses:\n      values: [Open, Done]\n      success: [Done]\n";
        let all = format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n{declared}  bug:\n    dir: \"docs/bugs\"\n{declared}{rules}"
        );
        assert!(parse(&all).is_ok(), "{:?}", parse(&all).err());

        // A type with no status field at all (a suppression record) is never
        // judged by a list, so it does not bring the requirement back.
        let with_waiver = all.replace(
            "rules:\n",
            "  waiver:\n    dir: \"docs/waiver\"\n    required_fields: [Rule]\nrules:\n",
        );
        assert!(
            parse(&with_waiver).is_ok(),
            "{:?}",
            parse(&with_waiver).err()
        );

        // A type that declares `Status` but no `statuses` does, for both rules.
        let one_missing =
            with_waiver.replace("required_fields: [Rule]", "required_fields: [Rule, Status]");
        let err = parse(&one_missing).unwrap_err().to_string();
        assert!(err.contains("closed_statuses"), "{err}");
        let only_stale = one_missing.replace(
            "  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n",
            "",
        );
        let err = parse(&only_stale).unwrap_err().to_string();
        assert!(err.contains("terminal_statuses"), "{err}");

        // A list declared anyway is still validated, never a silent no-op.
        let blank = all.replace(
            "narrative-field.stale: warn",
            "narrative-field.stale:\n    level: warn\n    terminal_statuses: [\"\"]",
        );
        assert!(parse(&blank)
            .unwrap_err()
            .to_string()
            .contains("terminal_statuses"));
    }

    #[test]
    fn a_rule_level_is_written_bare_or_as_a_table() {
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
rules:
  field.quality: error
  pointer.target-status:
    level: warn
    not_in: ["Superseded"]
"#;
        let c = parse(yaml).unwrap();
        assert_eq!(c.rules["field.quality"].level, RuleLevel::Error);
        assert_eq!(c.rules["field.quality"].not_in, None);
        assert_eq!(c.rules["pointer.target-status"].level, RuleLevel::Warn);
        assert_eq!(
            c.rules["pointer.target-status"].not_in.as_deref(),
            Some(["Superseded".to_string()].as_slice())
        );
    }

    /// A misspelled rule name would otherwise be a check that silently never
    /// runs -- indistinguishable, in the report, from one that ran clean.
    /// BUG-44: an option-taking rule given no option does not fall back to a
    /// sane default -- `claim.status-agreement` with an empty `closed_statuses`
    /// treats every claim as a violation, including correct ones.
    #[test]
    fn a_rule_missing_an_option_it_requires_is_rejected_observed_failing() {
        // The type declares a `Status` field and no `statuses`, so the fallback
        // lists are still needed (BUG-178).
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\nrules:\n";

        let no_statuses = format!(
            "{base}  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n"
        );
        let err = parse(&no_statuses).unwrap_err().to_string();
        assert!(err.contains("closed_statuses"), "{err}");
        assert!(err.contains("every claim"), "{err}");

        let no_paths = format!(
            "{base}  claim.status-agreement:\n    level: error\n    closed_statuses: [\"Fixed\"]\n"
        );
        assert!(parse(&no_paths)
            .unwrap_err()
            .to_string()
            .contains("claim_paths"));

        let no_not_in = format!("{base}  pointer.target-status: warn\n");
        assert!(parse(&no_not_in)
            .unwrap_err()
            .to_string()
            .contains("not_in"));

        // An empty list, and a list of blanks, are both the harm the message
        // describes rather than a declaration of it.
        for bad in ["[]", "[\"\"]", "[\"  \"]"] {
            let cfg =
                format!("{base}  pointer.target-status:\n    level: warn\n    not_in: {bad}\n");
            assert!(
                parse(&cfg).unwrap_err().to_string().contains("not_in"),
                "{bad} should be rejected"
            );
        }

        // `off` needs no options: a declined rule never reads them.
        assert!(parse(&format!("{base}  pointer.target-status: off\n")).is_ok());
        assert!(parse(&format!(
            "{base}  claim.status-agreement:\n    level: off\n"
        ))
        .is_ok());

        // Complete declarations still load.
        let complete = format!(
            "{base}  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n    closed_statuses: [\"Fixed\"]\n"
        );
        assert!(parse(&complete).is_ok());
    }

    #[test]
    fn published_paths_is_required_by_the_boundary_rule_and_by_no_other() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        for bad in ["[]", "[\"\"]"] {
            let cfg = format!(
                "{base}  boundary.published-path-reference:\n    level: error\n    published_paths: {bad}\n"
            );
            assert!(
                parse(&cfg)
                    .unwrap_err()
                    .to_string()
                    .contains("published_paths"),
                "{bad} should be rejected"
            );
        }
        assert!(parse(&format!(
            "{base}  boundary.published-path-reference: error\n"
        ))
        .is_err());
        assert!(parse(&format!("{base}  boundary.published-path-reference: off\n")).is_ok());
        assert!(parse(&format!(
            "{base}  boundary.published-path-reference:\n    level: error\n    published_paths: [\"pkg\"]\n"
        ))
        .is_ok());
        let elsewhere =
            format!("{base}  field.quality:\n    level: error\n    published_paths: [\"pkg\"]\n");
        assert!(parse(&elsewhere)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'published_paths'"));
    }

    #[test]
    fn sticky_states_belongs_to_consistency_and_may_not_name_a_computed_state() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        let ok = parse(&format!(
            "{base}  embodiment.consistency:\n    level: warn\n    sticky_states: [Deprecated, Inactive]\n"
        ))
        .unwrap();
        assert_eq!(
            ok.rules["embodiment.consistency"].sticky_states.as_deref(),
            Some(&["Deprecated".to_string(), "Inactive".to_string()][..])
        );
        let computed = parse(&format!(
            "{base}  embodiment.consistency:\n    level: warn\n    sticky_states: [Deprecated, Verified]\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(computed.contains("'Verified'") && computed.contains("sticky_states"));
        let elsewhere = parse(&format!(
            "{base}  field.quality:\n    level: error\n    sticky_states: [Deprecated]\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(elsewhere.contains("does not take the option 'sticky_states'"));
        let blank = parse(&format!(
            "{base}  embodiment.consistency:\n    level: warn\n    sticky_states: [\" \"]\n"
        ));
        assert!(blank.is_err(), "a blank entry is not a declaration");
    }

    #[test]
    fn revision_log_required_defaults_true_and_may_stand_alone() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";
        let c = parse(base).unwrap();
        assert!(c.revision_log_required("adr"));
        assert!(c.revision_log_required("unknown"));
        let c = parse(&format!(
            "{base}    revision_log: {{marker: \"**Log**\"}}\n"
        ))
        .unwrap();
        assert!(c.revision_log_required("adr"));
        let c = parse(&format!(
            "{base}    revision_log: {{heading: History, required: false}}\n"
        ))
        .unwrap();
        assert!(!c.revision_log_required("adr"));
        assert_eq!(
            c.revision_log_id("adr"),
            RevisionLogId::Heading("History".into())
        );
        // `required` alone leaves the identity at its default.
        let c = parse(&format!("{base}    revision_log: {{required: false}}\n")).unwrap();
        assert!(!c.revision_log_required("adr"));
        assert_eq!(c.revision_log_id("adr"), RevisionLogId::default());
        // An empty declaration still names nothing.
        assert!(parse(&format!("{base}    revision_log: {{}}\n")).is_err());
    }

    #[test]
    fn drift_audit_field_needs_a_window_a_declared_field_and_the_consistency_rule() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    known_fields: [Audited]\nrules:\n";
        let ok = parse(&format!(
            "{base}  embodiment.consistency:\n    level: warn\n    drift_grace_days: 7\n    drift_audit_field: Audited\n"
        ))
        .unwrap();
        assert_eq!(
            ok.rules["embodiment.consistency"]
                .drift_audit_field
                .as_deref(),
            Some("Audited")
        );
        let err = |extra: &str, rule: &str| {
            parse(&format!("{base}  {rule}:\n    level: warn\n{extra}"))
                .unwrap_err()
                .to_string()
        };
        let no_window = err("    drift_audit_field: Audited\n", "embodiment.consistency");
        assert!(no_window.contains("drift_grace_days"), "{no_window}");
        let undeclared = err(
            "    drift_grace_days: 7\n    drift_audit_field: Nope\n",
            "embodiment.consistency",
        );
        assert!(undeclared.contains("'Nope'"), "{undeclared}");
        let blank = err(
            "    drift_grace_days: 7\n    drift_audit_field: \" \"\n",
            "embodiment.consistency",
        );
        assert!(blank.contains("blank"), "{blank}");
        let elsewhere = err("    drift_audit_field: Audited\n", "field.quality");
        assert!(
            elsewhere.contains("does not take the option 'drift_audit_field'"),
            "{elsewhere}"
        );
    }

    #[test]
    fn back_pointer_paths_is_required_by_the_back_pointer_rule_and_by_no_other() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        assert!(parse(&format!(
            "{base}  embodiment.back-pointer-resolution: error\n"
        ))
        .unwrap_err()
        .to_string()
        .contains("back_pointer_paths"));
        let ok = parse(&format!(
            "{base}  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"./src/\"]\n"
        ))
        .unwrap();
        assert_eq!(
            ok.rules["embodiment.back-pointer-resolution"].back_pointer_paths,
            Some(vec!["src".to_string()])
        );
        let bad = parse(&format!(
            "{base}  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"../x\"]\n"
        ));
        assert!(bad.unwrap_err().to_string().contains("back_pointer_paths"));
        let elsewhere = format!(
            "{base}  field.quality:\n    level: error\n    back_pointer_paths: [\"src\"]\n"
        );
        assert!(parse(&elsewhere)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'back_pointer_paths'"));
    }

    #[test]
    fn reference_sections_is_required_by_citation_path_exists_and_by_no_other() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        assert!(parse(&format!("{base}  citation.path-exists: warn\n"))
            .unwrap_err()
            .to_string()
            .contains("reference_sections"));
        assert!(parse(&format!(
            "{base}  citation.path-exists:\n    level: warn\n    reference_sections: [\"References\"]\n"
        ))
        .is_ok());
        assert!(parse(&format!(
            "{base}  field.quality:\n    level: error\n    reference_sections: [\"References\"]\n"
        ))
        .unwrap_err()
        .to_string()
        .contains("does not take the option 'reference_sections'"));
    }

    /// `ADR-85`: `closes` and `strict` are options of `claim.status-agreement`
    /// alone, optional, and `strict` needs `closes`.
    #[test]
    fn closes_and_strict_belong_to_the_claim_rule_and_strict_needs_closes_observed_failing() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        let claim = "  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n    closed_statuses: [\"Fixed\"]\n";
        let ok = parse(&format!(
            "{base}{claim}    closes: closes\n    strict: true\n"
        ))
        .unwrap();
        let s = &ok.rules["claim.status-agreement"];
        assert_eq!(s.closes.as_deref(), Some("closes"));
        assert_eq!(s.strict, Some(true));
        // Declaring neither is the rule as it was.
        let none = parse(&format!("{base}{claim}")).unwrap();
        assert_eq!(none.rules["claim.status-agreement"].closes, None);
        // Another rule refuses both.
        for option in ["closes: closes", "strict: true"] {
            let err = parse(&format!(
                "{base}  field.quality:\n    level: error\n    {option}\n"
            ))
            .unwrap_err()
            .to_string();
            assert!(err.contains("does not take the option"), "{option}: {err}");
        }
        // `strict` judges the key, so it needs one.
        let err = parse(&format!("{base}{claim}    strict: false\n"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("strict") && err.contains("closes"), "{err}");
        // A key the one-line reader could not find is refused.
        for bad in ["\"\"", "\"a b\"", "\"a:b\"", "\"a#b\""] {
            let err = parse(&format!("{base}{claim}    closes: {bad}\n"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("closes"), "{bad}: {err}");
        }
    }

    #[test]
    fn skip_revision_log_belongs_to_citation_path_exists_and_is_optional() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";
        let ok = parse(&format!(
            "{base}  citation.path-exists:\n    level: warn\n    reference_sections: [\"References\"]\n    skip_revision_log: true\n"
        ))
        .unwrap();
        assert_eq!(
            ok.rules["citation.path-exists"].skip_revision_log,
            Some(true)
        );
        assert!(parse(&format!(
            "{base}  field.quality:\n    level: error\n    skip_revision_log: true\n"
        ))
        .unwrap_err()
        .to_string()
        .contains("does not take the option 'skip_revision_log'"));
        let both = parse(&format!(
            "{base}  citation.resolution:\n    level: warn\n    skip_revision_log: true\n"
        ))
        .unwrap();
        assert_eq!(
            both.rules["citation.resolution"].skip_revision_log,
            Some(true)
        );
    }

    #[test]
    fn revision_log_identity_defaults_and_is_declared_per_type() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";
        let c = parse(base).unwrap();
        assert_eq!(c.revision_log_id("adr"), RevisionLogId::default());
        assert_eq!(c.revision_log_id("unknown"), RevisionLogId::default());
        let c = parse(&format!("{base}    revision_log: {{heading: History}}\n")).unwrap();
        assert_eq!(
            c.revision_log_id("adr"),
            RevisionLogId::Heading("History".into())
        );
        let c = parse(&format!(
            "{base}    revision_log: {{marker: \"**Log**\"}}\n"
        ))
        .unwrap();
        assert_eq!(
            c.revision_log_id("adr"),
            RevisionLogId::Marker("**Log**".into())
        );
    }

    #[test]
    fn a_malformed_revision_log_declaration_is_refused() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";
        for bad in [
            "{}",
            "{marker: \"a\", heading: \"b\"}",
            "{marker: \"  \"}",
            "{heading: \"\"}",
            "{marker: \"a\\nb\"}",
        ] {
            let result = parse(&format!("{base}    revision_log: {bad}\n"));
            assert!(
                matches!(result, Err(ConfigError::InvalidRevisionLog { .. })),
                "{bad} must be refused: {result:?}"
            );
        }
        assert!(parse(&format!(
            "{base}    revision_log: {{marker: \"a\", other: 1}}\n"
        ))
        .is_err());
    }

    const LOG_BASE: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";

    fn log_shape(decl: &str) -> Result<RevisionLogShape, ConfigError> {
        parse(&format!("{LOG_BASE}    revision_log: {decl}\n")).map(|c| c.revision_log_shape("adr"))
    }

    #[test]
    fn an_undeclared_revision_log_shape_is_todays_behaviour() {
        // ADR-94 point 2: the defaults live in the engine.
        let want = RevisionLogShape {
            classes: vec!["substantive".into(), "structural".into()],
            class_column: None,
            date_column: None,
            tool_class: "structural".into(),
        };
        assert_eq!(RevisionLogShape::default(), want);
        let c = parse(LOG_BASE).unwrap();
        assert_eq!(c.revision_log_shape("adr"), want);
        assert_eq!(c.revision_log_shape("unknown"), want);
        assert_eq!(log_shape("{marker: \"**Revision log**\"}").unwrap(), want);
    }

    #[test]
    fn a_declared_revision_log_shape_is_read_stripped() {
        let shape = log_shape(
            "{required: true, classes: [cosmetic, \"**major**\", \" minor \"], class_column: \"**Kind**\", date_column: When, tool_class: minor}",
        )
        .unwrap();
        assert_eq!(shape.classes, ["cosmetic", "major", "minor"]);
        assert_eq!(shape.class_column.as_deref(), Some("Kind"));
        assert_eq!(shape.date_column.as_deref(), Some("When"));
        assert_eq!(shape.tool_class, "minor");
        // Each key may stand alone, like `required`.
        for alone in [
            "{classes: [structural]}",
            "{class_column: Class}",
            "{date_column: Date}",
            "{tool_class: structural}",
        ] {
            assert!(log_shape(alone).is_ok(), "{alone} must load");
        }
    }

    #[test]
    fn a_malformed_revision_log_vocabulary_is_refused_naming_the_type() {
        for bad in [
            "{classes: []}",
            "{classes: [\"  \"]}",
            "{classes: [\"**\"]}",
            "{classes: [structural, \"**structural**\"]}",
            "{classes: [a, a, structural]}",
            "{class_column: \" \"}",
            "{date_column: \"**\"}",
            "{class_column: Class, date_column: \"**Class**\"}",
            "{tool_class: \" \"}",
        ] {
            let err = log_shape(bad).expect_err(bad);
            assert!(
                matches!(err, ConfigError::InvalidRevisionLog { .. }),
                "{bad}: {err:?}"
            );
            assert!(err.to_string().contains("'adr'"), "{bad}: {err}");
        }
        // Case-sensitive: two names, not a duplicate.
        assert!(log_shape("{classes: [Structural, structural]}").is_ok());
    }

    #[test]
    fn a_version_bump_class_outside_the_declared_vocabulary_is_refused_naming_it() {
        let err = log_shape(
            "{classes: [major, structural], version_bump: {field: Version, classes: [major, huge]}}",
        )
        .unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidRevisionLog { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("huge"), "{err}");
        // A declared class that is not in version_bump.classes requires no bump.
        assert!(log_shape(
            "{classes: [cosmetic, structural], version_bump: {field: Version, classes: [structural]}}"
        )
        .is_ok());
        // With `classes` undeclared the bump classes are not checked.
        assert!(log_shape("{version_bump: {field: Version, classes: [anything]}}").is_ok());
    }

    #[test]
    fn the_tool_class_must_be_writable_under_the_declared_vocabulary() {
        let err = log_shape("{classes: [major, minor], tool_class: structural}").unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidRevisionLog { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("structural"), "{err}");
        // A vocabulary without `structural` and without `tool_class` names the missing key.
        let err = log_shape("{classes: [major, minor]}").unwrap_err();
        assert!(err.to_string().contains("tool_class"), "{err}");
        // `structural` in the vocabulary is the default tool class.
        let shape = log_shape("{classes: [major, structural]}").unwrap();
        assert_eq!(shape.tool_class, "structural");
        // With the vocabulary undeclared the tool class is not checked.
        assert!(log_shape("{tool_class: whatever}").is_ok());
    }

    #[test]
    fn a_revision_log_identity_matches_its_line_and_not_a_mention() {
        let marker = RevisionLogId::default();
        assert!(marker.opens_at("**Revision log**"));
        assert!(marker.opens_at("> **Revision log**  "));
        assert!(!marker.opens_at("The **Revision log** below lists changes."));
        assert!(!marker.opens_at("**Revision log** (append-only)"));
        let heading = RevisionLogId::Heading("Revision log".into());
        assert!(heading.opens_at("## Revision log"));
        assert!(heading.opens_at("### Revision log "));
        assert!(!heading.opens_at("Revision log"));
        assert!(!heading.opens_at("## Revision log of the world"));
        assert!(!heading.opens_at("#Revision log"));
        assert!(!heading.opens_at("####### Revision log"));
    }

    #[test]
    fn external_ids_belongs_to_citation_resolution_and_must_be_well_formed() {
        let base = "schema_version: 2\nrecord_types:\n  rfc:\n    dir: \"docs/rfc\"\nrules:\n";
        let ok = parse(&format!(
            "{base}  citation.resolution:\n    level: error\n    external_ids: [\"RFC-4122\", \"RFC-4000..4999\"]\n"
        ))
        .unwrap();
        assert_eq!(
            ok.rules["citation.resolution"]
                .external_ids
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
        // Still optional.
        assert!(parse(&format!("{base}  citation.resolution: error\n")).is_ok());
        for bad in ["\"FOO-1\"", "\"RFC-x\"", "\"RFC-9..1\"", "\"\"", "\" \""] {
            let err = parse(&format!(
                "{base}  citation.resolution:\n    level: error\n    external_ids: [{bad}]\n"
            ))
            .unwrap_err()
            .to_string();
            assert!(err.contains("external_ids"), "{bad}: {err}");
        }
        let elsewhere =
            format!("{base}  field.quality:\n    level: error\n    external_ids: [\"RFC-1\"]\n");
        assert!(parse(&elsewhere)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'external_ids'"));
    }

    #[test]
    fn non_reference_field_must_be_a_known_plain_field_of_citation_resolution() {
        let base = |extra: &str| {
            format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    known_fields: [\"Status\", \"Non-reference-ids\", \"Derives-from\", \"Note\"]\n{extra}rules:\n"
            )
        };
        let rule = |field: &str| {
            format!("  citation.resolution:\n    level: error\n    non_reference_field: {field}\n")
        };
        let ok = parse(&format!("{}{}", base(""), rule("Non-reference-ids"))).unwrap();
        assert_eq!(
            ok.rules["citation.resolution"]
                .non_reference_field
                .as_deref(),
            Some("Non-reference-ids")
        );
        // Still optional.
        assert!(parse(&format!("{}  citation.resolution: error\n", base(""))).is_ok());
        // Unknown to every type: the header is a closed structure.
        let err = parse(&format!("{}{}", base(""), rule("Nope")))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("non_reference_field") && err.contains("Nope"),
            "{err}"
        );
        // A pointer, a narrative or a relation-role field would resolve or
        // stale-check the ids, the opposite of the intent.
        for (extra, field) in [
            (
                "    pointer_fields: [\"Derives-from\"]\n    narrative_fields: []\n",
                "Derives-from",
            ),
            (
                "    pointer_fields: []\n    narrative_fields: [\"Note\"]\n",
                "Note",
            ),
            ("    relation_fields: {supersession: \"Note\"}\n", "Note"),
            (
                "    relation_fields: {status: \"Non-reference-ids\"}\n",
                "Non-reference-ids",
            ),
        ] {
            let err = parse(&format!("{}{}", base(extra), rule(field)))
                .unwrap_err()
                .to_string();
            assert!(err.contains("non_reference_field"), "{field}: {err}");
        }
        // The default relation roles count too.
        let err = parse(&format!("{}{}", base(""), rule("Status")))
            .unwrap_err()
            .to_string();
        assert!(err.contains("non_reference_field"), "{err}");
        // Another rule does not take it.
        let elsewhere = format!(
            "{}  field.quality:\n    level: error\n    non_reference_field: Note\n",
            base("")
        );
        assert!(parse(&elsewhere)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'non_reference_field'"));
    }

    #[test]
    fn an_unknown_rule_name_is_rejected_and_names_the_valid_set() {
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
rules:
  field.qualty: error
"#;
        let err = parse(yaml).unwrap_err().to_string();
        assert!(err.contains("field.qualty"), "{err}");
        assert!(err.contains("field.quality"), "{err}");
    }

    /// `MILE-141`: the extension-prefix option is a `header.field-set-consistency`
    /// option only, and is read as a bool, never a list.
    #[test]
    fn allow_extension_prefix_loads_on_field_set_consistency_only() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    known_fields: [\"Status\"]\nrules:\n";
        let config = parse(&format!(
            "{base}  header.field-set-consistency:\n    level: warn\n    allow_extension_prefix: true\n"
        ))
        .expect("the option loads on the rule that takes it");
        let setting = &config.rules["header.field-set-consistency"];
        assert_eq!(setting.allow_extension_prefix, Some(true));

        let err = parse(&format!(
            "{base}  header.required-fields:\n    level: warn\n    allow_extension_prefix: true\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(err.contains("header.required-fields"), "{err}");
        assert!(err.contains("allow_extension_prefix"), "{err}");
    }

    /// `MILE-141`: `extension_prefixes` loads on `header.field-set-consistency`
    /// only, and every entry is validated at load.
    fn prefix_cfg(entry: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    known_fields: [\"Status\", \"ext-Owner\"]\n  spec:\n    dir: \"docs/specs\"\nrules:\n  header.field-set-consistency:\n    level: warn\n    extension_prefixes:\n{entry}"
        )
    }

    #[test]
    fn extension_prefixes_load_on_field_set_consistency_only() {
        let config = parse(&prefix_cfg("      adr: \"xx-\"\n")).expect("a valid prefix loads");
        let setting = &config.rules["header.field-set-consistency"];
        assert_eq!(setting.extension_prefixes.as_ref().unwrap()["adr"], "xx-");

        let err = parse(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    known_fields: [\"Status\"]\nrules:\n  header.required-fields:\n    level: warn\n    extension_prefixes:\n      adr: \"xx-\"\n",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("header.required-fields"), "{err}");
        assert!(err.contains("extension_prefixes"), "{err}");
    }

    #[test]
    fn an_extension_prefix_must_be_non_empty_without_whitespace_and_clear_of_declared_fields() {
        for (entry, needle) in [
            ("      adr: \"\"\n", "non-empty"),
            ("      adr: \"x y-\"\n", "whitespace"),
            ("      adr: \" xx-\"\n", "whitespace"),
            ("      adr: \"ext-\"\n", "ext-Owner"),
            ("      adr: \"EXT-O\"\n", "ext-Owner"),
            ("      adr: \"Sta\"\n", "Status"),
            ("      nosuch: \"xx-\"\n", "nosuch"),
            ("      spec: \"xx-\"\n", "known_fields"),
        ] {
            let err = parse(&prefix_cfg(entry)).unwrap_err();
            assert!(
                matches!(err, ConfigError::InvalidExtensionPrefix { .. }),
                "{entry}: {err}"
            );
            assert!(err.to_string().contains(needle), "{entry}: {err}");
        }
    }

    #[test]
    fn an_option_on_a_rule_that_does_not_take_it_is_rejected() {
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
rules:
  field.quality:
    level: error
    not_in: ["Draft"]
"#;
        let err = parse(yaml).unwrap_err().to_string();
        assert!(err.contains("field.quality"), "{err}");
        assert!(err.contains("not_in"), "{err}");
    }

    /// BUG-46: `#[serde(untagged)]` discarded each variant's error, so all
    /// three of these produced "data did not match any variant of untagged enum
    /// Either" and the messages naming the valid levels and keys were dead.
    #[test]
    fn a_malformed_rule_setting_names_what_was_wrong_observed_failing() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";

        let bad_bare = parse(&format!("{base}  field.quality: eror\n"))
            .unwrap_err()
            .to_string();
        assert!(bad_bare.contains("eror"), "{bad_bare}");
        assert!(bad_bare.contains("\"warn\""), "{bad_bare}");

        let bad_table = parse(&format!("{base}  field.quality:\n    level: eror\n"))
            .unwrap_err()
            .to_string();
        assert!(bad_table.contains("eror"), "{bad_table}");
        assert!(bad_table.contains("\"warn\""), "{bad_table}");

        let bad_key = parse(&format!("{base}  field.quality:\n    levl: error\n"))
            .unwrap_err()
            .to_string();
        assert!(bad_key.contains("levl"), "{bad_key}");

        for msg in [bad_bare, bad_table, bad_key] {
            assert!(!msg.contains("untagged"), "{msg}");
        }
    }

    /// A rule option present with no value must not read as omitted: for
    /// `pointer.target-status` `fields:` that would silently widen the rule
    /// back to every pointer field (BUG-172 review).
    #[test]
    fn a_rule_option_present_with_no_value_is_rejected_not_treated_as_omitted() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    pointer_fields: [\"Implements\"]\nrules:\n";
        for opt in ["fields:", "fields: null", "not_in: ~"] {
            let err = parse(&format!(
                "{base}  pointer.target-status:\n    level: warn\n    {opt}\n"
            ))
            .unwrap_err()
            .to_string();
            assert!(err.contains("present with no value"), "{opt}: {err}");
        }
        let ok = parse(&format!(
            "{base}  pointer.target-status:\n    level: warn\n    not_in: [\"Superseded\"]\n    fields: [\"Implements\"]\n"
        ));
        assert!(ok.is_ok(), "{:?}", ok.err().map(|e| e.to_string()));
        assert!(parse(&format!(
            "{base}  pointer.target-status:\n    level: warn\n    not_in: [\"Superseded\"]\n"
        ))
        .is_ok());
    }

    #[test]
    fn omitting_rules_entirely_means_no_rule_runs() {
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
"#;
        assert!(parse(yaml).unwrap().rules.is_empty());
    }

    #[test]
    fn parses_record_types() {
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
    required_fields: ["Status", "Date"]
"#;
        let config = parse(yaml).unwrap();
        let adr = config.record_types.get("adr").unwrap();
        assert_eq!(adr.dir, "docs/adr");
        assert_eq!(adr.required_fields, vec!["Status", "Date"]);
    }

    #[test]
    fn spec_pointer_is_optional_and_parses_when_present() {
        let yaml = r#"
schema_version: 2
record_types:
  milestone:
    dir: "docs/milestones"
    spec: "SPEC-6"
  bug:
    dir: "docs/bugs"
"#;
        let config = parse(yaml).unwrap();
        assert_eq!(
            config.record_types.get("milestone").unwrap().spec,
            Some("SPEC-6".to_string())
        );
        assert_eq!(config.record_types.get("bug").unwrap().spec, None);
    }

    #[test]
    fn pointer_and_narrative_fields_round_trip() {
        let yaml = r#"
schema_version: 2
record_types:
  rfc:
    dir: "docs/rfc"
    pointer_fields: ["Implements", "Amends"]
    narrative_fields: []
  bug:
    dir: "docs/bugs"
"#;
        let config = parse(yaml).unwrap();
        assert_eq!(
            config.record_types.get("rfc").unwrap().pointer_fields,
            Some(vec!["Implements".to_string(), "Amends".to_string()])
        );
        assert_eq!(
            config.record_types.get("rfc").unwrap().narrative_fields,
            Some(vec![])
        );
        // Undeclared means None, not an implicit empty default -- distinct
        // from an explicit `= []`, per ADR-44's "no implicit default".
        assert_eq!(config.record_types.get("bug").unwrap().pointer_fields, None);
        assert_eq!(
            config.record_types.get("bug").unwrap().narrative_fields,
            None
        );
    }

    #[test]
    fn relation_fields_round_trip_and_default_when_undeclared() {
        let yaml = r#"
schema_version: 2
record_types:
  rfc:
    dir: "docs/rfc"
    required_fields: ["State"]
    relation_fields:
      status: State
  bug:
    dir: "docs/bugs"
    required_fields: ["Status"]
"#;
        let config = parse(yaml).unwrap();
        let rfc = config.record_types.get("rfc").unwrap();
        assert_eq!(rfc.relation_field(RelationRole::Status), "State");
        assert_eq!(
            rfc.relation_field(RelationRole::EmbodimentState),
            "Embodiment",
            "an undeclared role still falls back to its pre-RFC-42 default"
        );

        // Undeclared `relation_fields` means every role falls back, same as
        // an existing config with no changes needed.
        let bug = config.record_types.get("bug").unwrap();
        assert_eq!(bug.relation_field(RelationRole::Status), "Status");
    }

    #[test]
    fn a_removed_header_shape_is_a_clear_error_naming_the_form() {
        // MILE-85: `blockquote` and `bold-list` are no longer read; a config
        // declaring one must fail loudly, not fall back to another shape.
        for shape in ["blockquote", "bold-list"] {
            let yaml = format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    header_shape: \"{shape}\"\n"
            );
            let e = parse(&yaml).unwrap_err().to_string();
            assert!(
                e.contains(shape)
                    && e.contains("no longer supported")
                    && e.contains("yaml-frontmatter"),
                "{e}"
            );
        }
    }

    #[test]
    fn an_unrecognized_key_is_a_parse_error_not_silently_ignored() {
        // deny_unknown_fields makes this a hard error rather than silent
        // tolerance -- an invented config key that no rule reads and every
        // author trusts is exactly the failure this guards against.
        let yaml = r#"
schema_version: 2
record_types:
  adr:
    dir: "docs/adr"
    required_fielsd: ["Status"]
"#;
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn missing_schema_version_is_a_parse_error() {
        let yaml = r#"
record_types:
  adr:
    dir: "docs/adr"
"#;
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn an_unrecognized_schema_version_is_rejected() {
        let yaml = r#"
schema_version: 99
record_types:
  adr:
    dir: "docs/adr"
"#;
        let err = parse(yaml).unwrap_err();
        assert!(matches!(
            &err,
            ConfigError::SchemaVersionOutOfWindow(v) if v.found == 99
        ));
    }

    fn rt_config(types: &str) -> String {
        format!("schema_version: 2\nrecord_types:\n{types}rules:\n  identity.collision: error\n")
    }

    /// Round 30 F2: a prefix that cannot form a recognized `PREFIX-N-slug.md`
    /// name (empty, path separators, `..`, spaces, trailing hyphen,
    /// lower-case, all-digit segment) is refused at load.
    #[test]
    fn an_unsafe_prefix_is_refused_at_load() {
        for bad in [
            "", "../..", "../x", "a b", "ADR-", "-ADR", "A/B", "adr", "A--B", "2024", "A-1",
        ] {
            let yaml = rt_config(&format!(
                "  adr:\n    dir: \"docs/adr\"\n    prefix: \"{bad}\"\n"
            ));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::InvalidPrefix { .. })),
                "prefix {bad:?} must be refused"
            );
        }
        for good in ["ADR", "V2", "DOC-ADR", "MILE"] {
            let yaml = rt_config(&format!(
                "  adr:\n    dir: \"docs/adr\"\n    prefix: \"{good}\"\n"
            ));
            assert!(parse(&yaml).is_ok(), "prefix {good:?} must load");
        }
    }

    /// Round 30 F1: two types may not share a prefix, explicit or defaulted.
    #[test]
    fn two_types_sharing_a_prefix_are_refused_at_load() {
        let yaml = rt_config(
            "  adr:\n    dir: \"docs/adr\"\n    prefix: \"PFX\"\n  waiver:\n    dir: \"docs/waiver\"\n    prefix: \"PFX\"\n",
        );
        assert!(matches!(
            parse(&yaml),
            Err(ConfigError::DuplicatePrefix { .. })
        ));
        let yaml = rt_config(
            "  adr:\n    dir: \"docs/adr\"\n  other:\n    dir: \"docs/o\"\n    prefix: \"ADR\"\n",
        );
        assert!(matches!(
            parse(&yaml),
            Err(ConfigError::DuplicatePrefix { .. })
        ));
    }

    /// Round 30 F5: an empty or root `dir` examines nothing.
    #[test]
    fn an_empty_or_root_dir_is_refused_at_load() {
        for bad in ["", " ", ".", "./", "./."] {
            let yaml = rt_config(&format!("  adr:\n    dir: \"{bad}\"\n"));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::InvalidDir { .. })),
                "dir {bad:?} must be refused"
            );
        }
        assert!(parse(&rt_config("  adr:\n    dir: \"docs/adr\"\n")).is_ok());
    }

    /// `BUG-168`: a `dir` written `./docs/adr`, `docs/adr/` or `docs//adr`
    /// names a real directory but compares unequal to the one discovery
    /// builds, so the type examined nothing. It is a load error that names the
    /// canonical form, for `dir` and for each `extra_dirs` entry.
    #[test]
    fn a_non_canonical_dir_is_a_load_error_naming_the_canonical_form_observed_failing() {
        for bad in [
            "./docs/adr",
            "docs/adr/",
            "docs//adr",
            "docs/./adr",
            " docs/adr",
        ] {
            let yaml = rt_config(&format!("  adr:\n    dir: \"{bad}\"\n"));
            match parse(&yaml) {
                Err(e @ ConfigError::NonCanonicalDir { .. }) => {
                    let message = e.to_string();
                    assert!(message.contains("'docs/adr'"), "{bad:?}: {message}");
                    assert!(message.contains(&format!("'{bad}'")), "{bad:?}: {message}");
                }
                other => panic!("dir {bad:?} must be a NonCanonicalDir error, got {other:?}"),
            }
            let extra = bad.replace("adr", "arch");
            let yaml = rt_config(&format!(
                "  adr:\n    dir: docs/adr\n    extra_dirs: [\"{extra}\"]\n"
            ));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::NonCanonicalDir { .. })),
                "extra dir {extra:?} must be refused"
            );
        }
        assert!(parse(&rt_config("  adr:\n    dir: \"docs/adr\"\n")).is_ok());
    }

    /// `BUG-74`: the type that suppresses findings is declared by `role`,
    /// resolved by `suppression_type`; the literal name `waiver` is only the
    /// fallback when no type declares the role.
    #[test]
    fn the_suppression_type_is_declared_by_role_with_the_literal_as_fallback() {
        let declared = parse(&rt_config(
            "  exception:\n    dir: docs/exception\n    role: suppression\n  waiver:\n    dir: docs/waiver\n",
        ))
        .unwrap();
        assert_eq!(declared.suppression_type(), "exception");
        let fallback = parse(&rt_config("  waiver:\n    dir: docs/waiver\n")).unwrap();
        assert_eq!(fallback.suppression_type(), "waiver");
        let neither = parse(&rt_config("  adr:\n    dir: docs/adr\n")).unwrap();
        assert_eq!(neither.suppression_type(), "waiver");
    }

    #[test]
    fn a_second_suppression_type_or_an_unknown_role_is_refused_at_load() {
        let two = rt_config(
            "  a:\n    dir: docs/a\n    role: suppression\n  b:\n    dir: docs/b\n    role: suppression\n",
        );
        assert!(matches!(
            parse(&two),
            Err(ConfigError::DuplicateRole { .. })
        ));
        let unknown = rt_config("  a:\n    dir: docs/a\n    role: waiver\n");
        assert!(parse(&unknown).is_err());
    }

    /// `BUG-177`: `effective_requires` belongs to the suppression type, names
    /// non-blank fields the type declares, and is otherwise refused at load.
    #[test]
    fn effective_requires_is_validated_at_load() {
        let ok = parse(&rt_config(
            "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    known_fields: [Reason]\n    effective_requires: [Reason]\n",
        ))
        .unwrap();
        assert_eq!(
            ok.record_types["w"].effective_requires.as_deref(),
            Some(&["Reason".to_string()][..])
        );
        for bad in [
            // no role
            "  w:\n    dir: docs/w\n    required_fields: [Reason]\n    effective_requires: [Reason]\n",
            // a field the type does not declare
            "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    effective_requires: [Reason]\n",
            // blank, empty, repeated
            "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    effective_requires: [\" \"]\n",
            "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    effective_requires: []\n",
            "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    effective_requires: [Rule, Rule]\n",
        ] {
            assert!(
                matches!(
                    parse(&rt_config(bad)),
                    Err(ConfigError::InvalidEffectiveRequires { .. })
                ),
                "{bad}"
            );
        }
    }

    /// `ADR-86`: `permanent` belongs to the suppression type, names a rule this
    /// build ships, fields the type declares and a usable vocabulary.
    #[test]
    fn permanent_is_validated_at_load() {
        let head = "  w:\n    dir: docs/w\n    role: suppression\n    required_fields: [Rule]\n    known_fields: [Category, Cite]\n    pointer_fields: [Cite]\n    narrative_fields: []\n";
        let entry = |body: &str| format!("{head}    permanent:\n      - {body}\n");
        let good = "{rule: field.quality, reason_field: Category, categories: [{name: a}, {name: b, cite: required}], cite_field: Cite}";
        let ok = parse(&rt_config(&entry(good))).unwrap();
        let decl = &ok.record_types["w"].permanent.as_ref().unwrap()[0];
        assert_eq!(decl.categories.len(), 2);
        assert_eq!(decl.categories[1].cite, Some(CiteRequirement::Required));
        for (case, bad) in [
            ("unknown rule", good.replace("field.quality", "no.such")),
            ("undeclared field", good.replace("Category,", "Nope,")),
            (
                "empty categories",
                "{rule: field.quality, reason_field: Category, categories: []}".to_string(),
            ),
            ("duplicate", good.replace("name: b", "name: a")),
            ("blank", good.replace("name: a", "name: ' '")),
            (
                "cite not pointer",
                good.replace("cite_field: Cite", "cite_field: Category"),
            ),
            (
                "cite required, no field",
                good.replace(", cite_field: Cite", ""),
            ),
        ] {
            assert!(
                matches!(
                    parse(&rt_config(&entry(&bad))),
                    Err(ConfigError::InvalidPermanent { .. })
                ),
                "{case}"
            );
        }
        let twice = format!("{head}    permanent:\n      - {good}\n      - {good}\n");
        assert!(matches!(
            parse(&rt_config(&twice)),
            Err(ConfigError::InvalidPermanent { .. })
        ));
        let empty = format!("{head}    permanent: []\n");
        assert!(matches!(
            parse(&rt_config(&empty)),
            Err(ConfigError::InvalidPermanent { .. })
        ));
        let no_role = "  w:\n    dir: docs/w\n    required_fields: [Category]\n    permanent:\n      - {rule: field.quality, reason_field: Category, categories: [{name: a}]}\n";
        assert!(matches!(
            parse(&rt_config(no_role)),
            Err(ConfigError::InvalidPermanent { .. })
        ));
    }

    /// `MILE-130`: a type may own several directories; each is validated like
    /// `dir`, listed once, and owned by one type only.
    #[test]
    fn extra_dirs_are_validated_like_dir_and_owned_by_one_type() {
        let ok = parse(&rt_config(
            "  adr:\n    dir: docs/adr\n    extra_dirs: [docs/adr-archive, pkg/a/adr]\n",
        ))
        .unwrap();
        let adr = &ok.record_types["adr"];
        assert_eq!(
            adr.all_dirs().collect::<Vec<_>>(),
            ["docs/adr", "docs/adr-archive", "pkg/a/adr"]
        );
        assert!(adr.owns_dir(Some(std::path::Path::new("pkg/a/adr"))));
        assert!(!adr.owns_dir(Some(std::path::Path::new("pkg/a"))));
        for bad in ["..", "/etc", ".", ""] {
            let yaml = rt_config(&format!(
                "  adr:\n    dir: docs/adr\n    extra_dirs: [\"{bad}\"]\n"
            ));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::InvalidDir { .. })),
                "extra dir {bad:?} must be refused"
            );
        }
        for dup in ["[docs/adr]", "[a, a]"] {
            let yaml = rt_config(&format!(
                "  adr:\n    dir: docs/adr\n    extra_dirs: {dup}\n"
            ));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::DuplicateDir { .. })),
                "{dup} must be refused"
            );
        }
        let shared = rt_config(
            "  adr:\n    dir: docs/adr\n    extra_dirs: [shared]\n  rfc:\n    dir: shared\n",
        );
        assert!(matches!(parse(&shared), Err(ConfigError::SharedDir { .. })));
    }

    /// Round 30 review: a `dir` outside the repository is refused too.
    #[test]
    fn a_dir_escaping_the_repository_is_refused_at_load() {
        for bad in ["..", "../docs", "docs/../../x", "/etc", "/docs/adr"] {
            let yaml = rt_config(&format!("  adr:\n    dir: \"{bad}\"\n"));
            assert!(
                matches!(parse(&yaml), Err(ConfigError::InvalidDir { .. })),
                "dir {bad:?} must be refused"
            );
        }
    }

    /// Round 30 review: the default prefix (the upper-cased type name) is
    /// validated like a declared one.
    #[test]
    fn an_invalid_default_prefix_is_refused_at_load() {
        let yaml = rt_config("  my_type:\n    dir: \"docs/t\"\n");
        assert!(
            matches!(parse(&yaml), Err(ConfigError::InvalidPrefix { .. })),
            "{:?}",
            parse(&yaml)
        );
    }

    fn pair_cfg(rules: &str, pairs: &str) -> String {
        format!(
            "schema_version: 2\n{rules}{pairs}record_types:\n  rfc:\n    dir: \"docs/rfc\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Amends\", \"Amended-by\"]\n"
        )
    }

    const AMEND_PAIR: &str = "relation_pairs:\n  - {name: amendment, forward: Amends, inverse: Amended-by, reciprocity: required, cross_type: false}\n";

    #[test]
    fn fail_on_defaults_to_error_and_accepts_warning() {
        assert_eq!(parse(&pair_cfg("", "")).unwrap().fail_on, FailOn::Error);
        let c = parse(&pair_cfg("", "fail_on: warning\n")).unwrap();
        assert_eq!(c.fail_on, FailOn::Warning);
        let c = parse(&pair_cfg("", "fail_on: error\n")).unwrap();
        assert_eq!(c.fail_on, FailOn::Error);
    }

    /// `ADR-87`: `info` is a rule level and a `field_levels` level; every
    /// other value is refused with a message listing the four; `fail_on: info`
    /// does not exist.
    #[test]
    fn info_is_a_rule_level_and_a_field_level_and_not_a_fail_on_observed_failing() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Deciders\", \"Reviewers\"]\nrules:\n";
        let c = parse(&format!("{base}  field.quality: info\n")).expect("info loads");
        assert_eq!(c.rules["field.quality"].level, RuleLevel::Info);
        assert_eq!(RuleLevel::Info.as_str(), "info");
        let c = parse(&format!(
            "{base}  field.quality:\n    level: warn\n    field_levels: {{Deciders: info}}\n"
        ))
        .expect("field_levels accepts info");
        assert_eq!(
            c.rules["field.quality"].field_levels.as_ref().unwrap()["Deciders"],
            RuleLevel::Info
        );
        let err = parse(&format!("{base}  field.quality: fatal\n"))
            .unwrap_err()
            .to_string();
        for v in ["off", "warn", "error", "info"] {
            assert!(err.contains(&format!("\"{v}\"")), "{err}");
        }
        let err = parse(&pair_cfg("", "fail_on: info\n"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("error") && err.contains("warning"), "{err}");
    }

    #[test]
    fn a_boundary_severity_accepts_info() {
        assert_eq!(
            yaml_serde::from_str::<BoundarySeverity>("info").unwrap(),
            BoundarySeverity::Info
        );
    }

    #[test]
    fn fail_on_rejects_any_other_value() {
        assert!(parse(&pair_cfg("", "fail_on: info\n")).is_err());
        assert!(parse(&pair_cfg("", "fail_on: Warning\n")).is_err());
    }

    #[test]
    fn a_declared_relation_pair_loads_and_cross_type_defaults_to_true() {
        let c = parse(&pair_cfg("", AMEND_PAIR)).unwrap();
        assert_eq!(c.relation_pairs.len(), 1);
        assert!(!c.relation_pairs[0].cross_type);
        let y = "relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: none}\n";
        let c = parse(&pair_cfg("", y)).unwrap();
        assert!(c.relation_pairs[0].cross_type);
        assert_eq!(c.relation_pairs[0].reciprocity, PairReciprocity::None);
    }

    fn prov_cfg(pairs: &str) -> String {
        format!(
            "schema_version: 2\n{pairs}record_types:\n  rfc:\n    dir: \"docs/rfc\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Amends\", \"Amended-by\"]\n    statuses:\n      values: [Draft, Accepted]\n"
        )
    }

    #[test]
    fn a_provisional_split_loads_and_a_misdeclared_one_is_refused() {
        let pair = |prov: &str| {
            format!("relation_pairs:\n  - {{name: a, forward: Amends, inverse: Amended-by, reciprocity: required, provisional: {prov}}}\n")
        };
        let c = parse(&prov_cfg(&pair("{statuses: [Draft]}"))).unwrap();
        let p = c.relation_pairs[0].provisional.as_ref().unwrap();
        assert_eq!(p.statuses, vec!["Draft"]);
        assert_eq!(p.level, ProvisionalLevel::Warn);
        let c = parse(&prov_cfg(&pair("{statuses: [Draft], level: error}"))).unwrap();
        assert_eq!(
            c.relation_pairs[0].provisional.as_ref().unwrap().level,
            ProvisionalLevel::Error
        );
        for bad in [
            "{statuses: []}",
            "{statuses: [Proposed]}",
            "{statuses: [Draft, Draft]}",
            "{statuses: [' Draft']}",
            "{statuses: [Draft], level: off}",
            "{statuses: [Draft], colour: red}",
        ] {
            assert!(
                parse(&prov_cfg(&pair(bad))).is_err(),
                "{bad} must be refused"
            );
        }
    }

    #[test]
    fn a_declared_acknowledgement_field_loads_and_a_misdeclared_one_is_refused_observed_failing() {
        let cfg = |ack: &str| {
            format!("schema_version: 2\nrelation_pairs:\n  - {{name: a, forward: Amends, inverse: Amended-by, reciprocity: required, provisional: {{statuses: [Draft], acknowledged_by: {ack}}}}}\nrecord_types:\n  rfc:\n    dir: \"docs/rfc\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Amends\", \"Amended-by\", \"Amended-by-pending\"]\n    statuses:\n      values: [Draft, Accepted]\n")
        };
        let c = parse(&cfg("Amended-by-pending")).unwrap();
        assert_eq!(
            c.relation_pairs[0]
                .provisional
                .as_ref()
                .unwrap()
                .acknowledged_by
                .as_deref(),
            Some("Amended-by-pending")
        );
        for bad in ["''", "' Amended-by-pending'", "Nowhere", "Amended-by"] {
            assert!(parse(&cfg(bad)).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn a_diff_path_option_takes_globs_and_refuses_a_broken_one() {
        let cfg = |entry: &str| {
            format!("schema_version: 2\nrules:\n  diff.source-requires-record:\n    level: error\n    source_paths: [\"src\"]\n    exclude_paths: [\"{entry}\"]\nrecord_types: {{}}\n")
        };
        let c = parse(&cfg("**/*.test.*")).unwrap();
        let setting = &c.rules["diff.source-requires-record"];
        assert_eq!(
            setting.exclude_paths.as_deref(),
            Some(&["**/*.test.*".to_string()][..])
        );
        assert!(parse(&cfg("src/[ab")).is_err());
        // exclude_paths belongs to the diff rules only.
        let other = "schema_version: 2\nrules:\n  header.required-fields:\n    level: error\n    exclude_paths: [\"x\"]\nrecord_types: {}\n";
        assert!(parse(other).is_err());
    }

    #[test]
    fn a_pair_scoped_to_types_loads_and_a_misdeclared_scope_is_refused() {
        let pair = |types: &str| {
            format!("relation_pairs:\n  - {{name: a, forward: Amends, inverse: Amended-by, reciprocity: required, types: {types}}}\n")
        };
        let c = parse(&pair_cfg("", &pair("[rfc]"))).unwrap();
        assert_eq!(c.relation_pairs[0].types, Some(vec!["rfc".to_string()]));
        assert!(parse(&pair_cfg("", AMEND_PAIR)).unwrap().relation_pairs[0]
            .types
            .is_none());
        for bad in ["[]", "[rfc, rfc]", "[nope]", "rfc"] {
            assert!(
                parse(&pair_cfg("", &pair(bad))).is_err(),
                "{bad} must be refused"
            );
        }
        let neither = "schema_version: 2\nrelation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: required, types: [adr]}\nrecord_types:\n  rfc:\n    dir: docs/rfc\n    known_fields: [Amends, Amended-by]\n  adr:\n    dir: docs/adr\n    known_fields: [Title]\n";
        assert!(parse(neither).is_err());
    }

    #[test]
    fn a_config_without_relation_pairs_loads_unchanged() {
        assert!(parse(&pair_cfg("", "")).unwrap().relation_pairs.is_empty());
    }

    #[test]
    fn a_misdeclared_relation_pair_is_refused_at_load() {
        let bad = |pairs: &str| {
            matches!(
                parse(&pair_cfg("", pairs)),
                Err(ConfigError::InvalidRelationPair { .. }) | Err(ConfigError::Parse(_))
            )
        };
        // advisory is not a level this build has
        assert!(bad("relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: advisory}\n"));
        // a combined key is not a declaration (RFC-47 leaves it open)
        assert!(bad("relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: required, combined: x}\n"));
        // a field no type declares
        assert!(bad("relation_pairs:\n  - {name: a, forward: Amends, inverse: Nope, reciprocity: required}\n"));
        // forward == inverse
        assert!(bad("relation_pairs:\n  - {name: a, forward: Amends, inverse: Amends, reciprocity: required}\n"));
        // duplicate pair name
        assert!(bad("relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: required}\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: required}\n"));
        // blank name
        assert!(bad("relation_pairs:\n  - {name: ' ', forward: Amends, inverse: Amended-by, reciprocity: required}\n"));
    }

    #[test]
    fn a_companion_pair_naming_no_relation_role_is_refused_observed_failing() {
        let cfg = |pair: &str| {
            format!("schema_version: 2\nrecord_types:\n  spec:\n    dir: \"docs/specs\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - {pair}\n")
        };
        assert!(parse(&cfg("[embodiment_state, embodiment_locator]")).is_ok());
        for bad in [
            "[embodiment_state, Realized-by]",
            "[status, status]",
            "[status]",
        ] {
            assert!(parse(&cfg(bad)).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn enabling_a_pair_rule_with_no_pairs_declared_is_refused() {
        for rule in [
            "relation.pair-reciprocity",
            "relation.cross-type-disallowed",
        ] {
            let e = parse(&pair_cfg(&format!("rules: {{{rule}: error}}\n"), "")).unwrap_err();
            assert!(matches!(e, ConfigError::InvalidRelationPair { .. }), "{e}");
            assert!(parse(&pair_cfg(&format!("rules: {{{rule}: off}}\n"), "")).is_ok());
            assert!(parse(&pair_cfg(
                &format!("rules: {{{rule}: error}}\n"),
                AMEND_PAIR
            ))
            .is_ok());
        }
    }

    #[test]
    fn acyclic_defaults_to_false_and_parses_when_declared() {
        let c = parse(&pair_cfg("", AMEND_PAIR)).unwrap();
        assert!(!c.relation_pairs[0].acyclic);
        let y = "relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: none, acyclic: true}\n";
        assert!(parse(&pair_cfg("", y)).unwrap().relation_pairs[0].acyclic);
    }

    #[test]
    fn enabling_a_sequence_rule_without_an_acyclic_pair_is_refused() {
        let acyclic = "relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: none, acyclic: true}\n";
        for rule in ["sequence.cycle", "sequence.unresolved-blocker"] {
            // No pair at all, and a pair that is not acyclic: both would examine nothing.
            for pairs in ["", AMEND_PAIR] {
                let e =
                    parse(&pair_cfg(&format!("rules: {{{rule}: error}}\n"), pairs)).unwrap_err();
                assert!(matches!(e, ConfigError::InvalidRelationPair { .. }), "{e}");
                assert!(e.to_string().contains("acyclic"), "{e}");
            }
            assert!(parse(&pair_cfg(&format!("rules: {{{rule}: off}}\n"), "")).is_ok());
            assert!(parse(&pair_cfg(&format!("rules: {{{rule}: error}}\n"), acyclic)).is_ok());
        }
    }

    fn published_config(entry: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  boundary.published-path-reference:\n    level: error\n    published_paths: [{entry}]\n"
        )
    }

    #[test]
    fn published_paths_are_normalised_at_parse() {
        let c = parse(&published_config(
            "\"./pkg/sdk\", \"  pkg/lib/ \", \"a/b//\"",
        ))
        .unwrap();
        assert_eq!(
            c.rules["boundary.published-path-reference"]
                .published_paths
                .as_deref(),
            Some(
                [
                    "pkg/sdk".to_string(),
                    "pkg/lib".to_string(),
                    "a/b".to_string()
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn published_paths_that_are_absolute_parent_relative_or_backslashed_are_rejected() {
        for entry in ["/etc", "../x", "a/../b", "a\\\\b", "./.."] {
            let err = parse(&published_config(&format!("\"{entry}\""))).unwrap_err();
            assert!(
                matches!(err, ConfigError::InvalidPublishedPath { .. }),
                "entry {entry:?}: {err}"
            );
            assert!(err.to_string().contains("published_paths"), "{err}");
        }
    }

    fn boundary_config(rule_body: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  boundary.published-path-reference:\n    level: error\n{rule_body}"
        )
    }

    const GOOD_BOUNDARY: &str = "    boundaries:\n      - paths: [\"./pkg/*/src/\"]\n        kind: published\n        reason: \"ships to users\"\n        severity: warn\n        exclude: [\"pkg/a/src/fixtures/\"]\n";

    #[test]
    fn a_boundary_is_paths_kind_reason_severity_and_an_optional_exclude() {
        let c = parse(&boundary_config(&format!(
            "{GOOD_BOUNDARY}    ignore_marker: \"urzua:allow\"\n"
        )))
        .unwrap();
        let s = &c.rules["boundary.published-path-reference"];
        let b = &s.boundaries.as_ref().unwrap()[0];
        assert_eq!(b.kind, BoundaryKind::Published);
        assert_eq!(b.severity, BoundarySeverity::Warn);
        assert_eq!(b.reason, "ships to users");
        // Normalised like every path option.
        assert_eq!(b.paths, ["pkg/*/src"]);
        assert_eq!(
            b.exclude.as_deref(),
            Some(["pkg/a/src/fixtures".to_string()].as_slice())
        );
        assert_eq!(s.ignore_marker.as_deref(), Some("urzua:allow"));
    }

    #[test]
    fn all_three_kinds_load_and_any_other_is_a_config_error() {
        for kind in ["published", "vendored", "generated"] {
            let body = GOOD_BOUNDARY.replace("published", kind);
            assert!(parse(&boundary_config(&body)).is_ok(), "{kind}");
        }
        let err = parse(&boundary_config(
            &GOOD_BOUNDARY.replace("published", "mirrored"),
        ))
        .unwrap_err();
        assert!(err.to_string().contains("mirrored"), "{err}");
    }

    #[test]
    fn a_boundary_without_a_reason_severity_or_kind_does_not_load() {
        for missing in ["reason", "severity", "kind", "paths"] {
            let body: String = GOOD_BOUNDARY
                .lines()
                .filter(|l| {
                    !l.trim_start()
                        .trim_start_matches("- ")
                        .starts_with(&format!("{missing}:"))
                })
                .map(|l| format!("{l}\n"))
                .collect();
            // `paths` opens the list item; keep the item valid for the others.
            let body = if missing == "paths" {
                body.replacen("        kind", "      - kind", 1)
            } else {
                body
            };
            let err = parse(&boundary_config(&body)).unwrap_err();
            assert!(err.to_string().contains(missing), "{missing}: {err}");
        }
    }

    #[test]
    fn a_blank_reason_and_an_unsupported_severity_do_not_load() {
        let err = parse(&boundary_config(
            &GOOD_BOUNDARY.replace("ships to users", "   "),
        ))
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidBoundary { .. }), "{err}");
        assert!(err.to_string().contains("reason"), "{err}");
        for sev in ["off", "fatal"] {
            assert!(
                parse(&boundary_config(&GOOD_BOUNDARY.replace("warn", sev))).is_err(),
                "{sev}"
            );
        }
        // `info` is a declared severity too (`ADR-87` point 10).
        assert!(parse(&boundary_config(&GOOD_BOUNDARY.replace("warn", "info"))).is_ok());
    }

    #[test]
    fn a_boundary_path_that_cannot_match_is_refused() {
        for bad in ["/etc", "../x", "src/[ab", ""] {
            let body = GOOD_BOUNDARY.replace("./pkg/*/src/", bad);
            assert!(parse(&boundary_config(&body)).is_err(), "{bad}");
        }
        let body = GOOD_BOUNDARY.replace("pkg/a/src/fixtures/", "pkg/../x");
        assert!(parse(&boundary_config(&body)).is_err());
        let body = GOOD_BOUNDARY.replace("exclude: [\"pkg/a/src/fixtures/\"]", "exclude: []");
        assert!(parse(&boundary_config(&body)).is_err());
    }

    #[test]
    fn published_paths_alone_still_loads_and_stands_for_a_published_boundary() {
        let c = parse(&boundary_config("    published_paths: [pkg]\n")).unwrap();
        let opts: crate::rules::PublishedPathOpts = c
            .enabled_rule_opts(crate::rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
            .unwrap();
        assert_eq!(opts.boundaries.len(), 1);
        let b = &opts.boundaries[0];
        assert_eq!(b.kind, BoundaryKind::Published);
        assert_eq!(
            b.reason, None,
            "no reason is required for the compatible key"
        );
        assert_eq!(b.severity, crate::report::FindingSeverity::Error);
        // At level warn the compatible boundary's severity is the rule's level.
        let c = parse(
            &boundary_config("    published_paths: [pkg]\n").replace("level: error", "level: warn"),
        )
        .unwrap();
        let opts: crate::rules::PublishedPathOpts = c
            .enabled_rule_opts(crate::rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
            .unwrap();
        assert_eq!(
            opts.boundaries[0].severity,
            crate::report::FindingSeverity::Warning
        );
    }

    #[test]
    fn both_forms_declared_both_apply_and_a_declared_severity_ignores_the_rule_level() {
        let c = parse(
            &boundary_config(&format!("{GOOD_BOUNDARY}    published_paths: [other]\n"))
                .replace("level: error", "level: warn"),
        )
        .unwrap();
        let opts: crate::rules::PublishedPathOpts = c
            .enabled_rule_opts(crate::rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
            .unwrap();
        assert_eq!(opts.boundaries.len(), 2);
        // The declared boundary says `warn`; the compatible one takes the rule's level.
        assert_eq!(
            opts.boundaries[0].severity,
            crate::report::FindingSeverity::Warning
        );
        let c = parse(&boundary_config(&GOOD_BOUNDARY.replace("warn", "error"))).unwrap();
        let c_warn = parse(
            &boundary_config(&GOOD_BOUNDARY.replace("warn", "error"))
                .replace("level: error", "level: warn"),
        )
        .unwrap();
        for c in [c, c_warn] {
            let opts: crate::rules::PublishedPathOpts = c
                .enabled_rule_opts(crate::rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
                .unwrap();
            assert_eq!(
                opts.boundaries[0].severity,
                crate::report::FindingSeverity::Error
            );
        }
    }

    #[test]
    fn the_boundary_rule_with_no_boundary_at_all_does_not_load() {
        let err = parse(&boundary_config("    ignore_marker: \"x\"\n")).unwrap_err();
        assert!(matches!(err, ConfigError::OptionRequired { .. }), "{err}");
        // Off, the rule needs nothing.
        assert!(parse(&boundary_config("").replace("level: error", "level: off")).is_ok());
    }

    #[test]
    fn a_blank_or_multiline_ignore_marker_is_refused_and_the_options_belong_to_one_rule() {
        for marker in ["\"  \"", "\" x\"", "\"a\\nb\""] {
            let body = format!("{GOOD_BOUNDARY}    ignore_marker: {marker}\n");
            assert!(parse(&boundary_config(&body)).is_err(), "{marker}");
        }
        let cfg = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  filename.title-consistency:\n    level: error\n    ignore_marker: x\n";
        assert!(matches!(
            parse(cfg).unwrap_err(),
            ConfigError::OptionNotApplicable { .. }
        ));
    }

    #[test]
    fn the_back_pointer_rules_accept_a_valid_ignore_marker_and_refuse_a_blank_one() {
        for rule in [
            "embodiment.back-pointer-resolution",
            "embodiment.back-pointer-decision-accepted",
        ] {
            let cfg = |marker: &str| {
                format!(
                    "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  {rule}:\n    level: error\n    back_pointer_paths: [src]\n    ignore_marker: {marker}\n"
                )
            };
            let ok = parse(&cfg("\"urzua:allow\"")).unwrap();
            assert_eq!(ok.rules[rule].ignore_marker.as_deref(), Some("urzua:allow"));
            assert!(parse(&cfg("\"  \"")).is_err(), "{rule}");
        }
    }

    #[test]
    fn the_back_pointer_ignore_marker_must_not_be_part_of_a_back_pointer_keyword() {
        for marker in [
            "Implements:",
            "Implements",
            "ies:",
            "s",
            "Verifies:",
            "Verif",
        ] {
            let cfg = format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [src]\n    ignore_marker: \"{marker}\"\n"
            );
            assert!(parse(&cfg).is_err(), "{marker}");
        }
    }

    #[test]
    fn the_back_pointer_ignore_marker_must_be_specific_enough_to_excuse_one_line() {
        let cfg = |marker: &str| {
            format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n  milestone:\n    dir: docs/milestones\n    prefix: MILE\nrules:\n  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [src]\n    ignore_marker: \"{marker}\"\n"
            )
        };
        // Too short, only comment leaders or punctuation, or part of a declared id head.
        for marker in [
            "//", "#", "x", "ab", "///", "--", "# -", "/*", "ADR-", "ADR", "DR-", "MILE-", "MIL",
        ] {
            let err = parse(&cfg(marker)).unwrap_err().to_string();
            assert!(err.contains("ignore_marker"), "{marker}: {err}");
        }
        // Specific text still loads, including one that names a single record.
        for marker in ["urzua:allow", "record-id-ok", "ADR-1", "ok!x"] {
            assert!(parse(&cfg(marker)).is_ok(), "{marker}");
        }
    }

    #[test]
    fn the_two_back_pointer_rules_may_not_declare_different_markers() {
        let cfg = |a: &str, b: &str| {
            format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [src]\n    ignore_marker: \"{a}\"\n  embodiment.back-pointer-decision-accepted:\n    level: error\n    back_pointer_paths: [src]\n    ignore_marker: \"{b}\"\n"
            )
        };
        assert!(parse(&cfg("one", "one")).is_ok());
        let err = parse(&cfg("one", "two")).unwrap_err().to_string();
        assert!(
            err.contains("back-pointer-resolution")
                && err.contains("back-pointer-decision-accepted"),
            "{err}"
        );
    }

    fn index_config(indexes: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  index.freshness:\n    level: error\n    indexes: {indexes}\n"
        )
    }

    #[test]
    fn an_index_field_placeholder_must_name_a_declared_field() {
        let cfg = |format: &str| {
            format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Date\"]\nrules:\n  index.freshness:\n    level: error\n    indexes: [{{path: docs/adr/README.md, type: adr, format: \"{format}\"}}]\n"
            )
        };
        assert!(parse(&cfg("{id} {Status} {Date}")).is_ok());
        for bad in ["{id} {Statuss}", "{id} {status}", "{id} {Owner}"] {
            let err = parse(&cfg(bad)).unwrap_err().to_string();
            assert!(err.contains("declared field"), "{bad}: {err}");
        }
    }

    #[test]
    fn indexes_is_required_by_the_index_rule_and_by_no_other() {
        let good = "[{path: docs/adr/README.md, type: adr, format: \"{id}: {title}\"}]";
        assert!(parse(&index_config(good)).is_ok());
        for bad in [
            "[]",
            "[{path: docs/adr/README.md, type: nope, format: \"{id}\"}]",
            "[{path: docs/adr/README.md, type: adr, format: \"{title}\"}]",
            "[{path: /abs/README.md, type: adr, format: \"{id}\"}]",
            "[{path: docs/adr/README.md, type: adr, format: \"{id}\", end: \"<!-- e -->\"}]",
        ] {
            let err = parse(&index_config(bad)).unwrap_err().to_string();
            assert!(err.contains("indexes"), "{bad}: {err}");
        }
        let other = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  field.quality:\n    level: error\n    indexes: []\n";
        assert!(parse(other)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'indexes'"));
        let off = index_config("[]").replace("level: error", "level: off");
        assert!(parse(&off).is_ok());
    }

    fn issue_config(extra: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  citation.issue-exists:\n    level: warn\n    reference_sections: [References]\n{extra}"
        )
    }

    #[test]
    fn issue_refs_is_required_by_the_issue_rule_and_by_no_other() {
        let good = "    issue_refs: {snapshot: .urzua/known-refs.json, repository: urzuahq/urzua, forms: [number, url]}\n";
        assert!(parse(&issue_config(good)).is_ok());
        let missing = parse(&issue_config("")).unwrap_err().to_string();
        assert!(missing.contains("issue_refs"), "{missing}");
        for bad in [
            "{snapshot: '', repository: o/r, forms: [number]}",
            "{snapshot: /abs.json, repository: o/r, forms: [number]}",
            "{snapshot: ../x.json, repository: o/r, forms: [number]}",
            "{snapshot: dir/, repository: o/r, forms: [number]}",
            "{snapshot: s.json, repository: orr, forms: [number]}",
            "{snapshot: s.json, repository: o/r/x, forms: [number]}",
            "{snapshot: s.json, repository: o/r, forms: []}",
            "{snapshot: s.json, repository: o/r, forms: [number, number]}",
        ] {
            let err = parse(&issue_config(&format!("    issue_refs: {bad}\n")))
                .unwrap_err()
                .to_string();
            assert!(err.contains("issue_refs"), "{bad}: {err}");
        }
        // An unknown form, an unknown key and a missing key are all refused.
        for bad in [
            "{snapshot: s.json, repository: o/r, forms: [hash]}",
            "{snapshot: s.json, repository: o/r, forms: [number], extra: 1}",
            "{snapshot: s.json, forms: [number]}",
        ] {
            assert!(
                parse(&issue_config(&format!("    issue_refs: {bad}\n"))).is_err(),
                "{bad}"
            );
        }
        let other = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  field.quality:\n    level: error\n    issue_refs: {snapshot: s.json, repository: o/r, forms: [number]}\n";
        assert!(parse(other)
            .unwrap_err()
            .to_string()
            .contains("does not take the option 'issue_refs'"));
        let off = issue_config("").replace("level: warn", "level: off");
        assert!(parse(&off).is_ok());
    }

    #[test]
    fn issue_refs_fields_must_be_declared_header_fields_and_never_blank() {
        let base = |fields: &str| {
            format!(
                "schema_version: 2\nrecord_types:\n  bug:\n    dir: \"docs/bugs\"\n    required_fields: [Status, Found-in]\n    known_fields: [Fixed-in]\nrules:\n  citation.issue-exists:\n    level: warn\n    reference_sections: [References]\n    issue_refs: {{snapshot: s.json, repository: o/r, forms: [number]{fields}}}\n"
            )
        };
        assert!(parse(&base(", fields: [Found-in, Fixed-in]")).is_ok());
        assert!(parse(&base("")).is_ok());
        for bad in [
            ", fields: []",
            ", fields: ['']",
            ", fields: [Found-in, Found-in]",
        ] {
            let err = parse(&base(bad)).unwrap_err().to_string();
            assert!(err.contains("issue_refs"), "{bad}: {err}");
        }
        // A name no record type declares would match nothing.
        let err = parse(&base(", fields: [Nope]")).unwrap_err().to_string();
        assert!(err.contains("Nope") && err.contains("issue_refs"), "{err}");
    }

    #[test]
    fn the_issue_rule_needs_reference_sections_and_takes_skip_revision_log() {
        let refs = "    issue_refs: {snapshot: s.json, repository: o/r, forms: [number]}\n";
        let no_sections = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n  citation.issue-exists:\n    level: warn\n".to_string() + refs;
        assert!(parse(&no_sections)
            .unwrap_err()
            .to_string()
            .contains("reference_sections"));
        let ok = issue_config(&format!("{refs}    skip_revision_log: true\n"));
        assert!(parse(&ok).is_ok());
    }

    /// `MILE-140`: `field_levels` loads on the rules that report per field and
    /// is refused everywhere else, with `off`, empty, or an undeclared field.
    #[test]
    fn field_levels_loads_on_per_field_rules_and_is_refused_elsewhere_observed_failing() {
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Deciders\", \"Reviewers\"]\nrules:\n";
        let ok = parse(&format!(
            "{base}  field.quality:\n    level: warn\n    field_levels: {{Deciders: error, Reviewers: warn}}\n"
        ))
        .expect("loads on field.quality");
        let levels = ok.rules["field.quality"].field_levels.as_ref().unwrap();
        assert_eq!(levels["Deciders"], RuleLevel::Error);
        assert_eq!(levels["Reviewers"], RuleLevel::Warn);

        for (bad, needle) in [
            (
                "  pointer.resolution:\n    level: warn\n    field_levels: {Deciders: error}\n",
                "does not take",
            ),
            (
                "  field.quality:\n    level: warn\n    field_levels: {Deciders: off}\n",
                "off",
            ),
            (
                "  field.quality:\n    level: warn\n    field_levels: {}\n",
                "field_levels",
            ),
            (
                "  field.quality:\n    level: warn\n    field_levels: {Nope: error}\n",
                "Nope",
            ),
        ] {
            let err = parse(&format!("{base}{bad}")).unwrap_err().to_string();
            assert!(err.contains(needle), "{bad}: {err}");
        }
    }
}

#[cfg(test)]
mod exclude_files_tests {
    use super::*;

    fn with(entry: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    exclude_files: {entry}\nrules:\n  identity.collision: error\n"
        )
    }

    #[test]
    fn plain_file_names_load() {
        let config = parse(&with("[\"README.md\", \"_template.md\"]")).unwrap();
        assert_eq!(
            config.record_types["adr"].exclude_files,
            vec!["README.md", "_template.md"]
        );
    }

    #[test]
    fn an_entry_that_is_not_a_bare_file_name_is_refused_at_load() {
        for bad in [
            "\"\"",
            "\"docs/adr/README.md\"",
            "\"../README.md\"",
            "\"*.md\"",
            "\".\"",
        ] {
            let result = parse(&with(&format!("[{bad}]")));
            assert!(
                matches!(result, Err(ConfigError::InvalidExcludedFile { .. })),
                "{bad} must be refused: {result:?}"
            );
        }
    }
}

/// `ADR-90`, `MILE-203`: the `scope` rule option is a scalar, so it is
/// validated by its own checks, not by the list-option reader.
#[cfg(test)]
mod scope_option_tests {
    use super::*;

    const BASE: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n";
    const LOCATOR: &str = "embodiment.locator-on-back-pointer-path";

    fn locator(extra: &str) -> String {
        format!("{BASE}  {LOCATOR}:\n    level: info\n    back_pointer_paths: [src]\n{extra}")
    }

    #[test]
    fn scope_changed_loads_on_a_rule_that_lists_it() {
        let config = parse(&locator("    scope: changed\n")).unwrap();
        assert_eq!(config.rules[LOCATOR].scope, Some(RuleScope::Changed));
        let without = parse(&locator("")).unwrap();
        assert_eq!(without.rules[LOCATOR].scope, None);
    }

    #[test]
    fn a_value_other_than_changed_is_refused_naming_changed() {
        for value in [
            "all",
            "\"Changed\"",
            "3",
            "true",
            "[changed]",
            "{a: b}",
            "\"\"",
        ] {
            let err = parse(&locator(&format!("    scope: {value}\n")))
                .expect_err(value)
                .to_string();
            assert!(err.contains("'changed'"), "{value}: {err}");
            assert!(err.contains("scope"), "{value}: {err}");
        }
        // A key with no value is refused like every other option.
        let err = parse(&locator("    scope:\n")).unwrap_err().to_string();
        assert!(err.contains("scope"), "{err}");
    }

    #[test]
    fn scope_on_a_rule_that_does_not_list_it_is_refused_at_load() {
        // A config-level rule, a files rule, a diff rule, a run rule, the
        // directory-anchored rule, a cross-record rule and the two rules that
        // already judge only the changed set.
        for rule in [
            "type.dir-matches-nothing",
            "claim.status-agreement",
            "diff.source-requires-record",
            "waiver.matches-nothing",
            "config.scope-matches-nothing",
            "filename.identity-pattern",
            "identity.collision",
            "pointer.resolution",
            "header.required-fields",
        ] {
            let cfg = format!("{BASE}  {rule}:\n    level: warn\n    scope: changed\n");
            let err = parse(&cfg).expect_err(rule).to_string();
            assert!(
                err.contains(&format!("rule '{rule}' does not take the option 'scope'")),
                "{rule}: {err}"
            );
        }
    }

    /// The two rules that already judge only the changed set refuse the option
    /// (they need their own declarations to load, so the table is what is read).
    #[test]
    fn the_rules_that_already_judge_only_the_changed_set_do_not_take_scope() {
        for rule in [
            "status.transition-allowed",
            "revision-log.version-bump-required",
        ] {
            assert!(!crate::rules::rule_accepts_option(rule, "scope"), "{rule}");
        }
    }

    #[test]
    fn scope_survives_a_serialization_round_trip() {
        let config = parse(&locator("    scope: changed\n")).unwrap();
        let yaml = yaml_serde::to_string(&config.rules[LOCATOR]).unwrap();
        assert!(yaml.contains("scope: changed"), "{yaml}");
        let again: RuleSetting = yaml_serde::from_str(&yaml).unwrap();
        assert_eq!(again.scope, Some(RuleScope::Changed));
    }
}

#[cfg(test)]
mod option_table_tests {
    use super::*;

    /// Options that are not plain lists, or are validated together by their own
    /// check (`decision_gate_problem`), so `RULE_OPTIONS` (which carries the
    /// "required list" consequence text) does not describe them.
    const OTHER_OPTIONS: &[&str] = &[
        "decision_types",
        "fix_types",
        "link_fields",
        "indexes",
        "issue_refs",
        "allow_extension_prefix",
        "extension_prefixes",
        "pending_until_required",
        "template_placeholders",
        "skip_revision_log",
        "non_reference_field",
        "closes",
        "strict",
        "verb_claims",
        "require_evidence",
        "field_levels",
        "drift_grace_days",
        "drift_audit_field",
        "claim_max_age_days",
        "anchor_field",
        "require_pin",
        "max_batch",
        "periods",
        "boundaries",
        "ignore_marker",
        "scope",
    ];

    /// The option names `RuleSetting` accepts, read from the derived
    /// deserializer itself: its unknown-field error lists every field it knows,
    /// so the struct is the source and this test keeps no list of its own.
    fn struct_options() -> Vec<String> {
        let err = yaml_serde::from_str::<RuleSetting>("level: off\nzz_unknown: 1\n")
            .expect_err("an unknown key is refused")
            .to_string();
        let listed = err
            .split("expected one of ")
            .nth(1)
            .unwrap_or_else(|| panic!("no field list in: {err}"));
        listed
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|f| *f != "level")
            .map(str::to_string)
            .collect()
    }

    fn known_options() -> Vec<&'static str> {
        OTHER_OPTIONS
            .iter()
            .copied()
            .chain(RULE_OPTIONS.iter().map(|o| o.name))
            .collect()
    }

    /// An option the struct accepts but no table describes would be read by
    /// nothing; one a table names but the struct lacks would be refused at
    /// load. Either way is a silently ignored option (`ADR-55`, `ADR-64`).
    #[test]
    fn the_struct_and_the_option_tables_name_the_same_options() {
        let mut in_struct = struct_options();
        let mut in_tables: Vec<String> = known_options().iter().map(|s| s.to_string()).collect();
        in_struct.sort();
        in_tables.sort();
        assert_eq!(
            in_struct, in_tables,
            "RuleSetting's fields and RULE_OPTIONS + OTHER_OPTIONS must list the same options"
        );
    }

    /// The rule table names the options each rule accepts; a name that is not
    /// an option `RuleSetting` carries would be refused nowhere and read by
    /// nothing, and an option no rule accepts is dead.
    #[test]
    fn every_option_the_rule_table_names_is_a_struct_option_and_every_option_has_an_owner() {
        let in_struct = struct_options();
        for def in crate::rules::RULE_TABLE {
            for option in def.options {
                assert!(
                    in_struct.iter().any(|f| f == option),
                    "{} lists the option '{option}', which RuleSetting does not accept",
                    def.id
                );
            }
        }
        for option in &in_struct {
            assert!(
                crate::rules::RULE_TABLE
                    .iter()
                    .any(|d| d.options.contains(&option.as_str())),
                "RuleSetting accepts '{option}', which no rule in the rule table takes"
            );
        }
    }

    /// An option present on a rule that does not take it is refused at load.
    #[test]
    fn an_option_on_a_rule_that_does_not_take_it_is_refused() {
        let cfg = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  filename.title-consistency:\n    level: error\n    claim_paths: [docs]\n";
        let err = parse(cfg)
            .expect_err("an option the rule does not take")
            .to_string();
        assert!(err.contains("claim_paths"), "{err}");
    }

    // MILE-201 (ADR-89 section 5): the `attestation` key.
    const ATT_BASE: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";

    #[test]
    fn attestation_absent_is_gh_verified_without_require_verified() {
        let config = parse(ATT_BASE).unwrap();
        assert_eq!(config.attestation.identity, AttestationIdentity::GhVerified);
        assert!(!config.attestation.require_verified);
    }

    #[test]
    fn attestation_reads_each_declared_value() {
        let asserted = parse(&format!("{ATT_BASE}attestation:\n  identity: asserted\n")).unwrap();
        assert_eq!(asserted.attestation.identity, AttestationIdentity::Asserted);
        let strict = parse(&format!(
            "{ATT_BASE}attestation:\n  identity: gh-verified\n  require_verified: true\n"
        ))
        .unwrap();
        assert!(strict.attestation.require_verified);
    }

    #[test]
    fn an_unknown_attestation_identity_is_a_load_error_naming_the_allowed_values() {
        let err = parse(&format!("{ATT_BASE}attestation:\n  identity: signed\n"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("signed") && err.contains("gh-verified") && err.contains("asserted"),
            "{err}"
        );
    }

    #[test]
    fn require_verified_with_asserted_is_refused_at_load() {
        let err = parse(&format!(
            "{ATT_BASE}attestation:\n  identity: asserted\n  require_verified: true\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("attestation")
                && err.contains("require_verified")
                && err.contains("asserted"),
            "{err}"
        );
    }

    #[test]
    fn an_unknown_attestation_key_is_a_load_error() {
        assert!(parse(&format!("{ATT_BASE}attestation:\n  identiy: asserted\n")).is_err());
    }

    /// A rule setting reads back through the typed accessor, and a list option
    /// declared empty stays distinguishable from one not declared.
    #[test]
    fn serialization_round_trip_keeps_absent_apart_from_empty() {
        let absent: RuleSetting = yaml_serde::from_str("level: warn\n").unwrap();
        let empty: RuleSetting = yaml_serde::from_str("level: warn\nfields: []\n").unwrap();
        assert_eq!(absent.fields, None);
        assert_eq!(empty.fields, Some(Vec::new()));
        assert!(option_field("fields", &absent).is_none());
        assert_eq!(option_field("fields", &empty), Some(Vec::new()));
    }
}

/// Characterisation of load-time validation order: a config carrying one
/// fault per validation pass reports them one at a time, in a fixed order
/// and with fixed messages. Fixing the reported fault reveals the next.
#[cfg(test)]
mod validation_order_tests {
    use super::*;

    /// A valid config. Every `@NAME` is a slot: `SLOTS` holds its valid
    /// default and `FAULTS` the faulty value that replaces it.
    const TEMPLATE: &str = "schema_version: 2
@TOPA
@TOPB
@TOPC
@TOPD
record_types:
  adr:
    dir: @ADRDIR
    required_fields: [Status]
    known_fields: [Date, Accepted-on, Deciders, Reviewers, Blocks, Blocked-by, Realized-by]
    pointer_fields: [Reviewers]
@ADREXCL
@ADRLOG
@ADRPFX
  bug:
    dir: @BUGDIR
@BUGPFX
relation_pairs:
  - {name: ok, forward: Blocks, inverse: Blocked-by, reciprocity: required@ACYC}
@PAIRA
@PAIRB
@PAIRC
rules:
  index.freshness:
    level: error
    indexes: [{path: @IDXPATH, type: adr, format: '- {id}'}]
  citation.issue-exists:
    level: error
    reference_sections: [References]
    issue_refs: {snapshot: @SNAP, repository: o/r, forms: [number], fields: [@IRFIELD]}
  date.field-valid:
    level: error
    date_fields: [@DATEF]
@DVFL
  date.field-order:
    level: error
    date_order: [{earlier: Date, later: @LATER}]
  field.quality:
    level: error
    field_levels: {Deciders: @FQLEVEL}
@FQCLOSE
  embodiment.locator-exists:
    level: error
    locator_kinds: [file]
    pinnable: [@PINKIND]
  embodiment.claim-expired:
    level: error
    anchor_field: @ANCHOR
    periods:
@PERA
@PERB
      - {type: adr, days: 30}
@PERC
    require_pin: [@REQPIN]
    max_batch: @BATCH
  embodiment.consistency:
    level: error
    drift_grace_days: 3
    drift_audit_field: @AUDIT
  diff.source-requires-record:
    level: error
    source_paths: [@SRCPATH]
    exclude_paths: [docs]
  boundary.published-path-reference:
    level: error
    published_paths: [docs]
    ignore_marker: @MARKER
  citation.resolution:
    level: error
    external_ids: [@EXTID]
    non_reference_field: @NONREF
  claim.status-agreement:
    level: error
    claim_paths: [docs]
    closed_statuses: [Done]
    closes: @CLOSES
  embodiment.back-pointer-resolution:
    level: error
    back_pointer_paths: [src]
    test_paths: [@TESTPATH]
  pointer.target-status:
    level: error
    not_in: [Done]
    fields: [@PTRFIELD]
  sequence.cycle:
    level: error
@RULEA
@RULEB
@RULEC
@RULED
@RULEE
";

    const SLOTS: &[(&str, &str)] = &[
        ("@TOPA", ""),
        ("@TOPB", ""),
        ("@TOPC", ""),
        ("@TOPD", ""),
        ("@ADRDIR", "docs/adr"),
        ("@ADREXCL", ""),
        ("@ADRLOG", ""),
        ("@ADRPFX", ""),
        ("@BUGDIR", "docs/bug"),
        ("@BUGPFX", ""),
        ("@ACYC", ", acyclic: true"),
        ("@PAIRA", ""),
        ("@PAIRB", ""),
        ("@PAIRC", ""),
        ("@IDXPATH", "docs/index.md"),
        ("@SNAP", "snap.json"),
        ("@DATEF", "Date"),
        ("@DVFL", ""),
        ("@LATER", "Accepted-on"),
        ("@FQLEVEL", "error"),
        ("@FQCLOSE", ""),
        ("@IRFIELD", "Date"),
        ("@RULEE", ""),
        ("@PINKIND", "file"),
        ("@ANCHOR", "Date"),
        ("@PERA", ""),
        ("@PERB", ""),
        ("@PERC", ""),
        ("@REQPIN", "code"),
        ("@BATCH", "5"),
        ("@AUDIT", "Date"),
        ("@SRCPATH", "src"),
        ("@MARKER", "ok"),
        ("@EXTID", "ADR-1"),
        ("@NONREF", "Deciders"),
        ("@CLOSES", "closes"),
        ("@TESTPATH", "src/t.rs"),
        ("@PTRFIELD", "Reviewers"),
        ("@RULEA", ""),
        ("@RULEB", ""),
        ("@RULEC", ""),
        ("@RULED", ""),
    ];

    /// `(slot, faulty value, message)` in the order the loader reports them.
    const FAULTS: &[(&str, &str, &str)] = &[
        ("@PAIRA", "  - {name: bad, forward: Nope, inverse: Nope2, reciprocity: required}",
            "relation_pairs: pair 'bad' forward field 'Nope' is not in the required_fields or known_fields of any record type, so the pair could never examine a record",
        ),
        ("@PAIRB", "  - {name: typed, forward: Date, inverse: Accepted-on, reciprocity: required, types: [ghost]}",
            "relation_pairs: pair 'typed' types names 'ghost', which is not a configured record type",
        ),
        ("@PAIRC", "  - {name: prov, forward: Reviewers, inverse: Deciders, reciprocity: required, provisional: {statuses: []}}",
            "relation_pairs: pair 'prov' provisional.statuses must list at least one status",
        ),
        ("@ACYC", "",
            "relation_pairs: rule 'sequence.cycle' is enabled but no relation_pairs entry declares `acyclic: true`, so it would examine nothing",
        ),
        ("@TOPA", "override_log: /abs/log.md",
            "relation_pairs: override_log '/abs/log.md' is not a repository-relative file path",
        ),
        ("@TOPB", "block_on: nope",
            "relation_pairs: block_on 'nope' is not one of introduced, all",
        ),
        ("@TOPC", "new:\n  fetch_timeout_seconds: 0",
            "relation_pairs: new.fetch_timeout_seconds must be at least 1",
        ),
        ("@TOPD", "attestation:\n  identity: asserted\n  require_verified: true",
            "relation_pairs: attestation.require_verified: true cannot be combined with identity: asserted, which takes --by as given and verifies nothing",
        ),
        ("@RULEA", "  diff.decider-required:\n    level: error",
            "relation_pairs: rule 'diff.decider-required' is enabled but no top-level `authority` is declared, so it would judge no gate",
        ),
        ("@RULEB", "  override.unreviewed:\n    level: error",
            "relation_pairs: rule 'override.unreviewed' is enabled but no `override_log` is declared, so it would read no log",
        ),
        ("@RULEC", "  revision-log.version-bump-required:\n    level: error",
            "relation_pairs: rule 'revision-log.version-bump-required' is enabled but no record type declares revision_log.version_bump, so it would examine nothing",
        ),
        ("@ADRDIR", "docs/./adr",
            "record type 'adr' declares dir 'docs/./adr', which is not in canonical form -- write 'docs/adr'",
        ),
        ("@ADREXCL", "    exclude_files: [\"a/b\"]",
            "record type 'adr' declares exclude_files entry 'a/b', which is not a bare file name -- it would exclude nothing",
        ),
        ("@ADRLOG", "    revision_log: {}",
            "record type 'adr' declares revision_log neither `marker` nor `heading` -- name one",
        ),
        ("@ADRPFX", "    prefix: \"bad-\"",
            "record type 'adr' declares prefix 'bad-', which must be upper-case ASCII letters and digits in hyphen-separated segments",
        ),
        ("@BUGDIR", "docs/adr",
            "record types 'adr' and 'bug' both own directory 'docs/adr' -- a directory holds one type's records",
        ),
        ("@BUGPFX", "    prefix: ADR",
            "record types 'adr' and 'bug' both use the prefix 'ADR' -- `urzua new` would mint colliding identities",
        ),
        ("@RULED", "  not.a.rule:\n    level: error",
            "unknown rule 'not.a.rule' under `rules:` -- this build ships: ",
        ),
        ("@IDXPATH", "../escape.md",
            "rule 'index.freshness' option 'indexes' entry '../escape.md' path contains a `..` component; a published path must stay inside the repository",
        ),
        ("@SNAP", "\"\"",
            "rule 'citation.issue-exists' option 'issue_refs' has an empty snapshot path",
        ),
        ("@IRFIELD", "Nope",
            "rule 'citation.issue-exists' option 'issue_refs' 'fields' names 'Nope', which no record type declares",
        ),
        ("@RULEE", "  citation.path-exists:\n    level: error",
            "rule 'citation.path-exists' requires the option 'reference_sections' -- without it the rule reads no section and can never report",
        ),
        ("@LATER", "Date",
            "rule 'date.field-order' option 'date_order' entry '{earlier: Date, later: Date}' orders a field against itself",
        ),
        ("@DATEF", "Typo",
            "rule 'date.field-valid' option 'date_fields' entry 'Typo' names a field that is not declared (required_fields or known_fields) by any record type, so it could never be examined",
        ),
        ("@DVFL", "    field_levels: {Deciders: error}",
            "rule 'date.field-valid' does not take the option 'field_levels'",
        ),
        ("@PERA", "      - {type: ghost, days: 5}",
            "rule 'embodiment.claim-expired' option 'periods' entry 0 (type 'ghost') names a record type that is not declared",
        ),
        ("@PERB", "      - {type: adr, days: 0}",
            "rule 'embodiment.claim-expired' option 'periods' entry 0 (type 'adr') has days 0; a period is 1 day or more",
        ),
        ("@ANCHOR", "Nope",
            "rule 'embodiment.claim-expired' option 'anchor_field' names 'Nope', which entry 0 (type 'adr') does not declare (required_fields or known_fields)",
        ),
        ("@PERC", "      - {type: adr, days: 31}",
            "rule 'embodiment.claim-expired' option 'periods' has two entries for type 'adr' of equal specificity that can match the same record; narrow one with a status or a tier",
        ),
        ("@REQPIN", "nope",
            "rule 'embodiment.claim-expired' option 'require_pin' names the category 'nope', which the embodiment declaration does not use (test, code, spec)",
        ),
        ("@BATCH", "0",
            "rule 'embodiment.claim-expired' option 'max_batch' is 0; a batch is 1 record or more",
        ),
        ("@AUDIT", "Nope",
            "rule 'embodiment.consistency' option 'drift_audit_field' names 'Nope', which is not a declared field (required_fields or known_fields) of any record type",
        ),
        ("@PINKIND", "directory",
            "rule 'embodiment.locator-exists' option 'pinnable' names 'directory', which 'locator_kinds' does not declare; a locator of a kind the rule rejects can never be pinned (declared kinds: file)",
        ),
        ("@FQCLOSE", "    closes: closes",
            "rule 'field.quality' does not take the option 'closes'",
        ),
        ("@FQLEVEL", "off",
            "rule 'field.quality' option 'field_levels' gives 'Deciders' the level 'off'; use 'info', 'warn' or 'error' (a field is switched off by not declaring it)",
        ),
        ("@SRCPATH", "../esc",
            "rule 'diff.source-requires-record' option 'source_paths' entry '../esc' contains a `..` component; a published path must stay inside the repository",
        ),
        ("@MARKER", "' bad'",
            "rule 'boundary.published-path-reference' option 'ignore_marker' must be non-blank text on one line with no surrounding whitespace",
        ),
        ("@EXTID", "BAD",
            "rule 'citation.resolution' option 'external_ids' entry 'BAD' must look like PREFIX-N or PREFIX-LOW..HIGH",
        ),
        ("@CLOSES", "\"a b\"",
            "rule 'claim.status-agreement' option 'closes' must be one frontmatter key: not empty, with no whitespace, ':' or '#'",
        ),
        ("@NONREF", "Nope",
            "rule 'citation.resolution' option 'non_reference_field' names 'Nope', which no record type lists in `known_fields` (or `required_fields`)",
        ),
        ("@TESTPATH", "other/t.rs",
            "rule 'embodiment.back-pointer-resolution' option 'test_paths' entry 'other/t.rs' is not inside any 'back_pointer_paths' entry, so no file under it is scanned and a `Verifies:` comment there would count for nothing",
        ),
        ("@PTRFIELD", "Nope",
            "rule 'pointer.target-status' option 'fields' names 'Nope', which is not a declared pointer_fields or narrative_fields entry of any record type",
        ),
    ];

    fn render(active: &[(&str, &str, &str)]) -> String {
        let mut out = TEMPLATE.to_string();
        for (slot, default) in SLOTS {
            let value = active
                .iter()
                .find(|(s, _, _)| s == slot)
                .map_or(*default, |(_, v, _)| *v);
            out = out.replace(slot, value);
        }
        out
    }

    #[test]
    fn the_baseline_loads() {
        if let Err(e) = parse(&render(&[])) {
            panic!("{e}");
        }
    }

    #[test]
    fn simultaneous_faults_are_reported_in_a_fixed_order() {
        for (i, (slot, _, expected)) in FAULTS.iter().enumerate() {
            let message = parse(&render(&FAULTS[i..])).unwrap_err().to_string();
            // The unknown-rule message lists every rule this build ships.
            let same = if *slot == "@RULED" {
                message.starts_with(expected)
            } else {
                message == *expected
            };
            assert!(same, "step {i} ({slot}): {message}");
        }
        parse(&render(&[])).expect("with every fault fixed the config loads");
    }
}
