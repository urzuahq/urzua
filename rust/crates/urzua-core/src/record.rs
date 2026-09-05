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
        Record {
            path,
            record_type,
            header: header::parse_with_shape(content, shape),
        }
    }
}
