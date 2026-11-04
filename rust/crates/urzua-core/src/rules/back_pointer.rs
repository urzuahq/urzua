use super::*;

/// One `Implements:` comment back-pointer: the file and 1-indexed line it was
/// written on, and the record identifier it cites, as written (`MILE-128`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackPointer {
    pub file: PathBuf,
    pub line: usize,
    pub id: String,
}

/// Every back-pointer in `content`, in line order.
///
/// A back-pointer is a whole-line comment: a comment leader (`//`, `///`,
/// `//!`, `#`, `--`, `;`, `/*`, `*`, `<!--`), then the case-sensitive marker
/// `Implements:`, then one or more record identifiers (a declared type
/// prefix, a hyphen and a number) separated by commas or spaces. Reading stops
/// at the first token that is not an identifier, so prose after the ids is
/// ignored and a marker followed by no identifier is not a back-pointer. A
/// trailing comment after code is not read, and neither is a marker mid-line:
/// this matcher is deliberately narrow, because what it matches decides that
/// a record was realized (`RFC-11` section 3).
pub fn scan_back_pointers(
    path: &std::path::Path,
    content: &str,
    prefixes: &[String],
) -> Vec<BackPointer> {
    // Longest first, so `//!` and `///` are not read as `//` plus text.
    const LEADERS: &[&str] = &["<!--", "///", "//!", "//", "/*", "--", "#", ";", "*"];
    const MARKER: &str = "Implements:";

    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(leader) = LEADERS.iter().find(|l| trimmed.starts_with(**l)) else {
            continue;
        };
        let Some(rest) = trimmed[leader.len()..].trim_start().strip_prefix(MARKER) else {
            continue;
        };
        for token in rest.split(|c: char| c == ',' || c.is_whitespace()) {
            if token.is_empty() {
                continue;
            }
            let token = token.strip_suffix('.').unwrap_or(token);
            if !is_record_id(token, prefixes) {
                break;
            }
            out.push(BackPointer {
                file: path.to_path_buf(),
                line: i + 1,
                id: token.to_string(),
            });
        }
    }
    out
}

/// A declared type prefix, a hyphen and a number, and nothing else.
fn is_record_id(token: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| {
        token
            .strip_prefix(p.as_str())
            .and_then(|r| r.strip_prefix('-'))
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

/// A source file under a `source_paths` entry, as the caller read it.
pub type SourceFile = PublishedFile;

/// The paths of the records that at least one back-pointer in `files` (those
/// under a `source_paths` entry) cites, resolved through `index`. Feeds
/// `compute_embodiment` as code-tier evidence; pure, the caller read the files.
pub fn back_pointer_evidence(
    files: &[SourceFile],
    source_paths: &[String],
    config: &Config,
    index: &RecordIndex,
) -> HashSet<PathBuf> {
    let prefixes = declared_prefixes(config);
    let mut cited = HashSet::new();
    for file in files {
        if !source_paths
            .iter()
            .any(|e| path_is_published(&file.path, e))
        {
            continue;
        }
        let Some(content) = &file.content else {
            continue;
        };
        for pointer in scan_back_pointers(&file.path, content, &prefixes) {
            if let Some(record) = index.get(&crate::values::RecordId::new(&pointer.id)) {
                cited.insert(record.path.clone());
            }
        }
    }
    cited
}

/// One finding per declared path covering no tracked file: nothing under it
/// was checked, and an empty set of files reads exactly like a clean one.
fn uncovered_source_paths(
    rule: &str,
    files: &[SourceFile],
    source_paths: &[String],
) -> Vec<Finding> {
    source_paths
        .iter()
        .filter(|entry| !files.iter().any(|f| path_is_published(&f.path, entry)))
        .map(|entry| Finding {
            rule: rule.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(entry),
            line: None,
            message: format!(
                "source path '{entry}' covers no tracked file, so nothing under it was \
                 checked; remove the entry or correct the path"
            ),
            waived: None,
        })
        .collect()
}

/// Rule (`MILE-128`): a back-pointer comment citing a record that does not
/// exist. The population is the source files under the declared paths; a
/// declared path covering no file fails closed, as a prohibition does
/// elsewhere, because an empty set of files reads exactly like a clean one.
pub fn embodiment_back_pointer_resolution(
    files: &[SourceFile],
    source_paths: &[String],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_BACK_POINTER_RESOLUTION;
    let prefixes = declared_prefixes(config);
    let mut findings = Vec::new();

    let population = census(PopulationUnit::File, files.iter().collect(), |file| {
        if !source_paths
            .iter()
            .any(|e| path_is_published(&file.path, e))
        {
            return Outcome::OutOfScope;
        }
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        for pointer in scan_back_pointers(&file.path, content, &prefixes) {
            if !index.contains_key(&crate::values::RecordId::new(&pointer.id)) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: pointer.file,
                    line: Some(pointer.line),
                    message: format!(
                        "back-pointer cites {}, which resolves to no record",
                        pointer.id
                    ),
                    waived: None,
                });
            }
        }
        Outcome::Examined
    });

    findings.extend(uncovered_source_paths(RULE_ID, files, source_paths));

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

/// Rule (`MILE-133`): a back-pointer comment citing a record whose `Status`
/// is not an accepted one. A comment `Implements: ADR-12` says the code
/// implements that decision, so landing it while the decision is still
/// `Proposed` puts implementation ahead of the decision.
///
/// A status permits implementation when it is in the cited type's declared
/// `statuses.implementable`; a type that declares none falls back to `accepted_fallback` (the rule's
/// `accepted_statuses`), and with neither the status cannot be judged, which
/// is a finding rather than a pass. With `changed` (a `--base` run) only the
/// back-pointers in files the diff changed are judged. File granularity: a
/// changed file's untouched back-pointers are judged too.
pub fn embodiment_back_pointer_decision_accepted(
    files: &[SourceFile],
    source_paths: &[String],
    accepted_fallback: &[String],
    changed: Option<&[PathBuf]>,
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED;
    let prefixes = declared_prefixes(config);
    let declared = super::status::statuses_by_type(config);
    let mut findings = Vec::new();
    let mut push = |pointer: &BackPointer, message: String| {
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Error,
            file: pointer.file.clone(),
            line: Some(pointer.line),
            message,
            waived: None,
        });
    };

    let population = census(PopulationUnit::File, files.iter().collect(), |file| {
        if !source_paths
            .iter()
            .any(|e| path_is_published(&file.path, e))
            || changed.is_some_and(|c| !c.contains(&file.path))
        {
            return Outcome::OutOfScope;
        }
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        for pointer in scan_back_pointers(&file.path, content, &prefixes) {
            // A citation of no record is `embodiment.back-pointer-resolution`'s.
            let Some(target) = index.get(&crate::values::RecordId::new(&pointer.id)) else {
                continue;
            };
            let status_field = relation_field_name(
                config,
                &target.record_type,
                crate::config::RelationRole::Status,
            );
            let status = target.header.get(status_field).map(str::trim).unwrap_or("");
            // The type's own `implementable` list, then the rule's
            // `accepted_statuses`; a terminal `success` is not "implementation
            // allowed" (the engine holds no type knowledge).
            let own = declared
                .get(&target.record_type)
                .map(|d| d.implementable.as_slice())
                .filter(|l| !l.is_empty());
            let accepted: Option<bool> = match own {
                Some(list) => Some(list.iter().any(|s| s == status)),
                None if accepted_fallback.is_empty() => None,
                None => Some(accepted_fallback.iter().any(|s| s == status)),
            };
            match accepted {
                Some(true) => {}
                Some(false) => push(
                    &pointer,
                    format!(
                        "back-pointer cites {}, whose {status_field} is '{status}', not a status \
                         at which implementation is permitted; the decision must be accepted before the code that \
                         implements it lands",
                        pointer.id
                    ),
                ),
                None => push(
                    &pointer,
                    format!(
                        "back-pointer cites {}, but type '{}' declares no implementable statuses and \
                         the rule declares no accepted_statuses, so its status cannot be judged",
                        pointer.id, target.record_type
                    ),
                ),
            }
        }
        Outcome::Examined
    });

    findings.extend(uncovered_source_paths(RULE_ID, files, source_paths));

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
    use crate::rules::tests::*;

    fn prefixes() -> Vec<String> {
        vec!["ADR".to_string(), "RFC".to_string()]
    }

    fn ids(content: &str) -> Vec<(usize, String)> {
        scan_back_pointers(std::path::Path::new("src/a.rs"), content, &prefixes())
            .into_iter()
            .map(|b| (b.line, b.id))
            .collect()
    }

    #[test]
    fn a_comment_line_with_the_marker_yields_each_cited_id_observed_failing() {
        let got = ids("fn a() {}\n// Implements: ADR-12, RFC-5\n");
        assert_eq!(
            got,
            vec![(2, "ADR-12".to_string()), (2, "RFC-5".to_string())]
        );
    }

    #[test]
    fn each_comment_leader_is_read() {
        for line in [
            "/// Implements: ADR-1",
            "//! Implements: ADR-1",
            "    # Implements: ADR-1",
            "-- Implements: ADR-1",
            " * Implements: ADR-1 */",
            "/* Implements: ADR-1 */",
            "<!-- Implements: ADR-1 -->",
            "; Implements: ADR-1",
        ] {
            assert_eq!(ids(line), vec![(1, "ADR-1".to_string())], "{line}");
        }
    }

    #[test]
    fn a_marker_that_is_not_a_whole_line_comment_is_not_read() {
        for line in [
            "let x = 1; // Implements: ADR-1",
            "Implements: ADR-1",
            "// see Implements: ADR-1",
            "// implements: ADR-1",
            "// Implements:",
            "// Implements: the thing",
            "// Implements: ADR-x",
        ] {
            assert!(ids(line).is_empty(), "{line}");
        }
    }

    #[test]
    fn reading_stops_at_the_first_token_that_is_not_an_id() {
        assert_eq!(
            ids("// Implements: ADR-1 and ADR-2"),
            vec![(1, "ADR-1".to_string())]
        );
        assert_eq!(ids("// Implements: ADR-1."), vec![(1, "ADR-1".to_string())]);
    }

    fn adr(path: &str) -> Record {
        record_with_prefix(path, "adr", "ADR", "---\nStatus: Accepted\n---\n# t\n")
    }

    fn config() -> Config {
        let mut t = type_config(None);
        t.prefix = Some("ADR".to_string());
        config_with_types(vec![("adr", t)])
    }

    fn file(path: &str, content: &str) -> SourceFile {
        SourceFile {
            path: PathBuf::from(path),
            content: Some(content.to_string()),
        }
    }

    #[test]
    fn a_back_pointer_resolves_to_the_cited_record_by_number_observed_failing() {
        let records = vec![adr("docs/adr/ADR-0012-x.md"), adr("docs/adr/ADR-13-y.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("src/a.rs", "// Implements: ADR-12\n")];
        let got = back_pointer_evidence(&files, &["src".to_string()], &config(), &index);
        assert_eq!(
            got,
            HashSet::from([PathBuf::from("docs/adr/ADR-0012-x.md")])
        );
    }

    #[test]
    fn a_file_outside_source_paths_is_not_evidence() {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("other/a.rs", "// Implements: ADR-12\n")];
        let got = back_pointer_evidence(&files, &["src".to_string()], &config(), &index);
        assert!(got.is_empty());
    }

    #[test]
    fn a_back_pointer_to_no_record_is_an_error_observed_failing() {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("src/a.rs", "ok\n// Implements: ADR-12, ADR-99\n")];
        let (exec, findings) =
            embodiment_back_pointer_resolution(&files, &["src".to_string()], &config(), &index);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(2));
        assert_eq!(findings[0].file, PathBuf::from("src/a.rs"));
        assert!(findings[0].message.contains("ADR-99"));
        let pop = exec.population.unwrap();
        assert_eq!(pop.examined(), 1);
    }

    #[test]
    fn a_source_path_covering_no_file_is_a_finding_not_a_clean_pass() {
        let (index, _) = build_index_reporting_collisions(&[]);
        let (_, findings) = embodiment_back_pointer_resolution(
            &[file("src/a.rs", "")],
            &["src".to_string(), "gone".to_string()],
            &config(),
            &index,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("gone"));
    }

    #[test]
    fn a_non_text_file_is_unreadable_not_clean() {
        let (index, _) = build_index_reporting_collisions(&[]);
        let files = vec![SourceFile {
            path: PathBuf::from("src/a.bin"),
            content: None,
        }];
        let (exec, findings) =
            embodiment_back_pointer_resolution(&files, &["src".to_string()], &config(), &index);
        assert!(findings.is_empty());
        let pop = exec.population.unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.unreadable()),
            (1, 0, 1)
        );
    }

    fn adr_with(path: &str, status: &str) -> Record {
        record_with_prefix(
            path,
            "adr",
            "ADR",
            &format!("# t\n\n> **Status:** {status}\n"),
        )
    }

    fn gate(
        records: &[Record],
        files: &[SourceFile],
        fallback: &[&str],
        changed: Option<&[PathBuf]>,
        config: &Config,
    ) -> (RuleExecution, Vec<Finding>) {
        let (index, _) = build_index_reporting_collisions(records);
        let fallback: Vec<String> = fallback.iter().map(|s| s.to_string()).collect();
        embodiment_back_pointer_decision_accepted(
            files,
            &["src".to_string()],
            &fallback,
            changed,
            config,
            &index,
        )
    }

    fn declared_config() -> Config {
        let mut t = type_config(None);
        t.prefix = Some("ADR".to_string());
        t.statuses = Some(
            yaml_serde::from_str("values: [Proposed, Accepted, Rejected]\nsuccess: [Accepted]\nfailure: [Rejected]\nimplementable: [Accepted]\n")
                .unwrap(),
        );
        config_with_types(vec![("adr", t)])
    }

    #[test]
    fn a_back_pointer_to_an_unaccepted_record_is_reported_observed_failing() {
        let records = vec![
            adr_with("docs/adr/ADR-1-a.md", "Proposed"),
            adr_with("docs/adr/ADR-2-b.md", "Accepted"),
        ];
        let files = vec![file("src/a.rs", "ok\n// Implements: ADR-1, ADR-2\n")];
        let (exec, findings) = gate(&records, &files, &[], None, &declared_config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(2));
        assert!(findings[0].message.contains("ADR-1") && findings[0].message.contains("Proposed"));
        assert_eq!(exec.population.unwrap().examined(), 1);
    }

    #[test]
    fn a_rejected_record_is_not_accepted() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Rejected")];
        let files = vec![file("src/a.rs", "// Implements: ADR-1\n")];
        let (_, findings) = gate(&records, &files, &[], None, &declared_config());
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_type_without_declared_statuses_uses_the_fallback_observed_failing() {
        let records = vec![
            adr_with("docs/adr/ADR-1-a.md", "Accepted"),
            adr_with("docs/adr/ADR-2-b.md", "Draft"),
        ];
        let files = vec![file("src/a.rs", "// Implements: ADR-1, ADR-2\n")];
        let (_, findings) = gate(&records, &files, &["Accepted"], None, &config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-2"));
    }

    #[test]
    fn with_no_declared_or_fallback_statuses_the_status_cannot_be_judged_not_passed() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Accepted")];
        let files = vec![file("src/a.rs", "// Implements: ADR-1\n")];
        let (_, findings) = gate(&records, &files, &[], None, &config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("cannot be judged"));
    }

    #[test]
    fn a_citation_of_no_record_is_left_to_the_resolution_rule() {
        let files = vec![file("src/a.rs", "// Implements: ADR-9\n")];
        let (_, findings) = gate(&[], &files, &[], None, &declared_config());
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn on_a_diff_only_changed_files_are_judged_observed_failing() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Proposed")];
        let files = vec![
            file("src/a.rs", "// Implements: ADR-1\n"),
            file("src/b.rs", "// Implements: ADR-1\n"),
        ];
        let changed = vec![PathBuf::from("src/b.rs")];
        let (exec, findings) = gate(&records, &files, &[], Some(&changed), &declared_config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("src/b.rs"));
        assert_eq!(exec.population.unwrap().examined(), 1);
    }

    fn config_with_vocab(vocab: &str) -> Config {
        let mut t = type_config(None);
        t.prefix = Some("ADR".to_string());
        t.statuses = Some(yaml_serde::from_str(vocab).unwrap());
        config_with_types(vec![("adr", t)])
    }

    #[test]
    fn terminal_success_alone_does_not_permit_implementation_observed_failing() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Accepted")];
        let files = vec![file("src/a.rs", "// Implements: ADR-1\n")];
        let cfg = config_with_vocab("values: [Proposed, Accepted]\nsuccess: [Accepted]\n");
        let (_, findings) = gate(&records, &files, &[], None, &cfg);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("cannot be judged"));
    }

    #[test]
    fn a_non_terminal_implementable_status_passes_observed_failing() {
        let vocab = "values: [Planned, InProgress, Done]\nsuccess: [Done]\nimplementable: [Planned, InProgress]\n";
        let records = vec![
            adr_with("docs/adr/ADR-1-a.md", "InProgress"),
            adr_with("docs/adr/ADR-2-b.md", "Done"),
        ];
        let files = vec![file("src/a.rs", "// Implements: ADR-1, ADR-2\n")];
        let (_, findings) = gate(&records, &files, &[], None, &config_with_vocab(vocab));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-2"));
    }

    #[test]
    fn implementable_takes_precedence_over_the_rule_fallback() {
        let vocab = "values: [Proposed, Accepted]\nimplementable: [Accepted]\n";
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Proposed")];
        let files = vec![file("src/a.rs", "// Implements: ADR-1\n")];
        let (_, findings) = gate(
            &records,
            &files,
            &["Proposed"],
            None,
            &config_with_vocab(vocab),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_declared_type_without_implementable_falls_back_to_the_rule_option_observed_failing() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Accepted")];
        let files = vec![file("src/a.rs", "// Implements: ADR-1\n")];
        let cfg = config_with_vocab("values: [Proposed, Accepted]\nsuccess: [Accepted]\n");
        let (_, findings) = gate(&records, &files, &["Accepted"], None, &cfg);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
