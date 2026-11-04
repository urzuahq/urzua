use super::filename::first_h1;
use super::*;
use crate::config::IndexSpec;

/// What the caller found at an index artifact's path. The core never reads a
/// file (`ADR-5`); a missing file and one that is not text are told apart so
/// neither reads as an empty index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexContent {
    Text(String),
    Missing,
    NotText,
}

#[derive(Debug, Clone)]
pub struct IndexArtifact {
    pub spec: IndexSpec,
    pub content: IndexContent,
}

/// One piece of a parsed entry format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatSegment {
    Literal(String),
    Id,
    File,
    Title,
}

/// Splits an entry format into literals and the three placeholders. Every
/// reason it refuses is one that would leave the rule unable to match a line:
/// a rule that cannot match reports nothing, which reads as a fresh index.
pub fn parse_index_format(format: &str) -> Result<Vec<FormatSegment>, String> {
    let mut segments: Vec<FormatSegment> = Vec::new();
    let mut literal = String::new();
    let mut rest = format;
    while let Some(open) = rest.find(['{', '}']) {
        literal.push_str(&rest[..open]);
        if rest[open..].starts_with('}') {
            return Err("has a '}' with no matching '{'".to_string());
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err("has a '{' with no matching '}'".to_string());
        };
        let segment = match &after[..close] {
            "id" => FormatSegment::Id,
            "file" => FormatSegment::File,
            "title" => FormatSegment::Title,
            other => {
                return Err(format!(
                    "uses the placeholder '{{{other}}}'; the placeholders are {{id}}, {{file}} and {{title}}"
                ))
            }
        };
        if !literal.is_empty() {
            segments.push(FormatSegment::Literal(std::mem::take(&mut literal)));
        }
        if segments.contains(&segment) {
            return Err(format!("uses {segment:?} more than once"));
        }
        if matches!(
            segments.last(),
            Some(FormatSegment::Id | FormatSegment::File | FormatSegment::Title)
        ) {
            return Err(
                "has two placeholders with no text between them, so where one ends is ambiguous"
                    .to_string(),
            );
        }
        segments.push(segment);
        rest = &after[close + 1..];
    }
    literal.push_str(rest);
    if !literal.is_empty() {
        segments.push(FormatSegment::Literal(literal));
    }
    if !segments.contains(&FormatSegment::Id) {
        return Err("has no {id} placeholder, so an entry names no record".to_string());
    }
    Ok(segments)
}

#[derive(Default)]
struct Entry<'a> {
    id: &'a str,
    file: Option<&'a str>,
    title: Option<&'a str>,
}

/// One committed line read back through the format, or `None` when it does
/// not have the format's shape (or a placeholder captured nothing).
fn match_entry<'a>(line: &'a str, segments: &[FormatSegment]) -> Option<Entry<'a>> {
    let line = line.trim_end();
    let mut entry = Entry::default();
    let mut pos = 0;
    let mut i = 0;
    while i < segments.len() {
        match &segments[i] {
            FormatSegment::Literal(text) => {
                if !line[pos..].starts_with(text.as_str()) {
                    return None;
                }
                pos += text.len();
            }
            placeholder => {
                let end = match segments.get(i + 1) {
                    Some(FormatSegment::Literal(next)) => pos + line[pos..].find(next.as_str())?,
                    _ => line.len(),
                };
                let value = line[pos..end].trim();
                if value.is_empty() {
                    return None;
                }
                match placeholder {
                    FormatSegment::Id => entry.id = value,
                    FormatSegment::File => entry.file = Some(value),
                    _ => entry.title = Some(value),
                }
                pos = end;
            }
        }
        i += 1;
    }
    (pos == line.len()).then_some(entry)
}

/// The title the H1 carries after its number and separator
/// (`# 139 \u{2014} Title` is `Title`). `None` when the record has no H1.
fn record_title(record: &Record, full_text: &HashMap<PathBuf, String>) -> Option<String> {
    let text = full_text.get(&record.path)?;
    let h1 = first_h1(text, record.header.region)?;
    let numbered = !h1.first_token.is_empty() && h1.first_token.chars().all(|c| c.is_ascii_digit());
    let title = if numbered {
        h1.text[h1.first_token.len()..].trim_start_matches(|c: char| {
            c.is_whitespace() || matches!(c, '\u{2014}' | '\u{2013}' | '-' | ':')
        })
    } else {
        h1.text
    };
    Some(title.trim().to_string())
}

/// `index.freshness` (`MILE-139`): a committed index lists exactly the
/// records of its type, with their current file name and title.
///
/// The engine regenerates nothing. It reads each declared artifact (handed in
/// by the caller), parses the entries back through the declared format, and
/// reports the difference: a record with no entry, an entry naming no record,
/// an entry listed twice, a file or title that differs from the record's, and
/// a line in the generated region that is not an entry at all. A missing
/// artifact and a missing marker pair are findings, never a pass. Order is not
/// compared.
///
/// The population counts artifacts: one that could not be compared (missing,
/// not text, markers absent) is eligible and not examined, so a run over none
/// is distinguishable from a clean one.
pub fn index_freshness(
    records: &[Record],
    full_text: &HashMap<PathBuf, String>,
    artifacts: &[IndexArtifact],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_INDEX_FRESHNESS;
    let mut findings: Vec<Finding> = Vec::new();

    let population = census(
        PopulationUnit::File,
        artifacts.iter().collect(),
        |artifact| {
            let spec = &artifact.spec;
            let file = PathBuf::from(&spec.path);
            let mut push = |line: Option<usize>, message: String| {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: file.clone(),
                    line,
                    message,
                    waived: None,
                });
            };

            let content = match &artifact.content {
                IndexContent::Text(text) => text,
                IndexContent::Missing => {
                    push(
                        None,
                        format!(
                            "index file '{}' is missing; generate it or remove the declaration",
                            spec.path
                        ),
                    );
                    return Outcome::Absent;
                }
                IndexContent::NotText => {
                    push(
                        None,
                        format!(
                            "index file '{}' is not text, so it was not compared",
                            spec.path
                        ),
                    );
                    return Outcome::Unreadable;
                }
            };
            let Ok(segments) = parse_index_format(&spec.format) else {
                // Refused at config load; reaching here means the caller skipped
                // that, and a silent pass would hide it.
                push(
                    None,
                    format!("the format '{}' cannot be parsed", spec.format),
                );
                return Outcome::Absent;
            };

            let lines: Vec<&str> = content.lines().collect();
            let (first, last) = match (&spec.begin, &spec.end) {
                (Some(begin), Some(end)) => {
                    let find = |marker: &str, from: usize| {
                        (from..lines.len()).find(|&i| lines[i].trim() == marker.trim())
                    };
                    let Some(b) = find(begin, 0) else {
                        push(
                            None,
                            format!(
                                "the marker '{}' is missing from '{}'",
                                begin.trim(),
                                spec.path
                            ),
                        );
                        return Outcome::Absent;
                    };
                    let Some(e) = find(end, b + 1) else {
                        push(
                            None,
                            format!(
                                "the marker '{}' is missing from '{}' after '{}'",
                                end.trim(),
                                spec.path,
                                begin.trim()
                            ),
                        );
                        return Outcome::Absent;
                    };
                    (b + 1, e)
                }
                _ => (0, lines.len()),
            };

            let expected: Vec<(crate::values::RecordId, &Record)> = records
                .iter()
                .filter(|r| r.record_type == spec.record_type)
                .filter_map(|r| record_id(r).map(|id| (crate::values::RecordId::new(&id), r)))
                .collect();
            let by_id: HashMap<&crate::values::RecordId, &Record> =
                expected.iter().map(|(id, r)| (id, *r)).collect();

            let mut listed: HashSet<crate::values::RecordId> = HashSet::new();
            for (offset, line) in lines[first..last].iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let lineno = first + offset + 1;
                let Some(entry) = match_entry(line, &segments) else {
                    push(
                        Some(lineno),
                        format!(
                            "line does not match the format '{}': {}",
                            spec.format,
                            line.trim()
                        ),
                    );
                    continue;
                };
                let id = crate::values::RecordId::new(entry.id);
                let Some(record) = by_id.get(&id) else {
                    push(
                        Some(lineno),
                        format!("entry {} names no {} record", entry.id, spec.record_type),
                    );
                    continue;
                };
                if !listed.insert(id) {
                    push(
                        Some(lineno),
                        format!("entry {} is listed more than once", entry.id),
                    );
                    continue;
                }
                let record_file = record.path.file_name().and_then(|n| n.to_str());
                if let (Some(listed_file), Some(actual)) = (entry.file, record_file) {
                    if listed_file != actual {
                        push(
                            Some(lineno),
                            format!(
                            "entry {} lists the file '{listed_file}', but the record is '{actual}'",
                            entry.id
                        ),
                        );
                    }
                }
                if let (Some(listed_title), Some(actual)) =
                    (entry.title, record_title(record, full_text))
                {
                    if listed_title != actual {
                        push(
                        Some(lineno),
                        format!(
                            "entry {} lists the title '{listed_title}', but the record's title is '{actual}'",
                            entry.id
                        ),
                    );
                    }
                }
            }
            for (id, record) in &expected {
                if !listed.contains(id) {
                    push(
                        None,
                        format!(
                            "{} has no entry in '{}'",
                            record_id(record).unwrap_or_default(),
                            spec.path
                        ),
                    );
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
            examined_records: Vec::new(),
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMAT: &str = "- [{id}]({file}): {title}";

    fn adr(n: u32, title: &str) -> (Record, String) {
        let path = format!("docs/adr/ADR-{n}-x.md");
        let text = format!("---\nStatus: Accepted\n---\n# {n} \u{2014} {title}\n");
        let r = Record::parse_with_shape(
            PathBuf::from(&path),
            "adr".to_string(),
            &text,
            crate::header::HeaderShape::YamlFrontmatter,
        );
        (r, text)
    }

    fn corpus(items: &[(u32, &str)]) -> (Vec<Record>, HashMap<PathBuf, String>) {
        let mut records = Vec::new();
        let mut text = HashMap::new();
        for (n, t) in items {
            let (r, body) = adr(*n, t);
            text.insert(r.path.clone(), body);
            records.push(r);
        }
        (records, text)
    }

    fn spec(markers: bool) -> IndexSpec {
        IndexSpec {
            path: "docs/adr/README.md".to_string(),
            record_type: "adr".to_string(),
            format: FORMAT.to_string(),
            begin: markers.then(|| "<!-- adr:begin -->".to_string()),
            end: markers.then(|| "<!-- adr:end -->".to_string()),
        }
    }

    fn entry(n: u32, title: &str) -> String {
        format!("- [ADR-{n}](ADR-{n}-x.md): {title}")
    }

    fn marked(lines: &[String]) -> String {
        format!(
            "# Index\n\n<!-- adr:begin -->\n{}\n<!-- adr:end -->\n\ntrailer\n",
            lines.join("\n")
        )
    }

    fn run(
        items: &[(u32, &str)],
        spec: IndexSpec,
        content: IndexContent,
    ) -> (RuleExecution, Vec<Finding>) {
        let (records, text) = corpus(items);
        index_freshness(&records, &text, &[IndexArtifact { spec, content }])
    }

    fn msgs(findings: &[Finding]) -> Vec<String> {
        findings.iter().map(|f| f.message.clone()).collect()
    }

    #[test]
    fn a_current_index_is_clean_and_counted() {
        let body = marked(&[entry(1, "One"), entry(2, "Two")]);
        let (exec, findings) = run(
            &[(1, "One"), (2, "Two")],
            spec(true),
            IndexContent::Text(body),
        );
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!((pop.eligible(), pop.examined()), (1, 1));
    }

    #[test]
    fn a_record_with_no_entry_is_reported_by_id() {
        let body = marked(&[entry(1, "One")]);
        let (_, findings) = run(
            &[(1, "One"), (2, "Two")],
            spec(true),
            IndexContent::Text(body),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("ADR-2"),
            "{:?}",
            msgs(&findings)
        );
        assert!(
            findings[0].message.contains("no entry"),
            "{:?}",
            msgs(&findings)
        );
        assert_eq!(findings[0].file, PathBuf::from("docs/adr/README.md"));
    }

    #[test]
    fn an_entry_naming_no_record_is_reported_with_its_line() {
        let body = marked(&[entry(1, "One"), entry(9, "Gone")]);
        let (_, findings) = run(&[(1, "One")], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-9"));
        assert_eq!(findings[0].line, Some(5));
    }

    #[test]
    fn a_stale_title_names_both_titles() {
        let body = marked(&[entry(1, "Old name")]);
        let (_, findings) = run(&[(1, "New name")], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Old name"));
        assert!(findings[0].message.contains("New name"));
    }

    #[test]
    fn a_stale_file_name_is_a_finding() {
        let body = marked(&["- [ADR-1](ADR-1-renamed.md): One".to_string()]);
        let (_, findings) = run(&[(1, "One")], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-1-renamed.md"));
    }

    #[test]
    fn a_duplicated_entry_is_reported() {
        let body = marked(&[entry(1, "One"), entry(1, "One")]);
        let (_, findings) = run(&[(1, "One")], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("more than once"));
    }

    #[test]
    fn padding_in_the_listed_id_does_not_make_an_entry_extra() {
        let body = marked(&["- [ADR-0001](ADR-1-x.md): One".to_string()]);
        let (_, findings) = run(&[(1, "One")], spec(true), IndexContent::Text(body));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_line_the_format_does_not_match_is_a_finding_not_skipped() {
        let body = marked(&[entry(1, "One"), "- hand written".to_string()]);
        let (_, findings) = run(&[(1, "One")], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("does not match"));
    }

    #[test]
    fn a_missing_artifact_is_an_error_and_not_counted_as_compared() {
        let (exec, findings) = run(&[(1, "One")], spec(true), IndexContent::Missing);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("missing"));
        let pop = exec.population.unwrap();
        assert_eq!((pop.eligible(), pop.examined()), (1, 0));
    }

    #[test]
    fn an_artifact_that_is_not_text_is_unreadable_not_clean() {
        let (exec, findings) = run(&[(1, "One")], spec(true), IndexContent::NotText);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(exec.population.unwrap().unreadable(), 1);
    }

    #[test]
    fn a_removed_marker_pair_is_an_error() {
        let (exec, findings) = run(
            &[(1, "One")],
            spec(true),
            IndexContent::Text("# Index\n\n- [ADR-1](ADR-1-x.md): One\n".to_string()),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("<!-- adr:begin -->"));
        assert_eq!(exec.population.unwrap().examined(), 0);
    }

    #[test]
    fn an_end_marker_before_the_begin_marker_is_an_error() {
        let body = "<!-- adr:end -->\n<!-- adr:begin -->\n".to_string();
        let (_, findings) = run(&[], spec(true), IndexContent::Text(body));
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn without_markers_every_non_blank_line_is_an_entry() {
        let body = format!("{}\n\n{}\n", entry(1, "One"), entry(2, "Two"));
        let (_, findings) = run(
            &[(1, "One"), (2, "Two"), (3, "Three")],
            spec(false),
            IndexContent::Text(body),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-3"));
    }

    #[test]
    fn only_records_of_the_declared_type_are_expected() {
        let (mut records, mut text) = corpus(&[(1, "One")]);
        let other = Record::parse_with_shape(
            PathBuf::from("docs/rfc/RFC-1-x.md"),
            "rfc".to_string(),
            "# 1 \u{2014} R\n",
            crate::header::HeaderShape::default(),
        );
        text.insert(other.path.clone(), "# 1 \u{2014} R\n".to_string());
        records.push(other);
        let artifacts = [IndexArtifact {
            spec: spec(true),
            content: IndexContent::Text(marked(&[entry(1, "One")])),
        }];
        let (_, findings) = index_freshness(&records, &text, &artifacts);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_format_parser_rejects_what_cannot_be_matched() {
        assert!(parse_index_format("{id}").is_ok());
        assert!(parse_index_format("- {id}: {title}").is_ok());
        assert!(parse_index_format("no placeholder").is_err());
        assert!(parse_index_format("{title}").is_err(), "{{id}} is required");
        assert!(
            parse_index_format("{id}{title}").is_err(),
            "adjacent placeholders"
        );
        assert!(
            parse_index_format("{id} {id}").is_err(),
            "repeated placeholder"
        );
        assert!(
            parse_index_format("{id} {nope}").is_err(),
            "unknown placeholder"
        );
        assert!(parse_index_format("{id} {").is_err(), "unclosed brace");
    }
}
