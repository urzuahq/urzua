//! A parsed record: its declared type, path, and header. Pure -- built from
//! content handed in by a caller, never read from a path itself.

use crate::header::{self, Header, HeaderShape};
use std::path::PathBuf;

/// A tracked, non-template markdown file below a declared `dir` (`BUG-104`):
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BUG-104: `discovery.rs` and `rules.rs` duplicated this predicate and
    /// could drift; both now call the same function.
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
}
