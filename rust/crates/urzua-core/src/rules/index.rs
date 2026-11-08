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
    /// A declared header field of the index's record type, written `{Status}`
    /// by its exact field name (`MILE-164`). A name that is not `id`, `file`
    /// or `title` is a field; whether the type declares it is checked at
    /// config load.
    Field(String),
}

/// An entry format: one `Vec<FormatSegment>` per line of the entry. A single
/// line is the common case; a multi-line format (a YAML-style entry, or a
/// link followed by a description line) writes its lines separated by `\n`
/// (`MILE-152`).
pub type IndexFormat = Vec<Vec<FormatSegment>>;

/// Splits an entry format into lines of literals and placeholders: `{id}`,
/// `{file}`, `{title}` and `{Field}` for a declared header field.
/// Every reason it refuses is one that would leave the rule unable to match a
/// line: a rule that cannot match reports nothing, which reads as a fresh
/// index.
///
/// A placeholder appears at most once in the whole entry. The entry must name
/// its record: `{id}`, or `{file}` when no id is written (a link list). A
/// trailing newline (a YAML block scalar) is ignored; a blank line inside the
/// format is refused, since index checking skips blank lines.
pub fn parse_index_format(format: &str) -> Result<IndexFormat, String> {
    let format = format.strip_suffix('\n').unwrap_or(format);
    if format.is_empty() {
        return Err("is empty".to_string());
    }
    let mut lines: IndexFormat = Vec::new();
    for (n, text) in format.split('\n').enumerate() {
        if text.trim().is_empty() {
            return Err(format!(
                "has a blank line {} (blank lines are skipped when an index is read, so an entry cannot contain one)",
                n + 1
            ));
        }
        lines.push(parse_format_line(text)?);
    }
    let all: Vec<&FormatSegment> = lines.iter().flatten().collect();
    for (n, placeholder) in all.iter().enumerate() {
        if !matches!(placeholder, FormatSegment::Literal(_)) && all[..n].contains(placeholder) {
            return Err(format!("uses {placeholder:?} more than once"));
        }
    }
    if !all.contains(&&FormatSegment::Id) && !all.contains(&&FormatSegment::File) {
        return Err(
            "has neither {id} nor {file}, so an entry names no record ({title} alone is ambiguous)"
                .to_string(),
        );
    }
    Ok(lines)
}

fn parse_format_line(format: &str) -> Result<Vec<FormatSegment>, String> {
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
            other if other.trim().is_empty() => {
                return Err("has an empty placeholder '{}'".to_string())
            }
            other => FormatSegment::Field(other.to_string()),
        };
        if !literal.is_empty() {
            segments.push(FormatSegment::Literal(std::mem::take(&mut literal)));
        }
        if matches!(segments.last(), Some(s) if !matches!(s, FormatSegment::Literal(_))) {
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
    Ok(segments)
}

#[derive(Default)]
struct Entry<'a> {
    id: Option<&'a str>,
    file: Option<&'a str>,
    title: Option<&'a str>,
    /// `(field name, value)` for each `{Field}` placeholder read.
    fields: Vec<(&'a str, &'a str)>,
}

impl<'a> Entry<'a> {
    /// What the entry calls its record, for a message.
    fn name(&self) -> &'a str {
        self.id.or(self.file).unwrap_or_default()
    }
}

/// One committed line read back through the format, or `None` when it does
/// not have the format's shape (or a placeholder captured nothing).
fn match_entry<'a>(line: &'a str, segments: &'a [FormatSegment]) -> Option<Entry<'a>> {
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
                    FormatSegment::Id => entry.id = Some(value),
                    FormatSegment::File => entry.file = Some(value),
                    FormatSegment::Field(name) => entry.fields.push((name.as_str(), value)),
                    _ => entry.title = Some(value),
                }
                pos = end;
            }
        }
        i += 1;
    }
    (pos == line.len()).then_some(entry)
}

/// The title the H1 carries without its identifier (`MILE-152`): the leading
/// number (`# 139 \u{2014} Title`) or the record's own id (`# ADR-139: Title`,
/// compared by value so padding does not matter) is dropped along with the
/// separator after it. `None` when the record has no H1.
fn record_title(record: &Record, full_text: &HashMap<PathBuf, String>) -> Option<String> {
    let text = full_text.get(&record.path)?;
    let h1 = first_h1(text, record.header.region)?;
    let numbered = !h1.first_token.is_empty() && h1.first_token.chars().all(|c| c.is_ascii_digit());
    let own_id = record_id(record).is_some_and(|id| {
        crate::values::RecordId::new(h1.first_token.trim_end_matches([':', ',', '.']))
            == crate::values::RecordId::new(&id)
    });
    let title = if numbered || own_id {
        h1.text[h1.first_token.len()..].trim_start_matches(|c: char| {
            c.is_whitespace() || matches!(c, '\u{2014}' | '\u{2013}' | '-' | ':')
        })
    } else {
        h1.text
    };
    Some(title.trim().to_string())
}

/// Where `index.freshness` findings for one artifact go.
struct IndexSink<'a> {
    file: PathBuf,
    findings: &'a mut Vec<Finding>,
}

impl IndexSink<'_> {
    fn push(&mut self, variant: &'static str, line: Option<usize>, message: String) {
        self.findings.push(Finding {
            rule: RULE_INDEX_FRESHNESS.to_string(),
            severity: FindingSeverity::Error,
            file: self.file.clone(),
            line,
            message,
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: if variant == "index-stale" {
                ActionSource::new(variant).argv(["urzua", "index", "--apply"])
            } else {
                ActionSource::new(variant)
            },
        });
    }
}

/// The records an index of one type should list, by id and by file name. A
/// file of the type's directory with no identifier (the index itself, a stray
/// README) is not one of its records.
struct ExpectedRecords<'a> {
    all: Vec<&'a Record>,
    by_id: HashMap<crate::values::RecordId, &'a Record>,
    by_file: HashMap<&'a str, &'a Record>,
}

impl<'a> ExpectedRecords<'a> {
    fn of_type(records: &'a [Record], record_type: &str) -> Self {
        let all: Vec<&Record> = records
            .iter()
            .filter(|r| r.record_type == record_type && record_id(r).is_some())
            .collect();
        let by_id = all
            .iter()
            .filter_map(|r| record_id(r).map(|id| (crate::values::RecordId::new(&id), *r)))
            .collect();
        let by_file = all
            .iter()
            .filter_map(|r| Some((r.path.file_name()?.to_str()?, *r)))
            .collect();
        ExpectedRecords {
            all,
            by_id,
            by_file,
        }
    }
}

/// The line range (first, last exclusive) the generated region covers, the
/// whole file when no markers are declared. `None` after reporting a marker
/// that is missing.
fn index_region_bounds(
    lines: &[&str],
    spec: &IndexSpec,
    sink: &mut IndexSink<'_>,
) -> Option<(usize, usize)> {
    let (Some(begin), Some(end)) = (&spec.begin, &spec.end) else {
        return Some((0, lines.len()));
    };
    let find =
        |marker: &str, from: usize| (from..lines.len()).find(|&i| lines[i].trim() == marker.trim());
    let Some(b) = find(begin, 0) else {
        sink.push(
            "index-unregenerable",
            None,
            format!(
                "the marker '{}' is missing from '{}'",
                begin.trim(),
                spec.path
            ),
        );
        return None;
    };
    let Some(e) = find(end, b + 1) else {
        sink.push(
            "index-unregenerable",
            None,
            format!(
                "the marker '{}' is missing from '{}' after '{}'",
                end.trim(),
                spec.path,
                begin.trim()
            ),
        );
        return None;
    };
    Some((b + 1, e))
}

/// Reads one entry starting at `region[at]` whose first line already matched
/// as `head`, joining the further lines of a multi-line format. The entry is
/// `None` when a line is missing or does not match (reported); the count is how
/// many region lines the attempt consumed.
fn read_entry<'l>(
    head: Entry<'l>,
    region: &[(usize, &'l str)],
    at: usize,
    format: &'l IndexFormat,
    sink: &mut IndexSink<'_>,
) -> (Option<Entry<'l>>, usize) {
    let lineno = region[at].0;
    let width = format.len();
    let mut entry = head;
    let mut consumed = 1;
    for (k, segments) in format.iter().enumerate().skip(1) {
        let Some((next_no, next)) = region.get(at + k).copied() else {
            sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "the entry starting here is incomplete: the format has {width} lines and the region ends after {k}"
                ),
            );
            return (None, consumed);
        };
        let Some(part) = match_entry(next, segments) else {
            sink.push(
                "index-stale",
                Some(next_no),
                format!(
                    "line {} of the entry starting at line {lineno} does not match the format: {}",
                    k + 1,
                    next.trim()
                ),
            );
            return (None, consumed);
        };
        entry.id = entry.id.or(part.id);
        entry.file = entry.file.or(part.file);
        entry.title = entry.title.or(part.title);
        entry.fields.extend(part.fields);
        consumed += 1;
    }
    (Some(entry), consumed)
}

/// Compares one listed entry with its record: the file name, each listed
/// field and the title.
fn compare_entry_to_record(
    entry: &Entry<'_>,
    lineno: usize,
    record: &Record,
    full_text: &HashMap<PathBuf, String>,
    sink: &mut IndexSink<'_>,
) {
    let record_file = record.path.file_name().and_then(|n| n.to_str());
    if let (Some(listed_file), Some(actual)) = (entry.file, record_file) {
        if listed_file != actual {
            sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "entry {} lists the file '{listed_file}', but the record is '{actual}'",
                    entry.name()
                ),
            );
        }
    }
    for (name, listed_value) in &entry.fields {
        match record.header.get(name).map(str::trim) {
            Some(actual) if actual == *listed_value => {}
            Some(actual) => sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "entry {} lists {name} '{listed_value}', but the record's {name} is '{actual}'",
                    entry.name()
                ),
            ),
            None => sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "entry {} lists {name} '{listed_value}', but the record has no {name} field",
                    entry.name()
                ),
            ),
        }
    }
    if let (Some(listed_title), Some(actual)) = (entry.title, record_title(record, full_text)) {
        if listed_title != actual {
            sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "entry {} lists the title '{listed_title}', but the record's title is '{actual}'",
                    entry.name()
                ),
            );
        }
    }
}

/// Reads the entries of the region, `K` non-blank lines at a time for a
/// format of `K` lines. A line that does not start an entry is reported and
/// skipped alone, so one stray line does not misalign every entry after it.
/// Returns the records listed.
fn check_region_entries(
    region: &[(usize, &str)],
    format: &IndexFormat,
    spec: &IndexSpec,
    expected: &ExpectedRecords<'_>,
    full_text: &HashMap<PathBuf, String>,
    sink: &mut IndexSink<'_>,
) -> HashSet<PathBuf> {
    let mut listed: HashSet<PathBuf> = HashSet::new();
    let mut at = 0;
    while at < region.len() {
        let (lineno, line) = region[at];
        let Some(head) = match_entry(line, &format[0]) else {
            sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "line does not match the format '{}': {}",
                    spec.format.trim_end(),
                    line.trim()
                ),
            );
            at += 1;
            continue;
        };
        let (entry, consumed) = read_entry(head, region, at, format, sink);
        at += consumed;
        let Some(entry) = entry else {
            continue;
        };
        let found = match (entry.id, entry.file) {
            (Some(id), _) => expected
                .by_id
                .get(&crate::values::RecordId::new(id))
                .copied(),
            (None, Some(file)) => expected.by_file.get(file).copied(),
            (None, None) => None,
        };
        let Some(record) = found else {
            sink.push(
                "index-stale",
                Some(lineno),
                format!(
                    "entry {} names no {} record",
                    entry.name(),
                    spec.record_type
                ),
            );
            continue;
        };
        if !listed.insert(record.path.clone()) {
            sink.push(
                "index-stale",
                Some(lineno),
                format!("entry {} is listed more than once", entry.name()),
            );
            continue;
        }
        compare_entry_to_record(&entry, lineno, record, full_text, sink);
    }
    listed
}

/// Compares one declared index artifact with the records of its type.
fn check_index_artifact(
    records: &[Record],
    full_text: &HashMap<PathBuf, String>,
    artifact: &IndexArtifact,
    findings: &mut Vec<Finding>,
) -> Outcome {
    let spec = &artifact.spec;
    let mut sink = IndexSink {
        file: PathBuf::from(&spec.path),
        findings,
    };
    let content = match &artifact.content {
        IndexContent::Text(text) => text,
        IndexContent::Missing => {
            sink.push(
                "index-stale",
                None,
                format!(
                    "index file '{}' is missing; generate it or remove the declaration",
                    spec.path
                ),
            );
            return Outcome::Absent;
        }
        IndexContent::NotText => {
            sink.push(
                "index-unregenerable",
                None,
                format!(
                    "index file '{}' is not text, so it was not compared",
                    spec.path
                ),
            );
            return Outcome::Unreadable;
        }
    };
    let Ok(format) = parse_index_format(&spec.format) else {
        // Refused at config load; reaching here means the caller skipped
        // that, and a silent pass would hide it.
        sink.push(
            "index-unregenerable",
            None,
            format!("the format '{}' cannot be parsed", spec.format),
        );
        return Outcome::Absent;
    };

    let lines: Vec<&str> = content.lines().collect();
    let Some((first, last)) = index_region_bounds(&lines, spec, &mut sink) else {
        return Outcome::Absent;
    };
    let expected = ExpectedRecords::of_type(records, &spec.record_type);
    let region: Vec<(usize, &str)> = lines[first..last]
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(offset, l)| (first + offset + 1, *l))
        .collect();
    let listed = check_region_entries(&region, &format, spec, &expected, full_text, &mut sink);
    for record in &expected.all {
        if !listed.contains(&record.path) {
            sink.push(
                "index-stale",
                None,
                format!(
                    "{} has no entry in '{}'",
                    record_id(record).unwrap_or_else(|| record.path.display().to_string()),
                    spec.path
                ),
            );
        }
    }
    Outcome::Examined
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
    let mut findings: Vec<Finding> = Vec::new();
    let population = census(
        PopulationUnit::File,
        artifacts.iter().collect(),
        |artifact| check_index_artifact(records, full_text, artifact, &mut findings),
    );
    (
        RuleExecution {
            rule: RULE_INDEX_FRESHNESS.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// The entry lines a committed index of `spec.record_type` should hold, one
/// formatted entry (its lines joined by `\n`) per record, in record-id order
/// (`MILE-152`). A record that cannot fill a placeholder the format uses (no
/// identifier, no H1) is an error naming it, never an entry with a hole.
pub fn render_index_entries(
    spec: &IndexSpec,
    records: &[Record],
    full_text: &HashMap<PathBuf, String>,
) -> Result<Vec<String>, String> {
    let format = parse_index_format(&spec.format)
        .map_err(|e| format!("index '{}': the format {e}", spec.path))?;
    // Only records with an identifier are the type's records, as in
    // `index.freshness`; ordered by id value, so ADR-2 precedes ADR-10.
    let mut chosen: Vec<(&Record, String)> = records
        .iter()
        .filter(|r| r.record_type == spec.record_type)
        .filter_map(|r| record_id(r).map(|id| (r, id)))
        .collect();
    chosen.sort_by_cached_key(|(r, id)| (record_number_key(id), r.path.clone()));
    let mut out = Vec::new();
    for (record, id) in chosen {
        let mut lines = Vec::new();
        for segments in &format {
            let mut line = String::new();
            for segment in segments {
                match segment {
                    FormatSegment::Literal(text) => line.push_str(text),
                    FormatSegment::Id => line.push_str(&id),
                    FormatSegment::File => line.push_str(
                        record
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(|| {
                                format!(
                                    "index '{}': {} has no file name",
                                    spec.path,
                                    record.path.display()
                                )
                            })?,
                    ),
                    FormatSegment::Field(name) => {
                        line.push_str(record.header.get(name).map(str::trim).ok_or_else(|| {
                            format!(
                                "index '{}': {} has no {name} field to put in {{{name}}}",
                                spec.path,
                                record.path.display()
                            )
                        })?)
                    }
                    FormatSegment::Title => {
                        line.push_str(&record_title(record, full_text).ok_or_else(|| {
                            format!(
                                "index '{}': {} has no H1 title to put in {{title}}",
                                spec.path,
                                record.path.display()
                            )
                        })?)
                    }
                }
            }
            lines.push(line);
        }
        out.push(lines.join("\n"));
    }
    Ok(out)
}

/// (prefix, number) so ids order by numeric value within a prefix.
fn record_number_key(id: &str) -> (String, u64) {
    match id.rsplit_once('-') {
        Some((prefix, number)) => (prefix.to_string(), number.parse().unwrap_or(u64::MAX)),
        None => (id.to_string(), u64::MAX),
    }
}

/// The text an index artifact should hold after regeneration: `existing` with
/// the generated region replaced by `render_index_entries`' output
/// (`MILE-152`). With markers, only the lines between them change and the
/// rest of the file is kept byte for byte; without, the whole file is the
/// entries. A missing artifact is created (with its markers, when declared). A
/// file that is not text, or a declared marker that is absent, is an error:
/// regenerating around a region it cannot find would write somewhere the
/// author did not choose. Deterministic, so running it on its own output
/// changes nothing.
pub fn regenerate_index(
    spec: &IndexSpec,
    records: &[Record],
    full_text: &HashMap<PathBuf, String>,
    existing: &IndexContent,
) -> Result<String, String> {
    let entries = render_index_entries(spec, records, full_text)?;
    let generated: Vec<&str> = entries.iter().flat_map(|e| e.split('\n')).collect();
    let text = match existing {
        IndexContent::NotText => {
            return Err(format!(
                "index '{}' is not text, so it was not regenerated",
                spec.path
            ))
        }
        IndexContent::Missing => None,
        IndexContent::Text(text) => Some(text.as_str()),
    };
    let eol = if text.is_some_and(|t| t.contains("\r\n")) {
        "\r\n"
    } else {
        "\n"
    };
    match (&spec.begin, &spec.end) {
        (Some(begin), Some(end)) => {
            let Some(text) = text else {
                let mut lines = vec![begin.trim()];
                lines.extend(&generated);
                lines.push(end.trim());
                return Ok(lines.join(eol) + eol);
            };
            let lines: Vec<&str> = text.split('\n').collect();
            let trimmed = |l: &str| l.trim().to_string();
            let find = |marker: &str, from: usize| {
                (from..lines.len()).find(|&i| trimmed(lines[i]) == marker.trim())
            };
            let Some(b) = find(begin, 0) else {
                return Err(format!(
                    "the marker '{}' is missing from '{}', so there is no region to regenerate",
                    begin.trim(),
                    spec.path
                ));
            };
            let Some(e) = find(end, b + 1) else {
                return Err(format!(
                    "the marker '{}' is missing from '{}' after '{}', so there is no region to regenerate",
                    end.trim(),
                    spec.path,
                    begin.trim()
                ));
            };
            let mut out: Vec<String> = lines[..=b].iter().map(|l| l.to_string()).collect();
            out.extend(generated.iter().map(|l| {
                if eol == "\r\n" {
                    format!("{l}\r")
                } else {
                    l.to_string()
                }
            }));
            out.extend(lines[e..].iter().map(|l| l.to_string()));
            Ok(out.join("\n"))
        }
        _ => {
            if generated.is_empty() {
                return Ok(String::new());
            }
            Ok(generated.join(eol) + eol)
        }
    }
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
            parse_index_format("{id} {Status}").is_ok(),
            "a field placeholder"
        );
        assert!(parse_index_format("{id} {}").is_err(), "empty placeholder");
        assert!(
            parse_index_format("{id} {Status} {Status}").is_err(),
            "repeated field placeholder"
        );
        assert!(
            parse_index_format("{id}{Status}").is_err(),
            "adjacent field placeholder"
        );
        assert!(parse_index_format("{id} {").is_err(), "unclosed brace");
    }

    fn run_format(
        items: &[(u32, &str)],
        format: &str,
        body: &str,
    ) -> (RuleExecution, Vec<Finding>) {
        let mut sp = spec(false);
        sp.format = format.to_string();
        run(items, sp, IndexContent::Text(body.to_string()))
    }

    const YAML: &str = "- id: {id}\n  file: {file}\n  title: {title}";

    #[test]
    fn a_multi_line_entry_is_checked_as_one_entry_observed_failing() {
        let good = "- id: ADR-1\n  file: ADR-1-x.md\n  title: One\n\n- id: ADR-2\n  file: ADR-2-x.md\n  title: Two\n";
        let (_, findings) = run_format(&[(1, "One"), (2, "Two")], YAML, good);
        assert!(findings.is_empty(), "{findings:?}");

        let stale = "- id: ADR-1\n  file: ADR-1-x.md\n  title: Old\n- id: ADR-2\n  file: ADR-2-x.md\n  title: Two\n";
        let (_, findings) = run_format(&[(1, "One"), (2, "Two")], YAML, stale);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Old") && findings[0].message.contains("One"));
        assert_eq!(findings[0].line, Some(1));
    }

    #[test]
    fn a_broken_multi_line_entry_is_reported_at_the_failing_line_and_later_entries_still_read() {
        let body = "- id: ADR-1\n  filename: ADR-1-x.md\n  title: One\n- id: ADR-2\n  file: ADR-2-x.md\n  title: Two\n";
        let (_, findings) = run_format(&[(1, "One"), (2, "Two")], YAML, body);
        let named = msgs(&findings);
        assert!(
            named
                .iter()
                .any(|m| m.contains("line 2 of the entry starting at line 1")),
            "{named:?}"
        );
        // Line 3 then fails to start an entry; entry 2 is read normally.
        assert!(
            named.iter().any(|m| m.contains("ADR-1 has no entry")),
            "{named:?}"
        );
        assert!(!named.iter().any(|m| m.contains("ADR-2")), "{named:?}");
    }

    #[test]
    fn a_truncated_multi_line_entry_is_incomplete_not_ignored() {
        let (_, findings) = run_format(&[(1, "One")], YAML, "- id: ADR-1\n  file: ADR-1-x.md\n");
        assert!(
            msgs(&findings).iter().any(|m| m.contains("incomplete")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_link_list_needs_no_id_token_and_identifies_by_file() {
        let fmt = "- [{title}]({file})";
        let (_, findings) = run_format(
            &[(1, "One"), (2, "Two")],
            fmt,
            "- [One](ADR-1-x.md)\n- [Two](ADR-2-x.md)\n",
        );
        assert!(findings.is_empty(), "{findings:?}");
        let (_, findings) = run_format(
            &[(1, "One"), (2, "Two")],
            fmt,
            "- [One](ADR-1-x.md)\n- [Ghost](ADR-9-x.md)\n",
        );
        let named = msgs(&findings);
        assert!(
            named
                .iter()
                .any(|m| m.contains("ADR-9-x.md") && m.contains("names no")),
            "{named:?}"
        );
        assert!(
            named.iter().any(|m| m.contains("ADR-2 has no entry")),
            "{named:?}"
        );
    }

    #[test]
    fn a_title_excludes_the_records_own_id_whatever_its_form() {
        for h1 in [
            "# 1 \u{2014} One",
            "# ADR-1: One",
            "# ADR-0001 \u{2014} One",
            "# One",
        ] {
            let text = format!("---\nStatus: Accepted\n---\n{h1}\n");
            let r = Record::parse_with_shape(
                PathBuf::from("docs/adr/ADR-1-x.md"),
                "adr".to_string(),
                &text,
                crate::header::HeaderShape::YamlFrontmatter,
            );
            let mut t = HashMap::new();
            t.insert(r.path.clone(), text);
            assert_eq!(record_title(&r, &t).as_deref(), Some("One"), "{h1}");
        }
    }

    #[test]
    fn the_format_parser_accepts_multi_line_and_file_only_and_refuses_ambiguity() {
        assert_eq!(parse_index_format(YAML).unwrap().len(), 3);
        assert_eq!(
            parse_index_format("- id: {id}\n").unwrap().len(),
            1,
            "block scalar newline"
        );
        assert!(parse_index_format("[{title}]({file})").is_ok());
        assert!(parse_index_format("{title}").is_err());
        assert!(parse_index_format("{id}\n\n{title}").is_err(), "blank line");
        assert!(
            parse_index_format("{id}\n{id}").is_err(),
            "repeated across lines"
        );
        assert!(parse_index_format("").is_err());
    }

    fn regen(
        items: &[(u32, &str)],
        format: &str,
        markers: bool,
        existing: IndexContent,
    ) -> Result<String, String> {
        let (records, text) = corpus(items);
        let mut sp = spec(markers);
        sp.format = format.to_string();
        regenerate_index(&sp, &records, &text, &existing)
    }

    #[test]
    fn regeneration_fills_the_marked_region_keeps_the_rest_and_a_second_run_changes_nothing_observed_failing(
    ) {
        let existing = "# Index\n\n<!-- adr:begin -->\n- stale\n<!-- adr:end -->\n\ntrailer\n";
        let items = [(10, "Ten"), (2, "Two")];
        let once = regen(
            &items,
            FORMAT,
            true,
            IndexContent::Text(existing.to_string()),
        )
        .unwrap();
        assert_eq!(
            once,
            "# Index\n\n<!-- adr:begin -->\n- [ADR-2](ADR-2-x.md): Two\n- [ADR-10](ADR-10-x.md): Ten\n<!-- adr:end -->\n\ntrailer\n"
        );
        let twice = regen(&items, FORMAT, true, IndexContent::Text(once.clone())).unwrap();
        assert_eq!(once, twice);
        // And what it writes is what the rule calls fresh.
        let (_, findings) = run(&items, spec(true), IndexContent::Text(once));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn regeneration_of_a_whole_file_multi_line_and_missing_artifact() {
        let items = [(1, "One"), (2, "Two")];
        let made = regen(&items, YAML, false, IndexContent::Missing).unwrap();
        assert_eq!(
            made,
            "- id: ADR-1\n  file: ADR-1-x.md\n  title: One\n- id: ADR-2\n  file: ADR-2-x.md\n  title: Two\n"
        );
        let (_, findings) = run_format(&items, YAML, &made);
        assert!(findings.is_empty(), "{findings:?}");
        let with_markers = regen(&items, FORMAT, true, IndexContent::Missing).unwrap();
        assert!(
            with_markers.starts_with("<!-- adr:begin -->\n")
                && with_markers.ends_with("<!-- adr:end -->\n")
        );
    }

    #[test]
    fn regeneration_refuses_rather_than_guess() {
        let items = [(1, "One")];
        assert!(regen(
            &items,
            FORMAT,
            true,
            IndexContent::Text("no markers\n".into())
        )
        .is_err());
        assert!(regen(&items, FORMAT, true, IndexContent::NotText).is_err());
        // A record with no H1 cannot fill {title}.
        let r = Record::parse_with_shape(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nStatus: Accepted\n---\nno heading\n",
            crate::header::HeaderShape::YamlFrontmatter,
        );
        let mut t = HashMap::new();
        t.insert(
            r.path.clone(),
            "---\nStatus: Accepted\n---\nno heading\n".to_string(),
        );
        let err = regenerate_index(&spec(false), &[r], &t, &IndexContent::Missing).unwrap_err();
        assert!(err.contains("no H1"), "{err}");
    }

    const FIELD_FORMAT: &str = "- {id} [{Status}] {Date}";

    fn dated(n: u32, status: &str, date: Option<&str>) -> (Record, String) {
        let path = format!("docs/adr/ADR-{n}-x.md");
        let date_line = date.map(|d| format!("Date: {d}\n")).unwrap_or_default();
        let text = format!("---\nStatus: {status}\n{date_line}---\n# {n} \u{2014} T\n");
        let r = Record::parse_with_shape(
            PathBuf::from(&path),
            "adr".to_string(),
            &text,
            crate::header::HeaderShape::YamlFrontmatter,
        );
        (r, text)
    }

    fn run_fields(records: &[(Record, String)], body: &str) -> Vec<Finding> {
        let mut sp = spec(false);
        sp.format = FIELD_FORMAT.to_string();
        let recs: Vec<Record> = records.iter().map(|(r, _)| r.clone()).collect();
        let text: HashMap<PathBuf, String> = records
            .iter()
            .map(|(r, t)| (r.path.clone(), t.clone()))
            .collect();
        index_freshness(
            &recs,
            &text,
            &[IndexArtifact {
                spec: sp,
                content: IndexContent::Text(body.to_string()),
            }],
        )
        .1
    }

    #[test]
    fn declared_field_columns_are_checked_against_the_record_observed_failing() {
        let recs = [
            dated(1, "Accepted", Some("2026-01-02")),
            dated(2, "Draft", Some("2026-02-03")),
        ];
        let good = "- ADR-1 [Accepted] 2026-01-02\n- ADR-2 [Draft] 2026-02-03\n";
        assert!(run_fields(&recs, good).is_empty());

        let stale = "- ADR-1 [Draft] 2026-01-02\n- ADR-2 [Draft] 2026-02-04\n";
        let named = msgs(&run_fields(&recs, stale));
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(named[0].contains("Status 'Draft'") && named[0].contains("is 'Accepted'"));
        assert!(named[1].contains("Date '2026-02-04'") && named[1].contains("is '2026-02-03'"));
    }

    #[test]
    fn a_listed_field_the_record_lacks_is_reported() {
        let recs = [dated(1, "Accepted", None)];
        let named = msgs(&run_fields(&recs, "- ADR-1 [Accepted] 2026-01-02\n"));
        assert_eq!(named.len(), 1, "{named:?}");
        assert!(named[0].contains("has no Date field"), "{named:?}");
    }

    #[test]
    fn regeneration_fills_field_columns_and_a_missing_field_is_an_error() {
        let mut sp = spec(false);
        sp.format = FIELD_FORMAT.to_string();
        let (a, ta) = dated(1, "Accepted", Some("2026-01-02"));
        let (b, tb) = dated(2, "Draft", None);
        let text: HashMap<PathBuf, String> = [(a.path.clone(), ta), (b.path.clone(), tb)].into();
        let err = render_index_entries(&sp, &[a.clone(), b], &text).unwrap_err();
        assert!(err.contains("no Date field"), "{err}");
        let ok = render_index_entries(&sp, &[a], &text).unwrap();
        assert_eq!(ok, vec!["- ADR-1 [Accepted] 2026-01-02".to_string()]);
    }
}
