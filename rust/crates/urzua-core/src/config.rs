//! Minimal `.urzua/config.yaml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::document_model::DocumentModel;
use crate::header::{HeaderLayout, HeaderShape};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The only schema version defined so far (ADR-0012). A config declaring any
/// other value is a parse-time error, not a silent best-effort read.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

impl Config {
    /// Every declared record type's name, in `record_types`' own (sorted)
    /// order.
    pub fn sorted_type_names(&self) -> Vec<&String> {
        self.record_types.keys().collect()
    }
}

/// What a repository declared about one rule. Written either as a bare level
/// (`field.quality: error`) or as a table when a rule takes options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// Path prefixes of source code that must change with a governed record.
    /// Only meaningful to `diff.source-requires-record` (`MILE-124`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_paths: Option<Vec<String>>,
    /// Path prefixes of application source. Only meaningful to
    /// `diff.source-and-records-mixed` (`MILE-142`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_paths: Option<Vec<String>>,
    /// Path prefixes removed from `application_paths` (tests, fixtures). Only
    /// meaningful to `diff.source-and-records-mixed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_paths: Option<Vec<String>>,
    /// Admit header keys beginning `x-` (case-insensitive) as extension keys
    /// (`MILE-141`). Only meaningful to `header.field-set-consistency`, where
    /// omitted means `false`: every undeclared key is a finding. A declared
    /// `true` exempts prefixed keys from that rule and nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_extension_prefix: Option<bool>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleLevel {
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

impl<'de> Deserialize<'de> for RuleSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Table {
            level: RuleLevel,
            #[serde(default)]
            not_in: Option<Vec<String>>,
            #[serde(default)]
            claim_paths: Option<Vec<String>>,
            #[serde(default)]
            closed_statuses: Option<Vec<String>>,
            #[serde(default)]
            terminal_statuses: Option<Vec<String>>,
            #[serde(default)]
            fields: Option<Vec<String>>,
            #[serde(default)]
            published_paths: Option<Vec<String>>,
            #[serde(default)]
            indexes: Option<Vec<IndexSpec>>,
            #[serde(default)]
            source_paths: Option<Vec<String>>,
            #[serde(default)]
            application_paths: Option<Vec<String>>,
            #[serde(default)]
            exclude_paths: Option<Vec<String>>,
            #[serde(default)]
            allow_extension_prefix: Option<bool>,
        }

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
            let table = Table::deserialize(value).map_err(serde::de::Error::custom)?;
            Ok(RuleSetting {
                level: table.level,
                not_in: table.not_in,
                claim_paths: table.claim_paths,
                closed_statuses: table.closed_statuses,
                terminal_statuses: table.terminal_statuses,
                fields: table.fields,
                published_paths: table.published_paths,
                indexes: table.indexes,
                source_paths: table.source_paths,
                application_paths: table.application_paths,
                exclude_paths: table.exclude_paths,
                allow_extension_prefix: table.allow_extension_prefix,
            })
        } else {
            let level = RuleLevel::deserialize(value).map_err(serde::de::Error::custom)?;
            Ok(RuleSetting {
                level,
                not_in: None,
                claim_paths: None,
                closed_statuses: None,
                terminal_statuses: None,
                fields: None,
                published_paths: None,
                indexes: None,
                source_paths: None,
                application_paths: None,
                exclude_paths: None,
                allow_extension_prefix: None,
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTypeConfig {
    pub dir: String,
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
    #[serde(default)]
    pub required_fields: Vec<String>,
    /// RFC-0010: the header shape is declared per profile, never sniffed.
    /// Omitted means the long-standing blockquote shape, so an existing
    /// config needs no change to keep its current behavior.
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
    /// The expected line layout (MILE-0075) within `Blockquote`'s own
    /// tolerance for how fields are laid out -- one field per line, or
    /// several pipe-delimited on one line. Declared, not voted, the same
    /// principle Rule 1 already uses for required fields: a majority-rule
    /// inference would silently ratify whatever drifted in. Omitted means
    /// this axis isn't checked for the type -- additive, not a forced
    /// migration for a corpus that hasn't declared it.
    #[serde(default)]
    pub header_layout: Option<HeaderLayout>,
    /// The additional fields a type's records may carry beyond
    /// `required_fields` (MILE-0081) -- e.g. `Embodiment`/`Realized-by` for
    /// `adr`. Declared, not voted, same principle as `header_layout`: a
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
    /// fallback would need to preserve. Every field named here must also
    /// appear in this type's `required_fields`/`known_fields`
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
    /// Unlike `header_layout`, omission is not a way to opt out: there is no
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
}

impl RelationFields {
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

impl<'de> Deserialize<'de> for HeaderShape {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "blockquote" => Ok(HeaderShape::Blockquote),
            "bold-list" => Ok(HeaderShape::BoldList),
            "yaml-frontmatter" => Ok(HeaderShape::YamlFrontmatter),
            "none" => Ok(HeaderShape::None),
            other => Err(serde::de::Error::custom(format!(
                "unrecognized header_shape '{other}' -- expected \"blockquote\", \"bold-list\", \"yaml-frontmatter\", or \"none\""
            ))),
        }
    }
}

impl Serialize for HeaderShape {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            HeaderShape::Blockquote => "blockquote",
            HeaderShape::BoldList => "bold-list",
            HeaderShape::YamlFrontmatter => "yaml-frontmatter",
            HeaderShape::None => "none",
        })
    }
}

impl Serialize for HeaderLayout {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            HeaderLayout::OnePerLine => "one-per-line",
            HeaderLayout::PipeDelimited => "pipe-delimited",
        })
    }
}

impl<'de> Deserialize<'de> for HeaderLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "one-per-line" => Ok(HeaderLayout::OnePerLine),
            "pipe-delimited" => Ok(HeaderLayout::PipeDelimited),
            other => Err(serde::de::Error::custom(format!(
                "unrecognized header_layout '{other}' -- expected \"one-per-line\" or \"pipe-delimited\""
            ))),
        }
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
    #[error("rule '{rule}' requires the option '{option}' -- without it the rule {consequence}")]
    OptionRequired {
        rule: String,
        option: String,
        consequence: &'static str,
    },
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

/// One option's name, the one rule it belongs to, and what happens if a rule
/// that needs it is declared without it. The single source for three facts a
/// hand-maintained list per fact would let drift apart: which rule an option
/// is "misplaced" on if declared elsewhere, whether a rule "requires options"
/// to load at all (`rule_requires_options`, below), and what a missing
/// required option's consequence is (`parse`'s `OptionRequired` check).
/// Adding a fifth option-taking rule is one entry here instead of three edits
/// that could drift from each other.
struct RuleOption {
    name: &'static str,
    owner: &'static str,
    consequence_if_missing: &'static str,
    /// An optional option may be omitted, but a declared one is still
    /// validated (non-empty, no blanks) so it cannot be a silent no-op.
    optional: bool,
}

const RULE_OPTIONS: &[RuleOption] = &[
    RuleOption {
        name: "not_in",
        owner: crate::rules::RULE_POINTER_TARGET_STATUS,
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
    },
    RuleOption {
        name: "claim_paths",
        owner: crate::rules::RULE_CLAIM_STATUS_AGREEMENT,
        consequence_if_missing: "scans nothing and can never report",
        optional: false,
    },
    RuleOption {
        name: "closed_statuses",
        owner: crate::rules::RULE_CLAIM_STATUS_AGREEMENT,
        consequence_if_missing: "treats every claim as a violation, including correct ones",
        optional: false,
    },
    RuleOption {
        name: "terminal_statuses",
        owner: crate::rules::RULE_NARRATIVE_FIELD_STALE,
        consequence_if_missing: "examines every reference and can never report",
        optional: false,
    },
    RuleOption {
        name: "published_paths",
        owner: crate::rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        consequence_if_missing: "declares no boundary and can never report",
        optional: false,
    },
    RuleOption {
        name: "source_paths",
        owner: crate::rules::RULE_DIFF_SOURCE_REQUIRES_RECORD,
        consequence_if_missing: "declares no source and can never report",
        optional: false,
    },
    RuleOption {
        name: "application_paths",
        owner: crate::rules::RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        consequence_if_missing: "declares no application source and can never report",
        optional: false,
    },
    RuleOption {
        name: "exclude_paths",
        owner: crate::rules::RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        consequence_if_missing: "excludes nothing from the application source",
        optional: true,
    },
    RuleOption {
        name: "fields",
        owner: crate::rules::RULE_POINTER_TARGET_STATUS,
        consequence_if_missing: "checks every declared field",
        optional: true,
    },
];

/// `RULE_OPTIONS`'s field, by name -- the one place that knows which
/// `RuleSetting` field an option name maps to, so every check below reads it
/// once instead of re-matching the name itself.
fn option_field<'a>(name: &str, setting: &'a RuleSetting) -> Option<&'a Vec<String>> {
    match name {
        "not_in" => setting.not_in.as_ref(),
        "claim_paths" => setting.claim_paths.as_ref(),
        "closed_statuses" => setting.closed_statuses.as_ref(),
        "terminal_statuses" => setting.terminal_statuses.as_ref(),
        "fields" => setting.fields.as_ref(),
        "published_paths" => setting.published_paths.as_ref(),
        "source_paths" => setting.source_paths.as_ref(),
        "application_paths" => setting.application_paths.as_ref(),
        "exclude_paths" => setting.exclude_paths.as_ref(),
        _ => None,
    }
}

/// Rules that cannot be declared without their options. Adopt mode skips these
/// rather than proposing a declaration that will not load (BUG-44).
pub fn rule_requires_options(rule: &str) -> bool {
    rule == crate::rules::RULE_INDEX_FRESHNESS || RULE_OPTIONS.iter().any(|o| o.owner == rule)
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
    if let Err(reason) = crate::rules::parse_index_format(&spec.format) {
        return Some(format!("has an invalid format: {reason}"));
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
    validate_relation_pairs(&config)?;
    // Values that reach filenames and discovery: validated here, once, so
    // every command (and `doctor`, which loads through this) agrees.
    let mut seen_prefix: std::collections::BTreeMap<String, &String> = Default::default();
    for (name, rt) in &config.record_types {
        if dir_names_nothing(&rt.dir) || dir_escapes_repo(&rt.dir) {
            return Err(ConfigError::InvalidDir {
                record_type: name.clone(),
                dir: rt.dir.clone(),
            });
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
        let owner = name == crate::rules::RULE_INDEX_FRESHNESS;
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
    // A list of blanks is an empty list wearing a value.
    let declared = |v: &Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());

    for (name, setting) in &config.rules {
        for opt in RULE_OPTIONS {
            let present = option_field(opt.name, setting).is_some();
            if present && name != opt.owner {
                return Err(ConfigError::OptionNotApplicable {
                    rule: name.clone(),
                    option: opt.name.to_string(),
                });
            }
        }
        if setting.allow_extension_prefix.is_some()
            && name != crate::rules::RULE_HEADER_FIELD_SET_CONSISTENCY
        {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "allow_extension_prefix".to_string(),
            });
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
        for opt in RULE_OPTIONS.iter().filter(|o| o.owner == name) {
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
        let path_options: [(&'static str, &mut Option<Vec<String>>); 4] = [
            ("published_paths", &mut setting.published_paths),
            ("source_paths", &mut setting.source_paths),
            ("application_paths", &mut setting.application_paths),
            ("exclude_paths", &mut setting.exclude_paths),
        ];
        for (option, paths) in path_options {
            let Some(paths) = paths.as_mut() else {
                continue;
            };
            for entry in paths.iter_mut() {
                let normalised = normalize_published_path(entry);
                if level != RuleLevel::Off {
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
    // A pair rule with nothing to pair is a check that ran on nothing.
    if config.relation_pairs.is_empty() {
        for rule in crate::rules::RULES_REQUIRING_RELATION_PAIRS.iter().copied() {
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
    fn header_shape_and_layout_survive_a_serialize_deserialize_round_trip() {
        for shape in [
            HeaderShape::Blockquote,
            HeaderShape::BoldList,
            HeaderShape::YamlFrontmatter,
        ] {
            let s = yaml_serde::to_string(&shape).unwrap();
            assert_eq!(shape, yaml_serde::from_str::<HeaderShape>(&s).unwrap());
        }
        for layout in [HeaderLayout::OnePerLine, HeaderLayout::PipeDelimited] {
            let s = yaml_serde::to_string(&layout).unwrap();
            assert_eq!(layout, yaml_serde::from_str::<HeaderLayout>(&s).unwrap());
        }
    }

    /// BUG-106: three independent computations of this union existed;
    /// `known_fields` alone must not drop a field only in `required_fields`.
    #[test]
    fn declared_fields_unions_required_and_known() {
        let t = RecordTypeConfig {
            dir: "docs/adr".to_string(),
            identity: None,
            fields: None,
            sections: None,
            required_fields: vec!["Status".to_string()],
            header_shape: HeaderShape::default(),
            prefix: None,
            header_layout: None,
            known_fields: Some(vec!["Embodiment".to_string()]),
            pointer_fields: None,
            narrative_fields: None,
            spec: None,
            relation_fields: None,
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
    fn a_declared_relation_pair_loads_and_cross_type_defaults_to_true() {
        let c = parse(&pair_cfg("", AMEND_PAIR)).unwrap();
        assert_eq!(c.relation_pairs.len(), 1);
        assert!(!c.relation_pairs[0].cross_type);
        let y = "relation_pairs:\n  - {name: a, forward: Amends, inverse: Amended-by, reciprocity: none}\n";
        let c = parse(&pair_cfg("", y)).unwrap();
        assert!(c.relation_pairs[0].cross_type);
        assert_eq!(c.relation_pairs[0].reciprocity, PairReciprocity::None);
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
}
