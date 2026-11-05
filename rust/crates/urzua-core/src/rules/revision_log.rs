use super::*;

/// Rule 6 (ADR-0014): every existing revision-log entry names a real
/// `change_class`. A `—` or blank Class column is exactly the failure
/// ADR-0014 exists to prevent -- a substantive change folded silently into
/// "no class recorded," undetectable by any per-field presence check because
/// the field itself is present, just not a real value.
///
/// A record of a type that declares `revision_log: {required: true}` and has
/// no revision-log section is itself a finding (`BUG-50`): absence is not
/// compliance. A type that does not declare it, or declares `required: false`,
/// is not held to a log, and its records without one are eligible and
/// unexamined, as before.
pub fn revision_log_change_class(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED;
    let mut findings = Vec::new();

    // Every record is eligible. A record with no revision-log section (its type's declared identity) is
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
            let id = config.revision_log_id(&record.record_type);
            let Some(entries) = find_revision_log_entries(content, &id) else {
                if !config.revision_log_declared_required(&record.record_type) {
                    return Outcome::Absent;
                }
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("revision-log-missing").section(id.name()),
                    message: format!(
                        "record has no revision log; its type requires one ({})",
                        id.describe()
                    ),
                });
                return Outcome::Examined;
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
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("change-class-missing").section(config.revision_log_id(&record.record_type).name()),
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

/// `BUG-51`: a changed record that adds a revision-log row of a class its
/// type declares in `revision_log.version_bump` must change the declared
/// version field. "Adds" is a diff question, so the caller passes the changed
/// paths and the merge base's text of each changed record as data; the core
/// stays pure. A run with no diff reports `not-applicable`. A record with no
/// base text is new at this change and has nothing to bump from, so it is
/// eligible and unexamined; a renamed record is the same (a rename reads as a
/// deletion plus an addition).
pub fn revision_log_version_bump(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<std::path::PathBuf, String>,
    changed: Option<&[std::path::PathBuf]>,
    base_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_REVISION_LOG_VERSION_BUMP_REQUIRED;
    let Some(changed) = changed else {
        return (
            RuleExecution {
                rule: RULE_ID.to_string(),
                population: None,
                status: RuleStatus::NotApplicable,
                examined_records: Vec::new(),
            },
            Vec::new(),
        );
    };
    let changed: std::collections::HashSet<&std::path::PathBuf> = changed.iter().collect();
    // The changed records of a type that declares a bump, in path order.
    let mut candidates: Vec<&Record> = records
        .iter()
        .filter(|r| changed.contains(&r.path))
        .filter(|r| {
            config
                .record_types
                .get(&r.record_type)
                .is_some_and(|t| t.version_bump().is_some())
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let mut findings = Vec::new();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let Some(type_config) = config.record_types.get(&record.record_type) else {
                return Outcome::Absent;
            };
            let Some(bump) = type_config.version_bump() else {
                return Outcome::Absent;
            };
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            let (Some(now_text), Some(then_text)) =
                (full_text.get(&record.path), base_text.get(&record.path))
            else {
                return Outcome::Absent;
            };
            let id = config.revision_log_id(&record.record_type);
            let now = find_revision_log_entries(now_text, &id).unwrap_or_default();
            let mut then: Vec<String> = find_revision_log_entries(then_text, &id)
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.row)
                .collect();
            // A multiset difference: a row present before is consumed once.
            let added: Vec<&RevisionLogEntry> = now
                .iter()
                .filter(|e| match then.iter().position(|r| *r == e.row) {
                    Some(i) => {
                        then.swap_remove(i);
                        false
                    }
                    None => true,
                })
                .filter(|e| {
                    let class = e.change_class.trim_matches('*').trim();
                    bump.classes.iter().any(|c| c.trim() == class)
                })
                .collect();
            let Some(first) = added.first() else {
                return Outcome::Examined;
            };
            let before = crate::record::Record::parse_with_shape_and_prefix(
                record.path.clone(),
                record.record_type.clone(),
                then_text,
                type_config.header_shape,
                record.type_prefix.clone(),
            );
            let field = bump.field.trim();
            let norm = |v: Option<&str>| v.map(|v| v.trim().to_string());
            if norm(before.header.get(field)) == norm(record.header.get(field)) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(first.line),
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("version-bump").field(field),
                    message: format!(
                        "revision log entry dated {} adds a {} change but '{field}' is unchanged from the merge base ({}) -- bump it",
                        first.date,
                        first.change_class.trim_matches('*').trim(),
                        norm(record.header.get(field)).unwrap_or_else(|| "absent".to_string()),
                    ),
                });
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
    /// The row's cells, trimmed and joined, so a row is the same row at the
    /// merge base and now whatever the table's padding.
    pub(crate) row: String,
    pub(crate) date: String,
    pub(crate) change_class: String,
    pub(crate) line: usize,
}

/// Finds the line that opens the revision log (`id`, the type's declared
/// identity: a line that is the marker or heading, never one that merely
/// mentions it, and never one inside a fenced code block) and parses the
/// pipe-delimited table that follows it. `None` means no such section exists
/// in this record at all -- distinct from `Some(vec![])`, an existing but empty
/// table, though both currently produce no findings from this rule.
pub(crate) fn find_revision_log_entries(
    content: &str,
    id: &crate::config::RevisionLogId,
) -> Option<Vec<RevisionLogEntry>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut fence: Option<char> = None;
    let mut marker_idx = None;
    for (idx, line) in lines.iter().enumerate() {
        let stripped = line.trim_start_matches(|c: char| c == '>' || c.is_whitespace());
        let fence_char = ["```", "~~~"]
            .iter()
            .find(|f| stripped.starts_with(**f))
            .and_then(|f| f.chars().next());
        match (fence, fence_char) {
            (None, Some(c)) => fence = Some(c),
            (Some(open), Some(c)) if open == c => fence = None,
            (Some(_), _) => {}
            (None, None) => {
                if id.opens_at(line) {
                    marker_idx = Some(idx);
                    break;
                }
            }
        }
    }
    let marker_idx = marker_idx?;

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
            row: cells.join("|"),
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

        let (exec, findings) =
            revision_log_change_class(&[r], &config_with_types(vec![]), &full_text);
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

        let (_, findings) = revision_log_change_class(&[r], &config_with_types(vec![]), &full_text);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    fn log_decl(required: Option<bool>) -> crate::config::RevisionLogDecl {
        crate::config::RevisionLogDecl {
            marker: None,
            heading: None,
            required,
            version_bump: None,
        }
    }

    #[test]
    fn a_record_of_a_type_declaring_required_true_with_no_revision_log_section_is_a_finding() {
        // BUG-50: absence of the section is a finding, with the add-section action.
        let content = "---\nStatus: Draft\n---\n# 0001 — X\n";
        let r = record("docs/adr/0001-x.md", "adr", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        let mut t = type_config(None);
        t.revision_log = Some(log_decl(Some(true)));

        let (exec, findings) =
            revision_log_change_class(&[r], &config_with_types(vec![("adr", t)]), &full_text);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "revision-log-missing");
        assert_eq!(
            findings[0].action_source.target.section.as_deref(),
            Some("**Revision log**")
        );
        assert!(findings[0].message.contains("no revision log"));
    }

    #[test]
    fn a_record_of_a_type_that_does_not_declare_revision_log_with_no_log_is_absent_and_silent() {
        // Opt-in: a config that never declared `revision_log` is unchanged.
        let content = "---\nStatus: Draft\n---\n# 0001 — X\n";
        for t in [type_config(None), {
            let mut t = type_config(None);
            t.revision_log = Some(log_decl(None));
            t
        }] {
            let r = record("docs/adr/0001-x.md", "adr", content);
            let mut full_text = HashMap::new();
            full_text.insert(r.path.clone(), content.to_string());
            let (exec, findings) =
                revision_log_change_class(&[r], &config_with_types(vec![("adr", t)]), &full_text);
            assert_eq!(examined(&exec), 0);
            assert!(findings.is_empty(), "{findings:?}");
        }
    }

    #[test]
    fn a_record_of_a_type_declaring_required_false_with_no_log_is_absent_and_silent() {
        let mut t = type_config(None);
        t.revision_log = Some(crate::config::RevisionLogDecl {
            marker: None,
            heading: None,
            required: Some(false),
            version_bump: None,
        });
        let config = config_with_types(vec![("waiver", t)]);
        let content = "# 1\n\nNo log.\n";
        let r = record("docs/waivers/1.md", "waiver", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        let (exec, findings) = revision_log_change_class(&[r], &config, &full_text);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty(), "{findings:?}");
    }

    fn check(config: &Config, content: &str) -> (RuleExecution, Vec<Finding>) {
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        revision_log_change_class(&[r], config, &full_text)
    }

    const LOG: &str =
        "> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Split out. | — |\n";

    #[test]
    fn prose_mentioning_the_marker_does_not_hide_the_real_log_observed_failing() {
        let content = format!(
            "# 0001 — X\n\nThe **Revision log** below is append-only.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n> **Revision log**\n>\n{LOG}"
        );
        let (exec, findings) = check(&config_with_types(vec![]), &content);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("2026-08-20"));
    }

    #[test]
    fn a_marker_inside_a_fenced_block_is_not_the_log() {
        let content = format!("# 1\n\n```\n**Revision log**\n{LOG}```\n");
        let mut t = type_config(None);
        t.revision_log = Some(log_decl(Some(true)));
        let (exec, findings) = check(&config_with_types(vec![("spec", t)]), &content);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "revision-log-missing");
    }

    #[test]
    fn a_declared_heading_identity_finds_a_renamed_section() {
        let mut t = type_config(None);
        t.revision_log = Some(crate::config::RevisionLogDecl {
            marker: None,
            heading: Some("History".into()),
            required: Some(true),
            version_bump: None,
        });
        let config = config_with_types(vec![("spec", t)]);
        let content = format!("# 1\n\n## History\n\n{LOG}");
        let (exec, findings) = check(&config, &content);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        // The default marker is no longer this type's log.
        let old = format!("# 1\n\n**Revision log**\n\n{LOG}");
        let (_, findings) = check(&config, &old);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "revision-log-missing");
        assert_eq!(
            findings[0].action_source.target.section.as_deref(),
            Some("History")
        );
    }
}
