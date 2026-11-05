//! Minimal `.urzua/config.yaml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::document_model::DocumentModel;
use crate::header::HeaderShape;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The only schema version defined so far (ADR-0012). A config declaring any
/// other value is a parse-time error, not a silent best-effort read.
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
    /// Admit header keys beginning `x-` (case-insensitive) as extension keys
    /// (`MILE-141`). Only meaningful to `header.field-set-consistency`, where
    /// omitted means `false`: every undeclared key is a finding. A declared
    /// `true` exempts prefixed keys from that rule and nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_extension_prefix: Option<bool>,
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
    /// `Embodiment` values a human sets that no evidence can compute
    /// (`Deprecated`, `Inactive`): a record stating one is not reported as
    /// drift (`MILE-148`). Only meaningful to `embodiment.consistency`;
    /// omitted means every stated value must match the computed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky_states: Option<Vec<String>>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuleLevel {
    #[default]
    Off,
    Warn,
    Error,
}

impl RuleLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleLevel::Off => "off",
            RuleLevel::Warn => "warn",
            RuleLevel::Error => "error",
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
            "warn" => Ok(RuleLevel::Warn),
            "error" => Ok(RuleLevel::Error),
            other => Err(serde::de::Error::custom(format!(
                "unrecognized rule level '{other}' -- expected \"off\", \"warn\", or \"error\""
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
}

/// The default marker line: what every repository used before the identity
/// was declared.
pub const DEFAULT_REVISION_LOG_MARKER: &str = "**Revision log**";

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

    /// The version-bump requirement this type declares, if any (`BUG-51`).
    pub fn version_bump(&self) -> Option<&VersionBumpDecl> {
        self.revision_log
            .as_ref()
            .and_then(|d| d.version_bump.as_ref())
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
    #[error("unrecognized schema_version {found} -- this build of urzua understands version {CURRENT_SCHEMA_VERSION}")]
    UnrecognizedSchemaVersion { found: u32 },
    #[error("unknown rule '{found}' under `rules:` -- this build ships: {}", known.join(", "))]
    UnknownRule { found: String, known: Vec<String> },
    #[error("rule '{rule}' does not take the option '{option}'")]
    OptionNotApplicable { rule: String, option: String },
    #[error("rule '{rule}' option 'fields' names '{field}', which is not a declared pointer_fields or narrative_fields entry of any record type")]
    UndeclaredRuleField { rule: String, field: String },
    #[error("rule '{rule}' option 'field_levels' {reason}")]
    InvalidFieldLevels { rule: String, reason: String },
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
    #[error("rule '{rule}' option 'drift_audit_field' {reason}")]
    InvalidDriftAuditField { rule: String, reason: String },
    #[error("embodiment: {reason}")]
    InvalidEmbodiment { reason: String },
    #[error("locator_resolvers: {reason}")]
    InvalidLocatorResolvers { reason: String },
    #[error("record type '{record_type}' companion_fields: {reason}")]
    InvalidCompanionFields { record_type: String, reason: String },
    #[error("relation_pairs: {reason}")]
    InvalidRelationPair { reason: String },
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
    #[error("record type '{record_type}' declares revision_log {reason}")]
    InvalidRevisionLog {
        record_type: String,
        reason: &'static str,
    },
    #[error("rule '{rule}' option '{option}' entry '{entry}' {reason}")]
    InvalidPublishedPath {
        rule: String,
        option: &'static str,
        entry: String,
        reason: &'static str,
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
    #[error("rule '{rule}' option 'non_reference_field' {reason}")]
    InvalidNonReferenceField { rule: String, reason: String },
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
}

const RULE_OPTIONS: &[RuleOption] = &[
    RuleOption {
        name: "not_in",
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
    },
    RuleOption {
        name: "claim_paths",
        consequence_if_missing: "scans nothing and can never report",
        optional: false,
    },
    RuleOption {
        name: "closed_statuses",
        consequence_if_missing: "treats every claim as a violation, including correct ones",
        optional: false,
    },
    RuleOption {
        name: "terminal_statuses",
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
    },
    RuleOption {
        name: "published_paths",
        consequence_if_missing: "declares no boundary and can never report",
        optional: false,
    },
    RuleOption {
        name: "source_paths",
        consequence_if_missing: "declares no source and can never report",
        optional: false,
    },
    RuleOption {
        name: "application_paths",
        consequence_if_missing: "declares no application source and can never report",
        optional: false,
    },
    RuleOption {
        name: "exclude_paths",
        consequence_if_missing: "excludes nothing from the source",
        optional: true,
    },
    RuleOption {
        name: "back_pointer_paths",
        consequence_if_missing: "scans no source file and can never report",
        optional: false,
    },
    RuleOption {
        name: "test_paths",
        consequence_if_missing: "reads no `Verifies:` comment as test evidence",
        optional: true,
    },
    RuleOption {
        name: "sticky_states",
        consequence_if_missing: "holds every stated Embodiment to the computed value",
        optional: true,
    },
    RuleOption {
        name: "date_fields",
        consequence_if_missing: "names no date field and can never report",
        optional: false,
    },
    RuleOption {
        name: "date_order",
        consequence_if_missing: "names no date ordering and can never report",
        optional: false,
    },
    RuleOption {
        name: "accepted_statuses",
        consequence_if_missing: "leaves a type with no declared success statuses (implementable statuses for the back-pointer gate) unjudged",
        optional: true,
    },
    RuleOption {
        name: "reference_sections",
        consequence_if_missing: "reads no section and can never report",
        optional: false,
    },
    RuleOption {
        name: "external_ids",
        consequence_if_missing: "resolves every prefixed id as a record, including external standard numbers",
        optional: true,
    },
    RuleOption {
        name: "fields",
        consequence_if_missing: "checks every declared field",
        optional: true,
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
    crate::rules::rule_def(rule).is_some_and(|d| {
        d.options
            .iter()
            .any(|o| *o == "indexes" || RULE_OPTIONS.iter().any(|r| r.name == *o))
    })
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
fn published_path_problem(entry: &str) -> Option<&'static str> {
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

pub fn parse(content: &str) -> Result<Config, ConfigError> {
    parse_with_document_models(content).map(|(config, _)| config)
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
    let mut config: Config = yaml_serde::from_str(content)?;
    if config.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::UnrecognizedSchemaVersion {
            found: config.schema_version,
        });
    }
    let models = crate::document_model::validate_declarations(&config.record_types)?;
    config
        .embodiment
        .validate()
        .map_err(|reason| ConfigError::InvalidEmbodiment { reason })?;
    crate::locator::validate_resolvers(&config.locator_resolvers)
        .map_err(|reason| ConfigError::InvalidLocatorResolvers { reason })?;
    crate::status_model::validate(&config.record_types, &config.embodiment)?;
    validate_relation_pairs(&config)?;
    validate_companion_fields(&config)?;
    // Values that reach filenames and discovery: validated here, once, so
    // every command (and `doctor`, which loads through this) agrees.
    let mut seen_prefix: std::collections::BTreeMap<String, &String> = Default::default();
    let mut seen_dir: std::collections::BTreeMap<std::path::PathBuf, &String> = Default::default();
    let mut suppression_type: Option<&String> = None;
    for (name, rt) in &config.record_types {
        let mut own_dirs: std::collections::BTreeSet<std::path::PathBuf> = Default::default();
        for dir in rt.all_dirs() {
            if dir_names_nothing(dir) || dir_escapes_repo(dir) {
                return Err(ConfigError::InvalidDir {
                    record_type: name.clone(),
                    dir: dir.to_string(),
                });
            }
            // Compared as paths so `docs/adr/` and `docs/adr` are one directory.
            let path: std::path::PathBuf = std::path::Path::new(dir.trim()).components().collect();
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
            let reason = match (&decl.marker, &decl.heading) {
                (None, None) if decl.required.is_some() || decl.version_bump.is_some() => None,
                (None, None) => Some("neither `marker` nor `heading` -- name one"),
                (Some(_), Some(_)) => Some("both `marker` and `heading` -- name exactly one"),
                (Some(t), None) | (None, Some(t)) if t.trim().is_empty() => {
                    Some("a blank identity -- it would match no line")
                }
                (Some(t), None) | (None, Some(t)) if t.contains('\n') => {
                    Some("an identity spanning lines -- it would match no line")
                }
                _ => None,
            };
            let reason = reason.or_else(|| {
                let bump = decl.version_bump.as_ref()?;
                if bump.field.trim().is_empty() {
                    Some("a `version_bump` with a blank `field` -- it names no field")
                } else if bump.classes.is_empty() {
                    Some("a `version_bump` with no `classes` -- it would require nothing")
                } else if bump.classes.iter().any(|c| c.trim().is_empty()) {
                    Some("a `version_bump` with a blank class -- it would match no row")
                } else {
                    None
                }
            });
            if let Some(reason) = reason {
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
            if let Some(reason) = index_problem(spec, &config) {
                return Err(ConfigError::InvalidIndex {
                    rule: name.clone(),
                    path: spec.path.clone(),
                    reason,
                });
            }
        }
    }
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
    // A list of blanks is an empty list wearing a value.
    let declared = |v: Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());

    for (name, setting) in &config.rules {
        for opt in RULE_OPTIONS {
            let present = option_field(opt.name, setting).is_some();
            if present && !crate::rules::rule_accepts_option(name, opt.name) {
                return Err(ConfigError::OptionNotApplicable {
                    rule: name.clone(),
                    option: opt.name.to_string(),
                });
            }
        }
        if setting.non_reference_field.is_some()
            && !crate::rules::rule_accepts_option(name, "non_reference_field")
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "non_reference_field".to_string(),
            });
        }
        if setting.skip_revision_log.is_some()
            && !crate::rules::rule_accepts_option(name, "skip_revision_log")
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "skip_revision_log".to_string(),
            });
        }
        if setting.allow_extension_prefix.is_some()
            && !crate::rules::rule_accepts_option(name, "allow_extension_prefix")
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "allow_extension_prefix".to_string(),
            });
        }

        if setting.pending_until_required.is_some()
            && !crate::rules::rule_accepts_option(name, "pending_until_required")
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "pending_until_required".to_string(),
            });
        }

        if setting.template_placeholders.is_some()
            && !crate::rules::rule_accepts_option(name, "template_placeholders")
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "template_placeholders".to_string(),
            });
        }

        if let (Some(orders), true) = (&setting.date_order, setting.level != RuleLevel::Off) {
            for entry in orders {
                let bad = |reason: String| ConfigError::InvalidDateOrder {
                    rule: name.clone(),
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
        if let Some(levels) = &setting.field_levels {
            if !crate::rules::rule_accepts_option(name, "field_levels") {
                return Err(ConfigError::OptionNotApplicable {
                    rule: name.clone(),
                    option: "field_levels".to_string(),
                });
            }
            if levels.is_empty() {
                return Err(ConfigError::InvalidFieldLevels {
                    rule: name.clone(),
                    reason: "is empty; name a field or remove the option".to_string(),
                });
            }
            for (field, level) in levels {
                if *level == RuleLevel::Off {
                    return Err(ConfigError::InvalidFieldLevels {
                        rule: name.clone(),
                        reason: format!(
                            "gives '{field}' the level 'off'; use 'warn' or 'error' (a field is switched off by not declaring it)"
                        ),
                    });
                }
                let declared = config
                    .record_types
                    .values()
                    .any(|t| t.declared_fields().iter().any(|f| f.as_str() == field));
                if !declared {
                    return Err(ConfigError::InvalidFieldLevels {
                        rule: name.clone(),
                        reason: format!(
                            "names '{field}', which is not a declared field (required_fields or known_fields) of any record type"
                        ),
                    });
                }
            }
        }

        // A sticky state equal to a computed one would hide real drift: that
        // state is exactly what the evidence can reproduce.
        if setting.level != RuleLevel::Off {
            for state in setting.sticky_states.iter().flatten() {
                if config.embodiment.is_computed(state.trim()) {
                    return Err(ConfigError::StickyStateIsComputed {
                        rule: name.clone(),
                        state: state.clone(),
                    });
                }
            }
        }

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
                        rule: name.clone(),
                        reason,
                    });
                }
            }
        }

        for (option, present) in [
            ("drift_grace_days", setting.drift_grace_days.is_some()),
            ("drift_audit_field", setting.drift_audit_field.is_some()),
            ("claim_max_age_days", setting.claim_max_age_days.is_some()),
        ] {
            if present && !crate::rules::rule_accepts_option(name, option) {
                return Err(ConfigError::OptionNotApplicable {
                    rule: name.clone(),
                    option: option.to_string(),
                });
            }
        }

        // An option-taking rule given no option does not fall back to a sane
        // default -- it inverts or goes inert, and both report success
        // (BUG-44). `claim.status-agreement` with an empty `closed_statuses`
        // treats every claim as a violation; with no `claim_paths` it scans
        // nothing forever. A declined rule never runs, so `gated` never reads
        // its options -- requiring them anyway would mean a rule could not be
        // turned off without supplying values it will not use.
        if setting.level == RuleLevel::Off {
            continue;
        }
        for opt in RULE_OPTIONS
            .iter()
            .filter(|o| crate::rules::rule_accepts_option(name, o.name))
        {
            // Non-empty *and* no blank entries, not merely present: an empty
            // list is the harm each consequence describes, and a list of
            // blanks is the same list wearing a value.
            let value = option_field(opt.name, setting);
            let ok = if opt.optional {
                value.is_none_or(declared)
            } else {
                value.is_some_and(declared)
            };
            if !ok {
                return Err(ConfigError::OptionRequired {
                    rule: name.clone(),
                    option: opt.name.to_string(),
                    consequence: opt.consequence_if_missing,
                });
            }
        }
    }
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
                        "source_paths" | "application_paths" | "exclude_paths" | "test_paths"
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
    // `external_ids` entries must parse and sit under a declared record prefix:
    // an entry that can never match a citation is a rule option that is a
    // silent no-op.
    {
        let prefixes = crate::rules::declared_prefixes(&config);
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
    }
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
    Ok((config, models))
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

/// Load-time checks for `relation_pairs`: every misdeclaration is refused,
/// because a pair that cannot run reads as a clean corpus.
fn validate_relation_pairs(config: &Config) -> Result<(), ConfigError> {
    let bad = |reason: String| Err(ConfigError::InvalidRelationPair { reason });
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
    // A status-agreement rule with no type declaring `statuses.blocked` examines
    // nothing (`MILE-144`, ADR-7).
    if config
        .rules
        .get(crate::rules::RULE_SEQUENCE_STATUS_AGREEMENT)
        .is_some_and(|s| s.level != RuleLevel::Off)
        && !config
            .record_types
            .values()
            .any(|t| t.status_decl().is_some_and(|d| !d.blocked.is_empty()))
    {
        return bad(format!(
            "rule '{}' is enabled but no record type declares statuses.blocked, so it would examine nothing",
            crate::rules::RULE_SEQUENCE_STATUS_AGREEMENT
        ));
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
            matches!(e, ConfigError::UnrecognizedSchemaVersion { found: 99 }),
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
        };
        let declared = t.declared_fields();
        assert!(declared.contains("Status"));
        assert!(declared.contains("Embodiment"));
        assert_eq!(declared.len(), 2);
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
        let base = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\nrules:\n";

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
            err,
            ConfigError::UnrecognizedSchemaVersion { found: 99 }
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
        for dup in ["[docs/adr]", "[a, a]", "[\"docs/adr/\"]"] {
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

#[cfg(test)]
mod option_table_tests {
    use super::*;

    /// Options that are not list-valued, so `RULE_OPTIONS` (which carries the
    /// "required list" consequence text) does not describe them.
    const OTHER_OPTIONS: &[&str] = &[
        "indexes",
        "issue_refs",
        "allow_extension_prefix",
        "pending_until_required",
        "template_placeholders",
        "skip_revision_log",
        "non_reference_field",
        "field_levels",
        "drift_grace_days",
        "drift_audit_field",
        "claim_max_age_days",
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
