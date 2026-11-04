use super::*;
use crate::config::RelationRole;
use crate::document_model::{declared_model, RequiredSection, SectionShape, SectionsDecl};
use crate::section::{parse_sections, Section};

/// `section.required` (`MILE-4`) and `section.shape` (`MILE-5`, `MILE-121`):
/// what a record type's declared `sections.required` demands of a record.
///
/// Both read the type's declared document model (`RFC-33`), not the template
/// and not the corpus: a type that declares no `required` has nothing eligible,
/// and nothing is inferred from what its records happen to contain. The
/// population's unit is the `(record, required section)` slot, so a record
/// missing a section is counted rather than silently out of the denominator.
///
/// A record whose status is in the declaration's `exempt_statuses` made no
/// decision (`MILE-7`) and is counted out of scope: no edit to its body makes
/// the rule apply. A record with no readable status is never exempt.
///
/// Presence and shape are all either rule can know; neither judges whether a
/// section says anything true.
pub fn section_required(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let (population, examined_records) = run_slots(
        records,
        config,
        full_text,
        |_| true,
        |slot, sections| {
            if find_section(sections, &slot.required.name).is_none() {
                findings.push(Finding {
                    rule: RULE_SECTION_REQUIRED.to_string(),
                    severity: FindingSeverity::Error,
                    file: slot.record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message: format!("required section `## {}` is missing", slot.required.name),
                });
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: RULE_SECTION_REQUIRED.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

pub fn section_shape(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let (population, examined_records) = run_slots(
        records,
        config,
        full_text,
        |required| required.shape.is_some(),
        |slot, sections| {
            // A missing section is `section.required`'s to report.
            let Some(section) = find_section(sections, &slot.required.name) else {
                return Outcome::Absent;
            };
            let shape = slot.required.shape.as_ref().expect("filtered on shape");
            if let Err((line, message)) = check_shape(section, shape) {
                findings.push(Finding {
                    rule: RULE_SECTION_SHAPE.to_string(),
                    severity: FindingSeverity::Error,
                    file: slot.record.path.clone(),
                    line: Some(line),
                    waived: None,
                    declared_severity: None,
                    message: format!("section `## {}`: {message}", slot.required.name),
                });
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: RULE_SECTION_SHAPE.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// One `(record, required section)` candidate.
struct Slot<'a> {
    record: &'a Record,
    required: &'a RequiredSection,
    decl: &'a SectionsDecl,
}

/// The shared census: candidates are every required section (passing `keep`)
/// of every record whose type declares any; `judge` sees the record's sections
/// once the exemption and readability gates have passed.
fn run_slots(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    keep: impl Fn(&RequiredSection) -> bool,
    mut judge: impl FnMut(&Slot, &[Section]) -> Outcome,
) -> (Population, Vec<PathBuf>) {
    // The declarations were validated at config load, so `declared_model` has
    // no error to give for a loaded config; a hand-built one with a bad
    // declaration simply declares nothing here.
    let decls: HashMap<&str, SectionsDecl> = config
        .record_types
        .iter()
        .filter_map(|(name, rt)| {
            let decl = declared_model(name, rt).ok().flatten()?.sections?;
            Some((name.as_str(), decl))
        })
        .collect();

    let mut candidates: Vec<Slot> = Vec::new();
    for record in records {
        let Some(decl) = decls.get(record.record_type.as_str()) else {
            continue;
        };
        for required in decl.required.iter().filter(|r| keep(r)) {
            candidates.push(Slot {
                record,
                required,
                decl,
            });
        }
    }

    // Parsed once per record, not once per slot.
    let mut parsed: HashMap<&PathBuf, Vec<Section>> = HashMap::new();
    census_records(
        PopulationUnit::Section,
        candidates,
        |slot| slot.record.path.clone(),
        |slot| {
            match exempt(slot.record, slot.decl, config) {
                Ok(true) => return Outcome::OutOfScope,
                Ok(false) => {}
                Err(outcome) => return outcome,
            }
            let Some(text) = full_text.get(&slot.record.path) else {
                return Outcome::Absent;
            };
            let sections = parsed
                .entry(&slot.record.path)
                .or_insert_with(|| record_sections(slot.record, text));
            judge(slot, sections)
        },
    )
}

/// Whether the record's status is a declared decisionless one. A declaration
/// with no `exempt_statuses` never reads the status; one that does and meets an
/// unparsable header cannot say, which is `Unreadable`, not "not exempt".
fn exempt(record: &Record, decl: &SectionsDecl, config: &Config) -> Result<bool, Outcome> {
    if decl.exempt_statuses.is_empty() {
        return Ok(false);
    }
    // A type with no header has no status to match (`ADR-50`, `MILE-119`):
    // the declaration contradicts it, which is the configuration's to fix.
    if config
        .record_types
        .get(&record.record_type)
        .is_some_and(|t| t.has_no_header())
    {
        return Err(Outcome::OutOfScope);
    }
    let field = relation_field_name(config, &record.record_type, RelationRole::Status);
    match record.header.read_declared(field) {
        crate::header::FieldRead::Present(value) => {
            Ok(decl.exempt_statuses.iter().any(|s| s == value.trim()))
        }
        crate::header::FieldRead::Missing => Ok(false),
        crate::header::FieldRead::Unreadable => Err(Outcome::Unreadable),
    }
}

/// The record's `##` sections with file-relative line numbers, read from
/// after the header region.
pub(super) fn record_sections(record: &Record, text: &str) -> Vec<Section> {
    let skip = record.header.region.map_or(0, |(_, end)| end);
    let body: String = text
        .split_inclusive('\n')
        .skip(skip)
        .collect::<Vec<_>>()
        .concat();
    parse_sections(&body, skip + 1)
}

fn find_section<'a>(sections: &'a [Section], name: &str) -> Option<&'a Section> {
    sections.iter().find(|s| s.heading == name)
}

/// `Err((line, message))` when `section` does not have `shape`.
fn check_shape(section: &Section, shape: &SectionShape) -> Result<(), (usize, String)> {
    match shape {
        SectionShape::YStatement { marker } => y_statement(section, marker.as_deref()),
        SectionShape::TableMinRows { min_rows } => table_min_rows(section, *min_rows),
    }
}

const Y_PHRASES: [&str; 5] = [
    "in the context of",
    "facing",
    "we decided",
    "to achieve",
    "accepting",
];

/// The section body's lines with file-relative numbers, outside fenced code.
pub(super) fn prose_lines(section: &Section) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<char> = None;
    for (i, line) in section.body.lines().enumerate() {
        let number = section.line + 1 + i;
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"]
            .iter()
            .find(|m| trimmed.starts_with(**m))
            .map(|m| m.chars().next().unwrap());
        match (fence, marker) {
            (None, Some(m)) => fence = Some(m),
            (Some(open), Some(m)) if open == m => fence = None,
            (None, None) => out.push((number, line)),
            _ => {}
        }
    }
    out
}

fn starts_list_item(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
        || t.split_once(['.', ')']).is_some_and(|(n, rest)| {
            !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && rest.starts_with(' ')
        })
}

fn y_statement(section: &Section, marker: Option<&str>) -> Result<(), (usize, String)> {
    let lines = prose_lines(section);
    // Where the block's first text sits: after the marker on its own line, or
    // at the first non-blank line when no marker is declared.
    let (start, first_text, marker_line) = match marker {
        Some(m) => {
            let needle = m.to_ascii_lowercase();
            let found = lines
                .iter()
                .enumerate()
                .find_map(|(i, (n, l))| {
                    l.to_ascii_lowercase()
                        .find(&needle)
                        .map(|at| (i, *n, l[at + m.len()..].to_string()))
                })
                .ok_or_else(|| {
                    (
                        section.line,
                        format!(
                            "declares marker `{m}` but the section has none; a Y-statement is read only from the block the marker opens"
                        ),
                    )
                })?;
            let rest = found.2.trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, ':' | '*' | '_' | '-' | '\u{2014}')
            });
            (found.0 + 1, rest.to_string(), Some(found.1))
        }
        None => {
            let i = lines
                .iter()
                .position(|(_, l)| !l.trim().is_empty())
                .unwrap_or(lines.len());
            match lines.get(i) {
                Some((_, l)) if !starts_list_item(l) && !l.trim_start().starts_with('|') => {
                    (i + 1, l.to_string(), None)
                }
                _ => (i, String::new(), None),
            }
        }
    };
    let mut block = first_text;
    for (_, l) in lines.iter().skip(start) {
        if l.trim().is_empty() || starts_list_item(l) {
            break;
        }
        block.push(' ');
        block.push_str(l);
    }
    let at = marker_line.unwrap_or(section.line);
    if block.trim().is_empty() {
        return Err((
            at,
            match marker {
                Some(m) => format!("marker `{m}` is present but no block follows it"),
                None => "has no block: the section opens with no paragraph".to_string(),
            },
        ));
    }
    let text = normalize(&block);
    let missing: Vec<&str> = Y_PHRASES
        .iter()
        .copied()
        .filter(|p| !contains_phrase(&text, p))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err((
            at,
            format!(
                "Y-statement is missing the phrase(s) {}",
                missing
                    .iter()
                    .map(|p| format!("`{p}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ))
    }
}

/// Lowercased, whitespace collapsed to single spaces.
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `phrase` in `text` bounded on both sides by a non-alphanumeric character
/// or an edge, so `facing` is not found in `interfacing`.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + phrase.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn cells(line: &str) -> Vec<&str> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(str::trim).collect()
}

fn is_delimiter_row(line: &str) -> bool {
    line.contains('-')
        && cells(line).iter().all(|c| {
            let c = c.strip_prefix(':').unwrap_or(c);
            let c = c.strip_suffix(':').unwrap_or(c);
            !c.is_empty() && c.chars().all(|ch| ch == '-')
        })
}

fn table_min_rows(section: &Section, min_rows: usize) -> Result<(), (usize, String)> {
    let lines = prose_lines(section);
    // `(header line, data rows)` of the largest table so far.
    let mut best: Option<(usize, usize)> = None;
    let mut i = 0;
    while i + 1 < lines.len() {
        let (n, head) = lines[i];
        if head.contains('|') && is_delimiter_row(lines[i + 1].1) {
            let mut rows = 0;
            i += 2;
            while i < lines.len() && lines[i].1.contains('|') && !lines[i].1.trim().is_empty() {
                if cells(lines[i].1).iter().any(|c| !c.is_empty()) {
                    rows += 1;
                }
                i += 1;
            }
            if best.is_none_or(|(_, r)| rows > r) {
                best = Some((n, rows));
            }
        } else {
            i += 1;
        }
    }
    match best {
        None => Err((
            section.line,
            format!("has no table; at least {min_rows} data row(s) are required"),
        )),
        Some((_, rows)) if rows >= min_rows => Ok(()),
        Some((n, rows)) => Err((
            n,
            format!("the table has {rows} data row(s); at least {min_rows} are required"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    const Y: &str = "In the context of a, facing b, we decided c, to achieve d, accepting e.";

    fn doc(status: &str, body: &str) -> String {
        format!("> Status: {status}\n\n# 1 — X\n\n{body}")
    }

    fn config(sections_yaml: &str) -> Config {
        let mut t = type_config(None);
        t.sections = Some(yaml_serde::from_str(sections_yaml).unwrap());
        config_with_types(vec![("adr", t)])
    }

    type RuleFn =
        fn(&[Record], &Config, &HashMap<PathBuf, String>) -> (RuleExecution, Vec<Finding>);

    fn run(f: RuleFn, sections_yaml: &str, content: &str) -> (RuleExecution, Vec<Finding>) {
        let r = record("docs/adr/ADR-1-x.md", "adr", content);
        let text = HashMap::from([(PathBuf::from("docs/adr/ADR-1-x.md"), content.to_string())]);
        f(&[r], &config(sections_yaml), &text)
    }

    fn counts(e: &RuleExecution) -> (usize, usize) {
        let p = e.population.as_ref().expect("population");
        (p.eligible(), p.examined())
    }

    const REQ: &str = "{from: headings, required: [{name: Context}, {name: Decision}], exempt_statuses: [Withdrawn]}";

    #[test]
    fn a_missing_required_section_is_a_finding_naming_it_and_every_slot_is_counted() {
        let (e, f) = run(section_required, REQ, &doc("Accepted", "## Context\n\nc\n"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Decision"), "{}", f[0].message);
        assert_eq!(counts(&e), (2, 2));
        assert_eq!(
            e.population.as_ref().unwrap().unit(),
            PopulationUnit::Section
        );
    }

    #[test]
    fn a_record_with_every_required_section_is_clean() {
        let (e, f) = run(
            section_required,
            REQ,
            &doc("Accepted", "## Context\n\nc\n\n## Decision\n\nd\n"),
        );
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (2, 2));
    }

    #[test]
    fn a_heading_inside_a_code_fence_is_not_a_section() {
        let body = "## Context\n\n```\n## Decision\n```\n";
        let (_, f) = run(section_required, REQ, &doc("Accepted", body));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn a_type_declaring_sections_without_required_has_nothing_eligible() {
        let (e, f) = run(
            section_required,
            "{from: headings}",
            &doc("Accepted", "## Other\n"),
        );
        assert!(f.is_empty());
        assert_eq!(counts(&e), (0, 0));
    }

    #[test]
    fn a_type_declaring_no_sections_behaves_as_today() {
        let r = record("docs/adr/ADR-1-x.md", "adr", &doc("Accepted", ""));
        let cfg = config_with_types(vec![("adr", type_config(None))]);
        let (e, f) = section_required(&[r], &cfg, &HashMap::new());
        assert!(f.is_empty());
        assert_eq!(counts(&e), (0, 0));
    }

    #[test]
    fn a_decisionless_status_is_exempt_and_counted_out_of_scope_not_examined() {
        let (e, f) = run(section_required, REQ, &doc("Withdrawn", "## Other\n"));
        assert!(f.is_empty(), "{f:?}");
        let p = e.population.as_ref().unwrap();
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (2, 0, 2));
        let (_, f) = run(section_required, REQ, &doc("Accepted", "## Other\n"));
        assert_eq!(f.len(), 2, "the same body under a live status is flagged");
    }

    #[test]
    fn a_record_with_no_status_is_not_exempt() {
        let (_, f) = run(
            section_required,
            REQ,
            "> Author: a\n\n# 1 — X\n\n## Other\n",
        );
        assert_eq!(f.len(), 2, "{f:?}");
    }

    #[test]
    fn a_header_none_type_declaring_exempt_statuses_is_out_of_scope_not_unreadable() {
        let mut t = type_config(None);
        t.header_shape = crate::header::HeaderShape::None;
        t.sections = Some(yaml_serde::from_str(REQ).unwrap());
        let cfg = config_with_types(vec![("adr", t)]);
        let content = "# 1 — X\n\n## Other\n";
        let r = record("docs/adr/ADR-1-x.md", "adr", content);
        let text = HashMap::from([(PathBuf::from("docs/adr/ADR-1-x.md"), content.to_string())]);
        let (e, f) = section_required(&[r], &cfg, &text);
        assert!(f.is_empty(), "{f:?}");
        let p = e.population.as_ref().unwrap();
        assert_eq!((p.unreadable(), p.out_of_scope()), (0, 2));
    }

    #[test]
    fn an_unreadable_header_cannot_be_judged_exempt_and_is_counted_unreadable() {
        let (e, f) = run(section_required, REQ, "# 1 — X\n\n## Other\n");
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(e.population.as_ref().unwrap().unreadable(), 2);
    }

    const Y_DECL: &str = "{from: headings, required: [{name: Decision, shape: y-statement, marker: 'TL;DR'}], exempt_statuses: [Withdrawn]}";

    fn decision(body: &str) -> String {
        doc("Accepted", &format!("## Decision\n\n{body}"))
    }

    #[test]
    fn a_complete_y_statement_after_the_marker_passes() {
        let (e, f) = run(
            section_shape,
            Y_DECL,
            &decision(&format!("**TL;DR:** {Y}\n")),
        );
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 1));
    }

    #[test]
    fn a_wrapped_y_statement_is_one_block() {
        let body =
            "TL;DR: In the context of a, facing b,\nwe decided c, to achieve d,\naccepting e.\n";
        let (_, f) = run(section_shape, Y_DECL, &decision(body));
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_missing_phrase_is_named() {
        let body = "TL;DR: In the context of a, facing b, we decided c, to achieve d.\n";
        let (_, f) = run(section_shape, Y_DECL, &decision(body));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("accepting"), "{}", f[0].message);
        assert!(!f[0].message.contains("facing"), "{}", f[0].message);
    }

    #[test]
    fn phrases_outside_the_block_do_not_count() {
        let body = format!("TL;DR: In the context of a, facing b.\n\nLater: {Y}\n");
        let (_, f) = run(section_shape, Y_DECL, &decision(&body));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("we decided"), "{}", f[0].message);
    }

    #[test]
    fn a_list_item_ends_the_block() {
        let body =
            "TL;DR: In the context of a, facing b, we decided c, to achieve d.\n- accepting e\n";
        let (_, f) = run(section_shape, Y_DECL, &decision(body));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("accepting"), "{}", f[0].message);
    }

    #[test]
    fn a_marker_with_no_block_is_its_own_error() {
        let (_, f) = run(section_shape, Y_DECL, &decision("TL;DR:\n\n"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("no block"), "{}", f[0].message);
        assert!(!f[0].message.contains("missing"), "{}", f[0].message);
    }

    #[test]
    fn a_declared_marker_the_section_lacks_is_a_finding_not_a_whole_section_scan() {
        let (_, f) = run(section_shape, Y_DECL, &decision(&format!("{Y}\n")));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("TL;DR"), "{}", f[0].message);
    }

    #[test]
    fn matching_is_on_word_boundaries() {
        let body =
            "TL;DR: In the context of a, interfacing b, we decided c, to achieve d, accepting e.\n";
        let (_, f) = run(section_shape, Y_DECL, &decision(body));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("facing"), "{}", f[0].message);
    }

    #[test]
    fn without_a_declared_marker_the_first_paragraph_is_the_block() {
        let decl = "{from: headings, required: [{name: Decision, shape: y-statement}]}";
        let (_, f) = run(section_shape, decl, &decision(&format!("{Y}\n\nmore\n")));
        assert!(f.is_empty(), "{f:?}");
        let (_, f) = run(section_shape, decl, &decision(&format!("Intro.\n\n{Y}\n")));
        assert_eq!(
            f.len(),
            1,
            "a Y-statement in a later paragraph is not the first"
        );
    }

    #[test]
    fn a_missing_section_is_absent_for_shape_and_reported_by_required_only() {
        let (e, f) = run(section_shape, Y_DECL, &doc("Accepted", "## Other\n"));
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 0));
        assert_eq!(e.population.as_ref().unwrap().out_of_scope(), 0);
    }

    #[test]
    fn shape_skips_an_exempt_record() {
        let body = "## Decision\n\nnothing\n";
        let (e, f) = run(section_shape, Y_DECL, &doc("Withdrawn", body));
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(e.population.as_ref().unwrap().out_of_scope(), 1);
    }

    const T_DECL: &str =
        "{from: headings, required: [{name: Options, shape: table-min-rows, min_rows: 2}]}";

    fn options(body: &str) -> String {
        doc("Accepted", &format!("## Options\n\n{body}"))
    }

    const HEAD: &str = "| Option | Pros |\n|---|---|\n";

    #[test]
    fn two_data_rows_pass() {
        let body = format!("{HEAD}| A | x |\n| B | y |\n");
        let (e, f) = run(section_shape, T_DECL, &options(&body));
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 1));
    }

    #[test]
    fn one_data_row_fails_and_says_how_many() {
        let body = format!("{HEAD}| A | x |\n");
        let (_, f) = run(section_shape, T_DECL, &options(&body));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("1 data row"), "{}", f[0].message);
        assert!(f[0].message.contains("2"), "{}", f[0].message);
    }

    #[test]
    fn a_header_only_table_and_no_table_are_both_findings() {
        let (_, f) = run(section_shape, T_DECL, &options(HEAD));
        assert_eq!(f.len(), 1, "{f:?}");
        let (_, f) = run(section_shape, T_DECL, &options("prose only\n"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("no table"), "{}", f[0].message);
    }

    #[test]
    fn an_all_empty_row_is_not_a_data_row() {
        let body = format!("{HEAD}| A | x |\n|  |  |\n");
        let (_, f) = run(section_shape, T_DECL, &options(&body));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn a_table_inside_a_code_fence_is_not_a_table() {
        let body = format!("```\n{HEAD}| A | x |\n| B | y |\n```\n");
        let (_, f) = run(section_shape, T_DECL, &options(&body));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn the_larger_of_two_tables_decides() {
        let body = format!("{HEAD}| A | x |\n\n{HEAD}| A | x |\n| B | y |\n");
        let (_, f) = run(section_shape, T_DECL, &options(&body));
        assert!(f.is_empty(), "{f:?}");
    }
}
