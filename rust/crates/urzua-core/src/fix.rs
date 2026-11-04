//! `urzua fix` detect mode (ADR-0015 §3): read-only, emits one entry per
//! repairable fact. Apply mode (`--ids`, `--by`, `--force`) is not built
//! yet -- it needs real file mutation, identity resolution, and a
//! revision-log write-path, each its own piece of work. Detect mode alone
//! is "a real Phase 1 feature" per ADR-0015, not a stub.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::{Config, RelationRole};
use crate::line_edit::{table_row_prefix, LineDoc};
use crate::record::Record;
use crate::report::{census, Outcome, Population, PopulationUnit};
use crate::rules::{
    compute_embodiment, find_revision_log_entries, parse_realized_by, relation_field_name,
};

/// One mechanically-detected disagreement between a record's stated value
/// and what the tool computes -- the same shape RFC-0008 §3 specifies for
/// `urzua fix`'s detect-mode output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Repair {
    pub record: std::path::PathBuf,
    pub field: String,
    pub current_value: String,
    pub computed_value: String,
    /// RFC-0008 §2's repair tiers. Only Tier 1 (recompute in place) exists
    /// in this MVP.
    pub tier: u8,
    /// The evidence the computation read, named in the record it produces
    /// (RFC-0008 §6) -- a reader can disagree with the derivation, not just
    /// trust it.
    pub evidence: String,
}

/// Tier 1 repairs for `Embodiment`: the only tool-writable field this MVP
/// knows about (ADR-0018). Every other field stays permanently
/// hand-authored (RFC-0008 §1's eligibility test).
///
/// The candidate is the *pair*: a repair needs both `Embodiment` and
/// `Realized-by`, so a record carrying only one of them is handed to this
/// function and reaches no verdict -- `Absent`, not a candidate that never
/// existed. Every loaded record is eligible; `fix` has no config-declared
/// field set to filter against the way `check`'s rules do, so unlike
/// `embodiment_consistency` this reads every record rather than a declared
/// slot.
///
/// Returns the population alongside the repairs found, so a caller can
/// distinguish "examined some, found nothing to repair" from "examined
/// nothing" -- the same no-silent-no-op discipline `check` follows.
pub fn detect_repairs(
    records: &[Record],
    config: &Config,
    drifted: &HashSet<PathBuf>,
) -> (Population, Vec<Repair>) {
    let mut repairs = Vec::new();

    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        let state_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentState);
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        let Some(stated) = record.header.get(state_field) else {
            return Outcome::Absent;
        };
        let Some(realized_by_value) = record.header.get(locator_field) else {
            return Outcome::Absent;
        };

        // Drift (ADR-32) is computed by the caller from git history and
        // handed in as plain data, the same shape `embodiment_consistency`
        // takes, so this function stays pure (MILE-15).
        let computed = compute_embodiment(
            &parse_realized_by(realized_by_value),
            drifted.contains(&record.path),
        );
        let stated = stated.trim();
        if stated != computed {
            repairs.push(Repair {
                record: record.path.clone(),
                field: state_field.to_string(),
                current_value: stated.to_string(),
                computed_value: computed.to_string(),
                tier: 1,
                evidence: format!("{locator_field}: {realized_by_value}"),
            });
        }
        Outcome::Examined
    });

    (population, repairs)
}

/// Computes the new file content for applying one Tier-1 repair. Pure --
/// returns the new text; the caller performs the actual write, the identity
/// resolution, and the re-verification-before-write RFC-0008 §4 requires.
///
/// Refuses (rather than silently skipping the requirement) when the record
/// has no `Revision log` section to append to: ADR-0015's reversibility
/// clause depends on that entry existing, so a write with nowhere to record
/// itself is not a case this function will produce, not even once.
pub fn apply_repair(
    content: &str,
    repair: &Repair,
    today: &str,
    by: &str,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);

    let field_line = doc
        .find_line(|l| l.contains(&repair.field) && l.contains(&repair.current_value))
        .ok_or_else(|| {
            format!(
                "could not find a line containing both '{}' and '{}' to edit -- the record may have changed since detect ran",
                repair.field, repair.current_value
            )
        })?;
    let edited_line = doc.line(field_line).unwrap_or_default().replacen(
        &repair.current_value,
        &repair.computed_value,
        1,
    );
    doc.replace_line(field_line, edited_line);

    let edited_content = doc.to_string();
    let entries = find_revision_log_entries(&edited_content).ok_or_else(|| {
        "record has no Revision log section -- refusing to apply without one to record the change in (ADR-0015's reversibility requirement)".to_string()
    })?;
    let Some(last_entry) = entries.last() else {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    };

    // The new row repeats the existing last row's own prefix, so it lands
    // inside the table whether the log is a blockquote table or a plain one.
    let prefix = doc
        .line(last_entry.line)
        .and_then(table_row_prefix)
        .unwrap_or("> ")
        .to_string();
    let new_row = format!(
        "{prefix}| {today} | Tool-authored (by {by}): recomputed `{}` from `{}` -- stated `{}` disagreed with the computed value. | **structural** |",
        repair.field, repair.evidence, repair.current_value
    );
    doc.insert_after(last_entry.line, new_row);

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), "adr".to_string(), content)
    }

    fn no_relation_field_overrides() -> Config {
        Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            relation_pairs: Vec::new(),
            rules: std::collections::BTreeMap::new(),
            record_types: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn a_mismatched_embodiment_is_a_tier_1_repair_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "> Embodiment: Not started\n> Realized-by: code:src/lib.rs\n",
        );
        let (population, repairs) =
            detect_repairs(&[r], &no_relation_field_overrides(), &HashSet::new());
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].field, "Embodiment");
        assert_eq!(repairs[0].current_value, "Not started");
        assert_eq!(repairs[0].computed_value, "Implemented");
        assert_eq!(repairs[0].tier, 1);
    }

    #[test]
    fn a_drifted_record_is_repaired_to_drift_detected_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "> Embodiment: Implemented\n> Realized-by: code:src/lib.rs\n",
        );
        let drifted: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect_repairs(&[r], &no_relation_field_overrides(), &drifted);
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].computed_value, "Drift detected");
    }

    #[test]
    fn a_matching_embodiment_is_not_a_repair() {
        let r = record(
            "docs/adr/0001-x.md",
            "> Embodiment: Implemented\n> Realized-by: code:src/lib.rs\n",
        );
        let (population, repairs) =
            detect_repairs(&[r], &no_relation_field_overrides(), &HashSet::new());
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert!(repairs.is_empty());
    }

    #[test]
    fn a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing()
    {
        let r = record(
            "docs/adr/0001-x.md",
            "> Phase: Not started\n> Evidence: code:src/lib.rs\n",
        );
        let config = Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            relation_pairs: Vec::new(),
            rules: std::collections::BTreeMap::new(),
            record_types: [(
                "adr".to_string(),
                crate::config::RecordTypeConfig {
                    dir: "docs/adr".to_string(),
                    identity: None,
                    fields: None,
                    sections: None,
                    required_fields: vec![],
                    header_shape: crate::header::HeaderShape::default(),
                    prefix: None,
                    header_layout: None,
                    known_fields: Some(vec!["Phase".to_string(), "Evidence".to_string()]),
                    pointer_fields: None,
                    narrative_fields: None,
                    spec: None,
                    relation_fields: Some(crate::config::RelationFields {
                        embodiment_state: Some("Phase".to_string()),
                        embodiment_locator: Some("Evidence".to_string()),
                        ..Default::default()
                    }),
                },
            )]
            .into_iter()
            .collect(),
        };
        let (population, repairs) = detect_repairs(&[r], &config, &HashSet::new());
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1, "{repairs:?}");
        assert_eq!(repairs[0].field, "Phase");
        assert_eq!(repairs[0].computed_value, "Implemented");
    }

    #[test]
    fn no_realized_by_leaves_the_record_eligible_but_unexamined() {
        // Absent, not out of population: the record was handed to this
        // function and reached no verdict, which is what `eligible() > 0`
        // says and a bare examined-count could not.
        let r = record("docs/adr/0001-x.md", "> Embodiment: Not started\n");
        let (population, repairs) =
            detect_repairs(&[r], &no_relation_field_overrides(), &HashSet::new());
        assert_eq!((population.eligible(), population.examined()), (1, 0));
        assert!(repairs.is_empty());
    }

    fn sample_repair() -> Repair {
        Repair {
            record: PathBuf::from("docs/adr/0001-x.md"),
            field: "Embodiment".to_string(),
            current_value: "Not started".to_string(),
            computed_value: "Verified".to_string(),
            tier: 1,
            evidence: "Realized-by: code:src/lib.rs, test:tests/it.rs".to_string(),
        }
    }

    #[test]
    fn apply_repair_edits_the_field_and_appends_a_revision_log_row_observed_failing() {
        let content = "# 0001 — X\n\n> Embodiment: Not started\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(content, &sample_repair(), "2026-09-05", "beau").unwrap();
        assert!(result.contains("> Embodiment: Verified"));
        assert!(!result.contains("> Embodiment: Not started"));
        assert!(result.contains("Tool-authored (by beau)"));
        assert!(result.contains("**structural**"));
        // the original row survives untouched
        assert!(result.contains("| 2026-08-20 | Initial. | **structural** |"));
    }

    #[test]
    fn apply_repair_refuses_without_a_revision_log_section() {
        let content = "# 0001 — X\n\n> Embodiment: Not started\n";
        let err = apply_repair(content, &sample_repair(), "2026-09-05", "beau").unwrap_err();
        assert!(err.contains("Revision log"));
    }

    #[test]
    fn apply_repair_refuses_when_the_stated_value_has_already_changed() {
        // The record changed since detect ran (e.g. a concurrent edit) --
        // RFC-0008 §4's re-verify-before-write requirement, exercised at the
        // level this pure function can enforce: it can't find what it was
        // told to replace.
        let content = "# 0001 — X\n\n> Embodiment: Implemented\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let err = apply_repair(content, &sample_repair(), "2026-09-05", "beau").unwrap_err();
        assert!(err.contains("could not find"));
    }

    #[test]
    fn apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing() {
        let content = "# 0001 — X\n\n> Embodiment: Not started\n\n**Revision log**\n\n| Date | Change | Class |\n|---|---|---|\n| 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(content, &sample_repair(), "2026-09-05", "beau").unwrap();
        let last = result.lines().last().unwrap();
        assert!(last.starts_with("| 2026-09-05 |"), "{last}");
        assert!(!result.contains("> | 2026-09-05"));
    }

    #[test]
    fn apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing() {
        let content = "# 0001 — X\r\n\r\n> Embodiment: Not started\r\n\r\n> **Revision log**\r\n>\r\n> | Date | Change | Class |\r\n> |---|---|---|\r\n> | 2026-08-20 | Initial. | **structural** |";
        let result = apply_repair(content, &sample_repair(), "2026-09-05", "beau").unwrap();
        assert!(result.starts_with(
            "# 0001 — X\r\n\r\n> Embodiment: Verified\r\n\r\n> **Revision log**\r\n>\r\n"
        ));
        assert!(!result.replace("\r\n", "").contains('\n'), "bare LF");
        assert!(!result.ends_with('\n'), "trailing newline was added");
        assert_eq!(result.lines().count(), content.lines().count() + 1);
    }
}
