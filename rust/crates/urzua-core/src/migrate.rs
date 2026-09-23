//! `urzua migrate schema --report` (SPEC-0001): a preview of which existing
//! records would fail a proposed new required field, before it's ever added
//! to config. Read-only -- reuses the same blank/placeholder/pending/present
//! classification `field.quality` already applies to declared required
//! fields, just against a candidate field that isn't declared yet.

use crate::field_state::classify;
use crate::record::Record;
use crate::report::{census, Notice, NoticeSeverity, Outcome, Population, PopulationUnit};
use crate::FieldState;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SchemaReportEntry {
    pub record: std::path::PathBuf,
    pub record_type: String,
    pub state: FieldState,
}

/// Every record whose value for `field` is not a real, present value --
/// exactly what would newly fail `header.required-fields`/`field.quality`
/// if `field` were added to that record type's `required_fields` today.
///
/// A record whose header did not parse is eligible and unexamined, not
/// classified. `classify(None)` cannot distinguish "the header parsed and
/// the field is absent" from "nothing in the header is readable at all" --
/// both read as `Blank` -- so calling it on an unparsed header would tell
/// the operator this field specifically will fail, when the real defect is
/// broader and already has its own diagnosis (`header.required-fields`).
/// Excluded records are still disclosed, as a `Notice`, so a preview run
/// against a partially-broken corpus does not read as complete.
pub fn schema_report(
    records: &[Record],
    field: &str,
) -> (Population, Vec<SchemaReportEntry>, Vec<Notice>) {
    let mut report = Vec::new();
    let mut notices = Vec::new();

    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        if record.header.is_unreadable() {
            notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: "header-unreadable".to_string(),
                message: format!(
                    "{}: header did not parse -- excluded from this preview, not counted as failing",
                    record.path.display()
                ),
            });
            return Outcome::Unreadable;
        }
        let state = classify(record.header.get(field));
        if state != FieldState::Present {
            report.push(SchemaReportEntry {
                record: record.path.clone(),
                record_type: record.record_type.clone(),
                state,
            });
        }
        Outcome::Examined
    });

    (population, report, notices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    #[test]
    fn a_record_missing_the_candidate_field_is_reported_observed_failing() {
        let r = record("docs/adr/0001-x.md", "adr", "> Status: Accepted\n");
        let (population, report, notices) = schema_report(&[r], "Reviewers");
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].state, FieldState::Blank);
        assert!(notices.is_empty());
    }

    #[test]
    fn a_record_already_carrying_the_field_is_not_reported() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Reviewers: alice\n",
        );
        let (population, report, _notices) = schema_report(&[r], "Reviewers");
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert!(report.is_empty());
    }

    #[test]
    fn an_unparsed_header_is_not_reported_as_blank_observed_failing() {
        // A header that does not parse gives no way to know whether the
        // candidate field is genuinely absent -- classify(None) would say
        // Blank regardless, telling the operator "add this field and it
        // will fail here" when the real defect is that nothing in the
        // header is readable at all. header.required-fields already
        // reports that; this must not guess a second, false diagnosis.
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/0001-x.md"),
            "adr".to_string(),
            "---\nStatus: Accepted\n  Date  :: broken\n---\n# 1 — X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        );
        let (population, report, _notices) = schema_report(&[r], "Reviewers");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "handed to the report and not judged, not silently dropped"
        );
        assert!(
            report.is_empty(),
            "no state can be claimed for a field behind a header that did not parse: {report:?}"
        );
    }

    #[test]
    fn placeholder_text_is_reported_distinctly_from_blank() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Reviewers: TBD\n",
        );
        let (population, report, _notices) = schema_report(&[r], "Reviewers");
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].state, FieldState::Placeholder);
    }
}
