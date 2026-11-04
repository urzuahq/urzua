//! A record type's declared document model (RFC-33 layer 1, ADR-53,
//! MILE-98): how its identity, fields and sections are found.
//!
//! A declaration is validated at load. Slice 2 makes `identity` and `fields`
//! drive record loading (`RecordModel`, `Record::parse_with_model`); a type
//! that declares neither loads exactly as before. `sections` is still read by
//! nothing.
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
    /// A regular expression over the file name without its `.md` extension,
    /// stored as written. Validated at load by [`CompiledIdentity::compile`].
    pub pattern: String,
}

/// An identity pattern, compiled. Its named groups: `number` (required, all
/// ASCII digits), `slug` (optional, not read by the engine), and `prefix`
/// (optional): a pattern that captures a `prefix` declares that the record's
/// H1 title carries `PREFIX-N`, and one that does not declares a bare `N`
/// (RFC-44, "declared per type").
#[derive(Debug, Clone)]
pub struct CompiledIdentity {
    regex: regex::Regex,
}

/// What a declared identity pattern found in one record's file name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeclaredIdentity {
    /// The `number` capture as written (padding kept), or `None` when the
    /// file name does not match the pattern: the record has no identity.
    pub number: Option<String>,
    /// The `prefix` capture, when the pattern has one and matched.
    pub title_prefix: Option<String>,
}

impl CompiledIdentity {
    pub fn compile(decl: &IdentityDecl) -> Result<Self, String> {
        let regex = regex::Regex::new(&decl.pattern)
            .map_err(|e| format!("`pattern` is not a valid regular expression: {e}"))?;
        if !regex.capture_names().flatten().any(|n| n == "number") {
            return Err(
                "`pattern` must capture the record number as a `(?P<number>...)` group".into(),
            );
        }
        Ok(Self { regex })
    }

    /// Reads a file name (with or without `.md`). A name the pattern does not
    /// match, or whose `number` is not all ASCII digits, has no number.
    pub fn identify(&self, file_name: &str) -> DeclaredIdentity {
        let stem = file_name.strip_suffix(".md").unwrap_or(file_name);
        let Some(caps) = self.regex.captures(stem) else {
            return DeclaredIdentity::default();
        };
        let number = caps
            .name("number")
            .map(|m| m.as_str())
            .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            .map(str::to_string);
        if number.is_none() {
            return DeclaredIdentity::default();
        }
        DeclaredIdentity {
            number,
            title_prefix: caps.name("prefix").map(|m| m.as_str().to_string()),
        }
    }
}

/// A record type's declaration resolved for loading: the compiled identity
/// pattern and the fields source. `RecordModel::default()` declares nothing,
/// and loads exactly as before the declaration existed.
#[derive(Debug, Clone, Default)]
pub struct RecordModel {
    pub identity: Option<CompiledIdentity>,
    pub fields: Option<FieldsSource>,
}

impl DocumentModel {
    pub fn resolve(&self) -> Result<RecordModel, String> {
        Ok(RecordModel {
            identity: self
                .identity
                .as_ref()
                .map(CompiledIdentity::compile)
                .transpose()?,
            fields: self.fields.as_ref().map(|f| f.from),
        })
    }
}

/// The validated declaration of one record type, if it declares any part.
/// The config was validated at load, so an error here means a caller built a
/// `RecordTypeConfig` by hand.
pub fn declared_model(
    record_type: &str,
    config: &RecordTypeConfig,
) -> Result<Option<DocumentModel>, ConfigError> {
    let one = BTreeMap::from([(record_type.to_string(), config.clone())]);
    Ok(validate_declarations(&one)?.remove(record_type))
}

/// How a record's fields are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldsSource {
    /// Bare `Key: value` lines between the title and the first `##` heading.
    PrefixLines,
    YamlFrontmatter,
    /// The existing `header_shape` values, as cases of `fields.from`
    /// (RFC-33): a `> Key: value` blockquote block, a `- **Key:** value`
    /// list, or no header at all.
    Blockquote,
    BoldList,
    None,
}

impl FieldsSource {
    /// The existing parser a source other than `prefix-lines` reads with.
    pub fn header_shape(self) -> Option<crate::header::HeaderShape> {
        use crate::header::HeaderShape;
        match self {
            FieldsSource::PrefixLines => Option::None,
            FieldsSource::YamlFrontmatter => Some(HeaderShape::YamlFrontmatter),
            FieldsSource::Blockquote => Some(HeaderShape::Blockquote),
            FieldsSource::BoldList => Some(HeaderShape::BoldList),
            FieldsSource::None => Some(HeaderShape::None),
        }
    }
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
            CompiledIdentity::compile(i).map_err(|reason| invalid(name, "identity", reason))?;
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
        let e = invalid("    fields: {from: markdown-table}\n");
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

    fn identity(pattern: &str) -> CompiledIdentity {
        CompiledIdentity::compile(&IdentityDecl {
            from: IdentitySource::Filename,
            pattern: pattern.to_string(),
        })
        .unwrap()
    }

    #[test]
    fn a_bare_zero_padded_number_is_an_identity() {
        let id =
            identity(r"^(?P<number>\d+)-(?P<slug>.+)$").identify("0001-record-architecture.md");
        assert_eq!(id.number.as_deref(), Some("0001"));
        assert_eq!(id.title_prefix, None);
    }

    #[test]
    fn a_name_the_pattern_does_not_match_has_no_number() {
        let p = identity(r"^(?P<number>\d+)-(?P<slug>.+)$");
        for name in ["README.md", "ADR-1-x.md", "notes-0001.md"] {
            assert_eq!(p.identify(name), DeclaredIdentity::default(), "{name}");
        }
    }

    #[test]
    fn a_captured_prefix_is_the_declared_title_prefix() {
        let id = identity(r"^(?P<prefix>SPEC)-(?P<number>\d+)-(?P<slug>.+)$")
            .identify("SPEC-3-configuration.md");
        assert_eq!(id.number.as_deref(), Some("3"));
        assert_eq!(id.title_prefix.as_deref(), Some("SPEC"));
        let bare = identity(r"^ADR-(?P<number>\d+)-.+$").identify("ADR-3-x.md");
        assert_eq!(bare.title_prefix, None);
    }

    #[test]
    fn a_number_capture_that_is_not_ascii_digits_is_no_number() {
        let p = identity(r"^(?P<number>\w+)-x$");
        assert_eq!(p.identify("abc-x.md").number, None);
        assert_eq!(p.identify("\u{0663}-x.md").number, None);
    }

    #[test]
    fn a_pattern_that_does_not_compile_or_has_no_number_group_fails_the_load() {
        for bad in ["'('", "'^(?P<slug>.+)$'", "'^(\\d+)-.+$'"] {
            let e = invalid(&format!(
                "    identity: {{from: filename, pattern: {bad}}}\n"
            ));
            assert!(
                e.contains("adr") && e.contains("identity") && e.contains("pattern"),
                "{bad}: {e}"
            );
        }
    }

    #[test]
    fn every_existing_header_shape_is_a_fields_source() {
        for (name, shape) in [
            ("blockquote", crate::header::HeaderShape::Blockquote),
            ("bold-list", crate::header::HeaderShape::BoldList),
            (
                "yaml-frontmatter",
                crate::header::HeaderShape::YamlFrontmatter,
            ),
            ("none", crate::header::HeaderShape::None),
        ] {
            let m = split(&cfg(&format!("    fields: {{from: {name}}}\n"))).unwrap();
            assert_eq!(
                m["adr"].fields.as_ref().unwrap().from.header_shape(),
                Some(shape)
            );
        }
    }
}
