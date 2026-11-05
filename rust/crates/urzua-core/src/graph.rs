//! `urzua explain` and `urzua graph`: the relationship graph as queryable
//! data, built from what's already tracked (`Implements`/`Derives-from`,
//! `Supersedes`/`Superseded-by`, `Realized-by`) rather than a new schema
//! concept. No new field, no new rule -- a read over data every other rule
//! already parses.

use crate::config::{Config, RelationRole};
use crate::record::Record;
use crate::report::{Notice, NoticeSeverity, Population, PopulationUnit};
use crate::rules::{
    build_index_reporting_collisions, extract_references, record_id, relation_field_name,
    IdentifierCollision, RelationKind, NARRATIVE, POINTER,
};
use crate::values::RecordId;
use std::collections::HashMap;

/// What `graph` and `explain` read, as the population block `check` reports for
/// a rule (`BUG-169`): every record is eligible, a record whose header could be
/// read is examined, and one whose header could not is `unreadable`, so a
/// success status never hides records that contributed nothing. A record of a
/// `header_shape: none` type has nowhere for a header and is `out_of_scope`.
/// The notice lists the unreadable records by path; it is `None` when every
/// record was read. `subject` names the command.
pub fn read_population(
    records: &[Record],
    config: &Config,
    subject: &str,
) -> (Population, Option<Notice>) {
    let (mut examined, mut out_of_scope) = (0, 0);
    let mut unreadable: Vec<String> = Vec::new();
    for record in records {
        if config
            .record_types
            .get(&record.record_type)
            .is_some_and(|t| t.has_no_header())
        {
            out_of_scope += 1;
        } else if record.header.is_unreadable() {
            unreadable.push(record.path.display().to_string());
        } else {
            examined += 1;
        }
    }
    unreadable.sort();
    let population = Population::detailed(
        PopulationUnit::Record,
        records.len(),
        examined,
        out_of_scope,
        unreadable.len(),
    );
    let notice = (!unreadable.is_empty()).then(|| Notice {
        severity: NoticeSeverity::Warning,
        subject: subject.to_string(),
        message: format!(
            "{} record(s) left out because their header could not be read, so their relationships are not in this output: {}",
            unreadable.len(),
            unreadable.join(", ")
        ),
    });
    (population, notice)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GoverningRecord {
    pub record: std::path::PathBuf,
    pub record_type: String,
    /// The `Realized-by` category the match came from (`spec`, `code`, or
    /// `test`) -- the same tiers `embodiment.consistency` computes from.
    pub via: String,
    /// The scope the record cites within the file (`ADR-74`), when it cites
    /// one; absent for a whole-file locator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Every record whose `Realized-by` names `path` as evidence -- "which
/// decisions govern this file," answered from data the schema already
/// carries rather than a new declared-scope field (RFC-0011's boundaries
/// remain a separate, undesigned mechanism).
pub fn explain(records: &[Record], path: &str, config: &Config) -> Vec<GoverningRecord> {
    // Locators are stored normalized (`parse_realized_by`), so the query is
    // too: `./src/b.rs` and `src/b.rs` name the same file.
    let path = crate::rules::normalize_locator(path);
    let path = path.as_str();
    let mut matches = Vec::new();
    for record in records {
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        let Some(realized_by_value) = record.header.get(locator_field) else {
            continue;
        };
        for entry in crate::locator::realized_by_locators(realized_by_value) {
            let Ok(locator) = entry.parsed else { continue };
            if locator.path != path {
                continue;
            }
            let found = GoverningRecord {
                record: record.path.clone(),
                record_type: record.record_type.clone(),
                via: entry.category,
                scope: locator.scope.as_ref().map(ToString::to_string),
            };
            if !matches.contains(&found) {
                matches.push(found);
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
/// Normalized via `build_index_reporting_collisions`, closing `BUG-0011`
/// (this function's own index used to skip normalization, unlike
/// `pointer_resolution`'s) and matching `check`/`audit` (a plain
/// `build_normalized_index` silently picks one of two colliding records with
/// no way for a caller to know an edge might point at the arbitrary winner).
pub fn graph<'a>(
    records: &'a [Record],
    config: &Config,
    pointer_fields_by_type: &HashMap<String, Vec<String>>,
    narrative_fields_by_type: &HashMap<String, Vec<String>>,
) -> (Vec<GraphEdge>, Vec<IdentifierCollision<'a>>) {
    let (index, collisions) = build_index_reporting_collisions(records);

    let mut edges = Vec::new();
    for record in records {
        let Some(from) = record_id(record) else {
            continue;
        };

        // Supersedes / Superseded-by is pushed unconditionally below, its own
        // mechanism outside the pointer_fields/narrative_fields axis (ADR-44)
        // -- excluded here so a config that also lists it in either declared
        // list can't produce a duplicate edge or a conflicting `kind`.
        let supersession_field =
            relation_field_name(config, &record.record_type, RelationRole::Supersession);

        let mut fields: Vec<(&str, RelationKind)> = Vec::new();
        if let Some(pointer_fields) = pointer_fields_by_type.get(&record.record_type) {
            fields.extend(
                pointer_fields
                    .iter()
                    .filter(|f| f.as_str() != supersession_field)
                    .map(|f| (f.as_str(), POINTER.kind)),
            );
        }
        if let Some(narrative_fields) = narrative_fields_by_type.get(&record.record_type) {
            fields.extend(
                narrative_fields
                    .iter()
                    .filter(|f| f.as_str() != supersession_field)
                    .map(|f| (f.as_str(), NARRATIVE.kind)),
            );
        }
        fields.push((supersession_field, POINTER.kind));

        for (field_name, kind) in fields {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            for to in extract_references(value) {
                edges.push(GraphEdge {
                    from: from.clone(),
                    relation: field_name.to_string(),
                    dangling: !index.contains_key(&RecordId::new(&to)),
                    to,
                    kind,
                });
            }
        }
    }
    (edges, collisions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    fn no_relation_field_overrides() -> Config {
        Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            rules: std::collections::BTreeMap::new(),
            record_types: std::collections::BTreeMap::new(),
        }
    }

    fn config_with_relation_field(
        record_type: &str,
        relation_fields: crate::config::RelationFields,
    ) -> Config {
        Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            rules: std::collections::BTreeMap::new(),
            record_types: [(
                record_type.to_string(),
                crate::config::RecordTypeConfig {
                    exclude_files: Vec::new(),
                    dir: "docs/x".to_string(),
                    extra_dirs: Vec::new(),
                    identity: None,
                    fields: None,
                    sections: None,
                    statuses: None,
                    required_fields: vec![],
                    header_shape: crate::header::HeaderShape::default(),
                    prefix: None,
                    known_fields: None,
                    pointer_fields: None,
                    narrative_fields: None,
                    spec: None,
                    relation_fields: Some(relation_fields),
                    companion_fields: None,
                    revision_log: None,
                    role: None,
                    effective_requires: None,
                    permanent: None,
                },
            )]
            .into_iter()
            .collect(),
        }
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
            "---\nRealized-by: code:rust/src/lib.rs\n---\n",
        );
        let result = explain(&[r], "rust/src/lib.rs", &no_relation_field_overrides());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].via, "code");
    }

    /// Round 30 review: stored locators are normalized, so the query must be
    /// too, or `explain ./x` misses a record citing `x`.
    #[test]
    fn explain_normalizes_the_queried_path_like_the_stored_locator() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:./rust/src/lib.rs\n---\n",
        );
        for q in ["rust/src/lib.rs", "./rust/src/lib.rs", "rust/./src/lib.rs"] {
            assert_eq!(
                explain(std::slice::from_ref(&r), q, &no_relation_field_overrides()).len(),
                1,
                "{q}"
            );
        }
    }

    #[test]
    fn explain_finds_nothing_for_an_ungoverned_path() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:rust/src/lib.rs\n---\n",
        );
        assert!(explain(&[r], "rust/src/other.rs", &no_relation_field_overrides()).is_empty());
    }

    #[test]
    fn explain_reads_a_custom_embodiment_locator_field_name_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nEvidence: code:rust/src/lib.rs\n---\n",
        );
        let config = config_with_relation_field(
            "adr",
            crate::config::RelationFields {
                embodiment_locator: Some("Evidence".to_string()),
                ..Default::default()
            },
        );
        let result = explain(&[r], "rust/src/lib.rs", &config);
        assert_eq!(result.len(), 1, "{result:?}");
        assert_eq!(result[0].via, "code");
    }

    #[test]
    fn graph_reads_a_custom_supersession_field_name_observed_failing() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Superseded\n---\n",
        );
        let new = record("docs/adr/ADR-2-y.md", "adr", "---\nReplaces: ADR-1\n---\n");
        let config = config_with_relation_field(
            "adr",
            crate::config::RelationFields {
                supersession: Some("Replaces".to_string()),
                ..Default::default()
            },
        );
        let (edges, _collisions) = graph(&[old, new], &config, &HashMap::new(), &HashMap::new());
        assert_eq!(edges.len(), 1, "{edges:?}");
        assert_eq!(edges[0].relation, "Replaces");
        assert_eq!(edges[0].kind, RelationKind::Pointer);
    }

    #[test]
    fn graph_includes_a_resolving_implements_edge_not_dangling() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nStatus: Accepted\n---\n");
        let source = record(
            "docs/adr/ADR-1-y.md",
            "adr",
            "---\nImplements: RFC-1\n---\n",
        );
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let (edges, _collisions) = graph(
            &[target, source],
            &no_relation_field_overrides(),
            &pointer_fields,
            &HashMap::new(),
        );
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to, "RFC-1");
        assert_eq!(edges[0].kind, RelationKind::Pointer);
        assert!(!edges[0].dangling);
    }

    #[test]
    fn graph_marks_a_non_resolving_reference_as_dangling_observed_failing() {
        let source = record(
            "docs/adr/ADR-1-y.md",
            "adr",
            "---\nImplements: RFC-9999\n---\n",
        );
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let (edges, _collisions) = graph(
            &[source],
            &no_relation_field_overrides(),
            &pointer_fields,
            &HashMap::new(),
        );
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dangling);
    }

    #[test]
    fn graph_resolves_a_padded_reference_against_an_unpadded_filename_observed_failing() {
        // BUG-0011: graph()'s own index used to skip normalization, unlike
        // pointer_resolution's -- a padded reference (RFC-0001) against an
        // unpadded filename (RFC-1-x.md) was reported dangling even though
        // pointer_resolution correctly resolved the same reference.
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nStatus: Accepted\n---\n");
        let source = record(
            "docs/adr/ADR-1-y.md",
            "adr",
            "---\nImplements: RFC-0001\n---\n",
        );
        let pointer_fields = field_map(&[("adr", &["Implements"])]);
        let (edges, _collisions) = graph(
            &[target, source],
            &no_relation_field_overrides(),
            &pointer_fields,
            &HashMap::new(),
        );
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
        let target = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nStatus: Accepted\n---\n",
        );
        let source = record(
            "docs/specs/SPEC-2-y.md",
            "spec",
            "---\nParent: SPEC-1\n---\n",
        );
        let pointer_fields = field_map(&[("spec", &["Parent"])]);
        let (edges, _collisions) = graph(
            &[target, source],
            &no_relation_field_overrides(),
            &pointer_fields,
            &HashMap::new(),
        );
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relation, "Parent");
        assert!(!edges[0].dangling);
    }

    #[test]
    fn graph_includes_a_narrative_edge_kinded_correctly() {
        let bug = record("docs/bugs/BUG-1-x.md", "bug", "---\nStatus: Open\n---\n");
        let milestone = record(
            "docs/milestones/MILESTONE-1-y.md",
            "milestone",
            "---\nBlocked-on: BUG-1\n---\n",
        );
        let narrative_fields = field_map(&[("milestone", &["Blocked-on"])]);
        let (edges, _collisions) = graph(
            &[bug, milestone],
            &no_relation_field_overrides(),
            &HashMap::new(),
            &narrative_fields,
        );
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, RelationKind::Narrative);
    }

    #[test]
    fn graph_reports_supersedes_as_pointer_kind() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Superseded\n---\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-1\n---\n",
        );
        let (edges, _collisions) = graph(
            &[old, new],
            &no_relation_field_overrides(),
            &HashMap::new(),
            &HashMap::new(),
        );
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
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Superseded\n---\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-1\n---\n",
        );
        let pointer_fields = field_map(&[("adr", &["Supersedes / Superseded-by"])]);
        let (edges, _collisions) = graph(
            &[old, new],
            &no_relation_field_overrides(),
            &pointer_fields,
            &HashMap::new(),
        );
        assert_eq!(edges.len(), 1, "unexpected edges: {edges:?}");
        assert_eq!(edges[0].kind, RelationKind::Pointer);
    }

    #[test]
    fn the_read_population_counts_an_unreadable_header_and_names_it_observed_failing() {
        let good = record(
            "docs/adr/ADR-1-a.md",
            "adr",
            "---\nStatus: Accepted\n---\n# ADR-1\n",
        );
        let bad = record(
            "docs/adr/ADR-2-b.md",
            "adr",
            "---\nDerives-from: ADR-1: bad: yaml\n---\n# ADR-2\n",
        );
        let records = [good, bad];
        let (population, notice) =
            read_population(&records, &no_relation_field_overrides(), "graph");
        assert_eq!(population.unit(), crate::report::PopulationUnit::Record);
        assert_eq!(
            (
                population.eligible(),
                population.examined(),
                population.unreadable()
            ),
            (2, 1, 1)
        );
        let notice = notice.expect("a left-out record is disclosed");
        assert_eq!(notice.severity, crate::report::NoticeSeverity::Warning);
        assert_eq!(notice.subject, "graph");
        assert!(
            notice.message.contains("docs/adr/ADR-2-b.md"),
            "{}",
            notice.message
        );
        assert!(!notice.message.contains("ADR-1-a.md"), "{}", notice.message);
    }

    #[test]
    fn a_clean_or_empty_corpus_has_no_left_out_notice_and_an_empty_one_has_no_population() {
        let ok = record("docs/adr/ADR-1-a.md", "adr", "---\nStatus: Accepted\n---\n");
        let (population, notice) =
            read_population(&[ok], &no_relation_field_overrides(), "explain");
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert!(notice.is_none());
        let (population, notice) = read_population(&[], &no_relation_field_overrides(), "explain");
        assert_eq!((population.eligible(), population.examined()), (0, 0));
        assert!(notice.is_none());
    }
}
