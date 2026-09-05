//! A parsed record: its declared type, path, and header. Pure -- built from
//! content handed in by a caller, never read from a path itself.

use crate::header::{self, Header};
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
    pub fn parse(path: PathBuf, record_type: String, content: &str) -> Self {
        Record {
            path,
            record_type,
            header: header::parse(content),
        }
    }
}
