use super::*;
use crate::section::{parse_sections, Section};
use regex::Regex;

/// `section.decided-body-changed` (`MILE-350`, `MILE-88`): a record whose status
/// at the merge base was one its type declares decided (`decision_types`) has
/// a body that differs from the merge base's copy only by sections added whole
/// that match a declared `appendable_sections` pattern, and by rows added to the
/// end of its declared revision log. Header fields are judged by other rules.
///
/// Text is compared as words, so a formatter's re-wrapping, re-padding of a
/// table cell or re-drawn table separator is not a change. The status read is
/// the merge base's, so the change that decides a record is not judged against
/// the record it decides. Reports one finding per record, at the first changed
/// line of the current copy.
pub fn section_decided_body_changed(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    changed: Option<&[PathBuf]>,
    base_text: &HashMap<PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SECTION_DECIDED_BODY_CHANGED;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let opts: DecidedBodyOpts = config.rule_opts(RULE_ID);
    let changed: HashSet<&PathBuf> = changed.iter().collect();
    let mut candidates: Vec<&Record> = records
        .iter()
        .filter(|r| changed.contains(&r.path) && opts.decision_types.contains_key(&r.record_type))
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let patterns: Vec<Regex> = opts
        .appendable_sections
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let (Some(now), Some(then)) =
                (full_text.get(&record.path), base_text.get(&record.path))
            else {
                return Outcome::Absent;
            };
            let ctx = Judging {
                config,
                opts: &opts,
                patterns: &patterns,
            };
            match judge(record, &ctx, now, then) {
                Verdict::Absent => Outcome::Absent,
                Verdict::Unreadable => Outcome::Unreadable,
                Verdict::Clean => Outcome::Examined,
                Verdict::Changed(change) => {
                    findings.push(finding(record, &change));
                    Outcome::Examined
                }
            }
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
        findings,
    )
}

struct Judging<'a> {
    config: &'a Config,
    opts: &'a DecidedBodyOpts,
    patterns: &'a [Regex],
}

enum Verdict {
    /// The merge base's status is not one the type declares decided.
    Absent,
    Unreadable,
    Clean,
    Changed(Change),
}

/// The first change, in the current copy.
struct Change {
    line: usize,
    what: String,
}

fn finding(record: &Record, change: &Change) -> Finding {
    Finding::new(
        RULE_SECTION_DECIDED_BODY_CHANGED,
        FindingSeverity::Error,
        record.path.clone(),
        ActionSource::new("decided-body-changed"),
        format!(
            "the body of a decided record differs from the merge base's copy at line {}: {}; add an amendment section or a revision-log row instead of editing what was decided",
            change.line, change.what
        ),
    )
    .at(change.line)
}

fn judge(record: &Record, ctx: &Judging<'_>, now: &str, then: &str) -> Verdict {
    let Some(type_config) = ctx.config.record_types.get(&record.record_type) else {
        return Verdict::Absent;
    };
    let before = Record::parse_with_shape_and_prefix(
        record.path.clone(),
        record.record_type.clone(),
        then,
        type_config.header_shape,
        record.type_prefix.clone(),
    );
    if before.header.is_unreadable() || record.header.is_unreadable() {
        return Verdict::Unreadable;
    }
    let status_field = relation_field_name(
        ctx.config,
        &record.record_type,
        crate::config::RelationRole::Status,
    );
    let decided = ctx
        .opts
        .decision_types
        .get(&record.record_type)
        .zip(before.header.get(status_field))
        .is_some_and(|(statuses, status)| statuses.iter().any(|s| s == status.trim()));
    if !decided {
        return Verdict::Absent;
    }
    let id = ctx.config.revision_log_id(&record.record_type);
    let shape = ctx.config.revision_log_shape(&record.record_type);
    let (Some(now_log), Some(then_log)) = (read_log(now, &id, &shape), read_log(then, &id, &shape))
    else {
        return Verdict::Unreadable;
    };
    let after = Doc::read(now, record.header.region, &now_log);
    let base = Doc::read(then, before.header.region, &then_log);
    match first_change(ctx, &base, &after, &then_log, &now_log) {
        Some(change) => Verdict::Changed(change),
        None => Verdict::Clean,
    }
}

/// A revision log's rows by line and normalised cells; an absent section is
/// an empty log, and a log whose declared columns cannot be found is `None`.
struct LogRows {
    rows: Vec<(usize, String)>,
}

fn read_log(
    text: &str,
    id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Option<LogRows> {
    let entries = match find_revision_log_entries(text, id, shape) {
        None => Vec::new(),
        Some(Ok(log)) => log.entries,
        Some(Err(_)) => return None,
    };
    Some(LogRows {
        rows: entries
            .into_iter()
            .map(|e| {
                (
                    e.line,
                    e.row.split_whitespace().collect::<Vec<_>>().join(" "),
                )
            })
            .collect(),
    })
}

/// A body split into its preamble and its `##` sections, as words with the
/// line each came from, the revision log's rows left out.
struct Doc {
    preamble: Vec<Word>,
    /// The line the first section starts at, or the line after the body.
    preamble_line: usize,
    sections: Vec<Part>,
}

struct Part {
    heading: String,
    line: usize,
    words: Vec<Word>,
}

type Word = (String, usize);

impl Doc {
    fn read(text: &str, header: Option<(usize, usize)>, log: &LogRows) -> Doc {
        let header_end = header.map_or(0, |(_, end)| end);
        let skip: HashSet<usize> = log.rows.iter().map(|(line, _)| *line).collect();
        let body: String = text
            .lines()
            .skip(header_end)
            .map(|l| format!("{l}\n"))
            .collect();
        let sections: Vec<Section> = parse_sections(&body, header_end + 1);
        let first = sections.first().map_or(usize::MAX, |s| s.line);
        let preamble: String = body
            .lines()
            .take(first.saturating_sub(header_end + 1))
            .map(|l| format!("{l}\n"))
            .collect();
        Doc {
            preamble: words(&preamble, header_end + 1, &skip),
            preamble_line: sections.first().map_or(header_end + 1, |s| s.line),
            sections: sections
                .iter()
                .map(|s| Part {
                    heading: s.heading.clone(),
                    line: s.line,
                    words: words(&s.body, s.line + 1, &skip),
                })
                .collect(),
        }
    }
}

/// The words of `text`, each with its line. A table separator row is one word
/// whatever its dashes and colons, a leading blockquote mark is dropped and
/// `*` and `+` bullets read as `-`, so the changes a formatter makes to
/// whitespace, table drawing and bullets are not changes.
fn words(text: &str, first_line: usize, skip: &HashSet<usize>) -> Vec<Word> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = first_line + i;
        if skip.contains(&line) {
            continue;
        }
        let stripped = raw.trim_start_matches(|c: char| c == '>' || c.is_whitespace());
        if is_separator_row(stripped) {
            out.push(("|---|".to_string(), line));
            continue;
        }
        let spaced = stripped.replace('|', " | ");
        for (n, word) in spaced.split_whitespace().enumerate() {
            let word = if n == 0 && (word == "*" || word == "+") {
                "-"
            } else {
                word
            };
            out.push((word.to_string(), line));
        }
    }
    out
}

fn is_separator_row(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|')
        && line.contains('-')
        && line
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':') || c.is_whitespace())
}

fn first_change(
    ctx: &Judging<'_>,
    base: &Doc,
    now: &Doc,
    then_log: &LogRows,
    now_log: &LogRows,
) -> Option<Change> {
    let mut found: Vec<Change> = Vec::new();
    if let Some(change) = words_differ(
        &base.preamble,
        &now.preamble,
        now.preamble_line,
        "the text before the first section",
    ) {
        found.push(change);
    }
    let mut next = 0;
    for old in &base.sections {
        if exempt(ctx, &old.heading) {
            if let Some(at) = now.sections[next..]
                .iter()
                .position(|s| key(&s.heading) == key(&old.heading))
            {
                next += at + 1;
            }
            continue;
        }
        let Some(at) = now.sections[next..]
            .iter()
            .position(|s| key(&s.heading) == key(&old.heading))
        else {
            let line = now.sections.get(next).map_or(now.preamble_line, |s| s.line);
            found.push(Change {
                line,
                what: format!("the section `## {}` was removed or renamed", old.heading),
            });
            continue;
        };
        for added in &now.sections[next..next + at] {
            found.extend(unlisted(ctx, added));
        }
        let new = &now.sections[next + at];
        next += at + 1;
        let label = format!("the section `## {}`", old.heading);
        found.extend(words_differ(&old.words, &new.words, new.line, &label));
    }
    for added in &now.sections[next..] {
        found.extend(unlisted(ctx, added));
    }
    found.extend(log_change(then_log, now_log));
    found.into_iter().min_by_key(|c| c.line)
}

fn key(heading: &str) -> String {
    heading.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn exempt(ctx: &Judging<'_>, heading: &str) -> bool {
    ctx.opts
        .exempt_sections
        .iter()
        .any(|e| key(e).eq_ignore_ascii_case(&key(heading)))
}

/// An added section that is neither exempt nor one the repository lets be
/// added.
fn unlisted(ctx: &Judging<'_>, added: &Part) -> Option<Change> {
    (!exempt(ctx, &added.heading) && !ctx.patterns.iter().any(|p| p.is_match(&added.heading)))
        .then(|| Change {
            line: added.line,
            what: format!(
                "the section `## {}` was added and matches none of the declared `appendable_sections` ({})",
                added.heading,
                ctx.opts.appendable_sections.join(", ")
            ),
        })
}

/// The first difference between two word lists, at the line of the current
/// copy; `None` when they are the same words.
fn words_differ(old: &[Word], new: &[Word], section_line: usize, label: &str) -> Option<Change> {
    let at = old
        .iter()
        .zip(new)
        .position(|(a, b)| a.0 != b.0)
        .or_else(|| (old.len() != new.len()).then_some(old.len().min(new.len())))?;
    let line = new
        .get(at)
        .or_else(|| new.last())
        .map_or(section_line, |w| w.1);
    let context = |words: &[Word]| {
        let shown: Vec<&str> = words
            .iter()
            .skip(at)
            .take(6)
            .map(|w| w.0.as_str())
            .collect();
        if shown.is_empty() {
            "nothing".to_string()
        } else {
            format!("\"{}\"", shown.join(" "))
        }
    };
    Some(Change {
        line,
        what: format!("{label} changed: {} became {}", context(old), context(new)),
    })
}

/// An existing revision-log row that changed or went, or a row added before
/// the end of the log.
fn log_change(then: &LogRows, now: &LogRows) -> Option<Change> {
    then.rows
        .iter()
        .enumerate()
        .find_map(|(i, (_, row))| match now.rows.get(i) {
            Some((_, current)) if current == row => None,
            Some((line, _)) => Some(Change {
                line: *line,
                what: format!(
                    "revision-log row {} changed or a row was added before it",
                    i + 1
                ),
            }),
            None => Some(Change {
                line: now.rows.last().map_or(1, |(line, _)| *line),
                what: format!("revision-log row {} was removed", i + 1),
            }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RuleLevel, RuleSetting};
    use crate::rules::tests::*;

    const BASE: &str = "---\nStatus: Accepted\n---\n# 1 - X\n\n## Decision\n\nWe use a table.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n## References\n\nNone.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-01 | Initial. | **substantive** |\n";

    fn config() -> Config {
        let mut t = type_config(None);
        t.statuses = None;
        t.revision_log = Some(crate::config::RevisionLogDecl {
            marker: Some("**Revision log**".into()),
            ..Default::default()
        });
        let mut config = config_with_types(vec![("adr", t)]);
        config.rules.insert(
            RULE_SECTION_DECIDED_BODY_CHANGED.to_string(),
            RuleSetting {
                level: RuleLevel::Error,
                decision_types: Some(
                    [("adr".to_string(), vec!["Accepted".to_string()])]
                        .into_iter()
                        .collect(),
                ),
                appendable_sections: Some(vec![r"^Amendment \(\d{4}-\d{2}-\d{2}\)".into()]),
                ..Default::default()
            },
        );
        config
    }

    fn judged(config: &Config, now: &str, then: &str) -> (RuleExecution, Vec<Finding>) {
        let r = record("docs/adr/0001-x.md", "adr", now);
        let mut full = HashMap::new();
        full.insert(r.path.clone(), now.to_string());
        let mut base = HashMap::new();
        base.insert(r.path.clone(), then.to_string());
        let changed = vec![r.path.clone()];
        section_decided_body_changed(&[r], config, &full, Some(&changed), &base)
    }

    fn hits(now: &str, then: &str) -> Vec<Finding> {
        judged(&config(), now, then).1
    }

    #[test]
    fn a_reworded_decision_is_reported_at_its_line() {
        let now = BASE.replace("We use a table.", "We use a list.");
        let found = hits(&now, BASE);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, Some(8));
        assert!(
            found[0].message.contains("## Decision"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn an_added_dated_amendment_section_and_an_added_row_are_clean() {
        let now = BASE.replace(
            "## References",
            "## Amendment (2026-11-20): wider\n\nMore.\n\n## References",
        ) + "> | 2026-11-20 | Amended. | **substantive** |\n";
        assert!(hits(&now, BASE).is_empty());
    }

    #[test]
    fn a_reflow_and_a_re_padded_table_are_clean() {
        let now = BASE
            .replace("We use a table.", "We use\na table.")
            .replace(
                "| A | B |\n|---|---|\n| 1 | 2 |",
                "| A   | B   |\n| --- | --- |\n| 1   | 2   |",
            )
            .replace(
                "> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-01 | Initial. | **substantive** |",
                "> | Date       | Change   | Class           |\n> | ---------- | -------- | --------------- |\n> | 2026-11-01 | Initial. | **substantive** |",
            );
        assert!(hits(&now, BASE).is_empty(), "{:?}", hits(&now, BASE));
    }

    #[test]
    fn an_undated_amendment_and_one_inside_another_section_are_reported() {
        let undated = BASE.replace(
            "## References",
            "## Amendment: wider\n\nMore.\n\n## References",
        );
        assert_eq!(hits(&undated, BASE).len(), 1);
        let inside = BASE.replace(
            "We use a table.\n",
            "We use\n\n## Amendment (2026-11-20): wider\n\na table.\n",
        );
        assert_eq!(hits(&inside, BASE).len(), 1);
    }

    #[test]
    fn a_changed_or_removed_revision_row_and_an_edited_amendment_are_reported() {
        let changed = BASE.replace("Initial.", "First.");
        assert_eq!(hits(&changed, BASE).len(), 1);
        let removed = BASE.replace("> | 2026-11-01 | Initial. | **substantive** |\n", "");
        assert_eq!(hits(&removed, BASE).len(), 1);
        let amended = BASE.replace(
            "## References",
            "## Amendment (2026-11-20): a\n\nOne.\n\n## References",
        );
        let edited = amended.replace("One.", "Two.");
        assert_eq!(hits(&edited, &amended).len(), 1);
    }

    #[test]
    fn a_record_not_decided_at_the_merge_base_is_unexamined() {
        let proposed = BASE.replace("Accepted", "Proposed");
        let now = proposed.replace("We use a table.", "We use a list.");
        let (exec, found) = judged(&config(), &now, &proposed);
        assert!(found.is_empty());
        assert_eq!(examined(&exec), 0);
        // The change that decides a record is judged against the record it decides.
        let deciding = now.replace("Proposed", "Accepted");
        assert!(hits(&deciding, &proposed).is_empty());
    }

    #[test]
    fn an_exempt_section_may_change_and_no_diff_is_not_applicable() {
        let mut config = config();
        config
            .rules
            .get_mut(RULE_SECTION_DECIDED_BODY_CHANGED)
            .unwrap()
            .exempt_sections = Some(vec!["References".into()]);
        let now = BASE.replace("None.", "Some.");
        assert!(judged(&config, &now, BASE).1.is_empty());
        let r = record("docs/adr/0001-x.md", "adr", BASE);
        let (exec, _) =
            section_decided_body_changed(&[r], &config, &HashMap::new(), None, &HashMap::new());
        assert_eq!(exec.status, RuleStatus::NotApplicable);
    }
}
