use super::*;

/// Rule 4 (Phase 6): a filename's own claimed number must match its
/// document's H1 title. A rename that updates one and not the other is
/// silent otherwise -- nothing else in this corpus cross-checks the two.
pub fn filename_title_consistency(
    records: &[Record],
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_FILENAME_TITLE_CONSISTENCY;
    let mut findings = Vec::new();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            let Some(filename_number) = filename_number(record) else {
                return Outcome::OutOfScope;
            };
            let Some(content) = full_text.get(&record.path) else {
                return Outcome::Absent;
            };

            let mut push = |line: Option<usize>, message: String| {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("title-mismatch"),
                    message,
                });
            };

            // Skipping instead would drop a record the rule was asked about.
            let Ok(filename_number) = filename_number.parse::<u64>() else {
                push(
                    None,
                    format!("filename number {filename_number} is too large to compare"),
                );
                return Outcome::Examined;
            };

            match first_h1(content, record.header.region) {
                None => push(
                    None,
                    "no H1 title found to check against the filename's number".to_string(),
                ),
                Some(h1) => match h1_id(h1.first_token, record.identity.as_ref()) {
                    H1Id::Absent => {
                        let declared_form = record
                            .identity
                            .as_ref()
                            .and_then(|declared| title_form_problem(declared, h1.first_token));
                        push(
                            Some(h1.line),
                            declared_form.unwrap_or_else(|| {
                                format!(
                                    "H1 title '{}' carries no record id to check against the filename's number {filename_number}",
                                    elided(h1.text)
                                )
                            }),
                        );
                    }
                    H1Id::Overflow => push(
                        Some(h1.line),
                        format!(
                            "H1 title '{}' claims a number too large to compare",
                            elided(h1.text)
                        ),
                    ),
                    H1Id::Number(n) if n != filename_number => push(
                        Some(h1.line),
                        format!(
                            "filename claims number {filename_number}, but the H1 title '{}' claims {n}",
                            elided(h1.text)
                        ),
                    ),
                    H1Id::Number(_) => {}
                },
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

/// The number half of `record_id`'s `TYPE-NNNN` (ADR-36/BUG-2): any
/// non-empty numeric prefix, no fixed digit-count. Shares `record_id`'s
/// acceptance exactly by construction, rather than re-deriving it -- the two
/// were independently hand-written copies of the same filename grammar
/// before (`BUG-133`'s shape, one level over: not the reference/filename
/// segment predicates, but the id-from-filename extraction itself).
fn filename_number(record: &Record) -> Option<String> {
    let id = record_id(record)?;
    // The number is always the segment after the *last* hyphen: it is
    // all-digit by `record_id`'s own guard, so a hyphenated type prefix
    // (`DOC-ADR-2`) still splits correctly.
    id.rsplit_once('-').map(|(_, number)| number.to_string())
}

/// What an H1's leading token says about the record's id.
enum H1Id {
    Number(u64),
    /// The token is not an id: reported as such, never mangled into one.
    Absent,
    /// An id whose digits do not fit a `u64`.
    Overflow,
}

/// Reads an H1's id the way a filename's is read (`BUG-32`, `MILE-151`): the
/// id is the number of a `PREFIX-N` token, ASCII digits only, padding kept out
/// of the value. A type whose identity pattern declares a `prefix` capture
/// reads exactly that prefix; a type with no pattern keeps a strict read: a
/// leading `PREFIX-N` (an upper-case prefix, hyphenated if the type's is) or a
/// bare `N`. A trailing `.` or `:` is punctuation. Digits are never gathered
/// out of a token with other characters in it, so `2026-09-16` and `v1.2`
/// carry no id.
fn h1_id(first_token: &str, declared: Option<&crate::document_model::DeclaredIdentity>) -> H1Id {
    let token = first_token.trim_end_matches(['.', ':']);
    let digits = match declared.and_then(|d| d.title_prefix.as_deref()) {
        Some(prefix) => token
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('-')),
        None => match token.rsplit_once('-') {
            None => Some(token),
            Some((prefix, number))
                if prefix.starts_with(|c: char| c.is_ascii_uppercase())
                    && prefix.split('-').all(crate::new_record::is_prefix_segment) =>
            {
                Some(number)
            }
            Some(_) => None,
        },
    };
    match digits {
        Some(d) if crate::new_record::is_digit_segment(d) => {
            d.parse().map_or(H1Id::Overflow, H1Id::Number)
        }
        _ => H1Id::Absent,
    }
}

/// A type that declares an `identity` pattern also declares its title form
/// (RFC-44): `PREFIX-N`, the prefix being the pattern's `prefix` capture.
/// `None` when the title's first word has that form, or when the record has
/// no identity (no capture). A trailing `.` or `:` is punctuation
/// (`# ADR-1. Title`), not part of the number.
fn title_form_problem(
    declared: &crate::document_model::DeclaredIdentity,
    first_token: &str,
) -> Option<String> {
    let prefix = declared.title_prefix.as_ref()?;
    let token = first_token.trim_end_matches(['.', ':']);
    let expected = format!("'{prefix}-N'");
    let number_part = token.strip_prefix(&format!("{prefix}-"));
    if number_part.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    Some(format!(
        "H1 title starts '{}', but this type declares its title form as {expected}",
        elided(first_token)
    ))
}

/// Whether an H1 exists and whether it carries an id are independent
/// questions: this is the title, and [`h1_id`] reads the id from it.
pub(super) struct H1<'a> {
    /// 1-indexed, matching [`crate::header::Header::region`].
    line: usize,
    pub(super) text: &'a str,
    /// The first whitespace-separated word of the title.
    pub(super) first_token: &'a str,
}

/// Frontmatter is skipped because a YAML comment line inside it starts with
/// `# ` and would otherwise read as the title. Only a region opening at line
/// 1 is frontmatter, so a `prefix-lines` header (which sits *after* the H1)
/// is never skipped past, which would skip the title itself.
pub(super) fn first_h1(content: &str, header_region: Option<(usize, usize)>) -> Option<H1<'_>> {
    let skip_through = header_region
        .filter(|&(start, _)| start == 1)
        .map(|(_, end)| end);
    let mut fence: Option<(u8, usize)> = None;

    for (idx, line) in content.lines().enumerate() {
        let lineno = idx + 1;
        if skip_through.is_some_and(|end| lineno <= end) {
            continue;
        }

        let trimmed = line.trim_start();
        if let Some(marker @ (b'`' | b'~')) = trimmed.bytes().next() {
            let run = trimmed.bytes().take_while(|&b| b == marker).count();
            if run >= 3 {
                match fence {
                    // A fence closes only on its own marker, at its own
                    // length or longer, with nothing following. A shorter
                    // run or the other family is content -- closing on
                    // either reopens the body mid-block.
                    Some((open, len))
                        if marker == open && run >= len && trimmed[run..].trim().is_empty() =>
                    {
                        fence = None;
                    }
                    None => fence = Some((marker, run)),
                    Some(_) => {}
                }
                continue;
            }
        }
        // A fenced `# ` is sample text; matching it fabricates a mismatch
        // against a number that is not a record number.
        if fence.is_some() || !line.starts_with("# ") {
            continue;
        }

        let text = line[2..].trim();
        // Skip, never abandon: a bare `#` is not a title, but a real one may
        // follow it, and reporting "no H1 found" over it hides the mismatch
        // this rule exists to find (BUG-64).
        if text.is_empty() {
            continue;
        }
        return Some(H1 {
            line: lineno,
            text,
            first_token: text.split_whitespace().next().unwrap_or_default(),
        });
    }

    None
}

/// The text reaches the JSON contract, and a foreign corpus's H1 has no
/// length this repo controls.
fn elided(text: &str) -> String {
    const MAX: usize = 60;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    text.chars().take(MAX).chain(['…']).collect()
}

/// `MILE-120`: a file directly in a type's `dir` whose name yields no
/// identity. Such a file is a loaded record that every identity-reading rule
/// silently skips, so nothing else says it is there.
///
/// "On pattern" is exactly "`record_id` finds an identifier": the type's
/// declared `identity.pattern` when it has one, else the `TYPE-NNNN-slug.md`
/// convention (`ADR-36`). Reusing the loader's own answer keeps the rule from
/// disagreeing with the rules it explains.
///
/// When *every* file of a type is off-pattern (a bare-number corpus, `RFC-46`
/// rejected), one finding per type on the directory lists each file with its
/// expected name, rather than one finding per file: the fix is one rename pass.
pub fn filename_identity_pattern(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_FILENAME_IDENTITY_PATTERN;
    let mut findings = Vec::new();

    let mut off_pattern: HashMap<&str, Vec<&Record>> = HashMap::new();
    let mut totals: HashMap<&str, usize> = HashMap::new();
    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            *totals.entry(record.record_type.as_str()).or_default() += 1;
            if record_id(record).is_none() {
                off_pattern
                    .entry(record.record_type.as_str())
                    .or_default()
                    .push(record);
            }
            Outcome::Examined
        },
    );

    let mut type_names: Vec<&str> = off_pattern.keys().copied().collect();
    type_names.sort();
    for type_name in type_names {
        let mut off = off_pattern.remove(type_name).unwrap_or_default();
        off.sort_by(|a, b| a.path.cmp(&b.path));
        let type_config = config.record_types.get(type_name);
        let identity = type_config
            .and_then(|t| {
                crate::document_model::declared_model(type_name, t)
                    .ok()
                    .flatten()
            })
            .and_then(|m| m.resolve().ok())
            .and_then(|m| m.identity);
        let pattern = type_config
            .and_then(|t| t.identity.as_ref())
            .and_then(|v| v.get("pattern"))
            .and_then(|v| v.as_str());
        let convention = match pattern {
            Some(p) => format!("the declared identity pattern '{p}'"),
            None => "the 'TYPE-NNNN-slug.md' convention".to_string(),
        };
        let guidance = |record: &Record| -> String {
            let name = file_name_of(record);
            let expected = expected_name(&record.type_prefix, &name, identity.as_ref());
            format!("{name} -> {expected}")
        };

        if off.len() == totals.get(type_name).copied().unwrap_or(0) {
            let dir = type_config.map_or_else(
                || {
                    off[0]
                        .path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_default()
                },
                |t| std::path::PathBuf::from(&t.dir),
            );
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: dir,
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new("identity-pattern-none-match"),
                message: format!(
                    "none of record type '{type_name}''s {} file(s) matches {convention}, so \
                     no identity-reading rule can examine any of them; rename each: {}",
                    off.len(),
                    off.iter()
                        .map(|r| guidance(r))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            });
        } else {
            for record in off {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("identity-pattern-mismatch"),
                    message: format!(
                        "file name does not match {convention} for record type '{type_name}', \
                         so it has no identity and identity-reading rules skip it; rename: {}",
                        guidance(record)
                    ),
                });
            }
        }
    }

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

fn file_name_of(record: &Record) -> String {
    record
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The name a file should have. A leading run of digits is the number the
/// author already gave it (`0005-x.md` -> `ADR-0005-x.md`, padding kept, as
/// `ADR-0173` and `ADR-173` compare equal); otherwise there is no number to
/// carry and the name is a template. A candidate the type's declared pattern
/// would itself reject is not offered: the guidance must not point at another
/// off-pattern name.
fn expected_name(
    prefix: &str,
    file_name: &str,
    identity: Option<&crate::document_model::CompiledIdentity>,
) -> String {
    let stem = file_name.strip_suffix(".md").unwrap_or(file_name);
    let digits = stem.chars().take_while(char::is_ascii_digit).count();
    let (number, rest) = stem.split_at(digits);
    let rest = rest.trim_start_matches(['-', '_', ' ']);
    let (number, rest) = if number.is_empty() {
        ("NNNN", stem)
    } else {
        (number, rest)
    };
    let candidate = if rest.is_empty() {
        format!("{prefix}-{number}.md")
    } else {
        format!("{prefix}-{number}-{rest}.md")
    };
    match identity {
        Some(declared)
            if declared
                .identify(&candidate.replacen("NNNN", "0001", 1))
                .number
                .is_none() =>
        {
            "(a name the declared pattern accepts)".to_string()
        }
        _ => candidate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn filename_title_consistency_skips_a_legacy_shaped_filename() {
        // No type prefix means no derivable filename_number -- the rule has
        // nothing to compare a title against, so it skips rather than
        // silently accepting or falsely flagging it (BUG-9/ADR-36 amendment).
        let new_style = record(
            "docs/adr/ADR-36-x.md",
            "adr",
            "---\nStatus: Accepted\n---\n# 36 — X\n",
        );
        let legacy_style = record(
            "docs/adr/0037-y.md",
            "adr",
            "---\nStatus: Accepted\n---\n# 37 — Y\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            new_style.path.clone(),
            "---\nStatus: Accepted\n---\n# 36 — X\n".to_string(),
        );
        full_text.insert(
            legacy_style.path.clone(),
            "---\nStatus: Accepted\n---\n# 37 — Y\n".to_string(),
        );
        let (exec, findings) = filename_title_consistency(&[new_style, legacy_style], &full_text);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    /// `filename_number` derives from `record_id` rather than re-deriving
    /// the same filename grammar independently -- a hyphenated type prefix
    /// (`DOC-ADR`, `BUG-111`'s shape) must still isolate the trailing
    /// numeric segment correctly.
    #[test]
    fn filename_title_consistency_reads_the_number_past_a_hyphenated_prefix() {
        let r = record_with_prefix(
            "docs/adr/DOC-ADR-2-x.md",
            "adr",
            "DOC-ADR",
            "---\nStatus: Accepted\n---\n# 2 — X\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "---\nStatus: Accepted\n---\n# 2 — X\n".to_string(),
        );
        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn filename_title_consistency_still_catches_a_real_mismatch_on_the_new_shape() {
        let r = record(
            "docs/adr/ADR-0036-x.md",
            "adr",
            "---\nStatus: Accepted\n---\n# 0099 — Wrong Number\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "---\nStatus: Accepted\n---\n# 0099 — Wrong Number\n".to_string(),
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("36") && findings[0].message.contains("99"));
    }

    #[test]
    fn filename_title_consistency_distinguishes_an_unnumbered_h1_observed_failing() {
        // An H1 that exists but carries no number is a different defect from
        // no H1 at all -- "add a title" vs. "this corpus numbers its records
        // somewhere other than the H1" -- so it cannot share their message.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-7-x.md",
            "---\nStatus: Accepted\n---\n# Add Status Field\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].message.contains("no H1 title found"),
            "an H1 that is present must not be reported as absent, got: {}",
            findings[0].message
        );
        assert!(
            findings[0].message.contains("Add Status Field"),
            "expected the title quoted back, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn a_date_or_version_title_is_not_an_id_observed_failing() {
        // The leading token of `# 2026-09-16 Notes` held digits, and filtering
        // everything else out of it made the number 20260916, a number the
        // title never carried.
        for title in ["2026-09-16 Notes", "v1.2 Release", "2026-09 Notes", "Notes"] {
            let (r, full_text) = yaml_record(
                "docs/adr/ADR-7-x.md",
                &format!("---\nStatus: Accepted\n---\n# {title}\n"),
            );
            let (_, findings) = filename_title_consistency(&[r], &full_text);
            assert_eq!(findings.len(), 1, "{title}: {findings:?}");
            let m = &findings[0].message;
            assert!(m.contains("carries no record id"), "{title}: {m}");
            assert!(!m.contains("claims"), "{title}: {m}");
        }
    }

    #[test]
    fn a_title_id_is_a_leading_prefix_n_or_a_bare_n_when_no_pattern_is_declared() {
        for title in [
            "7 — X",
            "0007 — X",
            "ADR-7 — X",
            "ADR-0007: X",
            "7. X",
            "DOC-ADR-7 X",
        ] {
            let (r, full_text) = yaml_record(
                "docs/adr/ADR-7-x.md",
                &format!("---\nStatus: Accepted\n---\n# {title}\n"),
            );
            let (_, findings) = filename_title_consistency(&[r], &full_text);
            assert!(findings.is_empty(), "{title}: {findings:?}");
        }
    }

    #[test]
    fn a_title_number_too_large_to_compare_is_reported_as_overflow_not_as_absent_observed_failing()
    {
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-7-x.md",
            "---\nStatus: Accepted\n---\n# 99999999999999999999999 — X\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("too large to compare"),
            "{}",
            findings[0].message
        );
        assert!(!findings[0].message.contains("carries no"));
    }

    #[test]
    fn a_declared_pattern_reads_the_title_id_as_its_prefix_and_a_number() {
        let padded = r"^(?P<prefix>SPEC)-(?P<number>\d{4})-.+$";
        // The declared prefix and a number, padded or not, are the id.
        assert!(declared_title("d/SPEC-0003-x.md", padded, "# SPEC-3 — X\n").is_empty());
        assert!(declared_title("d/SPEC-0003-x.md", padded, "# SPEC-0003: X\n").is_empty());
        // A date is not an id, and is not read as 20260916.
        let f = declared_title("d/SPEC-0003-x.md", padded, "# 2026-09-16 Notes\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("'SPEC-N'"), "{}", f[0].message);
        assert!(!f[0].message.contains("claims"), "{}", f[0].message);
        // Another type's prefix is not this type's id.
        let f = declared_title("d/SPEC-0003-x.md", padded, "# ADR-3 — X\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("'SPEC-N'"), "{}", f[0].message);
        // A real mismatch is still a mismatch.
        let f = declared_title("d/SPEC-0003-x.md", padded, "# SPEC-4 — X\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("claims 4"), "{}", f[0].message);
    }

    #[test]
    fn filename_title_consistency_reports_the_real_h1_line_observed_failing() {
        // Every record in this corpus carries frontmatter, so an H1 is never
        // on line 1 and a hardcoded line points the reader at the wrong place.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-36-x.md",
            "---\nStatus: Accepted\n---\n# 99 — Wrong Number\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, Some(4));
    }

    #[test]
    fn filename_title_consistency_ignores_an_h1_inside_a_fenced_block_observed_failing() {
        // Without fence tracking this reports a mismatch against `1` from a
        // shell snippet -- a fabricated finding about a number that is not a
        // record number, which is worse than saying nothing.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-8-x.md",
            "---\nStatus: Accepted\n---\nProse, no heading.\n\n```sh\n# 1. install\n```\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("no H1 title found"),
            "a fenced heading is not a title, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn filename_title_consistency_closes_a_fence_only_on_its_own_marker_observed_failing() {
        // A fence closes on its own marker at its own length or longer. A
        // shorter run, or the other marker family, is content -- treating
        // either as a close reopens the body and the sample heading inside
        // it becomes the title.
        for body in [
            "````md\n```\n# 1. sample\n```\n````\n",
            "```md\n~~~\n# 1. sample\n~~~\n```\n",
        ] {
            let (r, full_text) = yaml_record(
                "docs/adr/ADR-8-x.md",
                &format!("---\nStatus: Accepted\n---\nProse.\n\n{body}"),
            );
            let (_, findings) = filename_title_consistency(&[r], &full_text);
            assert_eq!(findings.len(), 1);
            assert!(
                findings[0].message.contains("no H1 title found"),
                "nested fence {body:?} leaked a heading: {}",
                findings[0].message
            );
        }
    }

    #[test]
    fn filename_title_consistency_keeps_the_absent_h1_message() {
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-9-x.md",
            "---\nStatus: Accepted\n---\nNo heading anywhere.\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "no H1 title found to check against the filename's number"
        );
    }

    #[test]
    fn filename_title_consistency_treats_an_empty_h1_as_absent() {
        let (r, full_text) = yaml_record("docs/adr/ADR-9-x.md", "---\nStatus: Accepted\n---\n# \n");
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("no H1 title found"));
    }

    #[test]
    fn an_empty_h1_skips_that_line_and_keeps_looking() {
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-7-x.md",
            "---\nStatus: Accepted\n---\n# \n\n# ADR-9 — Wrong number\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].message.contains("no H1 title found"),
            "a real H1 follows the empty one: {}",
            findings[0].message
        );
    }

    #[test]
    fn filename_title_mismatch_is_a_planted_violation_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "# 0002 — Wrong Number\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0002 — Wrong Number\n".to_string());

        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
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
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    fn declared_title(path: &str, pattern: &str, content: &str) -> Vec<Finding> {
        let m = crate::document_model::RecordModel {
            identity: Some(
                crate::document_model::CompiledIdentity::compile(
                    &crate::document_model::IdentityDecl {
                        from: crate::document_model::IdentitySource::Filename,
                        pattern: pattern.to_string(),
                    },
                )
                .unwrap(),
            ),
            fields: None,
        };
        let r = Record::parse_with_model(
            std::path::PathBuf::from(path),
            "adr".to_string(),
            content,
            crate::header::HeaderShape::default(),
            "ADR".to_string(),
            &m,
        );
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        filename_title_consistency(&[r], &full_text).1
    }

    const PREFIXED: &str = r"^(?P<prefix>SPEC)-(?P<number>\d+)-.+$";

    #[test]
    fn a_declared_prefix_title_form_is_required() {
        assert!(declared_title("d/SPEC-3-x.md", PREFIXED, "# SPEC-3 — X\n").is_empty());
        let f = declared_title("d/SPEC-3-x.md", PREFIXED, "# 3 — X\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("'SPEC-N'"), "{}", f[0].message);
    }

    #[test]
    fn an_undeclared_type_keeps_accepting_any_title_form() {
        let r = record("docs/adr/ADR-98-x.md", "adr", "# ADR-98 — X\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# ADR-98 — X\n".to_string());
        assert!(filename_title_consistency(&[r], &full_text).1.is_empty());
    }

    fn adr_config(identity: Option<&str>) -> Config {
        let mut t = type_config(None);
        t.dir = "docs/adr".to_string();
        t.prefix = Some("ADR".to_string());
        t.identity = identity
            .map(|p| yaml_serde::from_str(&format!("{{from: filename, pattern: '{p}'}}")).unwrap());
        config_with_types(vec![("adr", t)])
    }

    fn adr_record(config: &Config, name: &str) -> Record {
        let model = crate::document_model::declared_model("adr", &config.record_types["adr"])
            .unwrap()
            .map(|m| m.resolve().unwrap())
            .unwrap_or_default();
        Record::parse_with_model(
            PathBuf::from(format!("docs/adr/{name}")),
            "adr".to_string(),
            "# 1 - X\n",
            crate::header::HeaderShape::default(),
            "ADR".to_string(),
            &model,
        )
    }

    #[test]
    fn one_off_pattern_file_among_conforming_ones_is_one_finding_observed_failing() {
        let config = adr_config(None);
        let records: Vec<Record> = ["ADR-1-a.md", "ADR-0002-b.md", "draft.md"]
            .iter()
            .map(|n| adr_record(&config, n))
            .collect();
        let (exec, findings) = filename_identity_pattern(&records, &config);
        assert_eq!(examined(&exec), 3, "the census counts every file");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("docs/adr/draft.md"));
        assert!(
            findings[0].message.contains("ADR-NNNN-draft.md"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_whole_bare_number_corpus_is_one_finding_per_type_with_every_expected_name() {
        let config = adr_config(None);
        let records: Vec<Record> = ["0001-use-a-queue.md", "0002-use-a-stream.md"]
            .iter()
            .map(|n| adr_record(&config, n))
            .collect();
        let (exec, findings) = filename_identity_pattern(&records, &config);
        assert_eq!(examined(&exec), 2);
        assert_eq!(
            findings.len(),
            1,
            "once per type, not per file: {findings:?}"
        );
        assert_eq!(findings[0].file, PathBuf::from("docs/adr"));
        let m = &findings[0].message;
        assert!(
            m.contains("0001-use-a-queue.md -> ADR-0001-use-a-queue.md"),
            "{m}"
        );
        assert!(
            m.contains("0002-use-a-stream.md -> ADR-0002-use-a-stream.md"),
            "{m}"
        );
    }

    #[test]
    fn a_declared_pattern_decides_what_is_off_pattern() {
        let config = adr_config(Some(r"^(?P<prefix>ADR)_(?P<number>\d+)$"));
        let records: Vec<Record> = ["ADR_1.md", "ADR-2-x.md"]
            .iter()
            .map(|n| adr_record(&config, n))
            .collect();
        let (_, findings) = filename_identity_pattern(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("docs/adr/ADR-2-x.md"));
        assert!(
            findings[0].message.contains("identity pattern"),
            "names the pattern: {findings:?}"
        );
    }

    #[test]
    fn a_bare_number_corpus_under_a_declared_pattern_gets_a_rename_that_matches_it() {
        let config = adr_config(Some(r"^(?P<prefix>ADR)-(?P<number>\d+)-(?P<slug>.+)$"));
        let records = vec![adr_record(&config, "0005-x.md")];
        let (_, findings) = filename_identity_pattern(&records, &config);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("0005-x.md -> ADR-0005-x.md"));
    }

    #[test]
    fn a_conforming_corpus_and_an_empty_dir_are_silent_but_still_counted() {
        let config = adr_config(None);
        let records = vec![adr_record(&config, "ADR-1-a.md")];
        let (exec, findings) = filename_identity_pattern(&records, &config);
        assert!(findings.is_empty());
        assert_eq!(examined(&exec), 1);
        let (exec, findings) = filename_identity_pattern(&[], &config);
        assert!(findings.is_empty());
        assert_eq!(examined(&exec), 0);
        assert!(
            exec.population.is_some(),
            "ran on nothing is reported, not hidden"
        );
    }

    #[test]
    fn a_hyphenated_type_prefix_is_kept_whole() {
        let mut t = type_config(None);
        t.dir = "docs/adr".to_string();
        t.prefix = Some("DOC-ADR".to_string());
        let config = config_with_types(vec![("adr", t)]);
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/7-x.md"),
            "adr".to_string(),
            "",
            crate::header::HeaderShape::default(),
            "DOC-ADR".to_string(),
        );
        let (_, findings) = filename_identity_pattern(&[r], &config);
        assert!(
            findings[0].message.contains("DOC-ADR-7-x.md"),
            "{findings:?}"
        );
    }
}
