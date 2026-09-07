//! A parsed record: its declared type, path, and header. Pure -- built from
//! content handed in by a caller, never read from a path itself.

use crate::header::{self, Header, HeaderShape};
use std::path::PathBuf;

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
