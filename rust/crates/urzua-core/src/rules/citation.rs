use super::*;

/// `citation.resolution` (`MILE-136`): a decision ID cited in a record's
/// prose resolves to a record.
///
/// `pointer.resolution` reads only declared header fields, so a citation in a
/// Context paragraph of a record that was renumbered, or never created, passed
/// every rule. This reads every record's body for identifiers carrying a
/// configured type prefix (a type's `prefix`, or its upper-cased name) and
/// reports each that no record answers to, comparing numbers by value so
/// `ADR-0012` and `ADR-12` are one id.
///
/// Opt-in. The prefixes come from the record types the repository already
/// declares. The optional `external_ids` option (`MILE-156`) names ids that
/// share a record prefix but are not records (a public standard number such as
/// `RFC-4122` under an `RFC` prefix): a matching citation is counted out of
/// scope rather than resolved, so it is visible in the population as eligible
/// and unexamined and never reported as dangling or as a misconfiguration. The header region is not read (the pointer and
/// narrative rules own it) and fenced code blocks are skipped; an inline code
/// span is read, since nothing declares otherwise. The population's unit is the
/// citation, so a run over a corpus that cites nothing reports `0` examined,
/// not a clean pass.
pub fn citation_resolution(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CITATION_RESOLUTION;

    let prefixes = declared_prefixes(config);
    // Validated at config load; an entry that does not parse here cannot
    // match anything, and a config built in code is not trusted to be loaded.
    let external: Vec<ExternalId> = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.external_ids.as_ref())
        .into_iter()
        .flatten()
        .filter_map(|e| parse_external_id(e, &prefixes).ok())
        .collect();

    struct Citation<'a> {
        record: &'a Record,
        line: usize,
        token: String,
    }
    let mut candidates: Vec<Citation> = Vec::new();
    for record in records {
        let Some(text) = full_text.get(&record.path) else {
            continue;
        };
        for (line, token) in cited_prose_identifiers(text, record.header.region, &prefixes) {
            candidates.push(Citation {
                record,
                line,
                token,
            });
        }
    }

    let mut findings = Vec::new();
    let (population, examined_records) = census_records(
        PopulationUnit::Citation,
        candidates,
        |c| c.record.path.clone(),
        |c| {
            if external.iter().any(|e| e.matches(&c.token)) {
                // Set aside on purpose, so eligible and unexamined (`Absent`), not
                // `OutOfScope`: `config.scope-matches-nothing` reads that as a
                // configuration the rule cannot reach, and a declared external
                // id is the declaration working.
                return Outcome::Absent;
            }
            if !index.contains_key(&crate::values::RecordId::new(&c.token)) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: c.record.path.clone(),
                    line: Some(c.line),
                    waived: None,
                    declared_severity: None,
                    message: format!("{} is cited in prose but resolves to no record", c.token),
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

/// `citation.path-exists` (`MILE-143`): a repository path cited in backticks
/// inside a declared reference section exists.
///
/// Reads only the `##` sections named by `reference_sections`; a record without
/// one is out of scope (counted, not examined). What counts as a cited path is
/// deliberately narrow so a warning is almost always right: see
/// [`cited_path`]. `present` is supplied by the caller (this crate does not
/// touch the filesystem) and answers "tracked and on disk" for a file, or "has
/// a tracked file below it" for a directory. The population's unit is the
/// cited path, so a corpus that cites none reports `0` examined.
pub fn citation_path_exists(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    present: &dyn Fn(&str) -> bool,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CITATION_PATH_EXISTS;
    let sections: Vec<String> = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.reference_sections.clone())
        .unwrap_or_default();
    let skip_revision_log = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.skip_revision_log)
        .unwrap_or(false);

    struct Cited<'a> {
        record: &'a Record,
        line: usize,
        raw: String,
        path: String,
    }
    let mut candidates: Vec<Cited> = Vec::new();
    for record in records {
        let Some(text) = full_text.get(&record.path) else {
            continue;
        };
        for section in super::section::record_sections(record, text)
            .iter()
            .filter(|s| sections.iter().any(|name| name == &s.heading))
        {
            let lines = super::section::prose_lines(section);
            let skipped = if skip_revision_log {
                skipped_table_lines(&lines, &config.revision_log_id(&record.record_type))
            } else {
                HashSet::new()
            };
            for (line, content) in lines {
                if skipped.contains(&line) {
                    continue;
                }
                for span in code_spans(content) {
                    if let Some(path) = cited_path(&span, &record.path) {
                        candidates.push(Cited {
                            record,
                            line,
                            raw: span,
                            path,
                        });
                    }
                }
            }
        }
    }

    let mut findings = Vec::new();
    let (population, examined_records) = census_records(
        PopulationUnit::Citation,
        candidates,
        |c| c.record.path.clone(),
        |c| {
            if !present(&c.path) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: c.record.path.clone(),
                    line: Some(c.line),
                    waived: None,
                    declared_severity: None,
                    message: format!("cites `{}`, which is not in the working tree", c.raw),
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

/// The line numbers set aside by `skip_revision_log`: the line opening the
/// type's declared revision log (matched as that line, not by containing the
/// text), and the pipe table after it (blank or `>`-only lines may sit between, as in
/// a blockquoted table; the table ends at the first line that is not a row).
/// Append-only history in such a table names paths as they were.
fn skipped_table_lines(
    lines: &[(usize, &str)],
    id: &crate::config::RevisionLogId,
) -> HashSet<usize> {
    let mut skipped = HashSet::new();
    // None outside a skipped block; Some(seen_a_row) inside one.
    let mut block: Option<bool> = None;
    for (number, content) in lines {
        if id.opens_at(content) {
            block = Some(false);
            skipped.insert(*number);
            continue;
        }
        let Some(seen_row) = block else { continue };
        let stripped = content.trim_start().trim_start_matches('>').trim();
        if stripped.starts_with('|') {
            block = Some(true);
            skipped.insert(*number);
        } else if stripped.is_empty() && !seen_row {
            skipped.insert(*number);
        } else {
            block = None;
        }
    }
    skipped
}

/// The contents of every inline code span on `line`: a run of N backticks
/// closed by the next run of exactly N.
fn code_spans(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let open = bytes[i..].iter().take_while(|b| **b == b'`').count();
        let start = i + open;
        let mut j = start;
        let mut closed = None;
        while j < bytes.len() {
            if bytes[j] == b'`' {
                let run = bytes[j..].iter().take_while(|b| **b == b'`').count();
                if run == open {
                    closed = Some(j);
                    break;
                }
                j += run;
            } else {
                j += 1;
            }
        }
        match closed {
            Some(end) => {
                out.push(line[start..end].trim().to_string());
                i = end + open;
            }
            None => i = start,
        }
    }
    out
}

/// The repository-relative path a code span cites, or `None` when it is not
/// read as a path. Narrow on purpose: no whitespace; a `/` present; no `://`;
/// none of `*?<>{}$|` (glob or placeholder); not absolute or `~`/`@`-led; the
/// first segment is not a host name (a `.` inside it, unless it is a dot
/// directory such as `.github`); and the last segment carries an extension, or
/// the span ends in `/`. A trailing `:line[:col]` or `#anchor` is dropped.
/// `./` and `../` resolve from the directory of `record_path`, then the result
/// is lexically normalised; a path escaping the repository is returned as
/// written and so matches nothing.
fn cited_path(span: &str, record_path: &std::path::Path) -> Option<String> {
    let mut s = span;
    if let Some((head, _)) = s.split_once('#') {
        s = head;
    }
    // `path:12` or `path:12:3`.
    while let Some((head, tail)) = s.rsplit_once(':') {
        if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
            s = head;
        } else {
            break;
        }
    }
    if s.is_empty()
        || s.contains(char::is_whitespace)
        || !s.contains('/')
        || s.contains("://")
        || s.contains(['*', '?', '<', '>', '{', '}', '$', '|', ':'])
        || s.starts_with(['/', '~', '@'])
    {
        return None;
    }
    let is_dir = s.ends_with('/');
    let trimmed = s.trim_end_matches('/');
    let last = trimmed.rsplit('/').next().unwrap_or("");
    if !is_dir && !last.trim_start_matches('.').contains('.') {
        return None;
    }
    let first = trimmed.split('/').next().unwrap_or("");
    if first != "." && first != ".." && !first.starts_with('.') && first.contains('.') {
        return None;
    }
    let joined = if first == "." || first == ".." {
        let dir = record_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned());
        match dir {
            Some(d) if !d.is_empty() => format!("{d}/{trimmed}"),
            _ => trimmed.to_string(),
        }
    } else {
        trimmed.to_string()
    };
    Some(normalize_locator(&joined))
}

/// One declared external id: an exact prefixed number or an inclusive range
/// under one prefix (`MILE-156`). Numbers compare by value, like record ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalId {
    prefix: String,
    low: u128,
    high: u128,
}

impl ExternalId {
    /// Whether `token` (`PREFIX-N`, as `cited_identifiers` yields it) is this
    /// external id.
    pub fn matches(&self, token: &str) -> bool {
        let Some((prefix, digits)) = token.rsplit_once('-') else {
            return false;
        };
        prefix == self.prefix
            && digits
                .parse::<u128>()
                .is_ok_and(|n| (self.low..=self.high).contains(&n))
    }
}

/// Parse one `external_ids` entry: `PREFIX-N` or `PREFIX-LOW..HIGH`, the
/// prefix one of `prefixes` (the repository's declared record prefixes). An
/// entry under no declared prefix could never match a citation, so it is an
/// error rather than a silent no-op.
pub fn parse_external_id(entry: &str, prefixes: &[String]) -> Result<ExternalId, String> {
    if entry.trim() != entry || entry.is_empty() {
        return Err("is empty or has surrounding whitespace".to_string());
    }
    let (prefix, number) = entry
        .rsplit_once('-')
        .ok_or_else(|| "must look like PREFIX-N or PREFIX-LOW..HIGH".to_string())?;
    if !prefixes.iter().any(|p| p == prefix) {
        return Err(format!(
            "uses the prefix '{prefix}', which no record type declares (declared: {})",
            prefixes.join(", ")
        ));
    }
    let number_of = |s: &str| {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("'{s}' is not a whole number"));
        }
        s.parse::<u128>().map_err(|_| format!("'{s}' is too large"))
    };
    let (low, high) = match number.split_once("..") {
        Some((a, b)) => (number_of(a)?, number_of(b)?),
        None => {
            let n = number_of(number)?;
            (n, n)
        }
    };
    if low > high {
        return Err(format!(
            "is a range whose start {low} is after its end {high}"
        ));
    }
    Ok(ExternalId {
        prefix: prefix.to_string(),
        low,
        high,
    })
}

/// Every `(line, identifier)` in `text` outside the header region and outside
/// fenced code blocks. Lines are 1-indexed, matching `Header::region`.
fn cited_prose_identifiers(
    text: &str,
    header_region: Option<(usize, usize)>,
    prefixes: &[String],
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    // The open fence: its marker character and length.
    let mut fence: Option<(char, usize)> = None;
    for (i, line) in text.lines().enumerate() {
        let number = i + 1;
        let trimmed = line.trim_start_matches(' ');
        let indent_ok = line.len() - trimmed.len() <= 3;
        let marker = trimmed
            .chars()
            .next()
            .filter(|c| (*c == '`' || *c == '~') && indent_ok);
        if let Some(m) = marker {
            let run = trimmed.chars().take_while(|c| *c == m).count();
            if run >= 3 {
                match fence {
                    None => {
                        fence = Some((m, run));
                        continue;
                    }
                    Some((open, len)) if open == m && run >= len => {
                        // A closing fence carries no info string.
                        if trimmed[run..].trim().is_empty() {
                            fence = None;
                            continue;
                        }
                    }
                    Some(_) => {}
                }
            }
        }
        if fence.is_some() {
            continue;
        }
        if header_region.is_some_and(|(start, end)| (start..=end).contains(&number)) {
            continue;
        }
        for token in cited_identifiers(line, prefixes) {
            out.push((number, token));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn run(docs: &[(&str, &str)]) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (index, _) = build_index_reporting_collisions(&records);
        citation_resolution(&records, &config, &full_text, &index)
    }

    const TARGET: (&str, &str) = (
        "docs/adr/ADR-12-x.md",
        "---\nStatus: Accepted\n---\n\n# 12 — X\n",
    );

    #[test]
    fn a_citation_of_a_missing_record_is_an_error_naming_file_and_line_observed_failing() {
        let (exec, findings) = run(&[
            TARGET,
            (
                "docs/adr/ADR-1-y.md",
                "---\nStatus: Accepted\n---\n\n# 1 — Y\n\nSee ADR-99 for why.\n",
            ),
        ]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_CITATION_RESOLUTION);
        assert!(findings[0].file.ends_with("ADR-1-y.md"));
        assert_eq!(findings[0].line, Some(7));
        assert!(findings[0].message.contains("ADR-99"));
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn removing_the_citation_clears_the_finding() {
        let (exec, findings) = run(&[(
            "docs/adr/ADR-1-y.md",
            "---\nStatus: Accepted\n---\n\n# 1 — Y\n\nNothing cited.\n",
        )]);
        assert!(findings.is_empty());
        assert_eq!(examined(&exec), 0);
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn zero_padding_resolves_numerically_observed_failing() {
        let (exec, findings) = run(&[
            TARGET,
            (
                "docs/adr/ADR-1-y.md",
                "# 1 — Y\n\nSee ADR-0012 and ADR-12.\n",
            ),
        ]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 2);
    }

    #[test]
    fn fenced_code_blocks_are_skipped_and_inline_spans_are_not() {
        let body =
            "# 1 — Y\n\n```\nADR-98\n```\n~~~text\nADR-97\n~~~\nand `ADR-96` inline\nthen ADR-95\n";
        let (exec, findings) = run(&[("docs/adr/ADR-1-y.md", body)]);
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(findings.len(), 2, "{named:?}");
        assert!(named[0].contains("ADR-96") && named[1].contains("ADR-95"));
        assert_eq!(examined(&exec), 2);
    }

    #[test]
    fn a_longer_closing_fence_and_an_unterminated_fence_hold_to_the_end() {
        let body = "# 1\n\n````\n```\nADR-98\n````\nADR-97\n```\nADR-96\n";
        let (_, findings) = run(&[("docs/adr/ADR-1-y.md", body)]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-97"));
    }

    #[test]
    fn identifiers_embedded_in_other_tokens_are_not_citations() {
        let body = "# 1\n\nxADR-90 1ADR-91 a-ADR-92 docs/ADR-93 _ADR-94 .ADR-95 ADR-96x ADR-9x\n";
        let (exec, findings) = run(&[("docs/adr/ADR-1-y.md", body)]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 0);
    }

    #[test]
    fn punctuation_and_possessives_around_a_citation_still_count() {
        let body = "# 1\n\n(ADR-91), ADR-92. ADR-93's \"ADR-94\" [ADR-95]\n";
        let (exec, findings) = run(&[("docs/adr/ADR-1-y.md", body)]);
        assert_eq!(findings.len(), 5, "{findings:?}");
        assert_eq!(examined(&exec), 5);
    }

    #[test]
    fn only_configured_prefixes_are_read() {
        let (exec, findings) = run(&[("docs/adr/ADR-1-y.md", "# 1\n\nSee FOO-5 and BUG-7.\n")]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 0);
    }

    #[test]
    fn a_configured_prefix_override_is_the_one_read() {
        let r = record_with_prefix(
            "docs/m/MILE-1-y.md",
            "milestone",
            "MILE",
            "# 1\n\nSee MILE-9.\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "# 1\n\nSee MILE-9 and MILESTONE-9.\n".to_string(),
        );
        let mut t = type_config(None);
        t.prefix = Some("MILE".to_string());
        let config = config_with_types(vec![("milestone", t)]);
        let records = [r];
        let (index, _) = build_index_reporting_collisions(&records);
        let (exec, findings) = citation_resolution(&records, &config, &full_text, &index);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn the_header_region_is_not_re_read() {
        let (_, findings) = run(&[(
            "docs/adr/ADR-1-y.md",
            "---\nStatus: Accepted\nImplements: ADR-99\n---\n# 1 — Y\n\nBody.\n",
        )]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_population_counts_citations_not_records() {
        let (exec, _) = run(&[
            TARGET,
            ("docs/adr/ADR-1-y.md", "# 1\n\nADR-12 ADR-12 ADR-12\n"),
        ]);
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (3, 3));
    }

    fn run_external(docs: &[(&str, &str)], external: &[&str]) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "rfc", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let mut config = config_with_types(vec![("rfc", type_config(None))]);
        let list = external
            .iter()
            .map(|e| format!("\"{e}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let setting: crate::config::RuleSetting =
            yaml_serde::from_str(&format!("level: error\nexternal_ids: [{list}]\n")).unwrap();
        config
            .rules
            .insert(RULE_CITATION_RESOLUTION.to_string(), setting);
        let (index, _) = build_index_reporting_collisions(&records);
        citation_resolution(&records, &config, &full_text, &index)
    }

    const RFC_DOC: &str = "docs/rfc/RFC-1-y.md";

    #[test]
    fn a_declared_external_id_is_skipped_and_a_dangling_one_is_still_reported_observed_failing() {
        let body = "# 1 — Y\n\nUUIDs follow RFC-4122; also see RFC-99.\n";
        let (exec, findings) = run_external(&[(RFC_DOC, body)], &["RFC-4122"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-99"));
        let p = exec.population.unwrap();
        // The external citation is counted, as eligible and unexamined (set
        // aside on purpose, never dropped), and is not out of scope, which
        // `config.scope-matches-nothing` reads as a misconfiguration.
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (2, 1, 0));
    }

    #[test]
    fn without_a_declaration_the_colliding_standard_number_is_reported() {
        let body = "# 1 — Y\n\nUUIDs follow RFC-4122.\n";
        let (_, findings) = run_external(&[(RFC_DOC, body)], &["RFC-9999"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-4122"));
    }

    #[test]
    fn an_external_range_matches_inclusively_and_numerically() {
        let body = "# 1 — Y\n\nRFC-4000 RFC-04122 RFC-4999 RFC-3999 RFC-5000\n";
        let (exec, findings) = run_external(&[(RFC_DOC, body)], &["RFC-4000..4999"]);
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(named[0].contains("RFC-3999") && named[1].contains("RFC-5000"));
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (5, 2, 0));
    }

    #[test]
    fn declared_external_ids_do_not_make_scope_matches_nothing_fire_observed_failing() {
        let body = "# 1 — Y\n\nUUIDs follow RFC-4122 and RFC-822.\n";
        let (exec, _) = run_external(&[(RFC_DOC, body)], &["RFC-4122", "RFC-822"]);
        let (_, findings) = crate::rules::config_scope_matches_nothing(
            &[exec],
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_external_declaration_never_hides_an_id_of_another_prefix() {
        let docs = [
            (RFC_DOC, "# 1 — Y\n\nRFC-4122 and ADR-4122.\n"),
            ("docs/rfc/RFC-2-z.md", "# 2 — Z\n"),
        ];
        let (_, findings) = run_external(&docs, &["RFC-4122"]);
        // ADR is not a configured prefix here, so it is simply not a citation.
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn parse_external_id_accepts_exact_and_range_and_refuses_the_rest() {
        let prefixes = vec!["RFC".to_string(), "DOC-ADR".to_string()];
        assert!(parse_external_id("RFC-4122", &prefixes).is_ok());
        assert!(parse_external_id("DOC-ADR-7..9", &prefixes).is_ok());
        for bad in [
            "",
            "RFC",
            "RFC-",
            "RFC-x",
            "RFC-1..",
            "RFC-9..1",
            "RFC-1..2..3",
            "FOO-1",
            "RFC-1x",
            "RFC-*",
            " RFC-1",
        ] {
            assert!(parse_external_id(bad, &prefixes).is_err(), "{bad:?}");
        }
    }

    fn paths(docs: &[(&str, &str)], present: &[&str]) -> (RuleExecution, Vec<Finding>) {
        paths_skipping(docs, present, "")
    }

    fn paths_skipping(
        docs: &[(&str, &str)],
        present: &[&str],
        extra: &str,
    ) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let mut config = config_with_types(vec![("adr", type_config(None))]);
        let setting: crate::config::RuleSetting = yaml_serde::from_str(&format!(
            "level: warn\nreference_sections: [References]\n{extra}"
        ))
        .unwrap();
        config
            .rules
            .insert(RULE_CITATION_PATH_EXISTS.to_string(), setting);
        let present: Vec<String> = present.iter().map(|p| p.to_string()).collect();
        citation_path_exists(&records, &config, &full_text, &|p| {
            present.iter().any(|q| q == p)
        })
    }

    const ADR: &str = "docs/adr/ADR-1-y.md";

    #[test]
    fn a_cited_path_that_does_not_exist_is_reported_and_one_that_does_is_not_observed_failing() {
        let body = "# 1\n\n## References\n\n- `docs/specs/SPEC-1.md` and `docs/gone.md`\n";
        let (exec, findings) = paths(&[(ADR, body)], &["docs/specs/SPEC-1.md"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("docs/gone.md"));
        assert_eq!(findings[0].line, Some(5));
        assert_eq!(findings[0].rule, RULE_CITATION_PATH_EXISTS);
        assert_eq!(examined(&exec), 2);
    }

    #[test]
    fn only_the_declared_sections_are_read_and_a_record_without_one_cites_nothing() {
        let body = "# 1\n\nSee `docs/gone.md`.\n\n## Context\n\n`docs/also-gone.md`\n";
        let (exec, findings) = paths(&[(ADR, body)], &[]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn fenced_blocks_in_a_reference_section_are_not_read() {
        let body = "# 1\n\n## References\n\n```\n`docs/gone.md`\n```\n";
        let (_, findings) = paths(&[(ADR, body)], &[]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn directories_anchors_line_numbers_and_relative_paths_resolve() {
        let body = "# 1\n\n## References\n\n`docs/adr/` `docs/a.md#top` `rust/x.rs:12:3` `../specs/S.md` `./b.md`\n";
        let present = [
            "docs/adr",
            "docs/a.md",
            "rust/x.rs",
            "docs/specs/S.md",
            "docs/adr/b.md",
        ];
        let (exec, findings) = paths(&[(ADR, body)], &present);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 5);
    }

    #[test]
    fn things_that_are_not_paths_are_not_read() {
        let body = "# 1\n\n## References\n\n`Accepted/Rejected` `https://x.org/a.md` `example.com/a.md` `docs/*.md` `docs/<n>.md` `/abs/a.md` `make records` `Makefile` `@scope/pkg` `a/b` `x/y.rs z`\n";
        let (exec, findings) = paths(&[(ADR, body)], &[]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn a_dot_directory_is_a_path_not_a_host() {
        let body = "# 1\n\n## References\n\n`.github/workflows/ci.yml`\n";
        let (_, findings) = paths(&[(ADR, body)], &[]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_path_escaping_the_repository_matches_nothing() {
        let body = "# 1\n\n## References\n\n`../../../etc/x.conf`\n";
        let (_, findings) = paths(&[(ADR, body)], &["etc/x.conf"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    const LOG_RECORD: &str = "# 1\n\n## References\n\n- `docs/live.md` and `docs/gone-now.md`\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Named `src/old/path.rs`. | **substantive** |\n> | 2026-01-02 | Named `src/older.rs`. | **substantive** |\n\nAfter: `docs/after-gone.md`\n";

    #[test]
    fn without_skip_revision_log_a_revision_log_row_is_read_observed_failing() {
        let (_, findings) = paths(&[(ADR, LOG_RECORD)], &["docs/live.md"]);
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 4, "{named:?}");
    }

    #[test]
    fn a_skipped_revision_log_is_set_aside_and_the_rest_of_the_section_is_still_read() {
        let (exec, findings) = paths_skipping(
            &[(ADR, LOG_RECORD)],
            &["docs/live.md"],
            "skip_revision_log: true\n",
        );
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(named[0].contains("docs/gone-now.md"));
        assert!(named[1].contains("docs/after-gone.md"));
        // The skipped paths are not in the population at all.
        assert_eq!(exec.population.unwrap().eligible(), 3);
    }

    #[test]
    fn skip_revision_log_false_changes_nothing() {
        let (_, findings) = paths_skipping(
            &[(ADR, LOG_RECORD)],
            &["docs/live.md"],
            "skip_revision_log: false\n",
        );
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn double_backtick_spans_are_read() {
        assert_eq!(code_spans("a ``b`c`` d `e`"), vec!["b`c", "e"]);
        assert!(code_spans("`unclosed").is_empty());
    }
}
