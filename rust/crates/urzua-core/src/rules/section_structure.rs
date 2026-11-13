use super::section::record_section_tree;
use super::*;
use crate::document_model::{declared_model, OrderSource, SectionsDecl};
use crate::section::{parse_sections_to_depth, Section};
use std::collections::BTreeMap;

/// `section.duplicate`, `section.level-skipped` and `section.order` (`MILE-104`):
/// the *structure* of a record's sections, as against `section.required`'s
/// policy on which sections a type wants. Each is opt-in per type through one
/// key of its `sections` declaration (`unique_headings`, `no_skipped_levels`,
/// `order`), reads the tree down to the type's declared `depth`, and judges
/// whether the document is well-formed, never what a section says.
///
/// A record's `exempt_statuses` do not set it aside: a duplicated section is a
/// broken document whatever the record decided. The population's unit is the
/// record, counted for every record of a type that declares the key.
pub fn section_duplicate(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let (population, examined) = structure_census(
        records,
        config,
        full_text,
        |d| d.unique_headings,
        |record, _, sections| {
            let mut first: HashMap<(Option<usize>, u8, &str), usize> = HashMap::new();
            for s in sections.iter().filter(|s| !s.heading.is_empty()) {
                let key = (s.parent, s.level, s.heading.as_str());
                match first.get(&key) {
                    Some(&line) => findings.push(
                        Finding::new(
                            RULE_SECTION_DUPLICATE,
                            FindingSeverity::Error,
                            record.path.clone(),
                            ActionSource::new("section-duplicate").section(s.heading.as_str()),
                            format!(
                                "section {} appears twice: first at line {line}, again at line {}",
                                heading_label(s.level, &s.heading),
                                s.line
                            ),
                        )
                        .at(s.line),
                    ),
                    None => {
                        first.insert(key, s.line);
                    }
                }
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_SECTION_DUPLICATE, population, examined),
        findings,
    )
}

pub fn section_level_skipped(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let (population, examined) = structure_census(
        records,
        config,
        full_text,
        |d| d.no_skipped_levels,
        |record, _, sections| {
            for s in sections {
                let (above, expected) = match s.parent {
                    Some(p) => (
                        format!(
                            "under {} (level {})",
                            heading_label(sections[p].level, &sections[p].heading),
                            sections[p].level
                        ),
                        sections[p].level + 1,
                    ),
                    None => ("with no `##` section above it".to_string(), 2),
                };
                if s.level != expected {
                    findings.push(
                        Finding::new(
                            RULE_SECTION_LEVEL_SKIPPED,
                            FindingSeverity::Error,
                            record.path.clone(),
                            ActionSource::new("section-level-skipped")
                                .section(s.heading.as_str()),
                            format!(
                                "section {} (level {}) skips a level: it sits {above}, so a heading of level {expected} was expected",
                                heading_label(s.level, &s.heading),
                                s.level
                            ),
                        )
                        .at(s.line),
                    );
                }
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_SECTION_LEVEL_SKIPPED, population, examined),
        findings,
    )
}

pub fn section_order(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    template_texts: &HashMap<String, String>,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let (population, examined) = structure_census(
        records,
        config,
        full_text,
        |d| d.order.is_some_and(|o| o.from == OrderSource::Template),
        |record, decl, sections| {
            let Some(template) = template_texts.get(&record.record_type) else {
                return Outcome::Absent;
            };
            let expected = sibling_groups(&parse_sections_to_depth(template, 1, decl.depth));
            if expected.is_empty() {
                return Outcome::Absent;
            }
            for (parent, listed) in sibling_groups(sections) {
                let Some(wanted) = expected.get(&parent) else {
                    continue;
                };
                let rank = |heading: &str| wanted.iter().position(|(h, _, _)| h == heading);
                let mut latest: Option<(usize, &str)> = None;
                for (heading, line, level) in &listed {
                    let Some(r) = rank(heading) else { continue };
                    match latest {
                        Some((best, after)) if r < best => findings.push(
                            Finding::new(
                                RULE_SECTION_ORDER,
                                FindingSeverity::Error,
                                record.path.clone(),
                                ActionSource::new("section-order").section(heading.as_str()),
                                format!(
                                    "section {} comes after {}, but the template puts it before",
                                    heading_label(*level, heading),
                                    heading_label(*level, after)
                                ),
                            )
                            .at(*line),
                        ),
                        Some((best, _)) if r == best => {}
                        _ => latest = Some((r, heading.as_str())),
                    }
                }
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_SECTION_ORDER, population, examined),
        findings,
    )
}

/// The types whose `sections.order` reads a template, with the `depth` the
/// template is read to: the types for which the caller must load
/// `.urzua/templates/<type>.md`.
pub fn types_reading_a_template_order(config: &Config) -> Vec<(String, u8)> {
    config
        .record_types
        .iter()
        .filter_map(|(name, rt)| {
            let decl = declared_model(name, rt).ok().flatten()?.sections?;
            decl.order
                .is_some_and(|o| o.from == OrderSource::Template)
                .then(|| (name.clone(), decl.depth))
        })
        .collect()
}

/// Whether a template holds a section at the given `depth`; one with none
/// gives `section.order` nothing to compare against.
pub fn template_has_sections(template: &str, depth: u8) -> bool {
    !parse_sections_to_depth(template, 1, depth).is_empty()
}

/// A template's section tree as `parent heading -> [(heading, line, level)]`,
/// the sibling lists order is judged within. A parentless section's key is `None`.
fn sibling_groups(sections: &[Section]) -> BTreeMap<Option<String>, Vec<(String, usize, u8)>> {
    let mut groups: BTreeMap<Option<String>, Vec<(String, usize, u8)>> = BTreeMap::new();
    for s in sections {
        let parent = s.parent.map(|p| sections[p].heading.clone());
        groups
            .entry(parent)
            .or_default()
            .push((s.heading.clone(), s.line, s.level));
    }
    groups
}

fn heading_label(level: u8, heading: &str) -> String {
    format!("`{} {heading}`", "#".repeat(usize::from(level)))
}

/// The census shared by the three rules: every record of a type whose
/// `sections` declaration `declares`, with the section tree read to the type's
/// declared depth.
fn structure_census<'a>(
    records: &'a [Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    declares: impl Fn(&SectionsDecl) -> bool,
    mut judge: impl FnMut(&Record, &SectionsDecl, &[Section]) -> Outcome,
) -> (Population, Vec<PathBuf>) {
    let decls: HashMap<&str, SectionsDecl> = config
        .record_types
        .iter()
        .filter_map(|(name, rt)| {
            let decl = declared_model(name, rt).ok().flatten()?.sections?;
            declares(&decl).then_some((name.as_str(), decl))
        })
        .collect();
    let candidates: Vec<&'a Record> = records
        .iter()
        .filter(|r| decls.contains_key(r.record_type.as_str()))
        .collect();
    census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let decl = &decls[record.record_type.as_str()];
            let Some(text) = full_text.get(&record.path) else {
                return Outcome::Absent;
            };
            let sections = record_section_tree(record, text, decl.depth);
            judge(record, decl, &sections)
        },
    )
}
