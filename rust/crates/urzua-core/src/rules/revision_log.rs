use super::*;
use crate::config::strip_emphasis;

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
            let shape = config.revision_log_shape(&record.record_type);
            let Some(log) = find_revision_log_entries(content, &id, &shape) else {
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
            let log = match log {
                Ok(log) => log,
                Err(problem) => {
                    // A declared column that matches nothing is reported, not
                    // skipped: one finding, and no row is judged (ADR-94).
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: Some(problem.line),
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("revision-log-column-unreadable")
                            .section(id.name()),
                        message: problem.message,
                    });
                    return Outcome::Examined;
                }
            };

            for entry in log.entries {
                let class = strip_emphasis(&entry.change_class);
                if !shape.classes.iter().any(|c| c == class) {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(entry.line),
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("change-class-missing").section(id.name()),
                    message: format!(
                        "revision log entry dated {} has no real change_class (found {:?}) -- expected {}",
                        entry.date, entry.change_class, list_classes(&shape.classes)
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
            let shape = config.revision_log_shape(&record.record_type);
            // A log with no section is empty; a log whose declared columns
            // cannot be found, now or at the merge base, is unreadable: reading
            // it as empty would make every current row look added (ADR-94).
            let Ok(now) = find_revision_log_entries(now_text, &id, &shape)
                .map_or_else(|| Ok(Vec::new()), |r| r.map(|log| log.entries))
            else {
                return Outcome::Unreadable;
            };
            let Ok(then) = find_revision_log_entries(then_text, &id, &shape)
                .map_or_else(|| Ok(Vec::new()), |r| r.map(|log| log.entries))
            else {
                return Outcome::Unreadable;
            };
            let mut then: Vec<String> = then.into_iter().map(|e| e.row).collect();
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
                    let class = strip_emphasis(&e.change_class);
                    bump.classes.iter().any(|c| strip_emphasis(c) == class)
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
                        strip_emphasis(&first.change_class),
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

/// Where the date and class sit in a log's table, by index, and how wide the
/// header is. `urzua fix` lays its row out by this (`ADR-94`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LogLayout {
    pub(crate) width: usize,
    pub(crate) date: usize,
    pub(crate) class: usize,
}

/// A parsed revision log: its rows and the layout of its header.
pub(crate) struct RevisionLog {
    pub(crate) entries: Vec<RevisionLogEntry>,
    pub(crate) layout: LogLayout,
}

/// A declared column the header lacks or repeats: the log cannot be read.
#[derive(Debug)]
pub(crate) struct ColumnProblem {
    /// The header row's line number.
    pub(crate) line: usize,
    pub(crate) message: String,
}

/// The classes as a sentence: `"a"`, `"a" or "b"`, `"a", "b" or "c"`.
fn list_classes(classes: &[String]) -> String {
    let quoted: Vec<String> = classes.iter().map(|c| format!("{c:?}")).collect();
    match quoted.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
    }
}

/// The index of the header cell named `wanted` (emphasis and whitespace
/// stripped, exact and case-sensitive), or why there is none (`ADR-94`).
fn column_index(header: &[&str], wanted: &str, key: &str) -> Result<usize, String> {
    let wanted = strip_emphasis(wanted);
    let at: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(_, cell)| strip_emphasis(cell) == wanted)
        .map(|(i, _)| i + 1)
        .collect();
    let found = header
        .iter()
        .map(|c| format!("{:?}", strip_emphasis(c)))
        .collect::<Vec<_>>()
        .join(", ");
    match at.as_slice() {
        [one] => Ok(one - 1),
        [] => Err(format!(
            "revision log header has no column named {wanted:?} (`{key}`); found cells: {found}"
        )),
        many => {
            let (last, rest) = many.split_last().expect("at least two");
            let positions = format!(
                "{} and {last}",
                rest.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Err(format!(
                "revision log header has the column {wanted:?} (`{key}`) more than once, at cells {positions}; found cells: {found}"
            ))
        }
    }
}

/// Finds the line that opens the revision log (`id`, the type's declared
/// identity: a line that is the marker or heading, never one that merely
/// mentions it, and never one inside a fenced code block) and parses the
/// pipe-delimited table that follows it. `None` means no such section exists
/// in this record at all -- distinct from an existing but empty table, though
/// both currently produce no findings from the class rule.
///
/// The class and date cells are read from the columns `shape` names by header
/// (`ADR-94`), or from the last and first cell when it names none. A header
/// lacking a declared column, or repeating it, is `Some(Err(..))`: the log
/// cannot be read, and no row is.
pub(crate) fn find_revision_log_entries(
    content: &str,
    id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Option<Result<RevisionLog, ColumnProblem>> {
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
    let mut layout: Option<LogLayout> = None;
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
            // The header row: "Date | Change | Class". Undeclared columns are
            // the first and last cell, whatever the header says.
            let mut problems = Vec::new();
            let mut by_name = |name: &Option<String>, key: &str, default: usize| match name {
                None => default,
                Some(name) => column_index(&cells, name, key).unwrap_or_else(|why| {
                    problems.push(why);
                    default
                }),
            };
            let date = by_name(&shape.date_column, "date_column", 0);
            let class = by_name(&shape.class_column, "class_column", cells.len() - 1);
            if !problems.is_empty() {
                return Some(Err(ColumnProblem {
                    line: idx + 1,
                    message: problems.join("; "),
                }));
            }
            layout = Some(LogLayout {
                width: cells.len(),
                date,
                class,
            });
            continue;
        }
        if super::section::is_delimiter_row(stripped) {
            continue; // separator row: |---|---|---|
        }

        let layout = layout.expect("the header row precedes every data row");
        // A row wider than the header (a cell holding an unescaped pipe) keeps
        // the header's last column as its last cell, so a declared trailing
        // column reads as an undeclared one always did. Any other column is
        // read at its header position.
        let cell = |at: usize| {
            let at = if cells.len() > layout.width && at + 1 == layout.width {
                cells.len() - 1
            } else {
                at
            };
            cells.get(at).copied().unwrap_or_default().to_string()
        };
        entries.push(RevisionLogEntry {
            row: cells.join("|"),
            date: cell(layout.date),
            // Undeclared, the class is the row's own last cell, whatever the
            // header's width.
            change_class: match shape.class_column {
                None => cells.last().copied().unwrap_or_default().to_string(),
                Some(_) => cell(layout.class),
            },
            line: idx + 1,
        });
    }

    // A section with no table has no header to read: the undeclared layout.
    let layout = layout.unwrap_or(LogLayout {
        width: 0,
        date: 0,
        class: 0,
    });
    Some(Ok(RevisionLog { entries, layout }))
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

    #[test]
    fn an_alignment_row_is_a_delimiter_not_a_data_row_observed_failing() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |:---|:---|:---:|\n> | 2026-08-20 | Split. | structural |\n";
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    // ---- ADR-94: declared vocabulary, columns and the unreadable-log outcome ----

    fn declared(decl: crate::config::RevisionLogDecl) -> Config {
        let mut t = type_config(None);
        t.revision_log = Some(decl);
        config_with_types(vec![("spec", t)])
    }

    fn decl(
        classes: Option<&[&str]>,
        class_column: Option<&str>,
        date_column: Option<&str>,
    ) -> crate::config::RevisionLogDecl {
        crate::config::RevisionLogDecl {
            required: Some(true),
            classes: classes.map(|c| c.iter().map(|c| c.to_string()).collect()),
            class_column: class_column.map(str::to_string),
            date_column: date_column.map(str::to_string),
            ..Default::default()
        }
    }

    fn table(header: &str, rows: &[&str]) -> String {
        let cols = header.matches('|').count() - 1;
        let sep = format!("|{}", "---|".repeat(cols));
        let mut out = format!("# 1\n\n> **Revision log**\n>\n> {header}\n> {sep}\n");
        for r in rows {
            out.push_str(&format!("> {r}\n"));
        }
        out
    }

    #[test]
    fn a_declared_vocabulary_accepts_exactly_its_classes_and_the_finding_lists_them() {
        let config = declared(decl(Some(&["major", "minor", "structural"]), None, None));
        let content = table(
            "| Date | Change | Class |",
            &[
                "| 2026-01-01 | A. | **major** |",
                "| 2026-01-02 | B. | minor |",
                "| 2026-01-03 | C. | substantive |",
            ],
        );
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("2026-01-03"), "{findings:?}");
        assert!(
            findings[0]
                .message
                .contains("expected \"major\", \"minor\" or \"structural\""),
            "{findings:?}"
        );
    }

    #[test]
    fn a_single_declared_class_is_listed_alone_and_the_default_message_is_unchanged() {
        let config = declared(decl(Some(&["structural"]), None, None));
        let content = table(
            "| Date | Change | Class |",
            &["| 2026-01-01 | A. | other |"],
        );
        let (_, findings) = check(&config, &content);
        assert!(
            findings[0].message.ends_with("expected \"structural\""),
            "{findings:?}"
        );
        // No declaration: the message reads as it always did.
        let (_, findings) = check(&config_with_types(vec![]), &content);
        assert_eq!(
            findings[0].message,
            "revision log entry dated 2026-01-01 has no real change_class (found \"other\") -- expected \"substantive\" or \"structural\""
        );
    }

    #[test]
    fn a_class_column_selects_the_cell_by_header_name_in_any_position() {
        let content = table(
            "| Class | Date | Change |",
            &[
                "| **major** | 2026-01-01 | A. |",
                "| other | 2026-01-02 | B. |",
            ],
        );
        // Undeclared: the last cell is the class, exactly as today.
        let (_, findings) = check(&config_with_types(vec![]), &content);
        assert_eq!(findings.len(), 2, "{findings:?}");
        // Declared: the Class column, wherever it is.
        let config = declared(decl(Some(&["major", "minor"]), Some("Class"), Some("Date")));
        let (exec, findings) = check(&config, &content);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("dated 2026-01-02"),
            "{findings:?}"
        );
        assert!(
            findings[0].message.contains("found \"other\""),
            "{findings:?}"
        );
    }

    #[test]
    fn a_trailing_notes_column_does_not_hide_the_class() {
        let config = declared(decl(None, Some("Class"), None));
        let content = table(
            "| Date | Change | **Class** | Notes |",
            &[
                "| 2026-01-01 | A. | structural | n |",
                "| 2026-01-02 | B. | — | n |",
            ],
        );
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("2026-01-02"), "{findings:?}");
    }

    #[test]
    fn a_row_shorter_than_the_class_column_has_a_blank_class() {
        let config = declared(decl(None, Some("Class"), None));
        let content = table("| Date | Change | Class |", &["| 2026-01-01 | A. |"]);
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("found \"\""), "{findings:?}");
    }

    #[test]
    fn a_header_without_the_declared_column_is_one_finding_with_an_action_and_no_row_is_judged() {
        // Case-sensitive: `class` is not `Class`.
        for (decl, header) in [
            (decl(None, Some("Class"), None), "| Date | Change | class |"),
            (decl(None, None, Some("Date")), "| When | Change | Class |"),
        ] {
            let config = declared(decl);
            let content = table(
                header,
                &["| 2026-01-01 | A. | — |", "| 2026-01-02 | B. | — |"],
            );
            let (exec, findings) = check(&config, &content);
            assert_eq!(examined(&exec), 1);
            assert_eq!(findings.len(), 1, "{header}: {findings:?}");
            let f = &findings[0];
            assert_eq!(f.action_source.variant, "revision-log-column-unreadable");
            assert_eq!(
                f.action_source.target.section.as_deref(),
                Some("**Revision log**")
            );
            assert_eq!(f.line, Some(5), "the header line");
            assert!(f.message.contains("no column named"), "{f:?}");
            assert!(f.message.contains("found"), "{f:?}");
            for cell in header.trim_matches('|').split('|').map(str::trim) {
                assert!(f.message.contains(cell), "{cell} in {}", f.message);
            }
        }
    }

    #[test]
    fn a_duplicate_header_cell_is_a_finding_naming_the_positions_not_the_first_match() {
        let config = declared(decl(None, Some("Class"), None));
        // Distinct is case-sensitive after stripping: `**Class**` and `Class` collide.
        let content = table(
            "| Date | **Class** | Class |",
            &["| 2026-01-01 | structural | structural |"],
        );
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let f = &findings[0];
        assert_eq!(f.action_source.variant, "revision-log-column-unreadable");
        assert!(f.message.contains("more than once"), "{f:?}");
        assert!(f.message.contains("2 and 3"), "{f:?}");
        // `class` and `Class` are two headers.
        let content = table(
            "| Date | class | Class |",
            &["| 2026-01-01 | x | structural |"],
        );
        let (_, findings) = check(&config, &content);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_declared_columns_are_checked_even_when_the_log_has_no_rows() {
        let config = declared(decl(None, Some("Class"), None));
        let content = table("| Date | Change | Kind |", &[]);
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn an_undeclared_type_reads_the_first_and_last_cell_whatever_the_header_says() {
        let content = table("| A | A | A |", &["| 2026-01-01 | x | — |"]);
        let (_, findings) = check(&config_with_types(vec![]), &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "change-class-missing");
    }

    // The version-bump rule.

    fn bump_config(decl: crate::config::RevisionLogDecl) -> Config {
        declared(crate::config::RevisionLogDecl {
            version_bump: Some(crate::config::VersionBumpDecl {
                field: "Version".into(),
                classes: vec!["major".into()],
            }),
            ..decl
        })
    }

    fn bump(config: &Config, now: &str, then: Option<&str>) -> (RuleExecution, Vec<Finding>) {
        let r = record("docs/specs/0001-x.md", "spec", now);
        let mut full = HashMap::new();
        full.insert(r.path.clone(), now.to_string());
        let mut base = HashMap::new();
        if let Some(then) = then {
            base.insert(r.path.clone(), then.to_string());
        }
        let changed = vec![r.path.clone()];
        revision_log_version_bump(&[r], config, &full, Some(&changed), &base)
    }

    fn spec_text(header: &str, rows: &[&str]) -> String {
        format!("---\nVersion: '1'\n---\n{}", table(header, rows))
    }

    #[test]
    fn the_bump_rule_reads_the_declared_class_column_and_a_class_outside_the_bump_list_requires_none(
    ) {
        let config = bump_config(decl(Some(&["major", "cosmetic"]), Some("Class"), None));
        let header = "| Class | Date | Change |";
        let then = spec_text(header, &["| major | 2026-01-01 | A. |"]);
        let cosmetic = spec_text(
            header,
            &[
                "| major | 2026-01-01 | A. |",
                "| cosmetic | 2026-01-02 | B. |",
            ],
        );
        let (exec, findings) = bump(&config, &cosmetic, Some(&then));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
        let major = spec_text(
            header,
            &["| major | 2026-01-01 | A. |", "| major | 2026-01-02 | B. |"],
        );
        let (_, findings) = bump(&config, &major, Some(&then));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("adds a major change"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_log_whose_declared_column_is_missing_makes_the_record_unreadable_to_the_bump_rule() {
        let config = bump_config(decl(None, Some("Class"), None));
        let ok = spec_text(
            "| Date | Change | Class |",
            &["| 2026-01-01 | A. | major |"],
        );
        let bad = spec_text(
            "| Date | Change | Kind |",
            &["| 2026-01-01 | A. | major |", "| 2026-01-02 | B. | major |"],
        );
        let (exec, findings) = bump(&config, &bad, Some(&ok));
        assert_eq!(unreadable(&exec), 1);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty(), "no second finding: {findings:?}");
    }

    #[test]
    fn a_base_log_the_rule_cannot_read_is_unreadable_not_an_empty_log() {
        let config = bump_config(decl(None, Some("Class"), None));
        let now = spec_text(
            "| Date | Change | Class |",
            &["| 2026-01-01 | A. | major |"],
        );
        let then = spec_text("| Date | Change | Kind |", &["| 2026-01-01 | A. | major |"]);
        let (exec, findings) = bump(&config, &now, Some(&then));
        assert_eq!(unreadable(&exec), 1);
        assert!(
            findings.is_empty(),
            "an unreadable base must not raise a false bump finding: {findings:?}"
        );
    }

    #[test]
    fn an_undeclared_class_is_each_rows_last_cell_even_when_an_escaped_pipe_widens_the_row() {
        let content = table(
            "| Date | Change | Class |",
            &["| 2026-01-01 | introduced \\| all. | **structural** |"],
        );
        let (_, findings) = check(&config_with_types(vec![]), &content);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_declared_trailing_class_column_is_the_last_cell_of_a_row_an_unescaped_pipe_widened() {
        let config = declared(decl(None, Some("Class"), Some("Date")));
        let content = table(
            "| Date | Change | Class |",
            &[
                "| 2026-01-01 | `true|false` is read. | **structural** |",
                "| 2026-01-02 | `true|false` is read. | — |",
            ],
        );
        let (_, findings) = check(&config, &content);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("2026-01-02"), "{findings:?}");
        // A middle column is still read at its header position.
        let config = declared(decl(None, Some("Class"), None));
        let content = table(
            "| Date | Class | Change |",
            &["| 2026-01-01 | structural | `a|b` |"],
        );
        let (_, findings) = check(&config, &content);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
