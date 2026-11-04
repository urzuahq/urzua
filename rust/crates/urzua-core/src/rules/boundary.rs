use super::*;

/// A tracked file under a declared published path, as the caller read it.
/// `content` is `None` when the file is not text (it is not valid UTF-8): the
/// rule is told so, and discloses it as unreadable rather than treating a file
/// it could not scan as one with nothing in it.
#[derive(Debug, Clone)]
pub struct PublishedFile {
    pub path: PathBuf,
    pub content: Option<String>,
}

/// Whether `path` sits at or below the declared `published` entry, a path
/// prefix (never a glob) already normalised by config parse. Compared by
/// path component, so `pkg` covers `pkg/a.rs` and not `pkg2/a.rs`. The one
/// definition both the caller (choosing which files to read) and the rule
/// (choosing which declared entry covers a file) use.
pub fn path_is_published(path: &std::path::Path, published: &str) -> bool {
    let declared = std::path::Path::new(published.trim());
    // `Path::starts_with` is component-wise; an empty or `.` entry would cover
    // every path, which is a boundary around the whole repository and not a
    // declaration anyone meant.
    declared
        .components()
        .any(|c| matches!(c, std::path::Component::Normal(_)))
        && path.starts_with(declared)
}

/// The record-identifier prefixes this repository's configuration declares,
/// longest first so `DOC-ADR` is matched before `ADR`. The type's effective
/// prefix, the same one `urzua new` mints and `check` resolves with.
fn declared_prefixes(config: &Config) -> Vec<String> {
    let mut prefixes: Vec<String> = config
        .record_types
        .iter()
        .map(|(name, rt)| {
            rt.prefix
                .clone()
                .unwrap_or_else(|| name.to_ascii_uppercase())
        })
        .collect();
    prefixes.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    prefixes.dedup();
    prefixes
}

/// Every identifier on `line` that is a declared prefix, a hyphen and a
/// number, standing on its own: not the tail of a longer word (`XADR-26`), a
/// URL or path segment (`x.org/ADR-1`) or a snake_case or dotted name, and not
/// run into letters after the number (`ADR-26x`). A trailing `.` is allowed, so
/// `(ADR-1.2)` reports `ADR-1`. An identifier split across lines is not
/// detected: each line is scanned alone.
///
/// Deliberately wider than the evidence collector's structured-marker matcher
/// (`RFC-11` section 3): this answers "was a record cited here at all", prose
/// included, and must never be the matcher that decides a record was realized.
fn cited_identifiers(line: &str, prefixes: &[String]) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    'scan: while i < bytes.len() {
        // Not the tail of a word, a hyphenated name, a URL or path segment, a
        // snake_case name or a dotted one: `/`, `_` and `.` join an identifier
        // to what precedes it the way a letter, digit or `-` does.
        let standing_alone = line.is_char_boundary(i)
            && !line[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '/' | '_' | '.'));
        if standing_alone {
            for prefix in prefixes {
                let p = prefix.as_bytes();
                if !bytes[i..].starts_with(p) || bytes.get(i + p.len()) != Some(&b'-') {
                    continue;
                }
                let digits_at = i + p.len() + 1;
                let digits = bytes[digits_at..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                let end = digits_at + digits;
                if digits == 0
                    || line[end..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric)
                {
                    continue;
                }
                out.push(line[i..end].to_string());
                i = end;
                continue 'scan;
            }
        }
        i += 1;
    }
    out
}

/// A record identifier cited from inside a declared published path
/// (`RFC-11` section 1, `MILE-125`).
///
/// A published path leaves the repository, and an identifier cited in it is a
/// pointer no reader outside can resolve. Any identifier carrying a declared
/// type prefix counts, whether or not it resolves to a record here: the
/// question is "was a record cited", not "is the citation valid".
///
/// A prohibition, so it fails closed (`ADR-9`): a declared published path that
/// covers no file is reported as a finding, because an empty set of files to
/// judge reads exactly like a corpus that complies. A file that is not text is
/// counted as unreadable, never as clean.
///
/// `files` is every tracked file the caller considered; one under no declared
/// entry is out of scope. The rule's own level is the boundary's severity
/// (`RFC-11` section 2).
pub fn published_path_reference(
    files: &[PublishedFile],
    published_paths: &[String],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE;
    let prefixes = declared_prefixes(config);
    let mut findings = Vec::new();

    // The first declared entry covering a file names it in the message; a file
    // under none is out of scope, never judged.
    let covering = |path: &std::path::Path| {
        published_paths
            .iter()
            .find(|entry| path_is_published(path, entry))
    };

    let candidates: Vec<&PublishedFile> = files.iter().collect();
    // Files are not records, so none is offered to `records_read_by_any_rule`:
    // that number counts records, and a published source file inflating it
    // would put a second denominator behind the same name (BUG-40).
    let population = census(PopulationUnit::File, candidates, |file| {
        let Some(entry) = covering(&file.path) else {
            return Outcome::OutOfScope;
        };
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        for (idx, line) in content.lines().enumerate() {
            for identifier in cited_identifiers(line, &prefixes) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: file.path.clone(),
                    line: Some(idx + 1),
                    message: format!(
                        "cites {identifier}, a record identifier no reader outside the \
                             repository can resolve, from inside the published path '{entry}'"
                    ),
                    waived: None,
                });
            }
        }
        Outcome::Examined
    });

    // A prohibition fails closed (ADR-9): an entry covering no file leaves the
    // rule nothing to judge, which is indistinguishable from compliance unless
    // it is said. Usually a directory renamed or removed under the declaration.
    for entry in published_paths {
        if !files.iter().any(|f| path_is_published(&f.path, entry)) {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: PathBuf::from(entry),
                line: None,
                message: format!(
                    "published path '{entry}' covers no tracked file, so nothing under it was \
                     checked; remove the entry or correct the path"
                ),
                waived: None,
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

    fn file(path: &str, content: &str) -> PublishedFile {
        PublishedFile {
            path: PathBuf::from(path),
            content: Some(content.to_string()),
        }
    }

    fn config() -> Config {
        let mut adr = super::super::tests::type_config(None);
        adr.prefix = Some("ADR".to_string());
        // No prefix: defaults to the type name, upper-cased (BUG).
        let bug = super::super::tests::type_config(None);
        super::super::tests::config_with_types(vec![("adr", adr), ("bug", bug)])
    }

    fn published() -> Vec<String> {
        vec!["rust/published".to_string()]
    }

    fn run(files: &[PublishedFile]) -> (RuleExecution, Vec<Finding>) {
        published_path_reference(files, &published(), &config())
    }

    fn examined(exec: &RuleExecution) -> usize {
        super::super::tests::examined(exec)
    }

    #[test]
    fn a_prose_citation_in_a_published_file_is_a_finding_with_its_line() {
        let f = file(
            "rust/published/lib.rs",
            "fn a() {}\n// kept narrow, see ADR-28 for why\n",
        );
        let (_, findings) = run(&[f]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE);
        assert_eq!(findings[0].file, PathBuf::from("rust/published/lib.rs"));
        assert_eq!(findings[0].line, Some(2));
        assert!(
            findings[0].message.contains("ADR-28"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn a_structured_marker_is_a_violation_too() {
        let (_, findings) = run(&[file("rust/published/a.rs", "// Implements: ADR-42\n")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn an_identifier_that_resolves_to_nothing_is_still_a_citation() {
        let (_, findings) = run(&[file("rust/published/a.rs", "// BUG-99999 was fixed\n")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_type_with_no_declared_prefix_is_matched_by_its_upper_cased_name() {
        let (_, findings) = run(&[file("rust/published/a.rs", "// see BUG-7\n")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn several_citations_on_one_line_are_each_reported() {
        let (_, findings) = run(&[file(
            "rust/published/a.rs",
            "// (ADR-26), ADR-27 BUG-3 and `ADR-28`.\n",
        )]);
        assert_eq!(findings.len(), 4, "{findings:?}");
    }

    #[test]
    fn text_that_only_looks_like_an_identifier_is_not_a_citation() {
        let body =
            "// SHA-256 and UTF-8 are not records\n// XADR-26 ADR-26x ADR- ADR-x adr-26 BADR-1\n";
        let (exec, findings) = run(&[file("rust/published/a.rs", body)]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1, "the file was still read");
    }

    #[test]
    fn a_file_outside_every_declared_entry_is_out_of_scope_and_never_reported() {
        let (exec, findings) = run(&[
            file("rust/published/a.rs", "clean\n"),
            file("rust/internal/b.rs", "// ADR-1\n"),
            file("rust/published2/c.rs", "// ADR-2\n"),
        ]);
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.as_ref().unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.out_of_scope()),
            (3, 1, 2)
        );
    }

    #[test]
    fn the_population_reports_what_was_examined_not_only_what_was_found() {
        let (exec, findings) = run(&[
            file("rust/published/a.rs", "clean\n"),
            file("rust/published/b.rs", "also clean\n"),
        ]);
        assert!(findings.is_empty());
        assert_eq!(exec.status, RuleStatus::Ran);
        let pop = exec.population.as_ref().unwrap();
        assert_eq!(pop.unit(), PopulationUnit::File);
        assert_eq!((pop.eligible(), pop.examined()), (2, 2));
    }

    #[test]
    fn a_file_that_is_not_text_is_unreadable_not_clean() {
        let binary = PublishedFile {
            path: PathBuf::from("rust/published/logo.png"),
            content: None,
        };
        let (exec, findings) = run(&[binary, file("rust/published/a.rs", "clean\n")]);
        assert!(findings.is_empty());
        let pop = exec.population.as_ref().unwrap();
        assert_eq!((pop.unreadable(), pop.examined()), (1, 1));
    }

    #[test]
    fn a_declared_path_covering_no_file_fails_closed() {
        let (exec, findings) = published_path_reference(
            &[file("rust/internal/b.rs", "// ADR-1\n")],
            &["rust/published".to_string()],
            &config(),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("rust/published"));
        assert!(
            findings[0].message.contains("no tracked file"),
            "{}",
            findings[0].message
        );
        assert_eq!(examined(&exec), 0);
    }

    #[test]
    fn a_blank_or_root_entry_covers_nothing_and_is_reported_not_treated_as_everything() {
        for entry in [".", "./", ""] {
            let (_, findings) = published_path_reference(
                &[file("rust/internal/b.rs", "// ADR-1\n")],
                &[entry.to_string()],
                &config(),
            );
            assert_eq!(findings.len(), 1, "entry {entry:?}: {findings:?}");
            assert!(findings[0].message.contains("no tracked file"));
        }
    }

    #[test]
    fn a_longer_declared_prefix_is_matched_whole() {
        let mut doc_adr = super::super::tests::type_config(None);
        doc_adr.prefix = Some("DOC-ADR".to_string());
        let mut adr = super::super::tests::type_config(None);
        adr.prefix = Some("ADR".to_string());
        let cfg = super::super::tests::config_with_types(vec![("doc", doc_adr), ("adr", adr)]);
        let (_, findings) = published_path_reference(
            &[file("rust/published/a.rs", "// DOC-ADR-3\n")],
            &published(),
            &cfg,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("DOC-ADR-3"));
    }

    #[test]
    fn path_is_published_compares_whole_components() {
        use std::path::Path;
        assert!(path_is_published(Path::new("a/b/c.rs"), "a/b"));
        assert!(path_is_published(Path::new("a/b/c.rs"), "a/b/"));
        assert!(path_is_published(Path::new("a/b"), "a/b"));
        assert!(!path_is_published(Path::new("a/bc/d.rs"), "a/b"));
        assert!(!path_is_published(Path::new("a/b.rs"), "a/b"));
        assert!(!path_is_published(Path::new("a/b/c.rs"), "."));
    }

    #[test]
    fn an_identifier_inside_a_url_path_or_snake_case_name_is_not_standing_alone() {
        let body = "// https://x.org/ADR-1 foo_ADR-1 file.ADR-1\n";
        let (_, findings) = run(&[file("rust/published/a.rs", body)]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_identifier_run_into_a_non_ascii_letter_is_not_a_citation() {
        let (_, findings) = run(&[file("rust/published/a.rs", "// ADR-1\u{e9} and ADR-2\n")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-2"));
    }

    #[test]
    fn a_parenthesised_dotted_identifier_still_reports_the_identifier() {
        let (_, findings) = run(&[file("rust/published/a.rs", "// (ADR-1.2)\n")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("cites ADR-1,"));
    }
}
