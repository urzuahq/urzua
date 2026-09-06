//! `urzua migrate schema --report` (SPEC-0001): a preview of which existing
//! records would fail a proposed new required field, before it's ever added
//! to config. Read-only -- reuses the same blank/placeholder/pending/present
//! classification `field.quality` already applies to declared required
//! fields, just against a candidate field that isn't declared yet.

use crate::field_state::classify;
use crate::record::Record;
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
pub fn schema_report(records: &[Record], field: &str) -> Vec<SchemaReportEntry> {
    records
        .iter()
        .filter_map(|record| {
            let state = classify(record.header.get(field));
            (state != FieldState::Present).then(|| SchemaReportEntry {
                record: record.path.clone(),
                record_type: record.record_type.clone(),
                state,
            })
        })
        .collect()
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
        let report = schema_report(&[r], "Reviewers");
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].state, FieldState::Blank);
    }

    #[test]
    fn a_record_already_carrying_the_field_is_not_reported() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Reviewers: alice\n",
        );
        let report = schema_report(&[r], "Reviewers");
        assert!(report.is_empty());
    }

    #[test]
    fn placeholder_text_is_reported_distinctly_from_blank() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Reviewers: TBD\n",
        );
        let report = schema_report(&[r], "Reviewers");
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].state, FieldState::Placeholder);
    }
}
