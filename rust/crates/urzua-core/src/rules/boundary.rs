use super::*;
use crate::config::BoundaryKind;

/// A tracked file under a declared published path, as the caller read it.
/// `content` is `None` when the file is not text (it is not valid UTF-8): the
/// rule is told so, and discloses it as unreadable rather than treating a file
/// it could not scan as one with nothing in it.
#[derive(Debug, Clone)]
pub struct PublishedFile {
    pub path: PathBuf,
    pub content: Option<String>,
}

/// Whether `line` carries the declared ignore marker, which excuses that one
/// line (`ADR-80`). The one definition the boundary rule and both back-pointer
/// rules share, so the marker means the same thing to each.
pub fn line_carries_ignore_marker(line: &str, marker: &str) -> bool {
    line.contains(marker)
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

/// Whether a path option entry is a glob (`MILE-158`): it carries `*`, `?` or
/// `[`. An entry with none is a plain prefix, compared by component exactly as
/// before.
pub fn is_path_glob(entry: &str) -> bool {
    entry.contains(['*', '?', '['])
}

/// The anchored regex for a glob entry, or why it is not one. `*` matches
/// within one path component, `**` across components (`**/` also matches no
/// directory at all), `?` one non-separator character, `[abc]` / `[!abc]` a
/// class. The match covers the path itself and anything below it, so a glob
/// naming a directory (`crates/*/tests`) covers its files, as a prefix does.
pub fn path_glob_regex(entry: &str) -> Result<regex::Regex, String> {
    let chars: Vec<char> = entry.chars().collect();
    let mut re = String::from("^(?:");
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                if chars.get(i) == Some(&'/') {
                    i += 1;
                    re.push_str("(?:.*/)?");
                } else {
                    re.push_str(".*");
                }
                continue;
            }
            '*' => re.push_str("[^/]*"),
            '?' => re.push_str("[^/]"),
            '[' => {
                let mut j = i + 1;
                let mut class = String::from("[");
                if matches!(chars.get(j), Some('!') | Some('^')) {
                    class.push('^');
                    j += 1;
                }
                let start = class.len();
                while j < chars.len() && (chars[j] != ']' || class.len() == start) {
                    if chars[j] == '\\' || chars[j] == '[' {
                        class.push('\\');
                    }
                    class.push(chars[j]);
                    j += 1;
                }
                if j >= chars.len() || class.len() == start {
                    return Err("has an unclosed or empty `[` class".to_string());
                }
                class.push(']');
                re.push_str(&class);
                i = j;
            }
            c => re.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    re.push_str(")(?:/.*)?$");
    regex::Regex::new(&re).map_err(|e| format!("is not a valid glob: {e}"))
}

/// Whether `path` is covered by a diff-rule path option entry (`MILE-158`): a
/// glob by `path_glob_regex`, anything else as the component-wise prefix
/// `path_is_published` has always been. An entry that cannot compile covers
/// nothing; config parse refuses one before it gets here.
pub fn path_matches_entry(path: &std::path::Path, entry: &str) -> bool {
    if !is_path_glob(entry) {
        return path_is_published(path, entry);
    }
    path_glob_regex(entry)
        .map(|re| re.is_match(&path.to_string_lossy()))
        .unwrap_or(false)
}

/// Where a resolved boundary was declared, so a finding about it names the
/// config key that holds the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryOrigin {
    /// The `idx`th entry of `boundaries`.
    Declared(usize),
    /// The compatible `published_paths` key, read as one published boundary.
    PublishedPaths,
}

/// A boundary as the rule reads it (`ADR-80`): the declared entry, or the
/// `published_paths` list standing in for one. The boundary's severity is the
/// level of every finding it produces; the rule's `level` only enables the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBoundary {
    pub origin: BoundaryOrigin,
    pub kind: BoundaryKind,
    /// `None` for `published_paths`, which is exempt from the required reason.
    pub reason: Option<String>,
    pub severity: FindingSeverity,
    pub paths: Vec<String>,
    pub exclude: Vec<String>,
}

impl ResolvedBoundary {
    /// Whether `path` is under one of the boundary's `paths`, exclusions
    /// ignored. The wide question evidence asks: a path an `exclude` excuses
    /// from the violation check is still inside the boundary (`ADR-80`).
    pub fn covers(&self, path: &std::path::Path) -> bool {
        self.paths.iter().any(|e| path_matches_entry(path, e))
    }

    /// Whether the violation check judges `path`: covered and not excluded.
    pub fn judges(&self, path: &std::path::Path) -> bool {
        self.covers(path) && !self.excluded(path)
    }

    fn excluded(&self, path: &std::path::Path) -> bool {
        self.exclude.iter().any(|e| path_matches_entry(path, e))
    }

    /// The first `paths` entry covering `path`, for a message.
    fn entry_for(&self, path: &std::path::Path) -> &str {
        self.paths
            .iter()
            .find(|e| path_matches_entry(path, e))
            .map_or("", String::as_str)
    }

    fn config_key(&self, tail: &str) -> Vec<String> {
        let mut key = vec![
            "rules".to_string(),
            RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE.to_string(),
        ];
        match self.origin {
            BoundaryOrigin::Declared(idx) => {
                key.extend(["boundaries".to_string(), idx.to_string(), tail.to_string()])
            }
            BoundaryOrigin::PublishedPaths => key.push("published_paths".to_string()),
        }
        key
    }
}

/// The declared boundaries (`ADR-80`, `MILE-125`, `RFC-11` sections 1 to 3).
///
/// A boundary names paths and a kind; the prohibitions follow from the kind:
/// a record identifier inside a `published` boundary is a violation (any
/// identifier carrying a declared type prefix, resolvable or not), a
/// back-pointer inside a `generated` one is reported, and a `vendored` one is
/// declared with no prohibition. A boundary's severity is the level of its
/// findings.
///
/// Violation and evidence are separate questions about the same text
/// (`RFC-11` section 3): this uses the wide identifier matcher, and the
/// evidence collector keeps its narrow structured one and skips files under a
/// `published` boundary. Neither matcher is ever the other.
///
/// Exceptions are declared. A boundary's `exclude` globs remove paths from the
/// violation check; the declared `ignore_marker` excuses the one line it is on.
/// A prohibition fails closed (`ADR-9`): a boundary path covering no judged
/// file, an `exclude` entry matching nothing and a marker suppressing nothing
/// are each reported, never passed. A file that is not text is counted as
/// unreadable, not clean.
///
/// `files` is every tracked file the caller read under a boundary path. One
/// excluded from every boundary covering it, or covered only by a `vendored`
/// one, is eligible and unexamined; one under no boundary is out of scope.
pub fn published_path_reference(
    files: &[PublishedFile],
    boundaries: &[ResolvedBoundary],
    ignore_marker: Option<&str>,
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    let prefixes = declared_prefixes(config);
    let mut findings = Vec::new();

    // Files are not records, so none is offered to `records_read_by_any_rule`:
    // that number counts records, and a published source file inflating it
    // would put a second denominator behind the same name (BUG-40).
    let population = census(PopulationUnit::File, files.iter().collect(), |file| {
        // Every boundary judging the file; a file under none is out of scope.
        let covering: Vec<&ResolvedBoundary> =
            boundaries.iter().filter(|b| b.judges(&file.path)).collect();
        if covering.is_empty() {
            // Excused by every boundary covering it: eligible and unexamined,
            // never out of scope, which `config.scope-matches-nothing` would
            // read as a declaration that reaches nothing (the diff rules'
            // `exclude_paths` do the same).
            return if boundaries.iter().any(|b| b.covers(&file.path)) {
                Outcome::Absent
            } else {
                Outcome::OutOfScope
            };
        }
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        let scan = FileScan {
            path: &file.path,
            covering: &covering,
            prefixes: &prefixes,
            ignore_marker,
        };
        scan.scan(content, &mut findings);
        // A vendored boundary declares the path and prohibits nothing, so a
        // file only it covers was not judged: eligible, not examined.
        if covering.iter().any(|b| b.kind != BoundaryKind::Vendored) {
            Outcome::Examined
        } else {
            Outcome::Absent
        }
    });

    findings.extend(boundary_declaration_findings(
        files,
        boundaries,
        config_path,
    ));

    (
        RuleExecution {
            rule: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// A finding of the boundary rule, at the boundary's own severity.
fn boundary_finding(
    b: &ResolvedBoundary,
    file: &std::path::Path,
    line: Option<usize>,
    message: String,
    source: ActionSource,
) -> Finding {
    Finding {
        rule: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE.to_string(),
        severity: b.severity,
        file: file.to_path_buf(),
        line,
        message,
        waived: None,
        declared_severity: Some(b.severity),
        suggested_action: None,
        action_source: source,
    }
}

fn boundary_reason_note(b: &ResolvedBoundary) -> String {
    b.reason
        .as_deref()
        .map_or(String::new(), |r| format!(" (declared because: {r})"))
}

/// Overlapping boundaries can each report the same citation on one line; one
/// finding per citation is kept, at the highest severity.
fn keep_highest(here: &mut Vec<(String, Finding)>, key: String, f: Finding) {
    match here.iter_mut().find(|(k, _)| *k == key) {
        Some((_, kept)) => {
            if f.severity.rank() > kept.severity.rank() {
                *kept = f;
            }
        }
        None => here.push((key, f)),
    }
}

/// One judged file read line by line: the boundaries judging it and the
/// declared inputs every line is read against.
struct FileScan<'a> {
    path: &'a std::path::Path,
    covering: &'a [&'a ResolvedBoundary],
    prefixes: &'a [String],
    ignore_marker: Option<&'a str>,
}

impl FileScan<'_> {
    fn scan(&self, content: &str, findings: &mut Vec<Finding>) {
        // The caller found at least one boundary judging the file.
        let first = self.covering[0];
        for (idx, line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let here = self.line_findings(line, line_no);
            match self.ignore_marker {
                Some(marker) if line_carries_ignore_marker(line, marker) => {
                    if here.is_empty() {
                        findings.push(boundary_finding(
                            first,
                            self.path,
                            Some(line_no),
                            format!(
                                "the ignore marker '{marker}' suppresses nothing: no boundary finding is on this line; remove the marker"
                            ),
                            ActionSource::new("boundary-marker-unused"),
                        ));
                    }
                }
                _ => findings.extend(here.into_iter().map(|(_, f)| f)),
            }
        }
    }

    fn line_findings(&self, line: &str, line_no: usize) -> Vec<(String, Finding)> {
        let mut here: Vec<(String, Finding)> = Vec::new();
        for b in self.covering {
            match b.kind {
                BoundaryKind::Published => self.published_citations(b, line, line_no, &mut here),
                BoundaryKind::Generated => self.generated_pointers(b, line, line_no, &mut here),
                BoundaryKind::Vendored => {}
            }
        }
        here
    }

    fn published_citations(
        &self,
        b: &ResolvedBoundary,
        line: &str,
        line_no: usize,
        here: &mut Vec<(String, Finding)>,
    ) {
        let entry = b.entry_for(self.path);
        for identifier in cited_identifiers(line, self.prefixes) {
            keep_highest(
                here,
                format!("cite {identifier}"),
                boundary_finding(
                    b,
                    self.path,
                    Some(line_no),
                    format!(
                        "cites {identifier}, a record identifier no reader outside the \
                     repository can resolve, from inside the published path '{entry}'{}",
                        boundary_reason_note(b)
                    ),
                    ActionSource::new("published-path-reference"),
                ),
            );
        }
    }

    fn generated_pointers(
        &self,
        b: &ResolvedBoundary,
        line: &str,
        line_no: usize,
        here: &mut Vec<(String, Finding)>,
    ) {
        let entry = b.entry_for(self.path);
        for marker in [MARKER_IMPLEMENTS, MARKER_VERIFIES] {
            for pointer in scan_marker_line(line, self.prefixes, marker) {
                keep_highest(here, format!("pointer {marker} {pointer}"), boundary_finding(
                    b,
                    self.path,
                    Some(line_no),
                    format!(
                        "carries the back-pointer '{marker} {pointer}' inside the generated path '{entry}', where a human-authored marker is overwritten{}",
                        boundary_reason_note(b)
                    ),
                    ActionSource::new("generated-back-pointer"),
                ));
            }
        }
    }
}

/// A prohibition fails closed (ADR-9): an entry covering no judged file
/// leaves the rule nothing to judge, which is indistinguishable from
/// compliance unless it is said. Usually a directory renamed or removed
/// under the declaration, or every file under it excluded. An exclusion
/// matching no file inside its own boundary excuses nothing.
fn boundary_declaration_findings(
    files: &[PublishedFile],
    boundaries: &[ResolvedBoundary],
    config_path: &std::path::Path,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for b in boundaries {
        for entry in &b.paths {
            let judged = files
                .iter()
                .any(|f| path_matches_entry(&f.path, entry) && !b.excluded(&f.path));
            if judged {
                continue;
            }
            let (variant, message) = match b.origin {
                BoundaryOrigin::PublishedPaths => (
                    "published-path-uncovered",
                    format!(
                        "published path '{entry}' covers no tracked file, so nothing under it was \
                         checked; correct the path"
                    ),
                ),
                BoundaryOrigin::Declared(_) => (
                    "boundary-uncovered",
                    format!(
                        "{} boundary path '{entry}' covers no tracked file that is not excluded, so nothing under it was \
                         checked; correct the path{}",
                        b.kind.as_str(),
                        boundary_reason_note(b)
                    ),
                ),
            };
            findings.push(boundary_finding(
                b,
                std::path::Path::new(entry),
                None,
                message,
                ActionSource::new(variant)
                    .file(config_path)
                    .key_path(b.config_key("paths")),
            ));
        }
        for entry in &b.exclude {
            if files
                .iter()
                .any(|f| b.covers(&f.path) && path_matches_entry(&f.path, entry))
            {
                continue;
            }
            findings.push(boundary_finding(
                b,
                std::path::Path::new(entry),
                None,
                format!(
                    "exclude entry '{entry}' of the {} boundary matches no tracked file inside it, so it excuses nothing; remove the entry or correct the path{}",
                    b.kind.as_str(),
                    boundary_reason_note(b)
                ),
                ActionSource::new("boundary-exclude-unused")
                    .file(config_path)
                    .key_path(b.config_key("exclude"))
                    .text(entry.as_str()),
            ));
        }
    }
    findings
}

/// The record identifiers a back-pointer comment on `line` cites with `marker`:
/// the evidence collector's own narrow reader, so a `generated` boundary and
/// the evidence rule agree on what a back-pointer is.
fn scan_marker_line(line: &str, prefixes: &[String], marker: &str) -> Vec<String> {
    scan_marker(std::path::Path::new(""), line, prefixes, marker)
        .into_iter()
        .map(|p| p.id)
        .collect()
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

    /// `published_paths: [rust/published]`, the compatible form.
    fn published() -> Vec<ResolvedBoundary> {
        compat(&["rust/published"])
    }

    fn compat(paths: &[&str]) -> Vec<ResolvedBoundary> {
        vec![ResolvedBoundary {
            origin: BoundaryOrigin::PublishedPaths,
            kind: BoundaryKind::Published,
            reason: None,
            severity: FindingSeverity::Error,
            paths: paths.iter().map(|p| p.to_string()).collect(),
            exclude: Vec::new(),
        }]
    }

    fn declared(
        idx: usize,
        kind: BoundaryKind,
        severity: FindingSeverity,
        paths: &[&str],
        exclude: &[&str],
    ) -> ResolvedBoundary {
        ResolvedBoundary {
            origin: BoundaryOrigin::Declared(idx),
            kind,
            reason: Some("a test boundary".to_string()),
            severity,
            paths: paths.iter().map(|p| p.to_string()).collect(),
            exclude: exclude.iter().map(|p| p.to_string()).collect(),
        }
    }

    fn run_with(
        files: &[PublishedFile],
        boundaries: &[ResolvedBoundary],
        marker: Option<&str>,
    ) -> (RuleExecution, Vec<Finding>) {
        published_path_reference(
            files,
            boundaries,
            marker,
            &config(),
            std::path::Path::new(".urzua/config.yaml"),
        )
    }

    fn run(files: &[PublishedFile]) -> (RuleExecution, Vec<Finding>) {
        run_with(files, &published(), None)
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
        let (exec, findings) = run_with(
            &[file("rust/internal/b.rs", "// ADR-1\n")],
            &compat(&["rust/published"]),
            None,
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
            let (_, findings) = run_with(
                &[file("rust/internal/b.rs", "// ADR-1\n")],
                &compat(&[entry]),
                None,
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
            None,
            &cfg,
            std::path::Path::new(".urzua/config.yaml"),
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
    fn a_glob_entry_matches_by_component_and_a_plain_entry_is_still_a_prefix() {
        use std::path::Path;
        let m = |p: &str, e: &str| path_matches_entry(Path::new(p), e);
        // Plain prefixes are unchanged.
        assert!(m("src/a.rs", "src") && !m("srcx/a.rs", "src"));
        // `*` stays inside one component; `**` crosses them.
        assert!(m("rust/crates/a/tests/t.rs", "rust/crates/*/tests"));
        assert!(!m("rust/crates/a/b/tests/t.rs", "rust/crates/*/tests"));
        assert!(m("rust/crates/a/b/tests/t.rs", "rust/**/tests"));
        assert!(m("a/b/foo.test.ts", "**/*.test.*"));
        assert!(m("foo.test.ts", "**/*.test.*"));
        assert!(!m("a/b/foo.ts", "**/*.test.*"));
        assert!(m("src/a_test.go", "src/*_test.go") && !m("src/x/a_test.go", "src/*_test.go"));
        assert!(m("src/a1.rs", "src/a?.rs") && !m("src/a12.rs", "src/a?.rs"));
        assert!(m("src/b.rs", "src/[ab].rs") && !m("src/c.rs", "src/[ab].rs"));
        assert!(!m("src/b.rs", "src/[!b].rs"));
        // A dot is a dot, not any character.
        assert!(!m("src/aXrs", "src/a.rs*"));
        assert!(path_glob_regex("src/[ab").is_err());
        assert!(path_glob_regex("src/[]").is_err());
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

    // ---- Declared boundaries (`ADR-80`, `MILE-125`) ----

    use FindingSeverity::{Error as Err_, Warning as Warn_};

    fn published_at(sev: FindingSeverity, paths: &[&str], exclude: &[&str]) -> ResolvedBoundary {
        declared(0, BoundaryKind::Published, sev, paths, exclude)
    }

    #[test]
    fn a_boundarys_severity_is_the_level_of_its_findings() {
        let b = [
            published_at(Warn_, &["lib"], &[]),
            declared(1, BoundaryKind::Published, Err_, &["mirror"], &[]),
        ];
        let (_, findings) = run_with(
            &[
                file("lib/a.rs", "// ADR-1\n"),
                file("mirror/b.rs", "// ADR-2\n"),
            ],
            &b,
            None,
        );
        let sev = |f: &str| {
            findings
                .iter()
                .find(|x| x.file == PathBuf::from(f))
                .map(|x| x.declared_severity)
        };
        assert_eq!(sev("lib/a.rs"), Some(Some(Warn_)));
        assert_eq!(sev("mirror/b.rs"), Some(Some(Err_)));
    }

    #[test]
    fn a_published_finding_names_the_boundarys_reason() {
        let b = [published_at(Err_, &["lib"], &[])];
        let (_, findings) = run_with(&[file("lib/a.rs", "// ADR-1\n")], &b, None);
        assert!(
            findings[0].message.contains("a test boundary"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn a_glob_boundary_covers_what_it_matches_and_nothing_else() {
        let b = [published_at(Err_, &["packages/*/src"], &[])];
        let (_, findings) = run_with(
            &[
                file("packages/a/src/x.rs", "// ADR-1\n"),
                file("packages/a/tests/y.rs", "// ADR-1\n"),
            ],
            &b,
            None,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("packages/a/src/x.rs"));
    }

    #[test]
    fn a_back_pointer_inside_a_published_boundary_is_a_violation_too() {
        let b = [published_at(Err_, &["lib"], &[])];
        let (_, findings) = run_with(&[file("lib/a.rs", "// Implements: ADR-1\n")], &b, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(
            findings[0].action_source.variant,
            "published-path-reference"
        );
    }

    #[test]
    fn a_back_pointer_inside_a_generated_boundary_is_reported() {
        let b = [declared(0, BoundaryKind::Generated, Warn_, &["gen"], &[])];
        let body = "// Implements: ADR-1\n// Verifies: ADR-2, ADR-3\n";
        let (_, findings) = run_with(&[file("gen/a.rs", body)], &b, None);
        assert_eq!(findings.len(), 3, "{findings:?}");
        assert!(findings
            .iter()
            .all(|f| f.action_source.variant == "generated-back-pointer"));
        assert_eq!(findings[0].line, Some(1));
        assert_eq!(findings[0].declared_severity, Some(Warn_));
        assert!(
            findings[0].message.contains("generated path 'gen'"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn a_plain_record_id_inside_a_generated_boundary_is_not_reported() {
        let b = [declared(0, BoundaryKind::Generated, Err_, &["gen"], &[])];
        let body = "see ADR-1 and BUG-2\n// the Implements: marker in prose, ADR-3\n";
        let (exec, findings) = run_with(&[file("gen/a.rs", body)], &b, None);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_vendored_boundary_is_declared_and_prohibits_nothing() {
        let b = [declared(0, BoundaryKind::Vendored, Err_, &["vendor"], &[])];
        let body = "// Implements: ADR-1, see BUG-2\n";
        let (exec, findings) = run_with(&[file("vendor/a.rs", body)], &b, None);
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.as_ref().unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.out_of_scope()),
            (1, 0, 0)
        );
    }

    #[test]
    fn an_empty_vendored_boundary_is_still_reported_as_covering_nothing() {
        let b = [declared(0, BoundaryKind::Vendored, Warn_, &["vendor"], &[])];
        let (_, findings) = run_with(&[file("src/a.rs", "x\n")], &b, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "boundary-uncovered");
        assert_eq!(findings[0].declared_severity, Some(Warn_));
    }

    #[test]
    fn an_exclude_removes_paths_from_the_violation_check() {
        let b = [published_at(Err_, &["lib"], &["lib/fixtures"])];
        let (exec, findings) = run_with(
            &[
                file("lib/a.rs", "clean\n"),
                file("lib/fixtures/f.rs", "// ADR-1 on purpose\n"),
            ],
            &b,
            None,
        );
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.as_ref().unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.out_of_scope()),
            (2, 1, 0)
        );
    }

    #[test]
    fn an_exclude_applies_to_its_own_boundary_only() {
        let b = [
            declared(
                0,
                BoundaryKind::Published,
                Err_,
                &["lib"],
                &["lib/fixtures"],
            ),
            declared(1, BoundaryKind::Generated, Err_, &["lib/fixtures"], &[]),
        ];
        let (_, findings) = run_with(
            &[
                file("lib/a.rs", "clean\n"),
                file("lib/fixtures/f.rs", "// ADR-1\n// Implements: ADR-2\n"),
            ],
            &b,
            None,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "generated-back-pointer");
    }

    #[test]
    fn an_exclude_matching_nothing_is_reported() {
        let b = [published_at(
            Err_,
            &["lib"],
            &["lib/gone", "other/fixtures"],
        )];
        let (_, findings) = run_with(
            &[
                file("lib/a.rs", "clean\n"),
                file("other/fixtures/x.rs", "y\n"),
            ],
            &b,
            None,
        );
        let unused: Vec<_> = findings
            .iter()
            .filter(|f| f.action_source.variant == "boundary-exclude-unused")
            .collect();
        // `other/fixtures` exists but is outside the boundary: it excuses nothing.
        assert_eq!(unused.len(), 2, "{findings:?}");
        assert!(
            unused[0].message.contains("lib/gone"),
            "{}",
            unused[0].message
        );
    }

    #[test]
    fn a_boundary_path_whose_every_file_is_excluded_covers_nothing() {
        let b = [published_at(Err_, &["lib"], &["lib"])];
        let (_, findings) = run_with(&[file("lib/a.rs", "// ADR-1\n")], &b, None);
        assert!(
            findings
                .iter()
                .any(|f| f.action_source.variant == "boundary-uncovered"),
            "{findings:?}"
        );
        assert!(!findings
            .iter()
            .any(|f| f.action_source.variant == "published-path-reference"));
    }

    #[test]
    fn the_declared_marker_excuses_its_own_line_only() {
        let b = [published_at(Err_, &["lib"], &[])];
        let body = "// ADR-1 kept on purpose urzua:ok\n// ADR-2 not excused\n";
        let (_, findings) = run_with(&[file("lib/a.rs", body)], &b, Some("urzua:ok"));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(2));
    }

    #[test]
    fn no_marker_is_compiled_in() {
        let b = [published_at(Err_, &["lib"], &[])];
        let (_, findings) = run_with(&[file("lib/a.rs", "// ADR-1 urzua:ok\n")], &b, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_marker_that_suppresses_nothing_is_reported_at_the_boundarys_severity() {
        let b = [published_at(Warn_, &["lib"], &[])];
        let (_, findings) = run_with(
            &[file("lib/a.rs", "let x = 1; // urzua:ok\n")],
            &b,
            Some("urzua:ok"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].action_source.variant, "boundary-marker-unused");
        assert_eq!(findings[0].line, Some(1));
        assert_eq!(findings[0].declared_severity, Some(Warn_));
    }

    #[test]
    fn a_marker_in_an_excluded_file_is_not_read() {
        let b = [published_at(Err_, &["lib"], &["lib/fixtures"])];
        let (_, findings) = run_with(
            &[
                file("lib/a.rs", "ok\n"),
                file("lib/fixtures/f.rs", "urzua:ok\n"),
            ],
            &b,
            Some("urzua:ok"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_marker_excuses_a_generated_back_pointer_too() {
        let b = [declared(0, BoundaryKind::Generated, Err_, &["gen"], &[])];
        let (_, findings) = run_with(
            &[file("gen/a.rs", "// Implements: ADR-1 urzua:ok\n")],
            &b,
            Some("urzua:ok"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn two_boundaries_over_one_file_each_apply() {
        let b = [
            declared(0, BoundaryKind::Published, Err_, &["x"], &[]),
            declared(1, BoundaryKind::Generated, Warn_, &["x"], &[]),
        ];
        let (_, findings) = run_with(&[file("x/a.rs", "// Implements: ADR-1\n")], &b, None);
        assert_eq!(findings.len(), 2, "{findings:?}");
        let mut variants: Vec<_> = findings.iter().map(|f| f.action_source.variant).collect();
        variants.sort();
        assert_eq!(
            variants,
            ["generated-back-pointer", "published-path-reference"]
        );
    }

    #[test]
    fn overlapping_published_boundaries_report_a_citation_once_at_the_highest_severity() {
        let b = [
            declared(0, BoundaryKind::Published, Warn_, &["lib"], &[]),
            declared(1, BoundaryKind::Published, Err_, &["lib/inner"], &[]),
        ];
        let (_, findings) = run_with(&[file("lib/inner/a.rs", "// ADR-1\n")], &b, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, Err_);
        assert_eq!(findings[0].declared_severity, Some(Err_));
        // Listed in the other order, the result is the same.
        let b = [b[1].clone(), b[0].clone()];
        let (_, findings) = run_with(&[file("lib/inner/a.rs", "// ADR-1\n")], &b, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, Err_);
    }

    #[test]
    fn every_uncovered_and_unused_finding_names_the_config_key_that_holds_it() {
        let b = [
            declared(0, BoundaryKind::Published, Err_, &["lib"], &["nope"]),
            declared(1, BoundaryKind::Generated, Err_, &["gone"], &[]),
        ];
        let (_, findings) = run_with(&[file("lib/a.rs", "x\n")], &b, None);
        let key = |variant: &str| {
            findings
                .iter()
                .find(|f| f.action_source.variant == variant)
                .and_then(|f| f.action_source.target.key_path.clone())
        };
        let s = |v: &[&str]| Some(v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(
            key("boundary-exclude-unused"),
            s(&[
                "rules",
                RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
                "boundaries",
                "0",
                "exclude"
            ])
        );
        assert_eq!(
            key("boundary-uncovered"),
            s(&[
                "rules",
                RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
                "boundaries",
                "1",
                "paths"
            ])
        );
    }
}
