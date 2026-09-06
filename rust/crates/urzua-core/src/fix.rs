//! `urzua fix` detect mode (ADR-0015 §3): read-only, emits one entry per
//! repairable fact. Apply mode (`--ids`, `--by`, `--force`) is not built
//! yet -- it needs real file mutation, identity resolution, and a
//! revision-log write-path, each its own piece of work. Detect mode alone
//! is "a real Phase 1 feature" per ADR-0015, not a stub.

use crate::record::Record;
use crate::rules::{compute_embodiment, parse_realized_by};

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
}
