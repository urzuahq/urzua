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
            Some(h1) => match h1.number {
                None => push(
                    Some(h1.line),
                    format!(
                        "H1 title '{}' carries no number to check against the filename's number {filename_number}",
                        elided(h1.text)
                    ),
                ),
                Some(n) if n != filename_number => push(
                    Some(h1.line),
                    format!(
                        "filename claims number {filename_number}, but the H1 title '{}' claims {n}",
                        elided(h1.text)
                    ),
                ),
                Some(_) => {
                    if let Some(declared) = &record.identity {
                        if let Some(problem) = title_form_problem(declared, h1.first_token) {
                            push(Some(h1.line), problem);
                        }
                    }
                }
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

/// A type that declares an `identity` pattern also declares its title form
/// (RFC-44): `PREFIX-N` when the pattern captures a `prefix`, a bare `N`
/// otherwise. `None` when the title's first word has that form. A trailing
/// `.` or `:` is punctuation (`# 1. Title`), not part of the number.
fn title_form_problem(
    declared: &crate::document_model::DeclaredIdentity,
    first_token: &str,
) -> Option<String> {
    let token = first_token.trim_end_matches(['.', ':']);
    let (expected, number_part) = match &declared.title_prefix {
        Some(prefix) => (
            format!("'{prefix}-N'"),
            token.strip_prefix(&format!("{prefix}-")),
        ),
        None => ("a bare number".to_string(), Some(token)),
    };
    if number_part.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    Some(format!(
        "H1 title starts '{}', but this type declares its title form as {expected}",
        elided(first_token)
    ))
}

/// Whether an H1 exists and whether it carries a number are independent
/// questions, so they stay independent `Option`s rather than collapsing into
/// one absent value.
struct H1<'a> {
    /// 1-indexed, matching [`crate::header::Header::region`].
    line: usize,
    text: &'a str,
    /// The first whitespace-separated word of the title.
    first_token: &'a str,
    /// Compared by numeric value (BUG-0002), so padding never mismatches.
    number: Option<u64>,
}

/// Frontmatter is skipped because a YAML comment line inside it starts with
/// `# ` and would otherwise read as the title. Only a region opening at line
/// 1 is frontmatter -- blockquote and bold-list headers sit *after* the H1,
/// where skipping past them would skip the title itself.
fn first_h1(content: &str, header_region: Option<(usize, usize)>) -> Option<H1<'_>> {
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
        let digits: String = text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .chars()
            .filter(char::is_ascii_digit)
            .collect();

        return Some(H1 {
            line: lineno,
            text,
            first_token: text.split_whitespace().next().unwrap_or_default(),
            number: digits.parse().ok(),
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
            "# 2 — X\n\n> Status: Accepted\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "# 2 — X\n\n> Status: Accepted\n".to_string(),
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

    const BARE: &str = r"^(?P<number>\d+)-.+$";
    const PREFIXED: &str = r"^(?P<prefix>SPEC)-(?P<number>\d+)-.+$";

    #[test]
    fn a_declared_bare_number_title_form_is_accepted_and_a_prefixed_one_refused() {
        assert!(declared_title("d/0098-x.md", BARE, "# 98 — X\n").is_empty());
        assert!(declared_title("d/0098-x.md", BARE, "# 98. X\n").is_empty());
        let f = declared_title("d/0098-x.md", BARE, "# ADR-98 — X\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("bare number"), "{}", f[0].message);
    }

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
}
