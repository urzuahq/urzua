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
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            Bare(RuleLevel),
            Table(Table),
        }

        Ok(match Either::deserialize(deserializer)? {
            Either::Bare(level) => RuleSetting {
                level,
                not_in: None,
            },
            Either::Table(t) => RuleSetting {
                level: t.level,
                not_in: t.not_in,
            },
        })
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
    #[error("unknown rule '{found}' in [rules] -- this build ships: {}", known.join(", "))]
    UnknownRule { found: String, known: Vec<String> },
    #[error("rule '{rule}' does not take the option '{option}'")]
    OptionNotApplicable { rule: String, option: String },
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
    for (name, setting) in &config.rules {
        if setting.not_in.is_some() && name != crate::rules::RULE_POINTER_TARGET_STATUS {
            return Err(ConfigError::OptionNotApplicable {
                rule: name.clone(),
                option: "not_in".to_string(),
            });
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
