use super::*;

/// What the caller found at the declared override log (`override_log`,
/// `SPEC-3`). The core never reads a file (`ADR-5`); a missing file and one
/// that is not text are told apart so neither reads as an empty log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideLogContent {
    Text(String),
    Missing,
    NotText,
}

#[derive(Debug, Clone)]
pub struct OverrideLog {
    /// The declared path, repository-relative.
    pub path: String,
    pub content: OverrideLogContent,
}

/// The columns of an override log's table, in order. The hook writes a row
/// with every cell but the last filled in; a reviewer sets the last
/// (`ADR-78` item 1, `MILE-188`).
pub const OVERRIDE_LOG_COLUMNS: [&str; 7] = [
    "Date",
    "Name",
    "Record",
    "Field",
    "Rule",
    "Reason",
    "Reviewed-by",
];

/// One parsed row of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideEntry {
    pub line: usize,
    pub date: String,
    pub name: String,
    pub record: String,
    pub field: String,
    pub rule: String,
    pub reason: String,
    pub reviewed_by: String,
}

/// A line of the log that begins a table row but is not an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedRow {
    pub line: usize,
    pub why: String,
}

/// The rows of the log's table. Prose, headings and the header and separator
/// rows are not entries; a row that is neither is reported rather than skipped.
pub fn parse_override_log(text: &str) -> (Vec<OverrideEntry>, Vec<MalformedRow>) {
    let mut entries = Vec::new();
    let mut malformed = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let is_separator = cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'));
        if is_separator || cells.first() == Some(&OVERRIDE_LOG_COLUMNS[0]) {
            continue;
        }
        if cells.len() != OVERRIDE_LOG_COLUMNS.len() {
            malformed.push(MalformedRow {
                line: i + 1,
                why: format!(
                    "has {} cells, an entry has {} ({})",
                    cells.len(),
                    OVERRIDE_LOG_COLUMNS.len(),
                    OVERRIDE_LOG_COLUMNS.join(", ")
                ),
            });
            continue;
        }
        entries.push(OverrideEntry {
            line: i + 1,
            date: cells[0].to_string(),
            name: cells[1].to_string(),
            record: cells[2].to_string(),
            field: cells[3].to_string(),
            rule: cells[4].to_string(),
            reason: cells[5].to_string(),
            reviewed_by: cells[6].to_string(),
        });
    }
    (entries, malformed)
}

/// What a `Reviewed-by` cell says, by the field-state vocabulary `field.quality`
/// and `field.pending` use: blank, a placeholder (the compiled-in tokens, or a
/// cell of nothing but dashes, which is a "not applicable" mark and names no
/// one) and a pending value are all "no one has reviewed this". A pending value
/// is judged by the pending vocabulary the repository's record types declare
/// (`StatusDecl::is_pending_value`), and by the compiled-in match when none
/// declares one. Only a cell that is `Present` names a reviewer.
fn reviewer_state(cell: &str, config: &Config) -> crate::FieldState {
    use crate::field_state::classify_with;
    let value = cell.trim();
    if !value.is_empty()
        && value
            .chars()
            .all(|c| matches!(c, '-' | '\u{2013}' | '\u{2014}'))
    {
        return crate::FieldState::Placeholder;
    }
    let declared: Vec<crate::status_model::StatusDecl> = super::status::statuses_by_type(config)
        .into_values()
        .filter(|d| d.pending.is_some())
        .collect();
    if declared.is_empty() {
        return classify_with(Some(value), None);
    }
    declared
        .iter()
        .map(|d| classify_with(Some(value), Some(d)))
        .find(|state| *state != crate::FieldState::Present)
        .unwrap_or(crate::FieldState::Present)
}

/// `override.unreviewed` (`ADR-78` item 1, `MILE-188`): an editor-hook override
/// that no one has reviewed is reported, so it surfaces in the pull request
/// that carries it. An entry is reviewed when its `Reviewed-by` cell names
/// someone (not blank, not a placeholder, not a declared pending value); the hook always writes it blank, and setting it is a human act made
/// in the same visible diff.
///
/// The population counts log entries. A declared log that does not exist, or
/// is not text, is unreadable and reported, and a row that is not an entry is a
/// finding: none of them reads as a clean log.
pub fn override_unreviewed(
    log: Option<&OverrideLog>,
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_OVERRIDE_UNREVIEWED;
    let mut findings = Vec::new();
    let Some(log) = log else {
        // Refused at config load; reaching here means the caller skipped it.
        return (
            RuleExecution {
                rule: RULE_ID.to_string(),
                population: Some(census(PopulationUnit::Entry, vec![()], |_| Outcome::Absent)),
                status: RuleStatus::Ran,
                examined_records: Vec::new(),
            },
            findings,
        );
    };
    let file = PathBuf::from(&log.path);
    let finding = |line: Option<usize>, variant: &'static str, message: String| Finding {
        rule: RULE_ID.to_string(),
        severity: FindingSeverity::Warning,
        file: file.clone(),
        line,
        message,
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source: ActionSource::new(variant),
    };
    let population = match &log.content {
        OverrideLogContent::Missing => {
            findings.push(finding(
                None,
                "override-log-missing",
                format!(
                    "the declared override log '{}' does not exist, so no override could be read; create it (the hook creates it on the first override) or remove `override_log`",
                    log.path
                ),
            ));
            census(PopulationUnit::Entry, vec![()], |_| Outcome::Unreadable)
        }
        OverrideLogContent::NotText => {
            findings.push(finding(
                None,
                "override-log-malformed",
                format!(
                    "override log '{}' is not text, so it was not read",
                    log.path
                ),
            ));
            census(PopulationUnit::Entry, vec![()], |_| Outcome::Unreadable)
        }
        OverrideLogContent::Text(text) => {
            let (entries, malformed) = parse_override_log(text);
            for row in &malformed {
                findings.push(finding(
                    Some(row.line),
                    "override-log-malformed",
                    format!(
                        "override log row {} is not an entry: it {}",
                        row.line, row.why
                    ),
                ));
            }
            census(PopulationUnit::Entry, entries, |e| {
                let state = reviewer_state(&e.reviewed_by, config);
                if state != crate::FieldState::Present {
                    let why = match state {
                        crate::FieldState::Blank => "its Reviewed-by cell is blank",
                        crate::FieldState::Placeholder => {
                            "its Reviewed-by cell holds a placeholder, not a person"
                        }
                        _ => "its Reviewed-by cell is declared pending",
                    };
                    findings.push(finding(
                        Some(e.line),
                        "override-unreviewed",
                        format!(
                            "override '{}' of {} ({}, field {}, {}) has not been reviewed ({why}): {}; set Reviewed-by on its row to the person who looked at it",
                            e.name, e.record, e.rule, e.field, e.date, e.reason
                        ),
                    ));
                }
                Outcome::Examined
            })
        }
    };
    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "# Override log\n\n| Date | Name | Record | Field | Rule | Reason | Reviewed-by |\n|---|---|---|---|---|---|---|\n";

    fn log(body: &str) -> OverrideLog {
        OverrideLog {
            path: ".urzua/override-log.md".to_string(),
            content: OverrideLogContent::Text(format!("{HEADER}{body}")),
        }
    }

    fn plain() -> Config {
        crate::config::parse("schema_version: 2\nrecord_types: {}\n").unwrap()
    }

    fn declaring_pending() -> Config {
        crate::config::parse("schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\n    statuses:\n      values: [Proposed, Accepted]\n      pending:\n        - \"pending\"\n        - \"unknown\"\n        - {pattern: '(?i)^(tbd|to be decided)\\b'}\n").unwrap()
    }

    fn row(reviewer: &str) -> OverrideLog {
        log(&format!("| 2026-11-05 | alice | docs/adr/ADR-1-a.md | Embodiment | embodiment.consistency | draft | {reviewer} |\n"))
    }

    #[test]
    fn a_blank_placeholder_or_pending_reviewer_is_not_a_review() {
        for config in [plain(), declaring_pending()] {
            for cell in [
                "", "   ", "-", "--", "\u{2014}", "TBD", "tbd", "todo", "name", "Pending",
            ] {
                let (_, findings) = override_unreviewed(Some(&row(cell)), &config);
                assert_eq!(findings.len(), 1, "{cell:?}: {findings:?}");
                assert!(
                    findings[0].message.contains("has not been reviewed"),
                    "{cell:?}"
                );
            }
        }
    }

    #[test]
    fn a_declared_pending_value_or_pattern_is_not_a_review() {
        let config = declaring_pending();
        for cell in [
            "Unknown",
            "TBD -- awaiting the board",
            "to be decided by Q3",
        ] {
            let (_, findings) = override_unreviewed(Some(&row(cell)), &config);
            assert_eq!(findings.len(), 1, "{cell:?}: {findings:?}");
        }
    }

    #[test]
    fn a_named_reviewer_is_a_review() {
        for config in [plain(), declaring_pending()] {
            for cell in ["carol", "Carol Diaz", "carol, dan"] {
                let (_, findings) = override_unreviewed(Some(&row(cell)), &config);
                assert!(findings.is_empty(), "{cell:?}: {findings:?}");
            }
        }
    }

    #[test]
    fn an_entry_with_a_blank_reviewer_is_reported_and_a_reviewed_one_is_not() {
        let l = log("| 2026-11-05 | alice | docs/adr/ADR-1-a.md | Embodiment | embodiment.consistency | pre-merge draft | |\n| 2026-11-05 | bob | docs/adr/ADR-2-b.md | Status | status.undeclared-value | fixture | carol |\n");
        let (exec, findings) = override_unreviewed(Some(&l), &plain());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'alice'"), "{findings:?}");
        assert_eq!(findings[0].line, Some(5));
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (2, 2));
    }

    #[test]
    fn a_row_that_is_not_an_entry_is_a_finding_not_a_skip() {
        let l = log("| 2026-11-05 | alice | docs/adr/ADR-1-a.md |\n");
        let (_, findings) = override_unreviewed(Some(&l), &plain());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("not an entry"), "{findings:?}");
    }

    #[test]
    fn a_declared_log_that_does_not_exist_is_reported_and_unreadable() {
        let l = OverrideLog {
            path: "x.md".into(),
            content: OverrideLogContent::Missing,
        };
        let (exec, findings) = override_unreviewed(Some(&l), &plain());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("does not exist"),
            "{findings:?}"
        );
        assert_eq!(findings[0].file, PathBuf::from("x.md"));
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined(), p.unreadable()), (1, 0, 1));
    }

    #[test]
    fn prose_headings_and_the_table_header_are_not_entries() {
        let (entries, malformed) = parse_override_log(HEADER);
        assert!(entries.is_empty() && malformed.is_empty());
    }
}
