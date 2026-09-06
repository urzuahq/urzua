//! Minimal `.urzua/config.toml` parsing (SPEC-0003): record types, their
//! directories, and required header fields. Phased -- this is the slice
//! Phase A of `urzua check` needs, not the full spec.

use crate::header::HeaderShape;
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
