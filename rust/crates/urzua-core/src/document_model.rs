//! A record type's declared document model (RFC-33 layer 1, ADR-53,
//! MILE-98): how its identity, fields and sections are found.
//!
//! This slice only parses and validates the declaration. Nothing in the
//! engine reads it yet, so declaring it changes no finding.
//!
//! ```yaml
//! record_types:
//!   adr:
//!     dir: doc/adr
//!     identity: {from: filename, pattern: '^(?P<number>\d+)-(?P<slug>.+)$'}
//!     fields:   {from: prefix-lines}
//!     sections: {from: headings, depth: 2, items: true}
//! ```
//!
//! The three keys sit under the record type in YAML but are held apart from
//! `RecordTypeConfig` (see `config::parse_with_document_models`).

use crate::config::{ConfigError, RecordTypeConfig};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The deepest `depth` a `sections` declaration may name: `##` through
/// `######` is five levels.
pub const MAX_SECTION_DEPTH: u8 = 5;

/// One record type's declaration. Each part is independently optional; a
/// part left out is simply not declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentModel {
    pub identity: Option<IdentityDecl>,
    pub fields: Option<FieldsDecl>,
    pub sections: Option<SectionsDecl>,
}

/// Where a record's identity is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentitySource {
    Filename,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDecl {
    pub from: IdentitySource,
    /// A regular expression over the file name, stored as written. Only
    /// checked to be non-blank here: compiling it needs a regex engine, which
    /// `urzua-core`'s dependency allowlist does not yet admit.
    pub pattern: String,
}

/// How a record's fields are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldsSource {
    PrefixLines,
    YamlFrontmatter,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldsDecl {
    pub from: FieldsSource,
}

/// How a record's sections are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SectionsSource {
    Headings,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionsDecl {
    pub from: SectionsSource,
    /// How many heading levels, counting from `##`, are sections: 1 is `##`
    /// only, 2 is `##` and `###`. Omitted means 1.
    #[serde(default = "default_depth", deserialize_with = "deserialize_depth")]
    pub depth: u8,
    /// Whether bullet items within a section are addressable. Omitted means
    /// they are not.
    #[serde(default, deserialize_with = "deserialize_items")]
    pub items: bool,
}

/// Hand-written so the error names `depth` and the valid range: serde's own
/// type errors do not say which key they came from.
fn deserialize_depth<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let v = yaml_serde::Value::deserialize(d)?;
    match v.as_u64() {
        Some(n) if (1..=u64::from(MAX_SECTION_DEPTH)).contains(&n) => Ok(n as u8),
        _ => Err(serde::de::Error::custom(format!(
            "`depth` must be an integer from 1 to {MAX_SECTION_DEPTH}"
        ))),
    }
}

fn deserialize_items<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    yaml_serde::Value::deserialize(d)?
        .as_bool()
        .ok_or_else(|| serde::de::Error::custom("`items` must be true or false"))
}

fn default_depth() -> u8 {
    1
}

/// Validates the `identity`, `fields` and `sections` declarations of every
/// record type of a typed-parsed config and returns them as declarations.
///
/// A type that declares none of the three is absent from the result.
pub(crate) fn validate_declarations(
    types: &BTreeMap<String, RecordTypeConfig>,
) -> Result<BTreeMap<String, DocumentModel>, ConfigError> {
    let mut models = BTreeMap::new();
    for (name, rt) in types {
        let identity = take::<IdentityDecl>(rt.identity.as_ref(), name, "identity")?;
        let fields = take::<FieldsDecl>(rt.fields.as_ref(), name, "fields")?;
        let sections = take::<SectionsDecl>(rt.sections.as_ref(), name, "sections")?;
        if let Some(i) = &identity {
            if i.pattern.trim().is_empty() {
                return Err(invalid(
                    name,
                    "identity",
                    "`pattern` must not be blank".into(),
                ));
            }
        }
        if identity.is_some() || fields.is_some() || sections.is_some() {
            models.insert(
                name.clone(),
                DocumentModel {
                    identity,
                    fields,
                    sections,
                },
            );
        }
    }
    Ok(models)
}

fn invalid(record_type: &str, key: &'static str, reason: String) -> ConfigError {
    ConfigError::InvalidDocumentModel {
        record_type: record_type.to_string(),
        key,
        reason,
    }
}

/// Parses one raw declaration. A key present with no value is refused rather
/// than read as absent, which would turn a half-written declaration into a
/// silent no-op.
fn take<T: serde::de::DeserializeOwned>(
    value: Option<&yaml_serde::Value>,
    record_type: &str,
    key: &'static str,
) -> Result<Option<T>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Err(invalid(
            record_type,
            key,
            "present with no value; give it a mapping or remove the key".into(),
        ));
    }
    yaml_serde::from_value(value.clone())
        .map(Some)
        .map_err(|e| invalid(record_type, key, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(yaml: &str) -> Result<BTreeMap<String, DocumentModel>, ConfigError> {
        let c: crate::config::Config = yaml_serde::from_str(yaml).unwrap();
        validate_declarations(&c.record_types)
    }

    fn cfg(type_body: &str) -> String {
        format!("schema_version: 2\nrecord_types:\n  adr:\n    dir: doc/adr\n{type_body}")
    }

    fn invalid(type_body: &str) -> String {
        match split(&cfg(type_body)) {
            Err(e) => e.to_string(),
            Ok(m) => panic!("expected an error, got {m:?}"),
        }
    }

    #[test]
    fn the_rfc_example_parses_in_full() {
        let m = split(&cfg(
            "    identity: {from: filename, pattern: '^(?P<number>\\d+)-(?P<slug>.+)$'}\n    fields: {from: prefix-lines}\n    sections: {from: headings, depth: 2, items: true}\n",
        ))
        .unwrap();
        let adr = &m["adr"];
        let id = adr.identity.as_ref().unwrap();
        assert_eq!(id.from, IdentitySource::Filename);
        assert_eq!(id.pattern, r"^(?P<number>\d+)-(?P<slug>.+)$");
        assert_eq!(adr.fields.as_ref().unwrap().from, FieldsSource::PrefixLines);
        let s = adr.sections.as_ref().unwrap();
        assert_eq!(
            (s.from, s.depth, s.items),
            (SectionsSource::Headings, 2, true)
        );
    }

    #[test]
    fn each_part_is_independently_optional_and_sections_default_narrow() {
        let m = split(&cfg("    sections: {from: headings}\n")).unwrap();
        let adr = &m["adr"];
        assert!(adr.identity.is_none() && adr.fields.is_none());
        let s = adr.sections.as_ref().unwrap();
        assert_eq!((s.depth, s.items), (1, false));
        let m = split(&cfg("    fields: {from: yaml-frontmatter}\n")).unwrap();
        assert_eq!(
            m["adr"].fields.as_ref().unwrap().from,
            FieldsSource::YamlFrontmatter
        );
    }

    #[test]
    fn a_type_declaring_nothing_is_absent() {
        let yaml = cfg("    required_fields: [Status]\n");
        assert!(split(&yaml).unwrap().is_empty());
    }

    #[test]
    fn declarations_are_kept_per_record_type() {
        let yaml = "schema_version: 2\nrecord_types:\n  adr:\n    dir: a\n    fields: {from: prefix-lines}\n  rfc:\n    dir: r\n    sections: {from: headings}\n  note:\n    dir: n\n";
        let m = split(yaml).unwrap();
        assert_eq!(
            m.keys().map(String::as_str).collect::<Vec<_>>(),
            ["adr", "rfc"]
        );
    }

    #[test]
    fn an_unknown_source_names_the_valid_ones() {
        let e = invalid("    fields: {from: blockquote}\n");
        assert!(e.contains("adr") && e.contains("fields"), "{e}");
        assert!(
            e.contains("prefix-lines") && e.contains("yaml-frontmatter"),
            "{e}"
        );
        let e = invalid("    sections: {from: h2}\n");
        assert!(e.contains("headings"), "{e}");
        let e = invalid("    identity: {from: content, pattern: x}\n");
        assert!(e.contains("filename"), "{e}");
    }

    #[test]
    fn from_is_required() {
        assert!(invalid("    sections: {depth: 2}\n").contains("from"));
        assert!(invalid("    fields: {}\n").contains("from"));
        assert!(invalid("    identity: {pattern: x}\n").contains("from"));
    }

    #[test]
    fn identity_requires_a_nonblank_pattern() {
        assert!(invalid("    identity: {from: filename}\n").contains("pattern"));
        assert!(invalid("    identity: {from: filename, pattern: ''}\n").contains("pattern"));
        assert!(invalid("    identity: {from: filename, pattern: '  '}\n").contains("pattern"));
    }

    #[test]
    fn depth_must_be_one_to_five() {
        for bad in ["0", "6", "-1", "2.5", "'2'"] {
            let e = invalid(&format!("    sections: {{from: headings, depth: {bad}}}\n"));
            assert!(e.contains("adr") && e.contains("depth"), "{bad}: {e}");
        }
        for ok in 1..=MAX_SECTION_DEPTH {
            let m = split(&cfg(&format!(
                "    sections: {{from: headings, depth: {ok}}}\n"
            )))
            .unwrap();
            assert_eq!(m["adr"].sections.as_ref().unwrap().depth, ok);
        }
    }

    #[test]
    fn items_must_be_a_boolean() {
        assert!(invalid("    sections: {from: headings, items: yes please}\n").contains("items"));
        assert!(invalid("    sections: {from: headings, items: 1}\n").contains("items"));
    }

    #[test]
    fn unknown_keys_inside_a_part_are_refused() {
        assert!(invalid("    sections: {from: headings, deep: 2}\n").contains("deep"));
        assert!(invalid("    fields: {from: prefix-lines, key: Date}\n").contains("key"));
        assert!(
            invalid("    identity: {from: filename, pattern: x, extra: 1}\n").contains("extra")
        );
    }

    #[test]
    fn a_present_but_empty_or_non_mapping_part_is_refused_not_ignored() {
        for body in [
            "    sections:\n",
            "    fields: null\n",
            "    identity: filename\n",
            "    sections: [headings]\n",
        ] {
            let e = invalid(body);
            assert!(e.contains("adr"), "{body}: {e}");
        }
    }
}
