//! The two seed rules (SPEC-0002 Phase A), chosen because both fire on this
//! project's own corpus today -- a validator whose first run is green has
//! told you nothing about itself. Plus the Phase 6 rules, each with its own
//! planted-violation test.

use crate::config::Config;
use crate::field_state::classify;
use crate::header::HeaderLayout;
use crate::record::Record;
use crate::report::{Finding, RuleExecution, Severity};
use crate::FieldState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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

/// Rule (MILE-0075): a record's header line layout matches its type's
/// declared `header_layout`, when one is declared. `HeaderShape::Blockquote`
/// deliberately tolerates one-per-line and pipe-delimited interchangeably for
/// parsing (RFC-0010), which is correct for reading a corpus that hasn't
/// picked one -- but it means drift within a type that HAS settled on one
/// (found live: SPEC-2 through SPEC-6 all pipe-delimited, SPEC-1 alone
/// one-per-line) previously went unflagged. Declared, not voted, same
/// principle as Rule 1: a type with no declared `header_layout` is skipped
/// entirely, examined stays 0 for it -- this is additive, never a forced
/// migration.
pub fn header_layout_consistency(
    records: &[Record],
    declared_by_type: &HashMap<String, HeaderLayout>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(&declared) = declared_by_type.get(&record.record_type) else {
            continue;
        };
        let Some(actual) = record.header.layout() else {
            continue;
        };
        examined += 1;

        if actual != declared {
            let (declared_label, actual_label) = (layout_label(declared), layout_label(actual));
            findings.push(Finding {
                rule: "header.layout-consistency".to_string(),
                severity: Severity::Warning,
                file: record.path.clone(),
                line: None,
                waived: None,
                message: format!(
                    "header layout '{actual_label}' disagrees with '{declared_label}', declared for record type '{}'",
                    record.record_type
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: "header.layout-consistency".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

fn layout_label(layout: HeaderLayout) -> &'static str {
    match layout {
        HeaderLayout::OnePerLine => "one-per-line",
        HeaderLayout::PipeDelimited => "pipe-delimited",
    }
}

/// Rule (MILE-0081): every header field on a record belongs to its type's
/// declared `required_fields` or `known_fields`. `header.required-fields`
/// only catches a *missing* required field; nothing catches the opposite --
/// a field present on some records of a type but not others (found live:
/// SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that SPEC-2 through
/// SPEC-6 don't). Declared, not voted, same principle as `header_layout`: a
/// type with no declared `known_fields` is skipped entirely, examined stays
/// 0 for it -- required so an existing corpus's legitimate optional fields
/// (`Stable-Id`, `Realized-by`, `Derives-from` on `adr`, none of which are
/// *required*) don't all become false positives the moment this rule ships.
pub fn header_field_set_consistency(
    records: &[Record],
    allowed_by_type: &HashMap<String, HashSet<String>>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        let Some(allowed) = allowed_by_type.get(&record.record_type) else {
            continue;
        };
        examined += 1;

        for field in &record.header.fields {
            if !allowed.contains(&field.key.to_ascii_lowercase()) {
                findings.push(Finding {
                    rule: "header.field-set-consistency".to_string(),
                    severity: Severity::Warning,
                    file: record.path.clone(),
                    line: Some(field.line),
                    waived: None,
                    message: format!(
                        "field '{}' is not declared (required_fields or known_fields) for record type '{}'",
                        field.key, record.record_type
                    ),
                });
            }
        }
    }

    (
        RuleExecution {
            rule: "header.field-set-consistency".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule (MILE-0077 follow-up, ADR-0043): unlike every other rule, this one
/// examines `Config` directly rather than `&[Record]` -- it's an inventory
/// of the *schema* itself, not a per-record check. "Does this type need a
/// spec" stays editorial (ADR-0041 rejected a mechanical formula for that);
/// this only makes visible which configured types currently have no `spec`
/// pointer declared at all, the same "declared, not voted" shape as
/// `header_layout`/`known_fields`. A type with none declared is a real,
/// permanently valid state under ADR-0041 -- this is a signal to review,
/// never a mandate to write one.
pub fn type_no_declared_spec(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    for type_name in type_names {
        let type_config = &config.record_types[type_name];
        examined += 1;
        if type_config.spec.is_none() {
            findings.push(Finding {
                rule: "type.no-declared-spec".to_string(),
                severity: Severity::Warning,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' has no declared spec -- add `spec = \"SPEC-N\"` once one exists, or leave undeclared if ADR-41's editorial judgment says one isn't warranted"
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: "type.no-declared-spec".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// ADR-0033: `blockquote`/`bold-list` are deprecated, `yaml-frontmatter` is
/// the one shape this project actively grows. Same config-level shape as
/// `type_no_declared_spec` -- a schema inventory check, not a per-record
/// one. Parsing support for the deprecated shapes stays (an external
/// adopter's un-migrated corpus, or a first-time evaluator's existing docs,
/// per ADR-0033's own stated reason for deprecating rather than removing);
/// this rule only surfaces which configured types still declare one.
pub fn header_deprecated_shape(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    for type_name in type_names {
        let type_config = &config.record_types[type_name];
        examined += 1;
        if type_config.header_shape != crate::header::HeaderShape::YamlFrontmatter {
            findings.push(Finding {
                rule: "header.deprecated-shape".to_string(),
                severity: Severity::Warning,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' declares a deprecated header shape -- migrate to `header_shape = \"yaml-frontmatter\"` (ADR-33)"
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: "header.deprecated-shape".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule 2: `Implements:`/`Derives-from:`/`Parent:`/`Blocked-on:` resolves to a
/// real record when it names one. The target's status is surfaced in the
/// message, never judged -- whether a `Draft` target is acceptable is a
/// policy decision (RFC-0012), not this checker's call. `Parent` was added
/// after every spec's own `Parent: SPEC-N` pointer was found, live, to be
/// completely unchecked. `Blocked-on` moved here from a free-text milestone
/// body section (MILE-0083): free text with no reference token still passes
/// through untouched (`extract_references` finds nothing to check), so a
/// blocker that isn't a record yet stays legal.
pub fn pointer_resolution(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    // Index every record by the identifiers a pointer could name: its
    // record-type-prefixed number (e.g. "RFC-0001") parsed from the filename.
    // Keyed by numeric value (BUG-0002), not the raw string, so a reference's
    // padding never has to match the filename's exactly.
    let mut index: HashMap<String, &Record> = HashMap::new();
    for record in records {
        if let Some(id) = record_id(record) {
            index.insert(normalize_id(&id), record);
        }
    }

    for record in records {
        for field_name in ["Implements", "Derives-from", "Parent", "Blocked-on"] {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            examined += 1;

            for reference in extract_references(value) {
                match index.get(&normalize_id(&reference)) {
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

/// The fields a record is expected to hold as a *clean* comma-separated
/// reference list -- nothing else. `Blocked-on` is deliberately excluded: it
/// legitimately mixes free text with an optional embedded reference
/// (SPEC-0006 -- "a milestone can exist before a decision does"), the same
/// tolerance `extract_references` already provides it and must keep.
const CLEAN_POINTER_FIELDS: &[&str] = &["Implements", "Derives-from", "Parent"];

/// BUG-0007: a pointer field's value silently tolerating trailing prose after
/// the reference token (`extract_references` only reads the leading token,
/// discarding the rest) turned out to be a corpus-wide habit, not two
/// isolated mistakes -- `Derives-from: RFC-1 (Accepted)`'s status annotation
/// alone appears in 33 files. That annotation is redundant with
/// `pointer.resolution`'s own live status report and a staleness risk
/// (nothing re-verifies it once written), and `Parent`'s freeform prose in
/// `SPEC-2`/`SPEC-4` is worse -- neither was ever a deliberate schema
/// decision, `extract_references` just never rejected it.
///
/// This rule is a check on the raw field value, independent of
/// `pointer_resolution`: a field is clean only if every comma-separated
/// entry, once trimmed, is *exactly* a reference token (or the `—` no-value
/// placeholder) -- nothing before or after it.
pub fn header_pointer_field_clean(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    for record in records {
        for field_name in CLEAN_POINTER_FIELDS {
            let Some(value) = record.header.get(field_name) else {
                continue;
            };
            examined += 1;

            if value.trim() == "—" {
                continue;
            }

            for entry in value.split(',') {
                let entry = entry.trim();
                let is_clean_reference = entry
                    .split_once('-')
                    .map(|(prefix, num)| {
                        prefix.chars().all(|c| c.is_ascii_uppercase())
                            && !num.is_empty()
                            && num.chars().all(|c| c.is_ascii_digit())
                    })
                    .unwrap_or(false);
                if !is_clean_reference {
                    findings.push(Finding {
                        rule: "header.pointer-field-clean".to_string(),
                        severity: Severity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field_name} entry {entry:?} isn't a clean reference -- pointer fields should hold only comma-separated reference IDs (e.g. \"RFC-1\", not \"RFC-1 (Accepted)\"); pointer.resolution already reports a resolved target's live Status"
                        ),
                    });
                }
            }
        }
    }

    (
        RuleExecution {
            rule: "header.pointer-field-clean".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

/// Rule (MILE-0083): a `Blocked-on` pointer whose target has reached a
/// terminal status is a staleness signal, distinct from `pointer_resolution`'s
/// generic "resolves; target Status = X" noise -- a blocker that has actually
/// cleared deserves its own actionable finding, not one more line among many
/// routine ones. Terminal-status sets are a small, hardcoded MVP per type
/// (ADR-18's own "ship the MVP, extend once a real case demands more"
/// precedent), not a config surface -- nothing has asked for one yet. A
/// dangling `Blocked-on` reference is deliberately not this rule's job; that's
/// `pointer_resolution`'s error case, reused rather than duplicated here.
pub fn blocked_on_stale(records: &[Record]) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut examined = 0;

    let mut index: HashMap<String, &Record> = HashMap::new();
    for record in records {
        if let Some(id) = record_id(record) {
            index.insert(normalize_id(&id), record);
        }
    }

    for record in records {
        let Some(value) = record.header.get("Blocked-on") else {
            continue;
        };
        let references = extract_references(value);
        if references.is_empty() {
            continue;
        }
        examined += 1;

        for reference in references {
            let Some(target) = index.get(&normalize_id(&reference)) else {
                continue; // pointer_resolution already reports a dangling reference
            };
            let Some(status) = target.header.get("Status") else {
                continue;
            };
            if is_terminal_status(&target.record_type, status) {
                findings.push(Finding {
                    rule: "blocked-on.stale".to_string(),
                    severity: Severity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "blocked on {reference}, which has reached a terminal status ({status}) -- re-examine whether this record's Status/Blocked-on should update"
                    ),
                });
            }
        }
    }

    (
        RuleExecution {
            rule: "blocked-on.stale".to_string(),
            records_examined: examined,
        },
        findings,
    )
}

fn is_terminal_status(record_type: &str, status: &str) -> bool {
    let status = status.trim();
    let terminal: &[&str] = match record_type {
        "bug" => &["Fixed", "WontFix"],
        "adr" | "rfc" => &["Accepted", "Rejected", "Superseded"],
        "spec" => &["Accepted"],
        "milestone" => &["Done", "WontDo"],
        _ => &[],
    };
    terminal.contains(&status)
}

/// A record's own identifier, derived from its filename: `TYPE-NNNN-slug.md`,
/// type prefix explicit in the filename, matching how a record is referenced
/// in prose everywhere else (e.g. `Implements: ADR-36`). Filename-derived,
/// not header-derived, so a record can be referenced before its own header
/// claims anything about itself.
///
/// BUG-2: no fixed digit-count is required (any non-empty numeric prefix
/// resolves) -- a hardcoded 4-digit check here would silently stop matching
/// past 9999 records of one type. Matching itself is done by numeric value
/// (`normalize_id`), not by this string, so `ADR-0034` and a hand-typed
/// `ADR-34` reference resolve identically regardless of padding.
///
/// ADR-36's amendment removed acceptance of the legacy, pre-type-prefix
/// `NNNN-slug.md` shape (BUG-9): it was never a real shape any adopter's own
/// corpus would independently use, only this repo's own pre-ADR-36 history,
/// and that history no longer exists in the corpus either.
pub(crate) fn record_id(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let type_prefix = record.type_prefix.clone();

    let rest = stem.strip_prefix(&format!("{type_prefix}-"))?;
    let number = rest.split_once('-').map_or(rest, |(n, _)| n);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!("{type_prefix}-{number}"))
}

/// Numeric-value equality for an id/reference like `ADR-0034` or `ADR-34`
/// (BUG-0002): strips the numeric part's leading zeros so a filename's
/// padding and a hand-typed reference's padding never have to match
/// exactly. Falls back to the original string unchanged if the numeric part
/// doesn't parse (defensive only -- both callers already validated theirs).
pub(crate) fn normalize_id(id: &str) -> String {
    match id.split_once('-') {
        Some((prefix, number)) => match number.parse::<u64>() {
            Ok(n) => format!("{prefix}-{n}"),
            Err(_) => id.to_string(),
        },
        None => id.to_string(),
    }
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

        if filename_number.parse::<u64>().ok() != title_number.parse::<u64>().ok() {
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

/// Same acceptance as `record_id` (ADR-36/BUG-2): `TYPE-NNNN-slug.md`, any
/// non-empty numeric prefix, no fixed digit-count.
fn filename_number(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let type_prefix = record.type_prefix.clone();

    let rest = stem.strip_prefix(&format!("{type_prefix}-"))?;
    let number = rest.split_once('-').map_or(rest, |(n, _)| n);
    (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit())).then(|| number.to_string())
}

/// The document's own claimed number, from its first H1 heading: `# 0001 —
/// Title` or `# SPEC-0001 — Title`. Compared against `filename_number` by
/// numeric value (BUG-0002), not fixed digit-count or exact string, so
/// `# 36 — Title` in a `37-slug.md`-adjacent file still correctly mismatches
/// while `0036`/`36` never falsely mismatch on padding alone.
fn title_number(content: &str) -> Option<String> {
    let first_line = content.lines().find(|l| l.starts_with("# "))?;
    let after_hash = first_line.trim_start_matches('#').trim();
    let first_token = after_hash.split_whitespace().next()?;
    let digits: String = first_token.chars().filter(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
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

/// Every locator path across all three categories, flattened -- what a
/// caller needs to walk each one's git history for drift detection
/// (ADR-0032). Order isn't meaningful; only membership is.
pub fn realized_by_locator_paths(value: &str) -> Vec<String> {
    let parsed = parse_realized_by(value);
    parsed
        .spec
        .into_iter()
        .chain(parsed.code)
        .chain(parsed.test)
        .collect()
}

/// ADR-0018's tier computation: the highest tier with a non-empty category.
/// `test` outranks `code` outranks `spec` -- a claim verified by a test is
/// stronger evidence than a claim only pointed at by a spec. `drifted`
/// (ADR-0032) overrides every tier unconditionally -- `DriftDetected` is an
/// alarm state reachable from any other state, per the schema's own
/// description, not just another tier to rank against the others.
pub(crate) fn compute_embodiment(realized_by: &RealizedBy, drifted: bool) -> &'static str {
    if drifted {
        "Drift detected"
    } else if !realized_by.test.is_empty() {
        "Verified"
    } else if !realized_by.code.is_empty() {
        "Implemented"
    } else if !realized_by.spec.is_empty() {
        "Specified"
    } else {
        "Not started"
    }
}

/// Rule (ADR-0018/ADR-0032): a record's stated `Embodiment` must agree with
/// what its own `Realized-by` locators compute to, including drift -- a
/// locator that changed, per git history, since the `Realized-by` line was
/// last touched. Both `Embodiment` and `Realized-by` have to be present --
/// a record with no `Realized-by` at all has nothing for this rule to check
/// yet, which is a real, expected `records_examined: 0` on a corpus that
/// hasn't adopted the field, not a defect in the rule. `drifted` is
/// precomputed by the caller (git history is I/O, this function isn't --
/// same shape as `full_text` elsewhere in this module).
pub fn embodiment_consistency(
    records: &[Record],
    drifted: &HashSet<PathBuf>,
) -> (RuleExecution, Vec<Finding>) {
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

        let computed = compute_embodiment(
            &parse_realized_by(realized_by_value),
            drifted.contains(&record.path),
        );
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
            index.insert(normalize_id(&id), record);
        }
    }

    for record in records {
        let Some(id) = record_id(record) else {
            continue;
        };
        let normalized_id = normalize_id(&id);
        let Some(value) = record.header.get("Supersedes / Superseded-by") else {
            continue;
        };
        if value.trim() == "—" {
            continue;
        }
        examined += 1;

        for reference in extract_references(value) {
            let Some(target) = index.get(&normalize_id(&reference)) else {
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
            let target_names_back = extract_references(target_value)
                .iter()
                .any(|r| normalize_id(r) == normalized_id);
            if !target_names_back {
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

    fn record_with_prefix(path: &str, record_type: &str, prefix: &str, content: &str) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from(path),
            record_type.to_string(),
            content,
            crate::header::HeaderShape::default(),
            prefix.to_string(),
        )
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
    fn a_declared_pipe_delimited_type_flags_a_one_per_line_record() {
        // The exact corpus defect this rule was chosen to catch: SPEC-0001
        // rendered one-field-per-line while SPEC-0002 through SPEC-0006
        // settled on pipe-delimited, and nothing said so.
        let r = record(
            "docs/specs/0001-x.md",
            "spec",
            "> Version: 0.1\n> Status: Draft\n",
        );
        let mut declared = HashMap::new();
        declared.insert("spec".to_string(), HeaderLayout::PipeDelimited);

        let (exec, findings) = header_layout_consistency(&[r], &declared);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("one-per-line"));
        assert!(findings[0].message.contains("pipe-delimited"));
    }

    #[test]
    fn a_matching_layout_produces_no_finding() {
        let r = record(
            "docs/specs/0002-x.md",
            "spec",
            "> Version: 0.1 | Status: Draft\n",
        );
        let mut declared = HashMap::new();
        declared.insert("spec".to_string(), HeaderLayout::PipeDelimited);

        let (exec, findings) = header_layout_consistency(&[r], &declared);
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_declared_layout_is_skipped_entirely() {
        let r = record("docs/adr/0001-x.md", "adr", "> Status: Accepted\n");
        let declared = HashMap::new();

        let (exec, findings) = header_layout_consistency(&[r], &declared);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn an_undeclared_field_is_a_finding() {
        // The exact corpus defect this rule was chosen to catch: SPEC-1
        // carries Embodiment/Author/Derives-from that SPEC-2 through SPEC-6
        // don't -- modeled here on the smaller adr case for a focused test.
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Date: 2026-09-07\n> Sponsor: someone\n",
        );
        let mut allowed = HashMap::new();
        allowed.insert(
            "adr".to_string(),
            HashSet::from(["status".to_string(), "date".to_string()]),
        );

        let (exec, findings) = header_field_set_consistency(&[r], &allowed);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Sponsor"));
    }

    #[test]
    fn a_known_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Realized-by: code:x.rs\n",
        );
        let mut allowed = HashMap::new();
        allowed.insert(
            "adr".to_string(),
            HashSet::from(["status".to_string(), "realized-by".to_string()]),
        );

        let (exec, findings) = header_field_set_consistency(&[r], &allowed);
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_declared_known_fields_is_skipped_entirely() {
        let r = record("docs/specs/0001-x.md", "spec", "> Version: 0.1\n");
        let allowed = HashMap::new();

        let (exec, findings) = header_field_set_consistency(&[r], &allowed);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    fn type_config(spec: Option<&str>) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: Vec::new(),
            header_shape: crate::header::HeaderShape::default(),
            prefix: None,
            header_layout: None,
            known_fields: None,
            spec: spec.map(|s| s.to_string()),
        }
    }

    fn type_config_with_shape(
        shape: crate::header::HeaderShape,
    ) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: Vec::new(),
            header_shape: shape,
            prefix: None,
            header_layout: None,
            known_fields: None,
            spec: None,
        }
    }

    #[test]
    fn a_type_with_no_declared_spec_is_a_finding_observed_failing() {
        // The exact live case this rule was built to catch: before this
        // session declared any `spec` pointer, every configured type
        // (including milestone/bug/waiver, which already had SPEC-6/9/10)
        // had none wired into config at all.
        let mut record_types = HashMap::new();
        record_types.insert("milestone".to_string(), type_config(None));
        let config = Config {
            schema_version: 1,
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.toml"));
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("milestone"));
    }

    #[test]
    fn a_declared_spec_pointer_produces_no_finding() {
        let mut record_types = HashMap::new();
        record_types.insert("milestone".to_string(), type_config(Some("SPEC-6")));
        let config = Config {
            schema_version: 1,
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.toml"));
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_still_declaring_blockquote_is_a_finding() {
        let mut record_types = HashMap::new();
        record_types.insert(
            "adr".to_string(),
            type_config_with_shape(crate::header::HeaderShape::Blockquote),
        );
        let config = Config {
            schema_version: 1,
            record_types,
        };

        let (exec, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.toml"));
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("adr"));
    }

    #[test]
    fn a_type_declaring_yaml_frontmatter_produces_no_finding() {
        let mut record_types = HashMap::new();
        record_types.insert(
            "adr".to_string(),
            type_config_with_shape(crate::header::HeaderShape::YamlFrontmatter),
        );
        let config = Config {
            schema_version: 1,
            record_types,
        };

        let (exec, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.toml"));
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_resolving_pointer_surfaces_target_status_without_judging_it() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Draft\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-0001\n");

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
    fn a_parent_pointer_resolves_the_same_as_implements() {
        // Every spec's `Parent: SPEC-N` pointer was completely unchecked
        // before Parent was added to this rule's scanned fields -- a typo'd
        // or dangling Parent would never have been caught.
        let parent = record("docs/specs/SPEC-1-v0-cli.md", "spec", "> Status: Draft\n");
        let child = record(
            "docs/specs/SPEC-2-urzua-check.md",
            "spec",
            "> Parent: SPEC-1 (v0 CLI). Extra trailing prose that isn't a reference.\n",
        );
        let (_, findings) = pointer_resolution(&[parent, child]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("Parent: SPEC-1 resolves"));
    }

    #[test]
    fn a_dangling_parent_pointer_is_an_error() {
        let child = record("docs/specs/0002-x.md", "spec", "> Parent: SPEC-9999\n");
        let (_, findings) = pointer_resolution(&[child]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("Parent: SPEC-9999"));
    }

    #[test]
    fn a_derives_from_status_annotation_is_flagged_observed_failing() {
        // The exact live pattern found across 33 files: a hand-typed status
        // snapshot baked into the field value itself, redundant with
        // pointer.resolution's own live report and never re-verified.
        let r = record(
            "docs/adr/0038-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1 (Accepted)\n",
        );
        let (exec, findings) = header_pointer_field_clean(&[r]);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("RFC-1 (Accepted)"));
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn a_parent_field_with_freeform_trailing_prose_is_flagged() {
        let r = record(
            "docs/specs/0002-x.md",
            "spec",
            "> Status: Draft\n> Parent: SPEC-1 (v0 CLI). Cross-cutting rules stated there.\n",
        );
        let (_, findings) = header_pointer_field_clean(&[r]);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_clean_bare_reference_is_not_flagged() {
        let r = record(
            "docs/adr/0010-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1\n> Parent: —\n",
        );
        let (exec, findings) = header_pointer_field_clean(&[r]);
        assert_eq!(exec.records_examined, 2);
        assert!(findings.is_empty());
    }

    #[test]
    fn multiple_clean_references_are_not_flagged() {
        let r = record(
            "docs/adr/0008-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-8, ADR-15, ADR-19\n",
        );
        let (_, findings) = header_pointer_field_clean(&[r]);
        assert!(findings.is_empty());
    }

    #[test]
    fn an_empty_entry_between_commas_is_flagged_not_silently_skipped() {
        // A trailing/double comma (`RFC-1,,ADR-2`) produces an empty
        // split segment -- that must be flagged, not treated as a second
        // "no value" placeholder alongside the real one, "—".
        let r = record(
            "docs/adr/0011-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1,,ADR-2\n",
        );
        let (_, findings) = header_pointer_field_clean(&[r]);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_bare_placeholder_field_is_still_not_flagged() {
        let r = record(
            "docs/adr/0012-x.md",
            "adr",
            "> Status: Accepted\n> Parent: —\n",
        );
        let (_, findings) = header_pointer_field_clean(&[r]);
        assert!(findings.is_empty());
    }

    #[test]
    fn blocked_on_is_excluded_even_with_trailing_prose() {
        // Blocked-on deliberately mixes free text with an optional embedded
        // reference (SPEC-6) -- must never be flagged by this rule.
        let r = record(
            "docs/milestones/MILE-38-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: MILE-38 (staleness detection) -- sequenced first.\n",
        );
        let (exec, findings) = header_pointer_field_clean(&[r]);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding() {
        // The exact live case this rule was built for: MILE-2/3's Blocked-on
        // named BUG-3, which had already shipped (Status: Fixed) with nothing
        // catching it, since Blocked-on used to be unchecked free-text prose.
        let bug = record("docs/bugs/BUG-3-x.md", "bug", "> Status: Fixed\n");
        let milestone = record(
            "docs/milestones/MILE-2-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-3\n",
        );
        let (exec, findings) = blocked_on_stale(&[bug, milestone]);
        assert_eq!(exec.records_examined, 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("BUG-3"));
        assert!(findings[0].message.contains("Fixed"));
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn a_blocked_on_pointer_to_a_non_terminal_record_is_not_stale() {
        let bug = record("docs/bugs/0004-x.md", "bug", "> Status: Open\n");
        let milestone = record(
            "docs/milestones/MILE-9-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-4\n",
        );
        let (exec, findings) = blocked_on_stale(&[bug, milestone]);
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn blocked_on_free_text_with_no_reference_is_skipped() {
        let milestone = record(
            "docs/milestones/MILE-5-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: a decision that hasn't been made\n",
        );
        let (exec, findings) = blocked_on_stale(&[milestone]);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_dangling_blocked_on_reference_is_not_this_rule_s_job() {
        let milestone = record(
            "docs/milestones/MILE-6-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-9999\n",
        );
        let (exec, findings) = blocked_on_stale(&[milestone]);
        assert_eq!(exec.records_examined, 1);
        assert!(
            findings.is_empty(),
            "a dangling reference is pointer_resolution's error case, not this rule's"
        );
    }

    #[test]
    fn a_five_digit_filename_still_resolves_bug_0002_observed_failing() {
        // Before the fix, record_id() rejected any numeric prefix that
        // wasn't exactly 4 digits -- past 9999 records of one type,
        // references would have silently stopped resolving.
        let target = record("docs/rfc/RFC-10000-x.md", "rfc", "> Status: Draft\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "> Implements: RFC-10000\n",
        );
        let (_, findings) = pointer_resolution(&[target, source]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn a_reference_resolves_regardless_of_zero_padding() {
        // ADR-0034 (the filename's own padding) and a hand-typed ADR-34
        // reference must resolve to the same record (BUG-0002).
        let target = record("docs/adr/ADR-0034-x.md", "adr", "> Status: Accepted\n");
        let source = record("docs/specs/SPEC-1-y.md", "spec", "> Implements: ADR-34\n");
        let (_, findings) = pointer_resolution(&[target, source]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn normalize_id_strips_leading_zeros_but_leaves_non_numeric_ids_alone() {
        assert_eq!(normalize_id("ADR-0034"), "ADR-34");
        assert_eq!(normalize_id("ADR-34"), "ADR-34");
        assert_eq!(normalize_id("not-an-id-at-all"), "not-an-id-at-all");
    }

    #[test]
    fn a_legacy_pre_type_prefix_filename_no_longer_resolves() {
        // ADR-36's amendment (BUG-9): the pre-type-prefix `NNNN-slug.md`
        // shape was never a real external-adopter case, only this repo's own
        // pre-ADR-36 history, and that history no longer exists in the
        // corpus. A reference to a filename in that shape is now correctly
        // dangling, not silently resolved.
        let legacy_style = record("docs/adr/0037-y.md", "adr", "> Status: Accepted\n");
        let source = record("docs/specs/0001-z.md", "spec", "> Implements: ADR-37\n");
        let (_, findings) = pointer_resolution(&[legacy_style, source]);
        assert_eq!(findings.len(), 1, "unexpected findings: {findings:?}");
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn a_configured_type_prefix_resolves_independently_of_the_type_name() {
        // A type's filename/ID prefix can be configured shorter than its own
        // name (e.g. `milestone` type configured with `prefix = "MILE"`) --
        // record_id must key off the configured prefix, not a derived
        // upper-cased type name, so `Implements: MILE-1` resolves.
        let milestone = record_with_prefix(
            "docs/milestones/MILE-1-x.md",
            "milestone",
            "MILE",
            "> Status: Planned\n",
        );
        let source = record("docs/adr/ADR-1-y.md", "adr", "> Implements: MILE-1\n");
        let (_, findings) = pointer_resolution(&[milestone, source]);
        assert_eq!(findings.len(), 1, "unexpected findings: {findings:?}");
        assert!(findings[0].message.contains("resolves"));
    }

    #[test]
    fn filename_title_consistency_skips_a_legacy_shaped_filename() {
        // No type prefix means no derivable filename_number -- the rule has
        // nothing to compare a title against, so it skips rather than
        // silently accepting or falsely flagging it (BUG-9/ADR-36 amendment).
        let new_style = record(
            "docs/adr/ADR-36-x.md",
            "adr",
            "# 36 — X\n\n> Status: Accepted\n",
        );
        let legacy_style = record(
            "docs/adr/0037-y.md",
            "adr",
            "# 37 — Y\n\n> Status: Accepted\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            new_style.path.clone(),
            "# 36 — X\n\n> Status: Accepted\n".to_string(),
        );
        full_text.insert(
            legacy_style.path.clone(),
            "# 37 — Y\n\n> Status: Accepted\n".to_string(),
        );
        let (exec, findings) = filename_title_consistency(&[new_style, legacy_style], &full_text);
        assert_eq!(exec.records_examined, 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn filename_title_consistency_still_catches_a_real_mismatch_on_the_new_shape() {
        let r = record(
            "docs/adr/ADR-0036-x.md",
            "adr",
            "# 0099 — Wrong Number\n\n> Status: Accepted\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "# 0099 — Wrong Number\n\n> Status: Accepted\n".to_string(),
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("36") && findings[0].message.contains("99"));
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
        let r = record("docs/adr/0001-x.md", "adr", "> Author: @someone\n");
        let mut required = HashMap::new();
        required.insert("adr".to_string(), vec!["Author".to_string()]);

        let (_, findings) = field_quality(&[r], &required);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn field_quality_flags_a_retired_placeholder_as_an_error() {
        // BUG-5: (project lead) used to be this project's own placeholder
        // convention and classified as Present -- field.quality never had a
        // chance to catch a regression back to it (the ADR-38 incident).
        let r = record("docs/adr/0001-x.md", "adr", "> Author: (project lead)\n");
        let mut required = HashMap::new();
        required.insert("adr".to_string(), vec!["Author".to_string()]);

        let (_, findings) = field_quality(&[r], &required);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn filename_title_mismatch_is_a_planted_violation_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "# 0002 — Wrong Number\n");
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
        let r = record("docs/adr/ADR-1-x.md", "adr", "# 0001 — Correct\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0001 — Correct\n".to_string());

        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(exec.records_examined, 1);
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
        let (exec, findings) = embodiment_consistency(&[r], &HashSet::new());
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
        let (_, findings) = embodiment_consistency(&[r], &HashSet::new());
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_test_locator_outranks_a_code_locator() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(compute_embodiment(&realized, false), "Verified");
    }

    #[test]
    fn a_drifted_record_computes_to_drift_detected_regardless_of_tier() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(compute_embodiment(&realized, true), "Drift detected");
    }

    #[test]
    fn a_record_marked_drifted_produces_a_finding_even_when_verified_matches_its_tier() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Verified\n> Realized-by: code:src/lib.rs, test:tests/it.rs\n",
        );
        let mut drifted = HashSet::new();
        drifted.insert(r.path.clone());
        let (_, findings) = embodiment_consistency(&[r], &drifted);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Drift detected"));
    }

    #[test]
    fn no_realized_by_field_is_not_examined() {
        let r = record("docs/adr/0001-x.md", "adr", "> Embodiment: Not started\n");
        let (exec, findings) = embodiment_consistency(&[r], &HashSet::new());
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn realized_by_locator_paths_flattens_all_three_categories() {
        let paths =
            realized_by_locator_paths("spec:docs/rfc/0001-x.md, code:src/lib.rs, test:tests/it.rs");
        assert_eq!(
            paths,
            vec!["docs/rfc/0001-x.md", "src/lib.rs", "tests/it.rs"]
        );
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
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
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
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0002\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let (_, findings) = supersession_reciprocity(&[old, new]);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn an_em_dash_supersession_value_is_not_examined() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let (exec, findings) = supersession_reciprocity(&[r]);
        assert_eq!(exec.records_examined, 0);
        assert!(findings.is_empty());
    }
}
