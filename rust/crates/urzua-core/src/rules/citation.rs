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
/// Opt-in and option-free: the prefixes come from the record types the
/// repository already declares. The header region is not read (the pointer and
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

    // Longest first is not needed for correctness (the boundary test rejects a
    // match inside a longer prefix) but keeps the order of work stable.
    let mut prefixes: Vec<String> = config
        .record_types
        .iter()
        .map(|(name, cfg)| {
            cfg.prefix
                .clone()
                .unwrap_or_else(|| name.to_ascii_uppercase())
        })
        .collect();
    prefixes.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    prefixes.dedup();

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
        for (line, token) in cited_identifiers(text, record.header.region, &prefixes) {
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
            if !index.contains_key(&crate::values::RecordId::new(&c.token)) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: c.record.path.clone(),
                    line: Some(c.line),
                    waived: None,
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

/// Every `(line, identifier)` in `text` outside the header region and outside
/// fenced code blocks. Lines are 1-indexed, matching `Header::region`.
fn cited_identifiers(
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
        for token in identifiers_in_line(line, prefixes) {
            out.push((number, token));
        }
    }
    out
}

/// Identifiers standing alone in `line`: not preceded by a letter, digit, `-`,
/// `/`, `_` or `.` (so a path, a filename or a longer token is not a
/// citation), and not followed by an alphanumeric (`ADR-12x`).
fn identifiers_in_line(line: &str, prefixes: &[String]) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for prefix in prefixes {
        let needle = format!("{prefix}-");
        for (start, _) in line.match_indices(&needle) {
            let before = line[..start].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '/' | '_' | '.')) {
                continue;
            }
            let digits = &line[start + needle.len()..];
            let n = digits.chars().take_while(|c| c.is_ascii_digit()).count();
            if n == 0 {
                continue;
            }
            if digits[n..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric())
            {
                continue;
            }
            found.push((start, line[start..start + needle.len() + n].to_string()));
        }
    }
    found.sort();
    found.dedup();
    found.into_iter().map(|(_, t)| t).collect()
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

    const TARGET: (&str, &str) = ("docs/adr/ADR-12-x.md", "> Status: Accepted\n\n# 12 — X\n");

    #[test]
    fn a_citation_of_a_missing_record_is_an_error_naming_file_and_line_observed_failing() {
        let (exec, findings) = run(&[
            TARGET,
            (
                "docs/adr/ADR-1-y.md",
                "> Status: Accepted\n\n# 1 — Y\n\nSee ADR-99 for why.\n",
            ),
        ]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_CITATION_RESOLUTION);
        assert!(findings[0].file.ends_with("ADR-1-y.md"));
        assert_eq!(findings[0].line, Some(5));
        assert!(findings[0].message.contains("ADR-99"));
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn removing_the_citation_clears_the_finding() {
        let (exec, findings) = run(&[(
            "docs/adr/ADR-1-y.md",
            "> Status: Accepted\n\n# 1 — Y\n\nNothing cited.\n",
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
            "# 1 — Y\n\n> Status: Accepted\n> Implements: ADR-99\n\nBody.\n",
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
}
