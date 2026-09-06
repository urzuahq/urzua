//! `urzua explain` and `urzua graph`: the relationship graph as queryable
//! data, built from what's already tracked (`Implements`/`Derives-from`,
//! `Supersedes`/`Superseded-by`, `Realized-by`) rather than a new schema
//! concept. No new field, no new rule -- a read over data every other rule
//! already parses.

use crate::record::Record;
use crate::rules::{extract_references, parse_realized_by, record_id};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GoverningRecord {
    pub record: std::path::PathBuf,
    pub record_type: String,
    /// The `Realized-by` category the match came from (`spec`, `code`, or
    /// `test`) -- the same tiers `embodiment.consistency` computes from.
    pub via: String,
}

/// Every record whose `Realized-by` names `path` as evidence -- "which
/// decisions govern this file," answered from data the schema already
/// carries rather than a new declared-scope field (RFC-0011's boundaries
/// remain a separate, undesigned mechanism).
pub fn explain(records: &[Record], path: &str) -> Vec<GoverningRecord> {
    let mut matches = Vec::new();
    for record in records {
        let Some(realized_by_value) = record.header.get("Realized-by") else {
            continue;
        };
        let parsed = parse_realized_by(realized_by_value);
        for (category, locators) in [
            ("spec", &parsed.spec),
            ("code", &parsed.code),
            ("test", &parsed.test),
        ] {
            if locators.iter().any(|l| l == path) {
                matches.push(GoverningRecord {
                    record: record.path.clone(),
                    record_type: record.record_type.clone(),
                    via: category.to_string(),
                });
            }
        }
    }
    matches
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub relation: String,
    pub to: String,
    /// `true` if `to` doesn't resolve to any discovered record -- the same
    /// condition `pointer.resolution`/`relation.supersession-reciprocity`
    /// report as findings, surfaced here as an edge property instead of a
    /// separate lookup.
    pub dangling: bool,
}

/// The full record-relationship graph: every `Implements`/`Derives-from` and
/// `Supersedes`/`Superseded-by` edge, across every record with a resolvable
/// identity. Read-only, same data `pointer.resolution` and
/// `relation.supersession-reciprocity` already parse -- this is a dump of
/// it, not a new computation.
pub fn graph(records: &[Record]) -> Vec<GraphEdge> {
    let mut index = std::collections::HashMap::new();
    for record in records {
        if let Some(id) = record_id(record) {
            index.insert(id, record);
        }
    }

    let mut edges = Vec::new();
    for record in records {
        let Some(from) = record_id(record) else {
            continue;
        };
        for field_name in ["Implements", "Derives-from", "Supersedes / Superseded-by"] {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            for to in extract_references(value) {
                edges.push(GraphEdge {
                    from: from.clone(),
                    relation: field_name.to_string(),
                    dangling: !index.contains_key(&to),
                    to,
                });
            }
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    #[test]
    fn explain_finds_a_record_whose_realized_by_names_the_path() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:rust/src/lib.rs\n",
        );
        let result = explain(&[r], "rust/src/lib.rs");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].via, "code");
    }

    #[test]
    fn explain_finds_nothing_for_an_ungoverned_path() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:rust/src/lib.rs\n",
        );
        assert!(explain(&[r], "rust/src/other.rs").is_empty());
    }

    #[test]
    fn graph_includes_a_resolving_implements_edge_not_dangling() {
        let target = record("docs/rfc/0001-x.md", "rfc", "> Status: Accepted\n");
        let source = record("docs/adr/0001-y.md", "adr", "> Implements: RFC-0001\n");
        let edges = graph(&[target, source]);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to, "RFC-0001");
        assert!(!edges[0].dangling);
    }

    #[test]
    fn graph_marks_a_non_resolving_reference_as_dangling_observed_failing() {
        let source = record("docs/adr/0001-y.md", "adr", "> Implements: RFC-9999\n");
        let edges = graph(&[source]);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dangling);
    }
}
