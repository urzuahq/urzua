//! Minimal `.urzua/config.yaml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::header::{HeaderLayout, HeaderShape};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The only schema version defined so far (ADR-0012). A config declaring any
/// other value is a parse-time error, not a silent best-effort read.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub record_types: HashMap<String, RecordTypeConfig>,
    /// Every rule a repository has turned on, and at what level. Absent or
    /// empty means no rule runs: governance is declared, never inherited
    /// (ADR-53).
    #[serde(default)]
    pub rules: HashMap<String, RuleSetting>,
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
        }

        // Dispatched on the parsed value rather than through `#[serde(untagged)]`.
        // Untagged discards each variant's error, so every malformed setting --
        // a bad level, a bad level in table form, an unknown key -- collapsed
        // into "data did not match any variant of untagged enum Either", and the
        // messages naming the valid levels and keys were unreachable (BUG-46).
        let value = yaml_serde::Value::deserialize(deserializer)?;
        if value.is_mapping() {
            let table = Table::deserialize(value).map_err(serde::de::Error::custom)?;
            Ok(RuleSetting {
                level: table.level,
                not_in: table.not_in,
                claim_paths: table.claim_paths,
                closed_statuses: table.closed_statuses,
            })
        } else {
            let level = RuleLevel::deserialize(value).map_err(serde::de::Error::custom)?;
            Ok(RuleSetting {
                level,
                not_in: None,
                claim_paths: None,
                closed_statuses: None,
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTypeConfig {
    pub dir: String,
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
    /// value meaning "decided, none needed", so the signal cannot be answered
    /// -- only silenced per-repo once rule severity is configurable
    /// (MILE-0080). An earlier version of this comment claimed a type "can
    /// permanently have none declared"; nothing implemented that (ADR-0051).
    #[serde(default)]
    pub spec: Option<String>,
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
            other => Err(serde::de::Error::custom(format!(
                "unrecognized header_shape '{other}' -- expected \"blockquote\", \"bold-list\", or \"yaml-frontmatter\""
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
    #[error("rule '{rule}' requires the option '{option}' -- without it the rule {consequence}")]
    OptionRequired {
        rule: String,
        option: String,
        consequence: &'static str,
    },
}

/// Rules that cannot be declared without their options. Adopt mode skips these
/// rather than proposing a declaration that will not load (BUG-44).
pub fn rule_requires_options(rule: &str) -> bool {
    rule == crate::rules::RULE_CLAIM_STATUS_AGREEMENT
        || rule == crate::rules::RULE_POINTER_TARGET_STATUS
}

pub fn parse(content: &str) -> Result<Config, ConfigError> {
    let config: Config = yaml_serde::from_str(content)?;
    if config.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::UnrecognizedSchemaVersion {
            found: config.schema_version,
        });
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
    // A list of blanks is an empty list wearing a value.
    let declared = |v: &Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());

    for (name, setting) in &config.rules {
        let misplaced = [
            (
                setting.not_in.is_some(),
                "not_in",
                crate::rules::RULE_POINTER_TARGET_STATUS,
            ),
            (
                setting.claim_paths.is_some(),
                "claim_paths",
                crate::rules::RULE_CLAIM_STATUS_AGREEMENT,
            ),
            (
                setting.closed_statuses.is_some(),
                "closed_statuses",
                crate::rules::RULE_CLAIM_STATUS_AGREEMENT,
            ),
        ];
        for (present, option, owner) in misplaced {
            if present && name != owner {
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
        // nothing forever.
        let required: &[(bool, &str, &'static str)] =
            if name == crate::rules::RULE_CLAIM_STATUS_AGREEMENT {
                &[
                    // Non-empty *and* no blank entries, not merely present. An
                    // empty list is the harm these messages describe --
                    // `closed_statuses: []` makes every claim a violation,
                    // `claim_paths: []` scans nothing -- and a list of blanks is
                    // the same list wearing a value.
                    (
                        setting.claim_paths.as_ref().is_some_and(declared),
                        "claim_paths",
                        "scans nothing and can never report",
                    ),
                    (
                        setting.closed_statuses.as_ref().is_some_and(declared),
                        "closed_statuses",
                        "treats every claim as a violation, including correct ones",
                    ),
                ]
            } else if name == crate::rules::RULE_POINTER_TARGET_STATUS {
                &[(
                    setting.not_in.as_ref().is_some_and(declared),
                    "not_in",
                    "examines every reference and can never report",
                )]
            } else {
                &[]
            };
        for (present, option, consequence) in required {
            // A declined rule never runs, so `gated` never reads its options.
            // Requiring them anyway would mean a rule could not be turned off
            // without supplying values it will not use.
            if setting.level != RuleLevel::Off && !present {
                return Err(ConfigError::OptionRequired {
                    rule: name.clone(),
                    option: option.to_string(),
                    consequence,
                });
            }
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
