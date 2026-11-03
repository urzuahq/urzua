use super::*;

/// Rule 6 (ADR-0014): every existing revision-log entry names a real
/// `change_class`. A `—` or blank Class column is exactly the failure
/// ADR-0014 exists to prevent -- a substantive change folded silently into
/// "no class recorded," undetectable by any per-field presence check because
/// the field itself is present, just not a real value.
///
/// This rule does not require a revision log to exist at all -- that is
/// SPEC-0002's separate "required sections per profile" rule. It only checks
/// entries in a section that is already there.
pub fn revision_log_change_class(
    records: &[Record],
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED;
    let mut findings = Vec::new();

    // Every record is eligible. A record with no `**Revision log**` marker is
    // *absent* from this rule's judgement, not outside its population -- and
    // absence is exactly what BUG-50 reports as indistinguishable from
    // compliance: SPEC-1 lost 18 revision rows by losing one line and the rule
    // stayed green. `eligible` minus `examined` is exactly that subtraction, expressed
    // where `records_examined` alone could not carry it.
    let candidates: Vec<&Record> = records.iter().collect();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let Some(content) = full_text.get(&record.path) else {
                return Outcome::Absent;
            };
            let Some(entries) = find_revision_log_entries(content) else {
                return Outcome::Absent;
            };

            for entry in entries {
                let class = entry.change_class.trim_matches('*').trim();
                if !matches!(class, "substantive" | "structural") {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(entry.line),
                    waived: None,
                    message: format!(
                        "revision log entry dated {} has no real change_class (found {:?}) -- expected \"substantive\" or \"structural\"",
                        entry.date, entry.change_class
                    ),
                });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

pub(crate) struct RevisionLogEntry {
    pub(crate) date: String,
    pub(crate) change_class: String,
    pub(crate) line: usize,
}

/// Finds a `**Revision log**` marker and parses the pipe-delimited table
/// that follows it. `None` means no such section exists in this record at
/// all -- distinct from `Some(vec![])`, an existing but empty table, though
/// both currently produce no findings from this rule.
pub(crate) fn find_revision_log_entries(content: &str) -> Option<Vec<RevisionLogEntry>> {
    let lines: Vec<&str> = content.lines().collect();
    let marker_idx = lines.iter().position(|l| l.contains("**Revision log**"))?;

    let mut entries = Vec::new();
    let mut row_index = 0; // 0 before the header row, 1 = header seen, 2+ = data rows
    for (idx, raw_line) in lines.iter().enumerate().skip(marker_idx + 1) {
        let stripped = raw_line.trim_start().trim_start_matches('>').trim();
        if stripped.is_empty() {
            if row_index == 0 {
                continue; // blank blockquote continuation before the table
            }
            break; // the table has ended
        }
        if !stripped.starts_with('|') {
            break;
        }

        let cells: Vec<&str> = stripped
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        row_index += 1;
        if row_index == 1 {
            continue; // header row: "Date | Change | Class"
        }
        if cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-'))
        {
            continue; // separator row: |---|---|---|
        }

        let (Some(date), Some(class)) = (cells.first(), cells.last()) else {
            continue;
        };
        entries.push(RevisionLogEntry {
            date: date.to_string(),
            change_class: class.to_string(),
            line: idx + 1,
        });
    }

    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn a_revision_log_entry_with_no_real_change_class_is_a_planted_violation_observed_failing() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Split out. | — |\n";
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (exec, findings) = revision_log_change_class(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("2026-08-20"));
    }

    #[test]
    fn a_revision_log_entry_with_a_real_change_class_produces_no_finding() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Renamed a field. | **structural** |\n";
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (_, findings) = revision_log_change_class(&[r], &full_text);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_record_with_no_revision_log_section_is_not_examined() {
        let content = "# 0001 — X\n\n> Status: Draft\n";
        let r = record("docs/adr/0001-x.md", "adr", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (exec, findings) = revision_log_change_class(&[r], &full_text);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }
}
