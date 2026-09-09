//! Minimal `.urzua/config.toml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::header::{HeaderLayout, HeaderShape};
use serde::Deserialize;
use std::collections::HashMap;

/// The only schema version defined so far (ADR-0012). A config declaring any
/// other value is a parse-time error, not a silent best-effort read.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub record_types: HashMap<String, RecordTypeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
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
    /// Which spec (if any) documents this type's schema as a coherent
    /// feature area (MILE-0077/ADR-0041 -- "does this type need a spec"
    /// stays editorial, never inferred). Declared, not voted, same
    /// principle as `header_layout`/`known_fields`: omitted means no spec
    /// currently covers this type, which `type.no-declared-spec` (MILE-0077
    /// follow-up) surfaces as an inventory signal, not a mandate -- a type
    /// can permanently have none declared if that's the right editorial
    /// call.
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
    Parse(#[from] toml::de::Error),
    #[error("unrecognized schema_version {found} -- this build of urzua understands version {CURRENT_SCHEMA_VERSION}")]
    UnrecognizedSchemaVersion { found: u32 },
}

pub fn parse(content: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(content)?;
    if config.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::UnrecognizedSchemaVersion {
            found: config.schema_version,
        });
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_record_types() {
        let toml = r#"
schema_version = 1

[record_types.adr]
dir = "docs/adr"
required_fields = ["Status", "Date"]
"#;
        let config = parse(toml).unwrap();
        let adr = config.record_types.get("adr").unwrap();
        assert_eq!(adr.dir, "docs/adr");
        assert_eq!(adr.required_fields, vec!["Status", "Date"]);
    }

    #[test]
    fn spec_pointer_is_optional_and_parses_when_present() {
        let toml = r#"
schema_version = 1

[record_types.milestone]
dir = "docs/milestones"
spec = "SPEC-6"

[record_types.bug]
dir = "docs/bugs"
"#;
        let config = parse(toml).unwrap();
        assert_eq!(
            config.record_types.get("milestone").unwrap().spec,
            Some("SPEC-6".to_string())
        );
        assert_eq!(config.record_types.get("bug").unwrap().spec, None);
    }

    #[test]
    fn pointer_and_narrative_fields_round_trip() {
        let toml = r#"
schema_version = 1

[record_types.rfc]
dir = "docs/rfc"
pointer_fields = ["Implements", "Amends"]
narrative_fields = []

[record_types.bug]
dir = "docs/bugs"
"#;
        let config = parse(toml).unwrap();
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
        let toml = r#"
schema_version = 1

[record_types.adr]
dir = "docs/adr"
required_fielsd = ["Status"]
"#;
        assert!(parse(toml).is_err());
    }

    #[test]
    fn missing_schema_version_is_a_parse_error() {
        let toml = r#"
[record_types.adr]
dir = "docs/adr"
"#;
        assert!(parse(toml).is_err());
    }

    #[test]
    fn an_unrecognized_schema_version_is_rejected() {
        let toml = r#"
schema_version = 99

[record_types.adr]
dir = "docs/adr"
"#;
        let err = parse(toml).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::UnrecognizedSchemaVersion { found: 99 }
        ));
    }
}
