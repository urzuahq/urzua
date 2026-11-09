use super::*;
use crate::config::RelationRole;
use crate::document_model::{declared_model, RequiredSection, SectionShape, SectionsDecl};
use crate::section::{parse_sections, parse_sections_to_depth, Section};

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
/// decision (`MILE-7`) and is counted eligible and unexamined: set aside by a
/// declaration, so the configuration did reach it (`BUG-187`). A record with no readable status is never exempt.
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
            if find_section(sections, slot.required).is_none()
                && !preamble_carries_marker(slot.required, &slot.preamble(sections))
            {
                findings.push(Finding::new(
                    RULE_SECTION_REQUIRED,
                    FindingSeverity::Error,
                    slot.record.path.clone(),
                    ActionSource::new("section-missing").section(slot.required.name.as_str()),
                    if slot.required.aliases.is_empty() {
                        format!(
                            "required section {} is missing",
                            label(slot.required, &slot.required.name)
                        )
                    } else {
                        let aliases: Vec<String> = slot
                            .required
                            .aliases
                            .iter()
                            .map(|a| label(slot.required, a))
                            .collect();
                        format!(
                            "required section {} is missing, and so is every alias ({})",
                            label(slot.required, &slot.required.name),
                            aliases.join(", ")
                        )
                    },
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_SECTION_REQUIRED, population, examined_records),
        findings,
    )
}

/// The exemptions a configuration declares that no record uses (`MILE-7`): a
/// status listed in a type's `sections.exempt_statuses` that none of the type's
/// records holds sets nothing aside, so the declaration silently does nothing
/// (a typo, a status that was renamed or never used). One finding per such
/// status, on the configuration file, naming the type and the status.
///
/// A type with no records is `type.dir-matches-nothing`'s to report, and a
/// header-less type has no status to match (its records are counted out of
/// scope by `section.required` itself). Reported under `type.dir-matches-nothing`,
/// the rule for a declaration that silently does nothing. A record whose header cannot be read
/// holds no status that could match.
pub fn section_exemptions_matching_nothing(
    records: &[Record],
    config: &Config,
    config_path: &std::path::Path,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (name, rt) in &config.record_types {
        let Some(decl) = declared_model(name, rt)
            .ok()
            .flatten()
            .and_then(|m| m.sections)
        else {
            continue;
        };
        if decl.exempt_statuses.is_empty() || rt.has_no_header() {
            continue;
        }
        let field = relation_field_name(config, name, RelationRole::Status);
        let mut held: std::collections::BTreeSet<String> = Default::default();
        let mut any = false;
        for record in records.iter().filter(|r| &r.record_type == name) {
            any = true;
            if let crate::header::FieldRead::Present(value) = record.header.read_declared(field) {
                held.insert(value.trim().to_string());
            }
        }
        if !any {
            continue;
        }
        for status in decl.exempt_statuses.iter().filter(|s| !held.contains(*s)) {
            findings.push(Finding::new(
                RULE_TYPE_DIR_MATCHES_NOTHING,
                FindingSeverity::Warning,
                config_path.to_path_buf(),
                ActionSource::new("exempt-status-matches-nothing")
                    .key_path(["record_types", name.as_str(), "sections", "exempt_statuses"]),
                format!(
                    "record type '{name}' declares `sections.exempt_statuses` entry '{status}', but \
                     no '{name}' record has that status -- the exemption sets nothing aside; \
                     remove it or correct the status"
                ),
            ));
        }
    }
    findings
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
            // A missing section is `section.required`'s to report, unless the
            // declaration also reads the preamble and the block's marker is
            // there: then the preamble is judged by the same shape check.
            let preamble;
            let (section, outside) = match find_section(sections, slot.required) {
                Some((section, _)) => (section, false),
                None => {
                    preamble = slot.preamble(sections);
                    if !preamble_carries_marker(slot.required, &preamble) {
                        return Outcome::Absent;
                    }
                    (&preamble, true)
                }
            };
            let shape = slot.required.shape.as_ref().expect("filtered on shape");
            if let Err((line, message)) = check_shape(section, shape) {
                findings.push(Finding::new(
                    RULE_SECTION_SHAPE,
                    FindingSeverity::Error,
                    slot.record.path.clone(),
                    ActionSource::new("section-shape").section(slot.required.name.as_str()),
                    if outside {
                        format!(
                            "the preamble (before the first `##` section, where {} is absent): {message}",
                            label(slot.required, &slot.required.name)
                        )
                    } else {
                        format!(
                            "section {}: {message}",
                            label(slot.required, &slot.required.name)
                        )
                    },
                )
                .at(line));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_SECTION_SHAPE, population, examined_records),
        findings,
    )
}

/// A required section as the messages name it: the heading at its level, and
/// the parent it must sit under when declared, e.g. `### Pros` under `## Options`.
fn label(required: &RequiredSection, heading: &str) -> String {
    let hashes = "#".repeat(usize::from(required.level));
    match &required.under {
        None => format!("`{hashes} {heading}`"),
        Some(under) => format!(
            "`{hashes} {heading}` under `{} {under}`",
            "#".repeat(usize::from(required.level) - 1)
        ),
    }
}

/// One `(record, required section)` candidate.
struct Slot<'a> {
    record: &'a Record,
    required: &'a RequiredSection,
    decl: &'a SectionsDecl,
    /// The record's full text, to read the preamble from.
    text: &'a str,
    /// Lines of the header region.
    header_end: usize,
}

impl Slot<'_> {
    /// The text before the first `##` heading, as a section with no heading.
    fn preamble(&self, sections: &[Section]) -> Section {
        preamble_of(self.text, self.header_end, sections)
    }
}

/// The text between the header region and the first section.
fn preamble_of(text: &str, header_end: usize, sections: &[Section]) -> Section {
    let end = sections
        .iter()
        .map(|s| s.line)
        .min()
        .map_or(usize::MAX, |line| line - 1);
    let lines: Vec<&str> = text
        .lines()
        .enumerate()
        .skip(header_end)
        .take_while(|(i, _)| *i < end)
        .map(|(_, l)| l)
        .collect();
    Section {
        heading: String::new(),
        level: 2,
        parent: None,
        line: header_end,
        end_line: header_end + lines.len(),
        body: lines.iter().map(|l| format!("{l}\n")).collect(),
    }
}

/// One section of a record as `export` hands it over: the declared name
/// (an alias resolves to the `required` name it satisfies, `MILE-165`), the
/// heading as written, and the parsed span and content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSection {
    pub name: String,
    pub heading: String,
    pub line: usize,
    pub end_line: usize,
    pub content: String,
}

/// The sections and preamble of a record whose type declares `sections`,
/// read by the same parser `section.required` uses; `None` when the type
/// declares none (so the caller can say "no declaration" rather than
/// "no sections").
pub fn exported_sections(
    record: &Record,
    record_type: &crate::config::RecordTypeConfig,
    text: &str,
) -> Option<(Vec<ExportedSection>, String)> {
    let decl = declared_model(&record.record_type, record_type)
        .ok()
        .flatten()?
        .sections?;
    let header_end = record.header.region.map_or(0, |(_, end)| end);
    let sections = record_sections(record, text);
    let preamble = preamble_of(text, header_end, &sections).body;
    let exported = sections
        .into_iter()
        .map(|s| {
            let name = decl
                .required
                .iter()
                .find(|r| r.level == s.level && r.headings().any(|h| h == s.heading))
                .map_or_else(|| s.heading.clone(), |r| r.name.clone());
            ExportedSection {
                name,
                heading: s.heading,
                line: s.line,
                end_line: s.end_line,
                content: s.body,
            }
        })
        .collect();
    Some((exported, preamble))
}

/// Whether the declaration reads the preamble and the block's marker sits in
/// it (case-insensitively, as the shape check finds it).
fn preamble_carries_marker(required: &RequiredSection, preamble: &Section) -> bool {
    let Some(SectionShape::YStatement { marker: Some(m) }) = &required.shape else {
        return false;
    };
    required.preamble
        && prose_lines(preamble).iter().any(|(_, l)| {
            strip_quote(l)
                .to_ascii_lowercase()
                .contains(&m.to_ascii_lowercase())
        })
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
        let text = full_text
            .get(&record.path)
            .map(String::as_str)
            .unwrap_or_default();
        let header_end = record.header.region.map_or(0, |(_, end)| end);
        for required in decl.required.iter().filter(|r| keep(r)) {
            candidates.push(Slot {
                record,
                required,
                decl,
                text,
                header_end,
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
                // A declared exemption sets the record aside on purpose: eligible
                // and unexamined, not out of scope, which
                // `config.scope-matches-nothing` reads as a configuration that
                // reaches nothing (`BUG-187`, as `BUG-186`).
                Ok(true) => return Outcome::Absent,
                Ok(false) => {}
                Err(outcome) => return outcome,
            }
            let Some(text) = full_text.get(&slot.record.path) else {
                return Outcome::Absent;
            };
            let sections = parsed
                .entry(&slot.record.path)
                .or_insert_with(|| record_section_tree(slot.record, text, slot.decl.depth));
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
    parse_sections(&after_header(text, skip), skip + 1)
}

/// The record's sections down to the type's declared `depth`, nested ones
/// included, with file-relative line numbers.
pub(super) fn record_section_tree(record: &Record, text: &str, depth: u8) -> Vec<Section> {
    let skip = record.header.region.map_or(0, |(_, end)| end);
    parse_sections_to_depth(&after_header(text, skip), skip + 1, depth)
}

fn after_header(text: &str, skip: usize) -> String {
    text.split_inclusive('\n')
        .skip(skip)
        .collect::<Vec<_>>()
        .concat()
}

/// The section satisfying `required`, with the heading it was found under:
/// the `name` first, then each alias in the order declared.
pub(super) fn find_section<'a>(
    sections: &'a [Section],
    required: &'a RequiredSection,
) -> Option<(&'a Section, &'a str)> {
    required.headings().find_map(|h| {
        sections
            .iter()
            .find(|s| {
                s.heading == h
                    && s.level == required.level
                    && required
                        .under
                        .as_deref()
                        .is_none_or(|under| s.parent.is_some_and(|p| sections[p].heading == under))
            })
            .map(|section| (section, h))
    })
}

/// One `Info` notice per `(required section, alias)` that at least one record
/// satisfied through the alias, listing those records (`MILE-165`). Never a
/// finding: an alias is a declared equivalent, but a repository should see
/// which records use one. Same scope as `section.required` (exempt, unreadable
/// and header-less records are not read).
pub fn section_alias_notices(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> Vec<Notice> {
    let mut used: std::collections::BTreeMap<(String, String, String), Vec<String>> =
        Default::default();
    let _ = run_slots(
        records,
        config,
        full_text,
        |required| !required.aliases.is_empty(),
        |slot, sections| {
            if let Some((_, heading)) = find_section(sections, slot.required) {
                if heading != slot.required.name {
                    used.entry((
                        slot.record.record_type.clone(),
                        label(slot.required, &slot.required.name),
                        label(slot.required, heading),
                    ))
                    .or_default()
                    .push(slot.record.path.display().to_string());
                }
            }
            Outcome::Examined
        },
    );
    used.into_iter()
        .map(|((record_type, name, alias), mut paths)| {
            paths.sort();
            Notice {
                severity: NoticeSeverity::Info,
                subject: RULE_SECTION_REQUIRED.to_string(),
                message: format!(
                    "{} {record_type} record(s) satisfy required section {name} through its alias {alias}: {}",
                    paths.len(),
                    paths.join(", ")
                ),
                detail: None,
            }
        })
        .collect()
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

fn strip_quote(line: &str) -> &str {
    let t = line.trim_start();
    match t.strip_prefix('>') {
        Some(rest) => rest.strip_prefix(' ').unwrap_or(rest),
        None => line,
    }
}

fn y_statement(section: &Section, marker: Option<&str>) -> Result<(), (usize, String)> {
    // A blockquoted block keeps its `>` on every line; the marker is not part
    // of the prose, and left in it splits a phrase that wraps (BUG-181).
    let lines: Vec<(usize, &str)> = prose_lines(section)
        .into_iter()
        .map(|(n, l)| (n, strip_quote(l)))
        .collect();
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

pub(crate) fn is_delimiter_row(line: &str) -> bool {
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
        format!("---\nStatus: {status}\n---\n# 1 — X\n\n{body}")
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
    fn a_decisionless_status_is_exempt_and_counted_eligible_and_unexamined_observed_failing() {
        let (e, f) = run(section_required, REQ, &doc("Withdrawn", "## Other\n"));
        assert!(f.is_empty(), "{f:?}");
        let p = e.population.as_ref().unwrap();
        // Set aside on purpose, so not out of scope (`BUG-187`).
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (2, 0, 0));
        let (_, scope) = crate::rules::config_scope_matches_nothing(
            &[e],
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(scope.is_empty(), "{scope:?}");
        let (_, f) = run(section_required, REQ, &doc("Accepted", "## Other\n"));
        assert_eq!(f.len(), 2, "the same body under a live status is flagged");
    }

    #[test]
    fn an_exemption_that_matches_no_record_is_a_finding_observed_failing() {
        // `Withdrawn` is declared but no record holds it: the exemption does
        // nothing, which must be said rather than left silent (`MILE-7`).
        let r = record("docs/adr/ADR-1-x.md", "adr", &doc("Accepted", ""));
        let f = section_exemptions_matching_nothing(
            &[r],
            &config(REQ),
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].rule, RULE_TYPE_DIR_MATCHES_NOTHING);
        assert!(f[0].file.ends_with(".urzua/config.yaml"));
        assert!(
            f[0].message.contains("Withdrawn") && f[0].message.contains("adr"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn an_exemption_a_record_holds_is_not_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", &doc("Withdrawn", ""));
        let f = section_exemptions_matching_nothing(
            &[r],
            &config(REQ),
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn only_the_unmatched_status_of_several_is_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", &doc("Withdrawn", ""));
        let f = section_exemptions_matching_nothing(
            &[r],
            &config("{from: headings, required: [{name: Context}], exempt_statuses: [Withdrawn, Retired]}"),
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Retired"), "{}", f[0].message);
    }

    #[test]
    fn a_record_with_no_status_is_not_exempt() {
        let (_, f) = run(
            section_required,
            REQ,
            "---\nAuthor: a\n---\n# 1 — X\n\n## Other\n",
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
    fn a_phrase_wrapped_across_blockquote_lines_is_found() {
        let body =
            "> TL;DR: In the context of a, facing b, we decided c, to\n> achieve d, accepting e.\n";
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
        let p = e.population.as_ref().unwrap();
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (1, 0, 0));
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

    const ALIASED: &str = "{from: headings, required: [{name: Decision, aliases: [Resolution]}]}";

    #[test]
    fn a_declared_alias_satisfies_the_requirement_and_is_disclosed_observed_failing() {
        let content = doc("Accepted", "## Resolution\n\nd\n");
        let (e, f) = run(section_required, ALIASED, &content);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 1));

        let r = record("docs/adr/ADR-1-x.md", "adr", &content);
        let text = HashMap::from([(PathBuf::from("docs/adr/ADR-1-x.md"), content.clone())]);
        let notices = section_alias_notices(&[r], &config(ALIASED), &text);
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(notices[0].message.contains("`## Decision`"));
        assert!(notices[0].message.contains("`## Resolution`"));
        assert!(notices[0].message.contains("docs/adr/ADR-1-x.md"));
    }

    #[test]
    fn the_name_wins_over_an_alias_and_neither_is_a_finding_naming_both() {
        let both = doc("Accepted", "## Resolution\n\nr\n\n## Decision\n\nd\n");
        let r = record("docs/adr/ADR-1-x.md", "adr", &both);
        let text = HashMap::from([(PathBuf::from("docs/adr/ADR-1-x.md"), both.clone())]);
        assert!(section_alias_notices(&[r], &config(ALIASED), &text).is_empty());

        let (_, f) = run(
            section_required,
            ALIASED,
            &doc("Accepted", "## Context\n\nc\n"),
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Decision") && f[0].message.contains("Resolution"));
    }

    #[test]
    fn the_shape_check_follows_an_alias() {
        let decl = "{from: headings, required: [{name: Decision, aliases: [Resolution], shape: y-statement}]}";
        let ok = doc("Accepted", &format!("## Resolution\n\n{Y}\n"));
        let (e, f) = run(section_shape, decl, &ok);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 1));
        let bad = doc("Accepted", "## Resolution\n\nNot a y-statement.\n");
        let (_, f) = run(section_shape, decl, &bad);
        assert_eq!(f.len(), 1, "{f:?}");
    }

    const PRE: &str = "{from: headings, required: [{name: Decision, shape: y-statement, marker: 'Y-statement:', preamble: true}]}";

    #[test]
    fn a_y_statement_in_the_preamble_is_judged_by_the_same_shape_check_observed_failing() {
        let good = doc(
            "Accepted",
            &format!("Y-statement: {Y}\n\n## Context\n\nc\n"),
        );
        let (_, f) = run(section_required, PRE, &good);
        assert!(f.is_empty(), "{f:?}");
        let (e, f) = run(section_shape, PRE, &good);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (1, 1));

        let bad = doc(
            "Accepted",
            "Y-statement: In the context of a, we decided c.\n\n## Context\n\nc\n",
        );
        let (_, f) = run(section_shape, PRE, &bad);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("preamble"), "{}", f[0].message);
        assert_eq!(f[0].line, Some(6));
    }

    #[test]
    fn the_preamble_is_read_only_when_declared_and_only_when_the_marker_is_there() {
        // Declared but no marker in the preamble: still a missing section.
        let none = doc("Accepted", "Just prose.\n\n## Context\n\nc\n");
        let (_, f) = run(section_required, PRE, &none);
        assert_eq!(f.len(), 1, "{f:?}");
        // Marker in the preamble but `preamble` not declared: missing.
        let not_declared = "{from: headings, required: [{name: Decision, shape: y-statement, marker: 'Y-statement:'}]}";
        let content = doc(
            "Accepted",
            &format!("Y-statement: {Y}\n\n## Context\n\nc\n"),
        );
        let (_, f) = run(section_required, not_declared, &content);
        assert_eq!(f.len(), 1, "{f:?}");
        // A marker after the first `##` heading is not the preamble.
        let late = doc("Accepted", &format!("## Context\n\nY-statement: {Y}\n"));
        let (_, f) = run(section_required, PRE, &late);
        assert_eq!(f.len(), 1, "{f:?}");
    }

    const NESTED: &str = "{from: headings, depth: 2, required: [{name: Options}, {name: 'Pros and cons', level: 3, under: Options}]}";

    #[test]
    fn a_required_h3_under_its_h2_is_found_and_counted_observed_failing() {
        let body = "## Options\n\n### Pros and cons\n\nx\n";
        let (e, f) = run(section_required, NESTED, &doc("Accepted", body));
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(counts(&e), (2, 2));
    }

    #[test]
    fn a_missing_required_h3_is_a_finding_naming_its_level_and_parent() {
        let (_, f) = run(
            section_required,
            NESTED,
            &doc("Accepted", "## Options\n\nplain\n"),
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].message.contains("### Pros and cons") && f[0].message.contains("## Options"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn an_h3_of_the_right_name_under_the_wrong_h2_does_not_satisfy_under() {
        let body = "## Options\n\n## Other\n\n### Pros and cons\n";
        let (_, f) = run(section_required, NESTED, &doc("Accepted", body));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn an_h2_of_the_same_name_does_not_satisfy_an_h3_requirement_and_the_reverse() {
        let (_, f) = run(
            section_required,
            NESTED,
            &doc("Accepted", "## Options\n\n## Pros and cons\n"),
        );
        assert_eq!(f.len(), 1, "{f:?}");
        let (_, f) = run(
            section_required,
            "{from: headings, depth: 2, required: [{name: Context}]}",
            &doc("Accepted", "### Context\n"),
        );
        assert_eq!(
            f.len(),
            1,
            "an h3 does not satisfy a level-2 requirement: {f:?}"
        );
    }

    #[test]
    fn a_depth_two_declaration_with_only_h2_requirements_behaves_as_depth_one() {
        let body = "## Context\n\n### Sub\n\n## Decision\n";
        let (e1, f1) = run(section_required, REQ, &doc("Accepted", body));
        let two = REQ.replace("from: headings,", "from: headings, depth: 2,");
        let (e2, f2) = run(section_required, &two, &doc("Accepted", body));
        assert_eq!((f1.len(), counts(&e1)), (f2.len(), counts(&e2)));
    }

    #[test]
    fn a_required_h3_can_carry_a_shape_and_it_is_judged_inside_the_h3() {
        let decl = "{from: headings, depth: 2, required: [{name: Rows, level: 3, under: Options, shape: table-min-rows, min_rows: 1}]}";
        let ok = "## Options\n\n### Rows\n\n| a |\n|---|\n| 1 |\n";
        let (_, f) = run(section_shape, decl, &doc("Accepted", ok));
        assert!(f.is_empty(), "{f:?}");
        let bad = "## Options\n\n### Rows\n\nno table\n\n## Later\n\n| a |\n|---|\n| 1 |\n";
        let (_, f) = run(section_shape, decl, &doc("Accepted", bad));
        assert_eq!(f.len(), 1, "{f:?}");
    }
}
