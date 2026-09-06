//! The two seed rules (SPEC-0002 Phase A), chosen because both fire on this
//! project's own corpus today -- a validator whose first run is green has
//! told you nothing about itself. Plus the Phase 6 rules, each with its own
//! planted-violation test.

use crate::field_state::classify;
use crate::record::Record;
use crate::report::{Finding, RuleExecution, Severity};
use crate::FieldState;
use std::collections::HashMap;

/// Rule 1: header format consistency within a record type, per a
/// config-declared required-field list. A majority-rule inference would
/// silently ratify whatever drifted in, so the shape is declared, not voted.
pub fn header_required_fields(
    records: &[Record],
    required_by_type: &HashMap<String, Vec<String>>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(required) = required_by_type.get(&record.record_type) else {
            continue;
        };
        examined += 1;

        if record.header.region.is_none() {
            findings.push(Finding {
                rule: "header.required-fields".to_string(),
                severity: Severity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
                message: format!(
                    "no header-shaped region found -- required fields {required:?} cannot be checked"
                ),
            });
            continue;
        }

        for dup in record.header.duplicate_keys() {
            findings.push(Finding {
                rule: "header.required-fields".to_string(),
                severity: Severity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
                message: format!("header key '{dup}' appears more than once -- ambiguous which value is operative"),
            });
        }

        for field in required {
            if record.header.get(field).is_none() {
                findings.push(Finding {
                    rule: "header.required-fields".to_string(),
                    severity: Severity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "missing required header field '{field}' for record type '{}'",
                        record.record_type
                    ),
                });
            }
        }
    }

    (
        RuleExecution {
            rule: "header.required-fields".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule 2: `Implements:`/`Derives-from:` resolves to a real record. The
/// target's status is surfaced in the message, never judged -- whether a
/// `Draft` target is acceptable is a policy decision (RFC-0012), not this
/// checker's call.
pub fn pointer_resolution(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    // Index every record by the identifiers a pointer could name: its
    // record-type-prefixed number (e.g. "RFC-0001") parsed from the filename.
    let mut index: HashMap<String, &Record> = HashMap::new();
    for record in records {
        if let Some(id) = record_id(record) {
            index.insert(id, record);
        }
    }

    for record in records {
        for field_name in ["Implements", "Derives-from"] {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            examined += 1;

            for reference in extract_references(value) {
                match index.get(&reference) {
                    Some(target) => {
                        let status = target.header.get("Status").unwrap_or("(no Status field)");
                        findings.push(Finding {
                            rule: "pointer.resolution".to_string(),
                            severity: Severity::Warning,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            message: format!(
                                "{field_name}: {reference} resolves; target Status = {status}"
                            ),
                        });
                    }
                    None => {
                        findings.push(Finding {
                            rule: "pointer.resolution".to_string(),
                            severity: Severity::Error,
                            file: record.path.clone(),
                            line: None,
 waived: None,
                            message: format!(
                                "{field_name}: {reference} does not resolve to any discovered record"
                            ),
                        });
                    }
                }
            }
        }
    }

    (
        RuleExecution {
            rule: "pointer.resolution".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// A record's own identifier, derived from its filename: `NNNN-slug.md` in a
/// directory whose configured type gives the prefix (e.g. `docs/rfc/0001-*`
/// -> `RFC-0001`). Filename-derived, not header-derived, so a record can be
/// referenced before its own header claims anything about itself.
pub(crate) fn record_id(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let (number, _rest) = stem.split_once('-')?;
    if number.len() != 4 || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}",
        record.record_type.to_ascii_uppercase(),
        number
    ))
}

/// Extract reference tokens like `RFC-0001` from a field value that may list
/// several, comma-separated, with trailing annotation in parentheses (e.g.
/// `RFC-0001 (Draft)`). A claim is the reference that begins an entry;
/// everything after it up to the next comma is annotation.
pub(crate) fn extract_references(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            let token = entry.split_whitespace().next()?;
            let token = token.trim_end_matches(['.', ':']);
            let is_reference = token
                .split_once('-')
                .map(|(prefix, num)| {
                    prefix.chars().all(|c| c.is_ascii_uppercase())
                        && !num.is_empty()
                        && num.chars().all(|c| c.is_ascii_digit())
                })
                .unwrap_or(false);
            is_reference.then(|| token.to_string())
        })
        .collect()
}

/// Rule 3 (Phase 6): a required field's *quality*, not just its presence.
/// `header.required-fields` only asks "is the key there"; a field holding
/// unedited template text or an explicit pending marker passes that check
/// and still isn't a real value. Blank is reported here too (redundantly
/// with Rule 1) so this rule's own report is self-contained.
pub fn field_quality(
    records: &[Record],
    required_by_type: &HashMap<String, Vec<String>>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(required) = required_by_type.get(&record.record_type) else {
            continue;
        };
        for field in required {
            let state = classify(record.header.get(field));
            examined += 1;
            let severity = match state {
                FieldState::Present => continue,
                FieldState::Blank => Severity::Error,
                FieldState::Placeholder => Severity::Error,
                FieldState::Pending => Severity::Warning,
            };
            findings.push(Finding {
                rule: "field.quality".to_string(),
                severity,
                file: record.path.clone(),
                line: None,
                waived: None,
                message: format!("field '{field}' is {state:?} -- not a real, present value"),
            });
        }
    }

    (
        RuleExecution {
            rule: "field.quality".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule 4 (Phase 6): a filename's own claimed number must match its
/// document's H1 title. A rename that updates one and not the other is
/// silent otherwise -- nothing else in this corpus cross-checks the two.
pub fn filename_title_consistency(
    records: &[Record],
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(filename_number) = filename_number(record) else {
            continue;
        };
        let Some(content) = full_text.get(&record.path) else {
            continue;
        };
        examined += 1;

        let Some(title_number) = title_number(content) else {
            findings.push(Finding {
                rule: "filename.title-consistency".to_string(),
                severity: Severity::Error,
                file: record.path.clone(),
                line: Some(1),
                waived: None,
                message: "no H1 title found to check against the filename's number".to_string(),
            });
            continue;
        };

        if filename_number != title_number {
            findings.push(Finding {
                rule: "filename.title-consistency".to_string(),
                severity: Severity::Error,
                file: record.path.clone(),
                line: Some(1),
 waived: None,
                message: format!(
                    "filename claims number {filename_number}, but the H1 title claims {title_number}"
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: "filename.title-consistency".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

fn filename_number(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let (number, _rest) = stem.split_once('-')?;
    (number.len() == 4 && number.chars().all(|c| c.is_ascii_digit())).then(|| number.to_string())
}

/// The document's own claimed number, from its first H1 heading: `# 0001 —
/// Title` or `# SPEC-0001 — Title`.
fn title_number(content: &str) -> Option<String> {
    let first_line = content.lines().find(|l| l.starts_with("# "))?;
    let after_hash = first_line.trim_start_matches('#').trim();
    let first_token = after_hash.split_whitespace().next()?;
    let digits: String = first_token.chars().filter(|c| c.is_ascii_digit()).collect();
    (digits.len() == 4).then_some(digits)
}

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
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(content) = full_text.get(&record.path) else {
            continue;
        };
        let Some(entries) = find_revision_log_entries(content) else {
            continue;
        };
        examined += 1;

        for entry in entries {
            let class = entry.change_class.trim_matches('*').trim();
            if !matches!(class, "substantive" | "structural") {
                findings.push(Finding {
                    rule: "revision-log.change-class-required".to_string(),
                    severity: Severity::Error,
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
    }

    (
        RuleExecution {
            rule: "revision-log.change-class-required".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

pub(crate) struct RevisionLogEntry {
    #[allow(dead_code)]
    pub(crate) date: String,
    #[allow(dead_code)]
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

/// A record's `Realized-by` field, categorized (ADR-0018's MVP scope --
/// three flat lists, not RFC-0005's full claim graph). `spec:`/`code:`/
/// `test:` prefixes; an unrecognized prefix is dropped rather than reported,
/// since this field's closure isn't this rule's concern.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RealizedBy {
    pub(crate) spec: Vec<String>,
    pub(crate) code: Vec<String>,
    pub(crate) test: Vec<String>,
}

pub(crate) fn parse_realized_by(value: &str) -> RealizedBy {
    let mut result = RealizedBy::default();
    for segment in value.split(',') {
        let segment = segment.trim();
        let Some((prefix, locator)) = segment.split_once(':') else {
            continue;
        };
        let locator = locator.trim().to_string();
        match prefix.trim() {
            "spec" => result.spec.push(locator),
            "code" => result.code.push(locator),
            "test" => result.test.push(locator),
            _ => {}
        }
    }
    result
}

/// ADR-0018's tier computation: the highest tier with a non-empty category.
/// `test` outranks `code` outranks `spec` -- a claim verified by a test is
/// stronger evidence than a claim only pointed at by a spec.
pub(crate) fn compute_embodiment(realized_by: &RealizedBy) -> &'static str {
    if !realized_by.test.is_empty() {
        "Verified"
    } else if !realized_by.code.is_empty() {
        "Implemented"
    } else if !realized_by.spec.is_empty() {
        "Specified"
    } else {
        "Not started"
    }
}

/// Rule (ADR-0018): a record's stated `Embodiment` must agree with what its
/// own `Realized-by` locators compute to. Both fields have to be present --
/// a record with no `Realized-by` at all has nothing for this rule to check
/// yet, which is a real, expected `records_examined: 0` on a corpus that
/// hasn't adopted the field, not a defect in the rule.
pub fn embodiment_consistency(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
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
        if stated.trim() != computed {
            findings.push(Finding {
                rule: "embodiment.consistency".to_string(),
                severity: Severity::Warning,
                file: record.path.clone(),
                line: None,
                waived: None,
                message: format!(
                    "stated Embodiment '{}' disagrees with '{computed}', computed from Realized-by",
                    stated.trim()
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: "embodiment.consistency".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule (ADR-0018): the same locator cited by more than one record's
/// `Realized-by` is a promotion candidate -- each record would otherwise
/// drift independently on what is really one piece of shared evidence (the
/// ADR-0072/0073/0074 shape RFC-0005 names). Reports only; a human runs the
/// actual promotion into a `claim` record, never this rule.
pub fn embodiment_locator_promotion_candidate(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    let mut citers: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<std::path::PathBuf>,
    > = std::collections::BTreeMap::new();
    for record in records {
        let Some(realized_by_value) = record.header.get("Realized-by") else {
            continue;
        };
        examined += 1;

        let parsed = parse_realized_by(realized_by_value);
        for locator in parsed
            .spec
            .iter()
            .chain(parsed.code.iter())
            .chain(parsed.test.iter())
        {
            // A BTreeSet, not a Vec: the same record citing one locator
            // under both `code:` and `test:` is still one record, not two
            // independent citers -- cross-record duplication is what needs
            // promotion, not cross-category duplication within one record.
            citers
                .entry(locator.clone())
                .or_default()
                .insert(record.path.clone());
        }
    }

    for (locator, paths) in citers {
        if paths.len() < 2 {
            continue;
        }
        let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let first_path = paths.iter().next().expect("checked len >= 2 above").clone();
        findings.push(Finding {
            rule: "embodiment.locator-promotion-candidate".to_string(),
            severity: Severity::Warning,
            file: first_path,
            line: None,
            waived: None,
            message: format!(
                "locator '{locator}' is cited by {} records ({}) -- consider promoting to a shared claim record",
                paths.len(),
                names.join(", ")
            ),
        });
    }

    (
        RuleExecution {
            rule: "embodiment.locator-promotion-candidate".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule 5 (Phase 6): `Supersedes`/`Superseded-by` reciprocity. Checking only
/// the forward claim leaves the reverse unguarded -- a record can claim to
/// supersede something that doesn't reciprocally point back, sending a
/// reader of the *target* to a record that denies the relation.
pub fn supersession_reciprocity(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    let mut index: HashMap<String, &Record> = HashMap::new();
    for record in records {
        if let Some(id) = record_id(record) {
            index.insert(id, record);
        }
    }

    for record in records {
        let Some(id) = record_id(record) else {
            continue;
        };
        let Some(value) = record.header.get("Supersedes / Superseded-by") else {
            continue;
        };
        if value.trim() == "—" {
            continue;
        }
        examined += 1;

        for reference in extract_references(value) {
            let Some(target) = index.get(&reference) else {
                findings.push(Finding {
                    rule: "relation.supersession-reciprocity".to_string(),
                    severity: Severity::Error,
                    file: record.path.clone(),
                    line: None,
 waived: None,
                    message: format!("Supersedes/Superseded-by: {reference} does not resolve to any discovered record"),
                });
                continue;
            };
            let target_value = target
                .header
                .get("Supersedes / Superseded-by")
                .unwrap_or("");
            if !target_value.contains(&id) {
                findings.push(Finding {
                    rule: "relation.supersession-reciprocity".to_string(),
                    severity: Severity::Error,
                    file: record.path.clone(),
                    line: None,
 waived: None,
                    message: format!(
                        "claims a Supersedes/Superseded-by relation with {reference}, but {reference} does not reciprocally name {id}"
                    ),
                });
            }
        }
    }

    (
        RuleExecution {
            rule: "relation.supersession-reciprocity".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use std::path::PathBuf;

    fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    #[test]
    fn missing_required_field_is_a_finding() {
        let r = record("docs/adr/0001-x.md", "adr", "> Status: Accepted\n");
        let mut required = HashMap::new();
        required.insert(
            "adr".to_string(),
            vec!["Status".to_string(), "Deciders".to_string()],
        );

        let (exec, findings) = header_required_fields(&[r], &required);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Deciders"));
    }

    #[test]
    fn a_present_required_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Deciders: someone\n",
        );
        let mut required = HashMap::new();
        required.insert(
            "adr".to_string(),
            vec!["Status".to_string(), "Deciders".to_string()],
        );

        let (exec, findings) = header_required_fields(&[r], &required);
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_planted_violation_is_observed_failing() {
        // The exact corpus defect this rule was chosen to catch: SPEC-0001's
        // shape lacks a field SPEC-0002-0005 share.
        let r = record(
            "docs/specs/0001-x.md",
            "spec",
            "> Status: Draft\n> Embodiment: Not started\n",
        );
        let mut required = HashMap::new();
        required.insert("spec".to_string(), vec!["Version".to_string()]);

        let (_, findings) = header_required_fields(&[r], &required);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Version"));
    }

    #[test]
    fn a_resolving_pointer_surfaces_target_status_without_judging_it() {
        let target = record("docs/rfc/0001-x.md", "rfc", "> Status: Draft\n");
        let source = record("docs/specs/0001-x.md", "spec", "> Implements: RFC-0001\n");

        let (exec, findings) = pointer_resolution(&[target, source]);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("Draft"));
    }

    #[test]
    fn a_non_resolving_pointer_is_an_error() {
        let source = record("docs/specs/0001-x.md", "spec", "> Implements: RFC-9999\n");
        let (_, findings) = pointer_resolution(&[source]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn field_quality_flags_placeholder_not_just_absence() {
        // A field present per Rule 1 (the key exists) but never actually
        // filled in -- Rule 1 alone would pass this record.
        let r = record("docs/adr/0001-x.md", "adr", "> Author: name\n");
        let mut required = HashMap::new();
        required.insert("adr".to_string(), vec!["Author".to_string()]);

        let (exec, findings) = field_quality(&[r], &required);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Placeholder"));
    }

    #[test]
    fn field_quality_treats_pending_as_a_warning_not_an_error() {
        let r = record("docs/adr/0001-x.md", "adr", "> Deciders: Pending\n");
        let mut required = HashMap::new();
        required.insert("adr".to_string(), vec!["Deciders".to_string()]);

        let (_, findings) = field_quality(&[r], &required);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn field_quality_passes_a_real_value() {
        let r = record("docs/adr/0001-x.md", "adr", "> Author: (project lead)\n");
        let mut required = HashMap::new();
        required.insert("adr".to_string(), vec!["Author".to_string()]);

        let (_, findings) = field_quality(&[r], &required);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn filename_title_mismatch_is_a_planted_violation_observed_failing() {
        let r = record("docs/adr/0001-x.md", "adr", "# 0002 — Wrong Number\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0002 — Wrong Number\n".to_string());

        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains('1'));
        assert!(findings[0].message.contains('2'));
    }

    #[test]
    fn matching_filename_and_title_produce_no_finding() {
        let r = record("docs/adr/0001-x.md", "adr", "# 0001 — Correct\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0001 — Correct\n".to_string());

        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_revision_log_entry_with_no_real_change_class_is_a_planted_violation_observed_failing() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Split out. | — |\n";
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (exec, findings) = revision_log_change_class(&[r], &full_text);
        assert_eq!(exec.records_examined, 1);
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
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn embodiment_disagreeing_with_realized_by_is_a_planted_violation_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Verified\n> Realized-by: code:src/lib.rs\n",
        );
        let (exec, findings) = embodiment_consistency(&[r]);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Verified"));
        assert!(findings[0].message.contains("Implemented"));
    }

    #[test]
    fn embodiment_agreeing_with_realized_by_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Implemented\n> Realized-by: code:src/lib.rs\n",
        );
        let (_, findings) = embodiment_consistency(&[r]);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_test_locator_outranks_a_code_locator() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(compute_embodiment(&realized), "Verified");
    }

    #[test]
    fn no_realized_by_field_is_not_examined() {
        let r = record("docs/adr/0001-x.md", "adr", "> Embodiment: Not started\n");
        let (exec, findings) = embodiment_consistency(&[r]);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/shared.rs\n",
        );
        let b = record(
            "docs/adr/0002-y.md",
            "adr",
            "> Realized-by: code:src/shared.rs\n",
        );
        let (exec, findings) = embodiment_locator_promotion_candidate(&[a, b]);
        assert_eq!(exec.records_examined, 2);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("src/shared.rs"));
    }

    #[test]
    fn one_record_citing_the_same_locator_under_two_categories_is_not_a_promotion_candidate() {
        // A single record's own code: and test: locators both naming the
        // same file is one record, not two independent citers -- promotion
        // exists for cross-record duplication, never cross-category
        // duplication within a single record's own Realized-by.
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/shared.rs, test:src/shared.rs\n",
        );
        let (_, findings) = embodiment_locator_promotion_candidate(&[a]);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_locator_cited_by_only_one_record_is_not_a_promotion_candidate() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/a.rs\n",
        );
        let (_, findings) = embodiment_locator_promotion_candidate(&[a]);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_one_directional_supersession_claim_is_a_reciprocity_violation() {
        // 0002 claims to supersede 0001, but 0001 doesn't reciprocally name
        // 0002 -- a reader of 0001 would never know it was superseded.
        let old = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/0002-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let (exec, findings) = supersession_reciprocity(&[old, new]);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn a_reciprocated_supersession_produces_no_finding() {
        let old = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0002\n",
        );
        let new = record(
            "docs/adr/0002-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let (_, findings) = supersession_reciprocity(&[old, new]);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn an_em_dash_supersession_value_is_not_examined() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let (exec, findings) = supersession_reciprocity(&[r]);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }
}
