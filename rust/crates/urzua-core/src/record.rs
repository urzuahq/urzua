//! A parsed record: its declared type, path, and header. Pure -- built from
//! content handed in by a caller, never read from a path itself.

use crate::document_model::{DeclaredIdentity, FieldsSource, RecordModel};
use crate::header::{self, Header, HeaderShape};
use std::path::PathBuf;

/// A tracked, non-template markdown file below a declared `dir` (`BUG-108`):
/// governed by that type whether or not its filename carries a number.
/// Deliberately looser than `new_record::parse_record_filename`, which asks a
/// different question -- "is this evidence of a numbering convention" -- for
/// `init`'s adopt-mode heuristic and `new`'s numbering, neither of which this
/// answers.
pub fn is_governed_record_filename(file_name: &str) -> bool {
    !file_name.starts_with('_') && file_name.ends_with(".md")
}

#[derive(Debug, Clone)]
pub struct Record {
    pub path: PathBuf,
    /// The record type as configured (e.g. "adr", "rfc", "spec") -- declared,
    /// not enumerated by the tool (RFC-0001 §1a).
    pub record_type: String,
    /// The filename/ID prefix this record's type resolves to (e.g. "ADR",
    /// or "MILE" for a `milestone` type configured with a shorter `prefix`).
    /// Defaults to `record_type` upper-cased when a caller has no
    /// configured override to supply.
    pub type_prefix: String,
    pub header: Header,
    /// What the type's declared `identity` pattern found in the file name
    /// (RFC-33 layer 1, MILE-98). `None` when the type declares none, in
    /// which case `record_id` reads the `TYPE-N-slug.md` convention as always.
    pub identity: Option<DeclaredIdentity>,
}

impl Record {
    /// Parse using the default (blockquote) header shape -- the long-standing
    /// behavior, kept for callers that have no per-type shape to declare.
    pub fn parse(path: PathBuf, record_type: String, content: &str) -> Self {
        Self::parse_with_shape(path, record_type, content, HeaderShape::default())
    }

    pub fn parse_with_shape(
        path: PathBuf,
        record_type: String,
        content: &str,
        shape: HeaderShape,
    ) -> Self {
        let type_prefix = record_type.to_ascii_uppercase();
        Self::parse_with_shape_and_prefix(path, record_type, content, shape, type_prefix)
    }

    /// Like `parse_with_shape`, but with an explicit filename/ID prefix --
    /// for a caller (`urzua-cli`'s config-aware discovery) that has a
    /// configured `prefix` override to supply rather than deriving one from
    /// the type name.
    pub fn parse_with_shape_and_prefix(
        path: PathBuf,
        record_type: String,
        content: &str,
        shape: HeaderShape,
        type_prefix: String,
    ) -> Self {
        Record {
            path,
            record_type,
            type_prefix,
            header: header::parse_with_shape(content, shape),
            identity: None,
        }
    }

    /// Like `parse_with_shape_and_prefix`, but reading the type's declared
    /// document model: `fields` decides how the header is read (overriding
    /// `shape`), and `identity` decides the record's number from its file
    /// name. A part the model leaves undeclared loads as it always did.
    pub fn parse_with_model(
        path: PathBuf,
        record_type: String,
        content: &str,
        shape: HeaderShape,
        type_prefix: String,
        model: &RecordModel,
    ) -> Self {
        let header = match model.fields {
            Some(FieldsSource::PrefixLines) => header::parse_prefix_lines(content),
            Some(source) => header::parse_with_shape(
                content,
                source
                    .header_shape()
                    .expect("every other source has a shape"),
            ),
            None => header::parse_with_shape(content, shape),
        };
        let identity = model.identity.as_ref().map(|pattern| {
            pattern.identify(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default(),
            )
        });
        Record {
            path,
            record_type,
            type_prefix,
            header,
            identity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BUG-108: `discovery.rs` and `rules.rs` duplicated this predicate;
    /// both call the same function.
    #[test]
    fn a_leading_underscore_is_a_template_not_a_record() {
        assert!(!is_governed_record_filename("_template.md"));
    }

    #[test]
    fn a_non_markdown_file_is_not_a_record() {
        assert!(!is_governed_record_filename("README"));
        assert!(!is_governed_record_filename("notes.txt"));
    }

    #[test]
    fn an_unnumbered_markdown_file_is_still_governed() {
        assert!(is_governed_record_filename("README.md"));
    }

    fn model(identity: Option<&str>, fields: Option<FieldsSource>) -> RecordModel {
        RecordModel {
            identity: identity.map(|p| {
                crate::document_model::CompiledIdentity::compile(
                    &crate::document_model::IdentityDecl {
                        from: crate::document_model::IdentitySource::Filename,
                        pattern: p.to_string(),
                    },
                )
                .unwrap()
            }),
            fields,
        }
    }

    fn load(path: &str, content: &str, m: &RecordModel) -> Record {
        Record::parse_with_model(
            PathBuf::from(path),
            "adr".into(),
            content,
            HeaderShape::Blockquote,
            "ADR".into(),
            m,
        )
    }

    #[test]
    fn a_type_declaring_nothing_loads_exactly_as_before() {
        let content = "# 1 — X\n\n> Status: Draft\n";
        let declared = load("docs/adr/ADR-1-x.md", content, &RecordModel::default());
        let old = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".into(),
            content,
            HeaderShape::Blockquote,
            "ADR".into(),
        );
        assert_eq!(declared.header, old.header);
        assert_eq!(declared.identity, None);
    }

    #[test]
    fn declared_fields_override_the_header_shape() {
        let content = "# 1. X\n\nDate: 2016-02-12\n\n## Status\n";
        let r = load(
            "docs/adr/0001-x.md",
            content,
            &model(None, Some(FieldsSource::PrefixLines)),
        );
        assert_eq!(r.header.get("Date"), Some("2016-02-12"));
        let y = load(
            "docs/adr/0001-x.md",
            "---\nStatus: Accepted\n---\n# 1\n",
            &model(None, Some(FieldsSource::YamlFrontmatter)),
        );
        assert_eq!(y.header.get("Status"), Some("Accepted"));
    }

    #[test]
    fn declared_identity_reads_the_file_name() {
        let r = load(
            "docs/adr/ADR-0007-x.md",
            "# ADR-7\n",
            &model(Some(r"^(?P<prefix>ADR)-(?P<number>\d+)-.+$"), None),
        );
        assert_eq!(r.identity.unwrap().number.as_deref(), Some("0007"));
    }
}
