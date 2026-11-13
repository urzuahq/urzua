//! A record type's declared document model (RFC-33 layer 1, ADR-53,
//! MILE-98): how its identity, fields and sections are found.
//!
//! A declaration is validated at load. Slice 2 makes `identity` and `fields`
//! drive record loading (`RecordModel`, `Record::parse_with_model`); a type
//! that declares neither loads exactly as before. `sections` locates sections for
//! `section.required` and `section.shape` only.
//!
//! ```yaml
//! record_types:
//!   adr:
//!     dir: doc/adr
//!     identity: {from: filename, pattern: '^(?P<prefix>ADR)-(?P<number>\d+)-(?P<slug>.+)$'}
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

/// An identity pattern, compiled. Its named groups: `prefix` (required, the
/// type prefix the file name carries, which is also the form the H1 title
/// opens with, `PREFIX-N`), `number` (required, all ASCII digits that fit a
/// `u64`) and `slug` (optional, not read by the engine). A pattern without a
/// `prefix` group is refused at load: a bare `0001-a.md` gets no identity
/// (RFC-46 was rejected; such a corpus is renamed, ADR-36).
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
    /// The `prefix` capture. Always `Some` when `number` is.
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
        if !regex.capture_names().flatten().any(|n| n == "prefix") {
            return Err(
                "`pattern` must capture the type prefix as a `(?P<prefix>...)` group: a file \
                 name without the type prefix gets no identity (rename it to `TYPE-NNNN-slug.md`)"
                    .into(),
            );
        }
        Ok(Self { regex })
    }

    /// Reads a file name (with or without `.md`). A name the pattern does not
    /// match, or whose `number` is not all ASCII digits, has no number.
    pub fn identify(&self, file_name: &str) -> DeclaredIdentity {
        let stem = crate::record::strip_md_extension(file_name).unwrap_or(file_name);
        let Some(caps) = self.regex.captures(stem) else {
            return DeclaredIdentity::default();
        };
        let number = caps
            .name("number")
            .map(|m| m.as_str())
            .filter(|n| n.chars().all(|c| c.is_ascii_digit()) && n.parse::<u64>().is_ok())
            .map(str::to_string);
        let prefix = caps
            .name("prefix")
            .map(|m| m.as_str())
            .filter(|p| !p.is_empty());
        match (number, prefix) {
            (Some(number), Some(prefix)) => DeclaredIdentity {
                number: Some(number),
                title_prefix: Some(prefix.to_string()),
            },
            _ => DeclaredIdentity::default(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldsSource {
    /// Bare `Key: value` lines between the title and the first `##` heading.
    PrefixLines,
    YamlFrontmatter,
    /// No header at all.
    None,
}

impl<'de> Deserialize<'de> for FieldsSource {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "prefix-lines" => Ok(FieldsSource::PrefixLines),
            "yaml-frontmatter" => Ok(FieldsSource::YamlFrontmatter),
            "none" => Ok(FieldsSource::None),
            removed @ ("blockquote" | "bold-list") => Err(serde::de::Error::custom(
                crate::config::removed_header_shape_message(removed),
            )),
            other => Err(serde::de::Error::custom(format!(
                "unknown variant `{other}`, expected one of `prefix-lines`, `yaml-frontmatter`, `none`"
            ))),
        }
    }
}

impl FieldsSource {
    /// The existing parser a source other than `prefix-lines` reads with.
    pub fn header_shape(self) -> Option<crate::header::HeaderShape> {
        use crate::header::HeaderShape;
        match self {
            FieldsSource::PrefixLines => Option::None,
            FieldsSource::YamlFrontmatter => Some(HeaderShape::YamlFrontmatter),
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
    /// The `##` sections a record of this type must carry, each with an
    /// optional content shape (`MILE-4`, `MILE-5`, `MILE-121`). Read only by
    /// `section.required` and `section.shape`; omitted means none required.
    #[serde(default, deserialize_with = "deserialize_required")]
    pub required: Vec<RequiredSection>,
    /// Statuses of a record that made no decision (`MILE-7`): such a record is
    /// not asked for `required` sections or their shapes. Matched exactly
    /// against the record's status field. Only meaningful with `required`.
    #[serde(default, deserialize_with = "deserialize_exempt_statuses")]
    pub exempt_statuses: Vec<String>,
    /// A heading may appear once among its siblings (`MILE-104`): the same
    /// heading text twice at one level under one parent is a
    /// `section.duplicate` finding. Omitted means `false`, and the rule has
    /// nothing eligible in the type.
    #[serde(default, deserialize_with = "deserialize_unique_headings")]
    pub unique_headings: bool,
    /// Read the section tree for a skipped level (`MILE-104`): a section deeper
    /// than the one above it by more than one level, or deeper than `##` with
    /// no section above it, is a `section.level-skipped` finding. Needs a
    /// `depth` of at least 2, because at depth 1 only `##` is a section and
    /// there is no level to skip. Omitted means `false`.
    #[serde(default, deserialize_with = "deserialize_no_skipped_levels")]
    pub no_skipped_levels: bool,
    /// Where the order sections must follow is read from (`MILE-104`), read by
    /// `section.order`. Omitted means no order is declared.
    #[serde(default)]
    pub order: Option<SectionOrder>,
}

/// `sections.order`: the source of the order a record's sections follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionOrder {
    pub from: OrderSource,
}

/// Where a declared section order is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSource {
    /// The headings of `.urzua/templates/<type>.md`, in the order the template
    /// gives them: the file an author is looking at while writing.
    Template,
}

impl<'de> Deserialize<'de> for OrderSource {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)
            .map_err(|_| serde::de::Error::custom("`order.from` must be `template`"))?;
        match s.as_str() {
            "template" => Ok(OrderSource::Template),
            other => Err(serde::de::Error::custom(format!(
                "`order.from` is '{other}'; the only source is `template`"
            ))),
        }
    }
}

/// One required section: its heading text and the shape its content takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredSection {
    /// The heading text, matched exactly after trimming.
    pub name: String,
    /// The heading level the section sits at: 2 for `##` (the default), 3 for
    /// `###`. A level deeper than 2 needs the type's `depth` to reach it.
    pub level: u8,
    /// The heading text of the section one level up this one must sit under;
    /// `None` accepts it under any parent. Only for a `level` of 3 or more.
    pub under: Option<String>,
    pub shape: Option<SectionShape>,
    /// Other `##` heading texts that satisfy this requirement (`MILE-165`),
    /// matched exactly after trimming, in the order tried after `name`.
    pub aliases: Vec<String>,
    /// Also read a `y-statement` from the preamble, the text before the first
    /// `##` heading, when the section itself is absent (`MILE-165`). Needs the
    /// shape's `marker`, because without one the block the preamble would be
    /// read from is the H1's first paragraph.
    pub preamble: bool,
}

impl RequiredSection {
    /// Every heading text that satisfies this requirement: `name`, then the
    /// aliases.
    pub fn headings(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

/// A content shape a required section can declare. Shape is structure only:
/// it never judges whether the content is true or sensible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionShape {
    /// The five Y-statement phrases in one block (`MILE-5`). With a `marker`
    /// the block starts at the first line containing it; without, it is the
    /// section's first paragraph.
    YStatement { marker: Option<String> },
    /// A table with at least `min_rows` non-empty data rows (`MILE-121`).
    TableMinRows { min_rows: usize },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequiredSection {
    name: String,
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    marker: Option<String>,
    #[serde(default)]
    min_rows: Option<yaml_serde::Value>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default)]
    preamble: Option<bool>,
    #[serde(default)]
    level: Option<yaml_serde::Value>,
    #[serde(default)]
    under: Option<String>,
}

impl RawRequiredSection {
    fn resolve(self) -> Result<RequiredSection, String> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err("a required section's `name` must not be blank".into());
        }
        let shape = self.resolve_shape(&name)?;
        let aliases = resolve_aliases(self.aliases.unwrap_or_default(), &name)?;
        let level = match &self.level {
            None => 2,
            Some(v) => v.as_u64().filter(|n| (2..=6).contains(n)).ok_or_else(|| {
                format!("section '{name}': `level` must be an integer from 2 (`##`) to 6")
            })? as u8,
        };
        let under = resolve_under(self.under, level, &name)?;
        let preamble = self.preamble.unwrap_or(false);
        if preamble && !matches!(shape, Some(SectionShape::YStatement { marker: Some(_) })) {
            return Err(format!(
                "section '{name}': `preamble` needs shape `y-statement` with a `marker`, which is how the block is found outside a section"
            ));
        }
        Ok(RequiredSection {
            name,
            level,
            under,
            shape,
            aliases,
            preamble,
        })
    }

    /// The declared `shape` with the `marker` or `min_rows` that belongs to it.
    fn resolve_shape(&self, name: &str) -> Result<Option<SectionShape>, String> {
        match self.shape.as_deref() {
            None => {
                if self.marker.is_some() || self.min_rows.is_some() {
                    return Err(format!(
                        "section '{name}' sets `marker` or `min_rows` without a `shape`"
                    ));
                }
                Ok(None)
            }
            Some("y-statement") => {
                if self.min_rows.is_some() {
                    return Err(format!(
                        "section '{name}': `min_rows` belongs to shape `table-min-rows`"
                    ));
                }
                let marker = match &self.marker {
                    Some(m) if m.trim().is_empty() => {
                        return Err(format!("section '{name}': `marker` must not be blank"))
                    }
                    Some(m) => Some(m.trim().to_string()),
                    None => None,
                };
                Ok(Some(SectionShape::YStatement { marker }))
            }
            Some("table-min-rows") => {
                if self.marker.is_some() {
                    return Err(format!(
                        "section '{name}': `marker` belongs to shape `y-statement`"
                    ));
                }
                let min_rows = self
                    .min_rows
                    .as_ref()
                    .and_then(yaml_serde::Value::as_u64)
                    .filter(|n| *n >= 1)
                    .ok_or_else(|| {
                        format!(
                            "section '{name}': shape `table-min-rows` needs `min_rows`, an integer of at least 1"
                        )
                    })?;
                Ok(Some(SectionShape::TableMinRows {
                    min_rows: min_rows as usize,
                }))
            }
            Some(other) => Err(format!(
                "section '{name}': unknown shape '{other}'; valid shapes are y-statement, table-min-rows"
            )),
        }
    }
}

/// The trimmed aliases, none blank and none repeating the name or another alias.
fn resolve_aliases(raw: Vec<String>, name: &str) -> Result<Vec<String>, String> {
    let aliases: Vec<String> = raw.into_iter().map(|a| a.trim().to_string()).collect();
    if aliases.iter().any(String::is_empty) {
        return Err(format!("section '{name}': an alias must not be blank"));
    }
    for (i, alias) in aliases.iter().enumerate() {
        if *alias == name || aliases[..i].contains(alias) {
            return Err(format!(
                "section '{name}': alias '{alias}' repeats a heading it already names"
            ));
        }
    }
    Ok(aliases)
}

/// The trimmed `under` heading, which only a section of level 3 or more can name.
fn resolve_under(under: Option<String>, level: u8, name: &str) -> Result<Option<String>, String> {
    match under {
        None => Ok(None),
        Some(u) if u.trim().is_empty() => {
            Err(format!("section '{name}': `under` must not be blank"))
        }
        Some(_) if level < 3 => Err(format!(
            "section '{name}': `under` names the section one level up, so it needs a `level` of 3 or more"
        )),
        Some(u) => Ok(Some(u.trim().to_string())),
    }
}

fn deserialize_required<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<RequiredSection>, D::Error> {
    let raw = Vec::<RawRequiredSection>::deserialize(d)
        .map_err(|e| serde::de::Error::custom(format!("`required` {e}")))?;
    let mut out: Vec<RequiredSection> = Vec::with_capacity(raw.len());
    for r in raw {
        let section = r.resolve().map_err(serde::de::Error::custom)?;
        for heading in section.headings() {
            if out.iter().any(|o| {
                o.level == section.level
                    && o.under == section.under
                    && o.headings().any(|h| h == heading)
            }) {
                return Err(serde::de::Error::custom(format!(
                    "`required` names section '{heading}' twice"
                )));
            }
        }
        out.push(section);
    }
    Ok(out)
}

fn deserialize_exempt_statuses<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<String>, D::Error> {
    let list = Vec::<String>::deserialize(d)
        .map_err(|e| serde::de::Error::custom(format!("`exempt_statuses` {e}")))?;
    if list.iter().any(|s| s.trim().is_empty()) {
        return Err(serde::de::Error::custom(
            "`exempt_statuses` must not hold a blank status",
        ));
    }
    Ok(list.into_iter().map(|s| s.trim().to_string()).collect())
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

fn deserialize_unique_headings<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    yaml_serde::Value::deserialize(d)?
        .as_bool()
        .ok_or_else(|| serde::de::Error::custom("`unique_headings` must be true or false"))
}

fn deserialize_no_skipped_levels<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    yaml_serde::Value::deserialize(d)?
        .as_bool()
        .ok_or_else(|| serde::de::Error::custom("`no_skipped_levels` must be true or false"))
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
        if let Some(sd) = &sections {
            for r in &sd.required {
                if r.level > sd.depth.saturating_add(1) {
                    return Err(invalid(
                        name,
                        "sections",
                        format!(
                            "section '{}' is at level {} but `depth` is {}, which reads headings only down to level {}; raise `depth` to {} or lower the `level`",
                            r.name,
                            r.level,
                            sd.depth,
                            sd.depth + 1,
                            r.level - 1
                        ),
                    ));
                }
            }
            if sd.no_skipped_levels && sd.depth < 2 {
                return Err(invalid(
                    name,
                    "sections",
                    "`no_skipped_levels` needs a `depth` of at least 2: at depth 1 only `##` is a section, so no level can be skipped"
                        .into(),
                ));
            }
            if sd.required.is_empty() && !sd.exempt_statuses.is_empty() {
                return Err(invalid(
                    name,
                    "sections",
                    "`exempt_statuses` has no effect without `required`; declare `required` or remove it"
                        .into(),
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
            "    identity: {from: filename, pattern: '^(?P<prefix>ADR)-(?P<number>\\d+)-(?P<slug>.+)$'}\n    fields: {from: prefix-lines}\n    sections: {from: headings, depth: 2, items: true}\n",
        ))
        .unwrap();
        let adr = &m["adr"];
        let id = adr.identity.as_ref().unwrap();
        assert_eq!(id.from, IdentitySource::Filename);
        assert_eq!(
            id.pattern,
            r"^(?P<prefix>ADR)-(?P<number>\d+)-(?P<slug>.+)$"
        );
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

    const PREFIXED: &str = r"^(?P<prefix>ADR)-(?P<number>\d+)-(?P<slug>.+)$";

    #[test]
    fn a_zero_padded_prefixed_number_keeps_its_padding() {
        let id = identity(PREFIXED).identify("ADR-0001-record-architecture.md");
        assert_eq!(id.number.as_deref(), Some("0001"));
        assert_eq!(id.title_prefix.as_deref(), Some("ADR"));
    }

    #[test]
    fn a_pattern_without_a_prefix_group_is_refused_naming_the_type() {
        let e = invalid("    identity: {from: filename, pattern: '^(?P<number>\\d+)-.+$'}\n");
        assert!(e.contains("adr") && e.contains("identity"), "{e}");
        assert!(e.contains("(?P<prefix>"), "{e}");
    }

    #[test]
    fn a_number_that_overflows_u64_or_an_empty_prefix_is_no_identity() {
        let p = identity(PREFIXED);
        assert_eq!(
            p.identify("ADR-99999999999999999999999-x.md"),
            DeclaredIdentity::default()
        );
        let q = identity(r"^(?P<prefix>[A-Z]*)-?(?P<number>\d+)-.+$");
        assert_eq!(q.identify("0001-x.md"), DeclaredIdentity::default());
    }

    #[test]
    fn a_name_the_pattern_does_not_match_has_no_number() {
        let p = identity(PREFIXED);
        for name in ["README.md", "0001-x.md", "notes-ADR-1-x.md"] {
            assert_eq!(p.identify(name), DeclaredIdentity::default(), "{name}");
        }
    }

    #[test]
    fn a_captured_prefix_is_the_declared_title_prefix() {
        let id = identity(r"^(?P<prefix>SPEC)-(?P<number>\d+)-(?P<slug>.+)$")
            .identify("SPEC-3-configuration.md");
        assert_eq!(id.number.as_deref(), Some("3"));
        assert_eq!(id.title_prefix.as_deref(), Some("SPEC"));
    }

    #[test]
    fn a_number_capture_that_is_not_ascii_digits_is_no_number() {
        let p = identity(r"^(?P<prefix>P)-(?P<number>\w+)-x$");
        assert_eq!(p.identify("P-abc-x.md").number, None);
        assert_eq!(p.identify("P-\u{0663}-x.md").number, None);
    }

    #[test]
    fn a_pattern_that_does_not_compile_or_has_no_number_group_fails_the_load() {
        for bad in ["'('", "'^(?P<prefix>A)-(?P<slug>.+)$'", "'^(\\d+)-.+$'"] {
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

    #[test]
    fn a_removed_header_shape_as_a_fields_source_is_a_clear_error() {
        for name in ["blockquote", "bold-list"] {
            let e = invalid(&format!("    fields: {{from: {name}}}\n"));
            assert!(
                e.contains(name) && e.contains("no longer supported"),
                "{name}: {e}"
            );
        }
    }

    #[test]
    fn required_sections_and_their_shapes_parse() {
        let m = split(&cfg(
            "    sections: {from: headings, required: [{name: Context}, {name: Decision, shape: y-statement, marker: 'TL;DR'}, {name: Options, shape: table-min-rows, min_rows: 2}], exempt_statuses: [Withdrawn]}\n",
        ))
        .unwrap();
        let s = m["adr"].sections.as_ref().unwrap();
        assert_eq!(s.exempt_statuses, vec!["Withdrawn".to_string()]);
        assert_eq!(s.required[0].shape, None);
        assert_eq!(
            s.required[1].shape,
            Some(SectionShape::YStatement {
                marker: Some("TL;DR".into())
            })
        );
        assert_eq!(
            s.required[2].shape,
            Some(SectionShape::TableMinRows { min_rows: 2 })
        );
    }

    #[test]
    fn a_malformed_required_declaration_fails_the_load_naming_the_problem() {
        for (decl, needle) in [
            ("{name: ' '}", "blank"),
            ("{name: A, shape: prose}", "unknown shape"),
            ("{name: A, shape: table-min-rows}", "min_rows"),
            ("{name: A, shape: table-min-rows, min_rows: 0}", "min_rows"),
            (
                "{name: A, shape: table-min-rows, min_rows: 2, marker: x}",
                "marker",
            ),
            ("{name: A, shape: y-statement, min_rows: 2}", "min_rows"),
            ("{name: A, marker: x}", "without a `shape`"),
            ("{name: A, shape: y-statement, marker: ' '}", "marker"),
            ("{name: A, bogus: 1}", "bogus"),
        ] {
            let e = invalid(&format!(
                "    sections: {{from: headings, required: [{decl}]}}\n"
            ));
            assert!(e.contains("adr") && e.contains(needle), "{decl}: {e}");
        }
        let e = invalid("    sections: {from: headings, required: [{name: A}, {name: A}]}\n");
        assert!(e.contains("twice"), "{e}");
    }

    #[test]
    fn the_structure_keys_default_off_and_parse_when_declared() {
        let m = split(&cfg("    sections: {from: headings}\n")).unwrap();
        let s = m["adr"].sections.as_ref().unwrap();
        assert!(!s.unique_headings && !s.no_skipped_levels && s.order.is_none());
        let m = split(&cfg(
            "    sections: {from: headings, depth: 2, unique_headings: true, no_skipped_levels: true, order: {from: template}}\n",
        ))
        .unwrap();
        let s = m["adr"].sections.as_ref().unwrap();
        assert!(s.unique_headings && s.no_skipped_levels);
        assert_eq!(s.order.unwrap().from, OrderSource::Template);
    }

    #[test]
    fn a_structure_key_that_could_never_report_or_is_malformed_is_refused() {
        let e = invalid("    sections: {from: headings, no_skipped_levels: true}\n");
        assert!(
            e.contains("no_skipped_levels") && e.contains("depth"),
            "{e}"
        );
        let e = invalid("    sections: {from: headings, unique_headings: 1}\n");
        assert!(e.contains("unique_headings"), "{e}");
        let e = invalid("    sections: {from: headings, order: {from: alphabet}}\n");
        assert!(e.contains("order") && e.contains("template"), "{e}");
        let e = invalid("    sections: {from: headings, order: {from: template, by: x}}\n");
        assert!(e.contains("order") || e.contains("by"), "{e}");
    }

    #[test]
    fn exempt_statuses_without_required_is_refused_not_a_silent_no_op() {
        let e = invalid("    sections: {from: headings, exempt_statuses: [Withdrawn]}\n");
        assert!(
            e.contains("exempt_statuses") && e.contains("required"),
            "{e}"
        );
        let e = invalid(
            "    sections: {from: headings, required: [{name: A}], exempt_statuses: [' ']}\n",
        );
        assert!(e.contains("blank"), "{e}");
    }

    #[test]
    fn aliases_and_preamble_load_and_malformed_ones_are_refused() {
        let models = split(&cfg(
            "    sections: {from: headings, required: [{name: Decision, aliases: [Resolution, Outcome], shape: y-statement, marker: 'Y-statement', preamble: true}]}\n",
        ))
        .unwrap();
        let r = &models["adr"].sections.as_ref().unwrap().required[0];
        assert_eq!(r.aliases, ["Resolution", "Outcome"]);
        assert!(r.preamble);
        assert_eq!(
            r.headings().collect::<Vec<_>>(),
            ["Decision", "Resolution", "Outcome"]
        );
        for (decl, needle) in [
            ("{name: A, aliases: [' ']}", "alias"),
            ("{name: A, aliases: [A]}", "repeats"),
            ("{name: A, aliases: [B, B]}", "repeats"),
            ("{name: A, preamble: true}", "preamble"),
            ("{name: A, shape: y-statement, preamble: true}", "marker"),
            (
                "{name: A, shape: table-min-rows, min_rows: 1, preamble: true}",
                "preamble",
            ),
        ] {
            let e = invalid(&format!(
                "    sections: {{from: headings, required: [{decl}]}}\n"
            ));
            assert!(e.contains(needle), "{decl}: {e}");
        }
        let e = invalid(
            "    sections: {from: headings, required: [{name: A, aliases: [B]}, {name: B}]}\n",
        );
        assert!(e.contains("twice"), "{e}");
    }

    #[test]
    fn a_required_section_declares_its_level_and_parent() {
        let m = split(&cfg(
            "    sections: {from: headings, depth: 2, required: [{name: Options}, {name: Pros, level: 3, under: Options}, {name: Notes, level: 3}]}\n",
        ))
        .unwrap();
        let r = &m["adr"].sections.as_ref().unwrap().required;
        assert_eq!((r[0].level, r[0].under.as_deref()), (2, None));
        assert_eq!((r[1].level, r[1].under.as_deref()), (3, Some("Options")));
        assert_eq!((r[2].level, r[2].under.as_deref()), (3, None));
    }

    #[test]
    fn a_level_beyond_depth_or_a_misplaced_under_fails_the_load_not_a_silent_no_op() {
        for (decl, needle) in [
            ("depth: 1, required: [{name: A, level: 3}]", "depth"),
            ("depth: 2, required: [{name: A, level: 4}]", "depth"),
            ("depth: 2, required: [{name: A, level: 1}]", "level"),
            ("depth: 2, required: [{name: A, level: x}]", "level"),
            ("depth: 2, required: [{name: A, under: B}]", "under"),
            (
                "depth: 2, required: [{name: A, level: 3, under: ' '}]",
                "under",
            ),
        ] {
            let e = invalid(&format!("    sections: {{from: headings, {decl}}}\n"));
            assert!(e.contains("adr") && e.contains(needle), "{decl}: {e}");
        }
    }

    #[test]
    fn the_same_name_at_a_different_level_or_parent_is_not_a_duplicate() {
        let ok = "    sections: {from: headings, depth: 2, required: [{name: Notes}, {name: Notes, level: 3, under: A}, {name: Notes, level: 3, under: B}]}\n";
        assert!(split(&cfg(ok)).is_ok());
        let e = invalid(
            "    sections: {from: headings, depth: 2, required: [{name: N, level: 3, under: A}, {name: N, level: 3, under: A}]}\n",
        );
        assert!(e.contains("twice"), "{e}");
    }
}
