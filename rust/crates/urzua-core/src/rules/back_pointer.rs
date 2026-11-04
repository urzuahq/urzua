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
                    declared_severity: None,
                });
            }
        }
        Outcome::Examined
    });

    for entry in source_paths {
        if !files.iter().any(|f| path_is_published(&f.path, entry)) {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: PathBuf::from(entry),
                line: None,
                message: format!(
                    "source path '{entry}' covers no tracked file, so nothing under it was \
                     checked; remove the entry or correct the path"
                ),
                waived: None,
                declared_severity: None,
            });
        }
    }

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
}
