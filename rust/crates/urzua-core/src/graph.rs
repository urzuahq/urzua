//! `urzua explain` and `urzua graph`: the relationship graph as queryable
//! data, built from what's already tracked (`Implements`/`Derives-from`,
//! `Supersedes`/`Superseded-by`, `Realized-by`) rather than a new schema
//! concept. No new field, no new rule -- a read over data every other rule
//! already parses.

use crate::record::Record;
use crate::rules::{
    build_normalized_index, extract_references, normalize_id, parse_realized_by, record_id,
    RelationKind, NARRATIVE, POINTER,
};
use std::collections::HashMap;

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
    /// `pointer` | `narrative` (MILE-0090/ADR-0044) -- describes the edge's
    /// value shape, not which rule resolved it. `Supersedes /
    /// Superseded-by` reports `Pointer`: its values are always clean
    /// references, even though a separate mechanism
    /// (`relation.supersession-reciprocity`) checks it.
    pub kind: RelationKind,
    /// `true` if `to` doesn't resolve to any discovered record -- the same
    /// condition `pointer.resolution`/`relation.supersession-reciprocity`
    /// report as findings, surfaced here as an edge property instead of a
    /// separate lookup.
    pub dangling: bool,
}

/// The full record-relationship graph: every config-declared
/// `pointer_fields`/`narrative_fields` edge, plus the still-hardcoded
/// `Supersedes`/`Superseded-by` (its own mechanism, outside that axis), for
/// every record with a resolvable identity. Config-driven since
/// MILE-0090/ADR-0044 -- no hardcoded field list, so a type's own `Parent`
/// or any other declared relationship field appears here for free.
/// Normalized via `build_normalized_index`, closing BUG-0011 (this
/// function's own index used to skip normalization, unlike
/// `pointer_resolution`'s).
pub fn graph(
    records: &[Record],
    pointer_fields_by_type: &HashMap<String, Vec<String>>,
    narrative_fields_by_type: &HashMap<String, Vec<String>>,
) -> Vec<GraphEdge> {
    let index = build_normalized_index(records);

    // Supersedes / Superseded-by is pushed unconditionally below, its own
    // mechanism outside the pointer_fields/narrative_fields axis (ADR-44) --
    // excluded here so a config that also lists it in either declared list
    // can't produce a duplicate edge or a conflicting `kind`.
    const SUPERSESSION_FIELD: &str = "Supersedes / Superseded-by";

    let mut edges = Vec::new();
    for record in records {
        let Some(from) = record_id(record) else {
            continue;
        };

        let mut fields: Vec<(&str, RelationKind)> = Vec::new();
        if let Some(pointer_fields) = pointer_fields_by_type.get(&record.record_type) {
            fields.extend(
                pointer_fields
                    .iter()
                    .filter(|f| f.as_str() != SUPERSESSION_FIELD)
                    .map(|f| (f.as_str(), POINTER.kind)),
            );
        }
        if let Some(narrative_fields) = narrative_fields_by_type.get(&record.record_type) {
            fields.extend(
                narrative_fields
                    .iter()
                    .filter(|f| f.as_str() != SUPERSESSION_FIELD)
                    .map(|f| (f.as_str(), NARRATIVE.kind)),
            );
        }
        fields.push((SUPERSESSION_FIELD, POINTER.kind));

        for (field_name, kind) in fields {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            for to in extract_references(value) {
                edges.push(GraphEdge {
                    from: from.clone(),
                    relation: field_name.to_string(),
                    dangling: !index.contains_key(&normalize_id(&to)),
                    to,
                    kind,
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

    fn field_map(entries: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(type_name, fields)| {
                (
                    type_name.to_string(),
                    fields.iter().map(|f| f.to_string()).collect(),
                )
            })
            .collect()
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
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Accepted\n");
        let source = record("docs/adr/ADR-1-y.md", "adr", "> Implements: RFC-1\n");
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let edges = graph(&[target, source], &pointer_fields, &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to, "RFC-1");
        assert_eq!(edges[0].kind, RelationKind::Pointer);
        assert!(!edges[0].dangling);
    }

    #[test]
    fn graph_marks_a_non_resolving_reference_as_dangling_observed_failing() {
        let source = record("docs/adr/ADR-1-y.md", "adr", "> Implements: RFC-9999\n");
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let edges = graph(&[source], &pointer_fields, &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dangling);
    }

    #[test]
    fn graph_resolves_a_padded_reference_against_an_unpadded_filename_observed_failing() {
        // BUG-0011: graph()'s own index used to skip normalization, unlike
        // pointer_resolution's -- a padded reference (RFC-0001) against an
        // unpadded filename (RFC-1-x.md) was reported dangling even though
        // pointer_resolution correctly resolved the same reference.
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Accepted\n");
        let source = record("docs/adr/ADR-1-y.md", "adr", "> Implements: RFC-0001\n");
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let edges = graph(&[target, source], &pointer_fields, &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert!(
            !edges[0].dangling,
            "unexpected dangling edge: {:?}",
            edges[0]
        );
    }

    #[test]
    fn graph_includes_a_parent_edge() {
        // A pre-existing gap this same rewrite fixes for free: graph() used
        // to hardcode a 3-field list that never included Parent, even though
        // README/SPEC-13 both document it as a real edge type.
        let target = record("docs/specs/SPEC-1-x.md", "spec", "> Status: Accepted\n");
        let source = record("docs/specs/SPEC-2-y.md", "spec", "> Parent: SPEC-1\n");
        let pointer_fields = field_map(&[("spec", &["Parent"])]);
        let edges = graph(&[target, source], &pointer_fields, &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relation, "Parent");
        assert!(!edges[0].dangling);
    }

    #[test]
    fn graph_includes_a_narrative_edge_kinded_correctly() {
        let bug = record("docs/bugs/BUG-1-x.md", "bug", "> Status: Open\n");
        let milestone = record(
            "docs/milestones/MILESTONE-1-y.md",
            "milestone",
            "> Blocked-on: BUG-1\n",
        );
        let narrative_fields = field_map(&[("milestone", &["Blocked-on"])]);
        let edges = graph(&[bug, milestone], &HashMap::new(), &narrative_fields);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, RelationKind::Narrative);
    }

    #[test]
    fn graph_reports_supersedes_as_pointer_kind() {
        let old = record("docs/adr/ADR-1-x.md", "adr", "> Status: Superseded\n");
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-1\n",
        );
        let edges = graph(&[old, new], &HashMap::new(), &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, RelationKind::Pointer);
    }

    #[test]
    fn a_config_that_also_declares_supersedes_does_not_duplicate_the_edge_observed_failing() {
        // Supersedes / Superseded-by is pushed unconditionally, its own
        // mechanism outside the pointer_fields/narrative_fields axis
        // (ADR-44) -- a config that also lists it in either declared list
        // must not produce a second, possibly differently-kinded edge for
        // the same reference.
        let old = record("docs/adr/ADR-1-x.md", "adr", "> Status: Superseded\n");
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-1\n",
        );
        let pointer_fields = field_map(&[("adr", &["Supersedes / Superseded-by"])]);
        let edges = graph(&[old, new], &pointer_fields, &HashMap::new());
        assert_eq!(edges.len(), 1, "unexpected edges: {edges:?}");
        assert_eq!(edges[0].kind, RelationKind::Pointer);
    }
}
