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
/// and unexamined and never reported as dangling or as a misconfiguration.
/// The optional `non_reference_field` option (`BUG-190`, `ADR-75`) names a
/// header field in which one record lists, as comma-separated exact ids, the
/// ids it names without citing as a reference: a citation of such an id in
/// that record only is set aside the same way (`external_ids` first), a
/// declared id the record never cites and a malformed entry are findings,
/// reported with their own population by [`citation_non_reference`], and
/// [`non_reference_notices`] tells which declared ids now match a real record.
/// The header region is not read (the pointer and
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

    let skip_revision_log = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.skip_revision_log)
        .unwrap_or(false);

    let declarations = read_declarations(records, config, &prefixes);

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
        for (line, token) in record_citations(record, text, config, &prefixes, skip_revision_log) {
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
            // Named, not cited (`ADR-75`): in the declaring record only.
            if declarations
                .get(&c.record.path)
                .is_some_and(|d| d.ids.iter().any(|(_, id)| id.matches(&c.token)))
            {
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
                    suggested_action: None,
                    action_source: ActionSource::new("citation-unresolved"),
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

/// The decision ids cited in one record's prose, as `(line, token)`: the header
/// region and fenced code are not read, nor the revision-log lines when
/// `skip_revision_log` is set. Shared by the citation census and the check of
/// what a record declares it names.
fn record_citations(
    record: &Record,
    text: &str,
    config: &Config,
    prefixes: &[String],
    skip_revision_log: bool,
) -> Vec<(usize, String)> {
    let skipped = if skip_revision_log {
        let lines: Vec<(usize, &str)> = text.lines().enumerate().map(|(i, l)| (i + 1, l)).collect();
        skipped_table_lines(&lines, &config.revision_log_id(&record.record_type))
    } else {
        HashSet::new()
    };
    cited_prose_identifiers(text, record.header.region, prefixes)
        .into_iter()
        .filter(|(line, _)| !skipped.contains(line))
        .collect()
}

// ADR-75 clause 3: a declared entry is judged in a population of its own.
/// The `non_reference_field` check, reported as its own population (`MILE-186`,
/// `ADR-75` clause 3): unit field entry, one candidate per entry a record
/// declares, valid or malformed. A declared id the record never cites in prose
/// and a malformed entry are findings of `citation.resolution`, counted here and
/// not in the citation population. `None` when the option is unset, so a
/// repository that does not use it sees no second population; with the option
/// set and no record declaring anything, the population is empty (zero
/// eligible), distinguishable from a run that was never made.
pub fn citation_non_reference(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> Option<(RuleExecution, Vec<Finding>)> {
    const RULE_ID: &str = RULE_CITATION_RESOLUTION;
    let field = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.non_reference_field.as_deref())?;
    let prefixes = declared_prefixes(config);
    let skip_revision_log = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.skip_revision_log)
        .unwrap_or(false);
    let declarations = read_declarations(records, config, &prefixes);
    let (candidates, cited) = declared_candidates(
        records,
        &declarations,
        full_text,
        config,
        &prefixes,
        skip_revision_log,
    );

    let mut findings = Vec::new();
    let (population, examined_records) = census_records(
        PopulationUnit::FieldEntry,
        candidates,
        |c| c.record.path.clone(),
        |c| {
            judge_declared_entry(c, field, &cited, &mut findings);
            Outcome::Examined
        },
    );

    Some((
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    ))
}

/// One entry of a record's `non_reference_field`: a valid id (as written, and
/// parsed) or a malformed entry with its reason.
enum DeclaredEntry<'a> {
    Valid(&'a str, &'a ExternalId),
    Malformed(&'a str, &'a str),
}

struct DeclaredCandidate<'a> {
    record: &'a Record,
    declaration: &'a NonReferenceDeclaration,
    entry: DeclaredEntry<'a>,
}

/// One candidate per declared entry, and the ids each declaring record cites
/// in prose, by path.
fn declared_candidates<'a>(
    records: &'a [Record],
    declarations: &'a HashMap<PathBuf, NonReferenceDeclaration>,
    full_text: &HashMap<PathBuf, String>,
    config: &Config,
    prefixes: &[String],
    skip_revision_log: bool,
) -> (
    Vec<DeclaredCandidate<'a>>,
    HashMap<&'a PathBuf, Vec<String>>,
) {
    let mut candidates: Vec<DeclaredCandidate> = Vec::new();
    let mut cited: HashMap<&PathBuf, Vec<String>> = HashMap::new();
    for record in records {
        let Some(d) = declarations.get(&record.path) else {
            continue;
        };
        let tokens = full_text
            .get(&record.path)
            .map(|text| record_citations(record, text, config, prefixes, skip_revision_log))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, token)| token)
            .collect();
        cited.insert(&record.path, tokens);
        for (entry, reason) in &d.malformed {
            candidates.push(DeclaredCandidate {
                record,
                declaration: d,
                entry: DeclaredEntry::Malformed(entry, reason),
            });
        }
        for (entry, id) in &d.ids {
            candidates.push(DeclaredCandidate {
                record,
                declaration: d,
                entry: DeclaredEntry::Valid(entry, id),
            });
        }
    }
    (candidates, cited)
}

/// Judges one declared entry: a malformed entry, or a valid id the record
/// never cites in prose, is a finding.
fn judge_declared_entry(
    c: &DeclaredCandidate<'_>,
    field: &str,
    cited: &HashMap<&PathBuf, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    let mut report = |message: String, source: ActionSource| {
        findings.push(Finding {
            rule: RULE_CITATION_RESOLUTION.to_string(),
            severity: FindingSeverity::Error,
            file: c.record.path.clone(),
            line: Some(c.declaration.line),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: source,
            message,
        });
    };
    match &c.entry {
        DeclaredEntry::Malformed(entry, reason) => report(
            format!("{field} entry '{entry}' {reason}"),
            ActionSource::new("non-reference-malformed").field(field),
        ),
        DeclaredEntry::Valid(entry, id) => {
            // Cited by `external_ids` or not, a citation of the id counts.
            if !cited[&c.record.path].iter().any(|t| id.matches(t)) {
                report(
                    format!("{field} declares {entry}, which this record never cites in prose"),
                    ActionSource::new("non-reference-uncited")
                        .field(field)
                        .text(*entry),
                );
            }
        }
    }
}

/// One record's `non_reference_field` declaration (`ADR-75`).
struct NonReferenceDeclaration {
    /// The line of the field in the file.
    line: usize,
    /// Valid entries: the text as written and the parsed id.
    ids: Vec<(String, ExternalId)>,
    /// Entries that are not an exact id of a declared prefix, with the reason.
    malformed: Vec<(String, String)>,
}

/// Every record's declaration, by path; empty when the option is unset.
/// A record with no header, or without the field, declares nothing.
fn read_declarations(
    records: &[Record],
    config: &Config,
    prefixes: &[String],
) -> HashMap<PathBuf, NonReferenceDeclaration> {
    let mut out = HashMap::new();
    let Some(field) = config
        .rules
        .get(RULE_CITATION_RESOLUTION)
        .and_then(|s| s.non_reference_field.as_deref())
    else {
        return out;
    };
    for record in records {
        let Some(f) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == field)
        else {
            continue;
        };
        let mut d = NonReferenceDeclaration {
            line: f.line,
            ids: Vec::new(),
            malformed: Vec::new(),
        };
        for entry in f.value.split(',') {
            let entry = entry.trim();
            match parse_exact_id(entry, prefixes) {
                Ok(id) => d.ids.push((entry.to_string(), id)),
                Err(reason) => d.malformed.push((entry.to_string(), reason)),
            }
        }
        out.insert(record.path.clone(), d);
    }
    out
}

/// One aggregated `Info` notice naming the declared non-reference ids that now
/// match a real record (`ADR-75`): the declaration still applies, the author
/// is told the id is no longer unclaimed. Never a finding, so never an exit
/// code. An id `external_ids` already sets aside is not listed. Empty when the
/// option is unset or nothing matches.
pub fn non_reference_notices(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> Vec<Notice> {
    let prefixes = declared_prefixes(config);
    let external: Vec<ExternalId> = config
        .rules
        .get(RULE_CITATION_RESOLUTION)
        .and_then(|s| s.external_ids.as_ref())
        .into_iter()
        .flatten()
        .filter_map(|e| parse_external_id(e, &prefixes).ok())
        .collect();
    let declarations = read_declarations(records, config, &prefixes);
    let mut paths: Vec<&PathBuf> = declarations.keys().collect();
    paths.sort();
    let mut parts = Vec::new();
    for path in paths {
        let ids: Vec<&str> = declarations[path]
            .ids
            .iter()
            .filter(|(entry, _)| {
                !external.iter().any(|e| e.matches(entry))
                    && index.contains_key(&crate::values::RecordId::new(entry))
            })
            .map(|(entry, _)| entry.as_str())
            .collect();
        if !ids.is_empty() {
            parts.push(format!("{} declares {}", path.display(), ids.join(", ")));
        }
    }
    if parts.is_empty() {
        return Vec::new();
    }
    vec![Notice {
        severity: NoticeSeverity::Info,
        subject: RULE_CITATION_RESOLUTION.to_string(),
        message: format!(
            "{} record(s) declare non-reference ids that now match a real record; the declaration still applies: {}",
            parts.len(),
            parts.join("; ")
        ),
        detail: None,
    }]
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
        /// The reference section it was read from, where the edit lands.
        section: String,
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
                            section: section.heading.clone(),
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
                    suggested_action: None,
                    action_source: ActionSource::new("path-missing").section(c.section.as_str()),
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

/// `citation.issue-exists` (`MILE-143`): an issue or pull request cited in a
/// declared reference section is in the committed snapshot.
///
/// Offline and deterministic: the judgement is against `snapshot`, a file the
/// caller loaded, never against the network (`urzua refs refresh` is the one
/// command that writes it). Reads the `reference_sections` sections as
/// `citation.path-exists` does and honours `skip_revision_log`. The forms read
/// are the declared `issue_refs.forms`: `number` (`#123`, not inside a code
/// span, not `owner/repo#123`, not an anchor link) and `url` (a link to an
/// issue or pull request of the declared repository). An unreadable snapshot
/// is one finding on the snapshot path, and every citation is counted as
/// eligible and unexamined: the rule never passes for want of an input.
pub fn citation_issue_exists(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    snapshot: &Result<crate::refs_snapshot::RefsSnapshot, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CITATION_ISSUE_EXISTS;
    let setting = config.rules.get(RULE_ID);
    let Some(refs) = setting.and_then(|s| s.issue_refs.as_ref()) else {
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
    let scan = IssueScan::new(
        refs,
        setting
            .and_then(|s| s.reference_sections.clone())
            .unwrap_or_default(),
        setting.and_then(|s| s.skip_revision_log).unwrap_or(false),
    );
    let candidates = scan.cited_issues(records, config, full_text);

    let mut findings = Vec::new();
    if let Err(reason) = snapshot {
        findings.push(snapshot_unusable_finding(&refs.snapshot, reason));
    }
    let (population, examined_records) = census_records(
        PopulationUnit::Citation,
        candidates,
        |c| c.record.path.clone(),
        |c| {
            let Ok(snapshot) = snapshot else {
                return Outcome::Absent;
            };
            if !snapshot.contains(c.number) {
                findings.push(missing_issue_finding(c, snapshot));
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

/// One issue or pull request number cited in a record.
struct CitedIssue<'a> {
    record: &'a Record,
    line: usize,
    raw: String,
    number: u64,
    /// The header field it was read from; `None` for a reference section.
    field: Option<String>,
    /// The reference section it was read from; `None` for a header field.
    section: Option<String>,
}

/// What `citation.issue-exists` reads, and the patterns that find a citation.
struct IssueScan {
    number_form: bool,
    url_form: bool,
    number_re: regex::Regex,
    url_re: regex::Regex,
    sections: Vec<String>,
    skip_revision_log: bool,
    header_fields: Vec<String>,
}

impl IssueScan {
    fn new(
        refs: &crate::config::IssueRefs,
        sections: Vec<String>,
        skip_revision_log: bool,
    ) -> Self {
        use crate::config::RefForm;
        IssueScan {
            number_form: refs.forms.contains(&RefForm::Number),
            url_form: refs.forms.contains(&RefForm::Url),
            number_re: regex::Regex::new(r"(?:^|[^\w/&#.\-])(#)(\d+)\b").expect("static pattern"),
            url_re: regex::Regex::new(&format!(
                r"(?i)https?://(?:www\.)?github\.com/{}/(?:issues|pull)/(\d+)\b",
                regex::escape(&refs.repository)
            ))
            .expect("escaped pattern"),
            sections,
            skip_revision_log,
            header_fields: refs.fields.clone().unwrap_or_default(),
        }
    }

    /// Citations in one piece of text: (offset, raw, number), in text order.
    fn scan(&self, content: &str) -> Vec<(usize, String, u64)> {
        let masked = mask_code_spans(content);
        let mut found: Vec<(usize, String, u64)> = Vec::new();
        if self.number_form {
            for caps in self.number_re.captures_iter(&masked) {
                let hash = caps.get(1).expect("group 1");
                // `[text](#123)` is an anchor link, not a citation.
                if masked[..hash.start()].ends_with("](") {
                    continue;
                }
                let digits = caps.get(2).expect("group 2");
                if let Ok(n) = digits.as_str().parse::<u64>() {
                    found.push((hash.start(), format!("#{}", digits.as_str()), n));
                }
            }
        }
        if self.url_form {
            for caps in self.url_re.captures_iter(&masked) {
                let whole = caps.get(0).expect("group 0");
                if let Ok(n) = caps[1].parse::<u64>() {
                    found.push((whole.start(), whole.as_str().to_string(), n));
                }
            }
        }
        found.sort_by_key(|(at, _, _)| *at);
        found
    }

    /// Every citation, record by record: declared header fields
    /// (`issue_refs.fields`) on the field's own line, then the reference sections.
    fn cited_issues<'a>(
        &self,
        records: &'a [Record],
        config: &Config,
        full_text: &HashMap<PathBuf, String>,
    ) -> Vec<CitedIssue<'a>> {
        let mut candidates: Vec<CitedIssue> = Vec::new();
        for record in records {
            self.header_citations(record, &mut candidates);
            let Some(text) = full_text.get(&record.path) else {
                continue;
            };
            self.section_citations(record, text, config, &mut candidates);
        }
        candidates
    }

    fn header_citations<'a>(&self, record: &'a Record, out: &mut Vec<CitedIssue<'a>>) {
        for name in &self.header_fields {
            let Some(field) = record
                .header
                .fields
                .iter()
                .find(|f| f.key.as_str() == name.as_str())
            else {
                continue;
            };
            for (_, raw, number) in self.scan(&field.value) {
                out.push(CitedIssue {
                    record,
                    line: field.line,
                    raw,
                    number,
                    field: Some(name.clone()),
                    section: None,
                });
            }
        }
    }

    fn section_citations<'a>(
        &self,
        record: &'a Record,
        text: &str,
        config: &Config,
        out: &mut Vec<CitedIssue<'a>>,
    ) {
        for section in super::section::record_sections(record, text)
            .iter()
            .filter(|s| self.sections.iter().any(|name| name == &s.heading))
        {
            let lines = super::section::prose_lines(section);
            let skipped = if self.skip_revision_log {
                skipped_table_lines(&lines, &config.revision_log_id(&record.record_type))
            } else {
                HashSet::new()
            };
            for (line, content) in lines {
                if skipped.contains(&line) {
                    continue;
                }
                for (_, raw, number) in self.scan(content) {
                    out.push(CitedIssue {
                        record,
                        line,
                        raw,
                        number,
                        field: None,
                        section: Some(section.heading.clone()),
                    });
                }
            }
        }
    }
}

fn snapshot_unusable_finding(snapshot_path: &str, reason: &str) -> Finding {
    Finding {
        rule: RULE_CITATION_ISSUE_EXISTS.to_string(),
        severity: FindingSeverity::Error,
        file: PathBuf::from(snapshot_path),
        line: None,
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source: ActionSource::new("snapshot-unusable"),
        message: format!(
            "the snapshot of issue and pull request numbers could not be used, so no citation was checked: {reason}; run `urzua refs refresh`"
        ),
    }
}

fn missing_issue_finding(
    c: &CitedIssue<'_>,
    snapshot: &crate::refs_snapshot::RefsSnapshot,
) -> Finding {
    Finding {
        rule: RULE_CITATION_ISSUE_EXISTS.to_string(),
        severity: FindingSeverity::Error,
        file: c.record.path.clone(),
        line: Some(c.line),
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source: match (&c.field, &c.section) {
            (Some(field), _) => ActionSource::new("issue-missing-in-field").field(field.as_str()),
            (None, section) => ActionSource::new("issue-missing-in-section")
                .section(section.clone().unwrap_or_default()),
        },
        message: format!(
            "cites {}{}, which is not an issue or pull request of {} in the snapshot generated {}; if it was opened since, run `urzua refs refresh`",
            c.raw,
            c.field
                .as_ref()
                .map(|f| format!(" in the {f} field"))
                .unwrap_or_default(),
            snapshot.repository,
            snapshot.generated_at
        ),
    }
}

/// `line` with each inline code span, delimiters included, replaced by spaces
/// of the same byte length, so offsets into it are offsets into `line`.
fn mask_code_spans(line: &str) -> String {
    let mut out = line.as_bytes().to_vec();
    for (start, end) in code_span_ranges(line) {
        for b in &mut out[start..end] {
            *b = b' ';
        }
    }
    // Only ASCII backticks and the bytes between them were replaced, and a span
    // starts and ends on an ASCII byte, so every multi-byte character is either
    // wholly inside a span (all of its bytes replaced) or wholly outside.
    String::from_utf8(out).unwrap_or_default()
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
    code_span_ranges_inner(line)
        .into_iter()
        .map(|(_, _, s, e)| line[s..e].trim().to_string())
        .collect()
}

/// The byte range of every inline code span on `line`, delimiters included.
fn code_span_ranges(line: &str) -> Vec<(usize, usize)> {
    code_span_ranges_inner(line)
        .into_iter()
        .map(|(a, b, _, _)| (a, b))
        .collect()
}

/// `(outer_start, outer_end, content_start, content_end)` per span.
fn code_span_ranges_inner(line: &str) -> Vec<(usize, usize, usize, usize)> {
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
                out.push((i, end + open, start, end));
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

/// Parse one exact id (`PREFIX-N`, no range) under a declared prefix. The one
/// id grammar of `external_ids`' exact path and `non_reference_field`
/// (`ADR-75`), so the two cannot disagree on what an id is.
pub fn parse_exact_id(entry: &str, prefixes: &[String]) -> Result<ExternalId, String> {
    if entry.contains("..") {
        return Err("is a range; give one exact id".to_string());
    }
    parse_external_id(entry, prefixes)
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

    fn run_skipping_log(docs: &[(&str, &str)], extra: &str) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let mut config = config_with_types(vec![("adr", type_config(None))]);
        let setting: crate::config::RuleSetting =
            yaml_serde::from_str(&format!("level: warn\n{extra}")).unwrap();
        config
            .rules
            .insert(RULE_CITATION_RESOLUTION.to_string(), setting);
        let (index, _) = build_index_reporting_collisions(&records);
        citation_resolution(&records, &config, &full_text, &index)
    }

    const LOG_CITES: &str = "# 1\n\nProse cites ADR-98.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Cited ADR-97. | **substantive** |\n\nAfter: ADR-96.\n";

    #[test]
    fn without_skip_revision_log_a_revision_log_citation_is_read_observed_failing() {
        let (_, findings) = run_skipping_log(&[("docs/adr/ADR-1-y.md", LOG_CITES)], "");
        assert_eq!(findings.len(), 3, "{findings:?}");
    }

    #[test]
    fn a_skipped_revision_log_citation_is_set_aside_and_prose_is_still_read() {
        let (exec, findings) = run_skipping_log(
            &[("docs/adr/ADR-1-y.md", LOG_CITES)],
            "skip_revision_log: true\n",
        );
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(named[0].contains("ADR-98") && named[1].contains("ADR-96"));
        assert_eq!(exec.population.unwrap().eligible(), 2);
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

    const NON_REF: &str = "Non-reference-ids";

    fn nonref_config(extra: &str) -> Config {
        let mut config = config_with_types(vec![("adr", type_config(None))]);
        let setting: crate::config::RuleSetting = yaml_serde::from_str(&format!(
            "level: error\nnon_reference_field: {NON_REF}\n{extra}"
        ))
        .unwrap();
        config
            .rules
            .insert(RULE_CITATION_RESOLUTION.to_string(), setting);
        config
    }

    fn run_nonref(
        docs: &[(&str, &str)],
        extra: &str,
    ) -> (RuleExecution, Vec<Finding>, Vec<Notice>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let config = nonref_config(extra);
        let (index, _) = build_index_reporting_collisions(&records);
        let (exec, mut findings) = citation_resolution(&records, &config, &full_text, &index);
        // The declaration's findings belong to the field-entry execution.
        let (_, declared) = citation_non_reference(&records, &config, &full_text).unwrap();
        findings.extend(declared);
        let notices = non_reference_notices(&records, &config, &index);
        (exec, findings, notices)
    }

    fn entries_execution(docs: &[(&str, &str)], extra: &str) -> RuleExecution {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        citation_non_reference(&records, &nonref_config(extra), &full_text)
            .unwrap()
            .0
    }

    #[test]
    fn each_declared_entry_is_one_field_entry_candidate_observed_failing() {
        let doc = declaring("ADR-72, ADR-73, nope", "Only ADR-72 is written here.");
        let exec = entries_execution(&[(DECLARER, &doc)], "");
        let p = exec.population.unwrap();
        assert_eq!(p.unit(), PopulationUnit::FieldEntry);
        // Valid, uncited and malformed entries are all examined; none is out of scope.
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (3, 3, 0));
        assert_eq!(exec.examined_records, vec![PathBuf::from(DECLARER)]);
        // The citation population is unchanged and excludes the declaration findings.
        let (_, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn no_declarations_is_zero_eligible_entries_and_an_unset_option_is_no_population() {
        let other = "---\nStatus: Accepted\n---\n# 2 — Y\n\nSee ADR-1.\n";
        let p = entries_execution(&[(OTHER, other)], "").population.unwrap();
        assert_eq!((p.unit(), p.eligible()), (PopulationUnit::FieldEntry, 0));
        let records = vec![record(OTHER, "adr", other)];
        let config = config_with_types(vec![("adr", type_config(None))]);
        assert!(citation_non_reference(&records, &config, &HashMap::new()).is_none());
    }

    fn declaring(ids: &str, body: &str) -> String {
        format!("---\nStatus: Accepted\n{NON_REF}: {ids}\n---\n# 1 — X\n\n{body}\n")
    }

    const DECLARER: &str = "docs/adr/ADR-1-x.md";
    const OTHER: &str = "docs/adr/ADR-2-y.md";

    #[test]
    fn a_declared_non_reference_id_is_neither_resolved_nor_reported_observed_failing() {
        let doc = declaring("ADR-72", "As in ADR-72/73/74.");
        let (exec, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
        assert!(findings.is_empty(), "{findings:?}");
        let p = exec.population.unwrap();
        // Eligible and unexamined, as an external id is; never out of scope.
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (1, 0, 0));
    }

    #[test]
    fn the_declaration_is_scoped_to_its_own_record_observed_failing() {
        let doc = declaring("ADR-72", "As in ADR-72.");
        let other = "---\nStatus: Accepted\n---\n# 2 — Y\n\nSee ADR-72.\n";
        let (_, findings, _) = run_nonref(&[(DECLARER, &doc), (OTHER, other)], "");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].file.ends_with("ADR-2-y.md"));
        assert!(findings[0].message.contains("ADR-72"));
    }

    #[test]
    fn a_declaration_hides_only_the_ids_it_names_observed_failing() {
        let doc = declaring("ADR-72", "ADR-72 and ADR-99.");
        let (_, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-99"));
    }

    #[test]
    fn ids_compare_by_value_and_a_list_is_comma_separated_observed_failing() {
        let doc = declaring("ADR-072, ADR-5", "ADR-72 and ADR-0005 and ADR-6.");
        let (_, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-6"));
    }

    #[test]
    fn a_declared_id_the_record_never_cites_is_a_finding_naming_field_id_and_line_observed_failing()
    {
        let doc = declaring("ADR-72, ADR-73", "Only ADR-72 is written here.");
        let (_, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
        assert_eq!(findings.len(), 1, "{findings:?}");
        let f = &findings[0];
        assert_eq!(f.line, Some(3));
        assert!(f.message.contains(NON_REF) && f.message.contains("ADR-73"));
        assert!(!f.message.contains("ADR-72"), "{}", f.message);
    }

    #[test]
    fn an_id_cited_only_in_the_header_a_fence_or_a_skipped_log_is_uncited_observed_failing() {
        let body = "```\nADR-72\n```\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Cited ADR-72. | **substantive** |\n";
        let doc = declaring("ADR-72", body);
        let records = vec![record(DECLARER, "adr", &doc)];
        let full_text: HashMap<PathBuf, String> = [(PathBuf::from(DECLARER), doc.clone())]
            .into_iter()
            .collect();
        let config = nonref_config("skip_revision_log: true\n");
        let (index, _) = build_index_reporting_collisions(&records);
        let (_, mut findings) = citation_resolution(&records, &config, &full_text, &index);
        findings.extend(
            citation_non_reference(&records, &config, &full_text)
                .unwrap()
                .1,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("never cites"));
    }

    #[test]
    fn a_malformed_entry_is_a_finding_and_never_dropped_observed_failing() {
        for bad in ["ADR-72,", "adr-72", "FOO-72", "ADR-7..9", "ADR-x", ""] {
            let doc = declaring(bad, "ADR-72 is written here.");
            let (_, findings, _) = run_nonref(&[(DECLARER, &doc)], "");
            assert!(
                findings
                    .iter()
                    .any(|f| f.line == Some(3) && f.message.contains(NON_REF)),
                "{bad:?}: {findings:?}"
            );
        }
    }

    #[test]
    fn external_ids_take_precedence_over_the_declaration_observed_failing() {
        let doc = declaring("ADR-72", "See ADR-72.");
        let (exec, findings, _) = run_nonref(&[(DECLARER, &doc)], "external_ids: [\"ADR-72\"]\n");
        // Precedence (ADR-75): external_ids takes the citation first. It still
        // counts as cited for the declaration, so the redundant declaration is
        // not reported as uncited.
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 1);
    }

    #[test]
    fn the_incident_an_id_later_created_as_a_real_record_changes_nothing_observed_failing() {
        let doc = declaring("ADR-72", "A worked example: ADR-72/73/74.");
        let real = "---\nStatus: Accepted\n---\n# 72 — Real\n";
        let before = run_nonref(&[(DECLARER, &doc)], "");
        let after = run_nonref(&[(DECLARER, &doc), ("docs/adr/ADR-72-real.md", real)], "");
        assert!(before.1.is_empty() && after.1.is_empty(), "{:?}", after.1);
        assert!(before.2.is_empty());
        assert_eq!(after.2.len(), 1, "{:?}", after.2);
        let n = &after.2[0];
        assert_eq!(n.severity, NoticeSeverity::Info);
        assert!(n.message.contains("ADR-72") && n.message.contains("ADR-1-x.md"));
    }

    #[test]
    fn the_notice_is_one_aggregate_and_skips_external_ids() {
        let a = declaring("ADR-72, ADR-73", "ADR-72 ADR-73");
        let b = "---\nStatus: Accepted\nNon-reference-ids: ADR-72\n---\n# 2 — Y\n\nADR-72\n";
        let docs = [
            (DECLARER, a.as_str()),
            (OTHER, b),
            ("docs/adr/ADR-72-r.md", "---\nStatus: Accepted\n---\n# 72\n"),
            ("docs/adr/ADR-73-r.md", "---\nStatus: Accepted\n---\n# 73\n"),
        ];
        let (_, _, notices) = run_nonref(&docs, "external_ids: [\"ADR-73\"]\n");
        assert_eq!(notices.len(), 1, "{notices:?}");
        let m = &notices[0].message;
        assert!(m.contains("ADR-1-x.md") && m.contains("ADR-2-y.md"), "{m}");
        assert!(m.contains("ADR-72") && !m.contains("ADR-73"), "{m}");
    }

    #[test]
    fn without_the_option_nothing_changes() {
        let doc = declaring("ADR-72", "See ADR-72.");
        let r = record(DECLARER, "adr", &doc);
        let full_text: HashMap<PathBuf, String> = [(PathBuf::from(DECLARER), doc.clone())]
            .into_iter()
            .collect();
        let config = config_with_types(vec![("adr", type_config(None))]);
        let records = vec![r];
        let (index, _) = build_index_reporting_collisions(&records);
        let (_, findings) = citation_resolution(&records, &config, &full_text, &index);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(non_reference_notices(&records, &config, &index).is_empty());
    }

    #[test]
    fn parse_exact_id_accepts_one_id_and_refuses_ranges() {
        let prefixes = vec!["ADR".to_string()];
        assert!(parse_exact_id("ADR-072", &prefixes)
            .unwrap()
            .matches("ADR-72"));
        for bad in ["ADR-1..3", "adr-1", "ADR-", "ADR-1,", " ADR-1", "FOO-1"] {
            assert!(parse_exact_id(bad, &prefixes).is_err(), "{bad:?}");
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

    use crate::refs_snapshot::RefsSnapshot;

    fn snap(issues: &[u64], pulls: &[u64]) -> Result<RefsSnapshot, String> {
        Ok(RefsSnapshot::new(
            "urzuahq/urzua",
            "2026-11-04",
            issues.to_vec(),
            pulls.to_vec(),
        ))
    }

    fn issues_with(
        docs: &[(&str, &str)],
        snapshot: &Result<RefsSnapshot, String>,
        extra: &str,
    ) -> (RuleExecution, Vec<Finding>) {
        issues_forms(docs, snapshot, "number, url", extra)
    }

    fn issues_forms(
        docs: &[(&str, &str)],
        snapshot: &Result<RefsSnapshot, String>,
        forms: &str,
        extra: &str,
    ) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let mut config = config_with_types(vec![("adr", type_config(None))]);
        let setting: crate::config::RuleSetting = yaml_serde::from_str(&format!(
            "level: warn\nreference_sections: [References]\nissue_refs: {{snapshot: .urzua/known-refs.json, repository: urzuahq/urzua, forms: [{forms}]}}\n{extra}"
        ))
        .unwrap();
        config
            .rules
            .insert(RULE_CITATION_ISSUE_EXISTS.to_string(), setting);
        citation_issue_exists(&records, &config, &full_text, snapshot)
    }

    fn issues(
        docs: &[(&str, &str)],
        snapshot: &Result<RefsSnapshot, String>,
    ) -> (RuleExecution, Vec<Finding>) {
        issues_with(docs, snapshot, "")
    }

    #[test]
    fn a_cited_number_missing_from_the_snapshot_is_reported_with_the_snapshot_date() {
        let body = "# 1\n\n## References\n\n- #12 and #99, PR #7\n";
        let (exec, findings) = issues(&[(ADR, body)], &snap(&[12], &[7]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("#99"), "{findings:?}");
        assert!(findings[0].message.contains("2026-11-04"), "{findings:?}");
        assert_eq!(findings[0].line, Some(5));
        assert_eq!(findings[0].rule, RULE_CITATION_ISSUE_EXISTS);
        assert_eq!(examined(&exec), 3);
    }

    #[test]
    fn a_pull_request_url_of_the_declared_repository_is_read_and_another_repository_is_not() {
        let body = "# 1\n\n## References\n\n[a](https://github.com/urzuahq/urzua/pull/5) [b](https://github.com/UrzuaHQ/Urzua/issues/6) [c](https://github.com/other/thing/pull/77) [d](https://github.com/urzuahq/urzua/pull/8#issuecomment-1)\n";
        let (exec, findings) = issues(&[(ADR, body)], &snap(&[], &[5]));
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(named[0].contains("/issues/6"));
        assert!(named[1].contains("/pull/8"));
        assert_eq!(examined(&exec), 3);
    }

    #[test]
    fn only_the_declared_forms_are_read() {
        let body = "# 1\n\n## References\n\n#40 https://github.com/urzuahq/urzua/pull/41\n";
        let (_, only_numbers) = issues_forms(&[(ADR, body)], &snap(&[], &[]), "number", "");
        assert_eq!(only_numbers.len(), 1, "{only_numbers:?}");
        assert!(only_numbers[0].message.contains("#40"));
    }

    fn issues_fields(
        docs: &[(&str, &str)],
        snapshot: &Result<RefsSnapshot, String>,
        refs_extra: &str,
    ) -> (RuleExecution, Vec<Finding>) {
        let records: Vec<Record> = docs.iter().map(|(p, c)| record(p, "adr", c)).collect();
        let full_text: HashMap<PathBuf, String> = docs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.to_string()))
            .collect();
        let mut config = config_with_types(vec![("adr", type_config(None))]);
        let setting: crate::config::RuleSetting = yaml_serde::from_str(&format!(
            "level: warn\nreference_sections: [References]\nissue_refs: {{snapshot: s.json, repository: urzuahq/urzua, forms: [number, url]{refs_extra}}}\n"
        ))
        .unwrap();
        config
            .rules
            .insert(RULE_CITATION_ISSUE_EXISTS.to_string(), setting);
        citation_issue_exists(&records, &config, &full_text, snapshot)
    }

    const FIELD_DOC: &str = "---\nStatus: Open\nFound-in: 'review of PR #99 and https://github.com/urzuahq/urzua/pull/98, not `#97`'\nNote: see #96\n---\n# 1\n\n## References\n\n- #12\n";

    #[test]
    fn a_declared_header_field_is_read_and_an_undeclared_one_is_not() {
        let (exec, findings) = issues_fields(
            &[(ADR, FIELD_DOC)],
            &snap(&[12], &[]),
            ", fields: [Found-in]",
        );
        let named: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(
            named[0].contains("#99") && named[0].contains("Found-in"),
            "{named:?}"
        );
        assert!(named[1].contains("/pull/98"), "{named:?}");
        // The line is the field's own line, not the section's.
        assert_eq!(findings[0].line, Some(3));
        // `Note` is not declared, `#97` is in a code span, `#12` is in the section.
        assert!(named
            .iter()
            .all(|m| !m.contains("#96") && !m.contains("#97")));
        assert_eq!(examined(&exec), 3);
    }

    #[test]
    fn with_no_fields_declared_header_fields_are_not_read() {
        let (exec, findings) = issues_fields(&[(ADR, FIELD_DOC)], &snap(&[12], &[]), "");
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_declared_field_a_record_lacks_is_absent_not_a_finding() {
        let doc = "---\nStatus: Open\n---\n# 1\n\n## References\n\n- #12\n";
        let (exec, findings) =
            issues_fields(&[(ADR, doc)], &snap(&[12], &[]), ", fields: [Found-in]");
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn things_that_are_not_citations_are_not_read() {
        let body = "# 1\n\n## References\n\n`#12` owner/repo#13 [x](#14) C# F#15 a-#16 &#39; #17x color #fff and a heading\n\n```\n#18\n```\n";
        let (exec, findings) = issues(&[(ADR, body)], &snap(&[], &[]));
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn a_number_after_punctuation_or_at_the_line_start_is_read() {
        let body = "# 1\n\n## References\n\n(#21), [#22], #23. \"#24\"\n#25\n";
        let (_, findings) = issues(&[(ADR, body)], &snap(&[], &[]));
        assert_eq!(findings.len(), 5, "{findings:?}");
    }

    #[test]
    fn only_declared_sections_are_read() {
        let body = "# 1\n\nSee #12.\n\n## Context\n\n#13\n";
        let (exec, findings) = issues(&[(ADR, body)], &snap(&[], &[]));
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn skip_revision_log_sets_aside_the_log_rows() {
        let body = "# 1\n\n## References\n\n- #1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Closed #2. | **substantive** |\n\nAfter #3\n";
        let (_, all) = issues(&[(ADR, body)], &snap(&[], &[]));
        assert_eq!(all.len(), 3, "{all:?}");
        let (_, kept) = issues_with(&[(ADR, body)], &snap(&[], &[]), "skip_revision_log: true\n");
        assert_eq!(kept.len(), 2, "{kept:?}");
    }

    #[test]
    fn an_unusable_snapshot_is_a_finding_on_its_path_and_no_citation_is_examined() {
        let body = "# 1\n\n## References\n\n#12\n";
        let (exec, findings) = issues(&[(ADR, body)], &Err("is absent (No such file)".to_string()));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from(".urzua/known-refs.json"));
        assert!(findings[0].message.contains("is absent"));
        assert!(findings[0].message.contains("urzua refs refresh"));
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (1, 0));
        // Even a corpus that cites nothing is told the input is missing.
        let (_, none) = issues(&[(ADR, "# 1\n")], &Err("x".to_string()));
        assert_eq!(none.len(), 1);
    }

    #[test]
    fn a_snapshot_of_another_repository_or_schema_is_refused() {
        let ok = RefsSnapshot::new("UrzuaHQ/Urzua", "2026-11-04", vec![1], vec![]);
        assert!(crate::refs_snapshot::validate_snapshot(ok, "urzuahq/urzua").is_ok());
        let other = RefsSnapshot::new("a/b", "2026-11-04", vec![], vec![]);
        let err = crate::refs_snapshot::validate_snapshot(other, "urzuahq/urzua").unwrap_err();
        assert!(err.contains("a/b"), "{err}");
        let mut old = RefsSnapshot::new("urzuahq/urzua", "2026-11-04", vec![], vec![]);
        old.schema_version = 9;
        assert!(
            crate::refs_snapshot::validate_snapshot(old, "urzuahq/urzua")
                .unwrap_err()
                .contains("schema_version 9")
        );
    }

    #[test]
    fn a_snapshot_is_sorted_and_deduplicated() {
        let s = RefsSnapshot::new("o/r", "2026-01-01", vec![3, 1, 3], vec![9, 2]);
        assert_eq!(
            (s.issues.clone(), s.pulls.clone()),
            (vec![1, 3], vec![2, 9])
        );
        assert!(s.contains(2) && s.contains(3) && !s.contains(4));
        assert_eq!(s.newest(), Some(9));
    }

    #[test]
    fn double_backtick_spans_are_read() {
        assert_eq!(code_spans("a ``b`c`` d `e`"), vec!["b`c", "e"]);
        assert!(code_spans("`unclosed").is_empty());
    }
}
