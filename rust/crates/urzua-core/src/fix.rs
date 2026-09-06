//! `urzua fix` detect mode (ADR-0015 §3): read-only, emits one entry per
//! repairable fact. Apply mode (`--ids`, `--by`, `--force`) is not built
//! yet -- it needs real file mutation, identity resolution, and a
//! revision-log write-path, each its own piece of work. Detect mode alone
//! is "a real Phase 1 feature" per ADR-0015, not a stub.

use crate::record::Record;
use crate::rules::{compute_embodiment, find_revision_log_entries, parse_realized_by};

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
/// Returns the count of records actually examined (both `Embodiment` and
/// `Realized-by` present) alongside the repairs found, so a caller can
/// distinguish "examined some, found nothing to repair" from "examined
/// nothing" -- the same no-silent-no-op discipline `check` follows.
pub fn detect_repairs(records: &[Record]) -> (usize, Vec<Repair>) {
    let mut repairs = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(stated) = record.header.get("Embodiment") else {
            continue;
        };
        let Some(realized_by_value) = record.header.get("Realized-by") else {
            continue;
        };
        examined += 1;

        let computed = compute_embodiment(&parse_realized_by(realized_by_value));
        let stated = stated.trim();
        if stated != computed {
            repairs.push(Repair {
                record: record.path.clone(),
                field: "Embodiment".to_string(),
                current_value: stated.to_string(),
                computed_value: computed.to_string(),
                tier: 1,
                evidence: format!("Realized-by: {realized_by_value}"),
            });
        }
    }

    (examined, repairs)
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
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let field_line_idx = lines
        .iter()
        .position(|l| l.contains(&repair.field) && l.contains(&repair.current_value))
        .ok_or_else(|| {
            format!(
                "could not find a line containing both '{}' and '{}' to edit -- the record may have changed since detect ran",
                repair.field, repair.current_value
            )
        })?;
    lines[field_line_idx] =
        lines[field_line_idx].replacen(&repair.current_value, &repair.computed_value, 1);
    let edited_content = lines.join("\n") + "\n";

    let entries = find_revision_log_entries(&edited_content).ok_or_else(|| {
        "record has no Revision log section -- refusing to apply without one to record the change in (ADR-0015's reversibility requirement)".to_string()
    })?;
    let Some(last_entry) = entries.last() else {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    };

    let mut lines: Vec<String> = edited_content.lines().map(|l| l.to_string()).collect();
    let insert_at = last_entry.line; // 1-indexed; inserting at this index in a 0-indexed Vec places it right after
    let new_row = format!(
        "> | {today} | Tool-authored (by {by}): recomputed `{}` from `{}` -- stated `{}` disagreed with the computed value. | **structural** |",
        repair.field, repair.evidence, repair.current_value
    );
    lines.insert(insert_at, new_row);

    Ok(lines.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), "adr".to_string(), content)
    }

    #[test]
    fn a_mismatched_embodiment_is_a_tier_1_repair_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "> Embodiment: Not started\n> Realized-by: code:src/lib.rs\n",
        );
        let (examined, repairs) = detect_repairs(&[r]);
        assert_eq!(examined, 1);
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].field, "Embodiment");
        assert_eq!(repairs[0].current_value, "Not started");
        assert_eq!(repairs[0].computed_value, "Implemented");
        assert_eq!(repairs[0].tier, 1);
    }

    #[test]
    fn a_matching_embodiment_is_not_a_repair() {
        let r = record(
            "docs/adr/0001-x.md",
            "> Embodiment: Implemented\n> Realized-by: code:src/lib.rs\n",
        );
        let (examined, repairs) = detect_repairs(&[r]);
        assert_eq!(examined, 1);
        assert!(repairs.is_empty());
    }

    #[test]
    fn no_realized_by_produces_no_repair_and_is_not_examined() {
        let r = record("docs/adr/0001-x.md", "> Embodiment: Not started\n");
        let (examined, repairs) = detect_repairs(&[r]);
        assert_eq!(examined, 0);
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
}
