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
/// A back-pointer is a comment: a comment leader (`//`, `///`, `//!`, `#`,
/// `--`, `;`, `/*`, `*`, `<!--`), then the case-sensitive marker
/// `Implements:`, then record identifiers (a declared type prefix, a hyphen
/// and a number). The leader either opens the line or, after code, is one of
/// the leaders that cannot appear in code as an operator (`//`, `#`, `--`,
/// `/*`, `<!--`) and follows whitespace; `;` and `*` are read only at the
/// start of a line. The marker must follow the leader directly, so prose that
/// mentions it is not a back-pointer, and neither is a marker in no comment.
///
/// The list is a run of segments separated by commas. A segment opens with an
/// identifier, may list further identifiers separated by spaces, and may then
/// carry a note (`ADR-1 (why), ADR-2`; a parenthesis may hold commas). Reading
/// stops at the first segment that does not open with an identifier, so a
/// marker followed by no identifier is not a back-pointer and prose after an
/// identifier cannot name one (`ADR-1 and ADR-2` yields `ADR-1`). The matcher
/// is deliberately narrow, because what it matches decides that a record was
/// realized (`RFC-11` section 3).
pub fn scan_back_pointers(
    path: &std::path::Path,
    content: &str,
    prefixes: &[String],
) -> Vec<BackPointer> {
    scan_marker(path, content, prefixes, MARKER_IMPLEMENTS)
}

/// The comment marker that makes a record's code evidence.
pub const MARKER_IMPLEMENTS: &str = "Implements:";
/// The comment marker that makes a record's test evidence (`MILE-146`), read
/// only in a file of the declared test class.
pub const MARKER_VERIFIES: &str = "Verifies:";

/// [`scan_back_pointers`] for any marker word; the same reading applies.
pub fn scan_marker(
    path: &std::path::Path,
    content: &str,
    prefixes: &[String],
    marker: &str,
) -> Vec<BackPointer> {
    // Longest first, so `//!` and `///` are not read as `//` plus text.
    const LEADERS: &[&str] = &["<!--", "///", "//!", "//", "/*", "--", "#", ";", "*"];
    // The leaders that open a trailing comment after code.
    const TRAILING: &[&str] = &["<!--", "//", "/*", "--", "#"];

    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let Some(rest) = marker_rest(line, LEADERS, TRAILING, marker) else {
            continue;
        };
        for id in cited_ids(rest, prefixes) {
            out.push(BackPointer {
                file: path.to_path_buf(),
                line: i + 1,
                id,
            });
        }
    }
    out
}

/// The one ignore marker both back-pointer rules and the evidence collector
/// share (`ADR-80` amendment): the one declared on
/// `embodiment.back-pointer-resolution`, else the one on
/// `embodiment.back-pointer-decision-accepted`, each only while enabled. Config
/// load refuses two different values.
pub fn back_pointer_ignore_marker(config: &Config) -> Option<String> {
    [
        RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
    ]
    .iter()
    .find_map(|rule| {
        config
            .enabled_rule_opts::<IgnoreMarkerOpts>(rule)
            .and_then(|o| o.ignore_marker)
    })
}

/// What a scan under a declared ignore marker found (`ADR-80` amendment).
#[derive(Debug, Default)]
pub struct MarkerScan {
    /// The back-pointers on lines the marker does not excuse, in line order.
    pub kept: Vec<BackPointer>,
    /// The lines that carry a back-pointer and the marker: excused.
    pub excused: std::collections::BTreeSet<usize>,
    /// Every line that carries a back-pointer, excused or not.
    pub seen: std::collections::BTreeSet<usize>,
}

/// [`scan_marker`], leaving out a back-pointer on a line that carries the
/// declared `ignore` marker. With no marker declared this is `scan_marker`
/// unchanged. The marker excuses its own line only.
pub fn scan_marker_excusing(
    path: &std::path::Path,
    content: &str,
    prefixes: &[String],
    marker: &str,
    ignore: Option<&str>,
) -> MarkerScan {
    let mut out = MarkerScan::default();
    let lines: Vec<&str> = content.lines().collect();
    for pointer in scan_marker(path, content, prefixes, marker) {
        out.seen.insert(pointer.line);
        let excused = ignore.is_some_and(|m| {
            lines
                .get(pointer.line - 1)
                .is_some_and(|l| line_carries_ignore_marker(l, m))
        });
        if excused {
            out.excused.insert(pointer.line);
        } else {
            out.kept.push(pointer);
        }
    }
    out
}

/// The 1-indexed lines that carry the ignore marker but no back-pointer the
/// rule reads (`read_markers`): a marker that excuses nothing (`ADR-80`).
fn unused_marker_lines(
    content: &str,
    prefixes: &[String],
    path: &std::path::Path,
    ignore: &str,
    read_markers: &[&str],
) -> Vec<usize> {
    let pointer_lines: std::collections::BTreeSet<usize> = read_markers
        .iter()
        .flat_map(|m| scan_marker(path, content, prefixes, m))
        .map(|p| p.line)
        .collect();
    content
        .lines()
        .enumerate()
        .filter(|(i, l)| line_carries_ignore_marker(l, ignore) && !pointer_lines.contains(&(i + 1)))
        .map(|(i, _)| i + 1)
        .collect()
}

fn unused_marker_finding(
    rule: &str,
    variant: &'static str,
    path: &std::path::Path,
    line: usize,
    ignore: &str,
) -> Finding {
    Finding {
        rule: rule.to_string(),
        severity: FindingSeverity::Error,
        file: path.to_path_buf(),
        line: Some(line),
        message: format!(
            "the ignore marker '{ignore}' suppresses nothing: no back-pointer is on this line; remove the marker"
        ),
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source: ActionSource::new(variant),
    }
}

/// The text after the marker of the first comment on `line` that carries one.
fn marker_rest<'a>(
    line: &'a str,
    leaders: &[&str],
    trailing: &[&str],
    marker: &str,
) -> Option<&'a str> {
    let trimmed = line.trim_start();
    if let Some(leader) = leaders.iter().find(|l| trimmed.starts_with(**l)) {
        if let Some(rest) = trimmed[leader.len()..].trim_start().strip_prefix(marker) {
            return Some(rest);
        }
    }
    for (at, _) in line.char_indices() {
        let here = &line[at..];
        let Some(leader) = trailing.iter().find(|l| here.starts_with(**l)) else {
            continue;
        };
        let after_space = line[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        if !after_space {
            continue;
        }
        // `///` and `//!` are `//` plus one more character.
        let tail = &here[leader.len()..];
        let tail = if *leader == "//" {
            tail.strip_prefix(['/', '!']).unwrap_or(tail)
        } else {
            tail
        };
        if let Some(rest) = tail.trim_start().strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

/// The identifiers a marker's text cites; see [`scan_back_pointers`].
// Implements: ADR-66 (a source comment is realization evidence), MILE-147
fn cited_ids(rest: &str, prefixes: &[String]) -> Vec<String> {
    let mut ids = Vec::new();
    let mut depth = 0i32; // Implements: MILE-147 (trailing marker, read like a whole-line one)
    let mut in_note = false;
    // True at the start of the list and after each top-level comma: the next
    // word must be an identifier or reading ends.
    let mut segment_start = true;
    for token in rest.split_whitespace() {
        // A word may carry commas (`ADR-1,ADR-2`, `(note),`).
        let mut words = token.split(',').peekable();
        while let Some(word) = words.next() {
            let comma_after = words.peek().is_some();
            if !word.is_empty() {
                if depth > 0 || in_note {
                    depth += paren_delta(word);
                } else {
                    let word = word.strip_suffix('.').unwrap_or(word);
                    if is_record_id(word, prefixes) {
                        ids.push(word.to_string());
                        segment_start = false;
                    } else if segment_start {
                        return ids;
                    } else {
                        in_note = true;
                        depth += paren_delta(word);
                    }
                }
            }
            if comma_after && depth <= 0 {
                depth = 0;
                in_note = false;
                segment_start = true;
            }
        }
    }
    ids
}

fn paren_delta(word: &str) -> i32 {
    word.chars().fold(0, |d, c| match c {
        '(' => d + 1,
        ')' => d - 1,
        _ => d,
    })
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

/// The records source comments cite, by the tier the citing file gives them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackPointerEvidence {
    /// `Implements:` in a file under a `back_pointer_paths` entry: code tier.
    pub code: HashSet<PathBuf>,
    /// `Verifies:` in a file under a `test_paths` entry: test tier (`MILE-146`).
    pub test: HashSet<PathBuf>,
    /// Lines a declared `ignore_marker` kept out of the evidence (`ADR-80`
    /// amendment), so a command that reports no population can still say so.
    pub excused_lines: usize,
}

impl BackPointerEvidence {
    /// Whether any back-pointer, of either tier, cites `record`.
    pub fn cites(&self, record: &std::path::Path) -> bool {
        self.code.contains(record) || self.test.contains(record)
    }
}

/// The records that at least one back-pointer in `files` (those under a
/// `source_paths` entry) cites, resolved through `index`, split by tier. A
/// `Verifies:` comment counts only in a file under a `test_paths` entry; an
/// `Implements:` comment counts as code wherever it is, so one record can be
/// cited at both tiers. Feeds `compute_embodiment`; pure, the caller read the
/// files.
pub fn back_pointer_evidence(
    files: &[SourceFile],
    source_paths: &[String],
    test_paths: &[String],
    config: &Config,
    index: &RecordIndex,
) -> BackPointerEvidence {
    let prefixes = declared_prefixes(config);
    let mut evidence = BackPointerEvidence::default();
    // A path under a `published` boundary is never evidence (`RFC-11` section 3,
    // `ADR-80`): the boundary's `paths` alone decide, so a path its `exclude`
    // excuses from the violation check does not become evidence by being
    // excused. Read only while the boundary rule is enabled.
    let published: Vec<ResolvedBoundary> = config
        .enabled_rule_opts::<PublishedPathOpts>(RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
        .map_or_else(Vec::new, |o| o.boundaries)
        .into_iter()
        .filter(|b| b.kind == crate::config::BoundaryKind::Published)
        .collect();
    // A line the declared `ignore_marker` excuses is not evidence either.
    let ignore = back_pointer_ignore_marker(config);
    let ignore = ignore.as_deref();
    for file in files {
        if !source_paths
            .iter()
            .any(|e| path_is_published(&file.path, e))
            || published.iter().any(|b| b.covers(&file.path))
        {
            continue;
        }
        let Some(content) = &file.content else {
            continue;
        };
        let implements =
            scan_marker_excusing(&file.path, content, &prefixes, MARKER_IMPLEMENTS, ignore);
        let mut excused = implements.excused.clone();
        for pointer in implements.kept {
            if let Some(record) = index.get(&crate::values::RecordId::new(&pointer.id)) {
                evidence.code.insert(record.path.clone());
            }
        }
        if is_test_file(&file.path, test_paths) {
            let verifies =
                scan_marker_excusing(&file.path, content, &prefixes, MARKER_VERIFIES, ignore);
            excused.extend(&verifies.excused);
            for pointer in verifies.kept {
                if let Some(record) = index.get(&crate::values::RecordId::new(&pointer.id)) {
                    evidence.test.insert(record.path.clone());
                }
            }
        }
        evidence.excused_lines += excused.len();
    }
    evidence
}

fn is_test_file(path: &std::path::Path, test_paths: &[String]) -> bool {
    test_paths.iter().any(|e| path_matches_entry(path, e))
}

/// One finding per declared path covering no tracked file: nothing under it
/// was checked, and an empty set of files reads exactly like a clean one.
fn uncovered_source_paths(
    rule: &str,
    variant: &'static str,
    files: &[SourceFile],
    source_paths: &[String],
    config_path: &std::path::Path,
) -> Vec<Finding> {
    uncovered_paths(
        rule,
        variant,
        config_path,
        files,
        source_paths,
        ("source path", "back_pointer_paths"),
        None,
    )
}

/// `label` names the entry in the message; `option` is the rule option it was
/// declared under, which is where the suggested edit lands.
fn uncovered_paths(
    rule: &str,
    variant: &'static str,
    config_path: &std::path::Path,
    files: &[SourceFile],
    paths: &[String],
    (label, option): (&str, &str),
    scanned: Option<&[String]>,
) -> Vec<Finding> {
    // A test-path glob can match a file the scan never reads, so it must match
    // a file under a scanned path to count as covering anything (`MILE-160`).
    paths
        .iter()
        .filter(|entry| {
            !files.iter().any(|f| {
                path_matches_entry(&f.path, entry)
                    && scanned.is_none_or(|s| s.iter().any(|b| path_is_published(&f.path, b)))
            })
        })
        .map(|entry| Finding {
            rule: rule.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(entry),
            line: None,
            message: format!(
                "{label} '{entry}' covers no tracked file, so nothing under it was \
                 checked; remove the entry or correct the path"
            ),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            // The edit lands in the configuration, not in the finding's own
            // file (the declared source path).
            action_source: ActionSource::new(variant)
                .file(config_path)
                .key_path(["rules", rule, option]),
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
    test_paths: &[String],
    config: &Config,
    config_path: &std::path::Path,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_BACK_POINTER_RESOLUTION;
    let prefixes = declared_prefixes(config);
    let mut findings = Vec::new();
    let ignore = back_pointer_ignore_marker(config);
    let ignore = ignore.as_deref();
    let mut excused_lines = 0;

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
        let verifies = if is_test_file(&file.path, test_paths) {
            scan_marker_excusing(&file.path, content, &prefixes, MARKER_VERIFIES, ignore)
        } else {
            MarkerScan::default()
        };
        let implements =
            scan_marker_excusing(&file.path, content, &prefixes, MARKER_IMPLEMENTS, ignore);
        let excused: std::collections::BTreeSet<usize> = implements
            .excused
            .iter()
            .chain(&verifies.excused)
            .copied()
            .collect();
        excused_lines += excused.len();
        if let Some(marker) = ignore {
            let read: &[&str] = if is_test_file(&file.path, test_paths) {
                &[MARKER_IMPLEMENTS, MARKER_VERIFIES]
            } else {
                &[MARKER_IMPLEMENTS]
            };
            for line in unused_marker_lines(content, &prefixes, &file.path, marker, read) {
                findings.push(unused_marker_finding(
                    RULE_ID,
                    "back-pointer-marker-unused",
                    &file.path,
                    line,
                    marker,
                ));
            }
        }
        for pointer in implements.kept.into_iter().chain(verifies.kept) {
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
                    suggested_action: None,
                    action_source: ActionSource::new("back-pointer-unresolved"),
                });
            }
        }
        Outcome::Examined
    });

    findings.extend(uncovered_source_paths(
        RULE_ID,
        "back-pointer-uncovered-path",
        files,
        source_paths,
        config_path,
    ));
    findings.extend(uncovered_paths(
        RULE_ID,
        "back-pointer-uncovered-test-path",
        config_path,
        files,
        test_paths,
        ("test path", "test_paths"),
        Some(source_paths),
    ));

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population.with_excused_lines(excused_lines)),
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
    config_path: &std::path::Path,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED;
    let prefixes = declared_prefixes(config);
    let declared = super::status::statuses_by_type(config);
    let mut findings = Vec::new();
    let ignore = back_pointer_ignore_marker(config);
    let ignore = ignore.as_deref();
    // The shared marker's unused lines are reported once: by the resolution
    // rule while it is enabled (it knows the test class), else here.
    let reports_unused = config
        .enabled_rule_opts::<IgnoreMarkerOpts>(RULE_EMBODIMENT_BACK_POINTER_RESOLUTION)
        .is_none();
    let mut excused_lines = 0;
    let mut unused: Vec<Finding> = Vec::new();
    let mut push = |variant: &'static str, pointer: &BackPointer, message: String| {
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Error,
            file: pointer.file.clone(),
            line: Some(pointer.line),
            message,
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: ActionSource::new(variant),
        });
    };

    // The diff narrows what is handed to the rule; it is not the configuration
    // failing to reach a file, so an unchanged file is left out of the
    // candidates rather than counted `OutOfScope` (which would make
    // `config.scope-matches-nothing` warn on every `--base` run).
    let candidates: Vec<&SourceFile> = files
        .iter()
        .filter(|f| changed.is_none_or(|c| c.contains(&f.path)))
        .collect();
    let population = census(PopulationUnit::File, candidates, |file| {
        if !source_paths
            .iter()
            .any(|e| path_is_published(&file.path, e))
        {
            return Outcome::OutOfScope;
        }
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        let scanned =
            scan_marker_excusing(&file.path, content, &prefixes, MARKER_IMPLEMENTS, ignore);
        excused_lines += scanned.excused.len();
        if let Some(marker) = ignore.filter(|_| reports_unused) {
            for line in
                unused_marker_lines(content, &prefixes, &file.path, marker, &[MARKER_IMPLEMENTS])
            {
                unused.push(unused_marker_finding(
                    RULE_ID,
                    "decision-marker-unused",
                    &file.path,
                    line,
                    marker,
                ));
            }
        }
        for pointer in scanned.kept {
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
                    "decision-not-implementable",
                    &pointer,
                    format!(
                        "back-pointer cites {}, whose {status_field} is '{status}', not a status \
                         at which implementation is permitted; the decision must be accepted before the code that \
                         implements it lands",
                        pointer.id
                    ),
                ),
                None => push(
                    "decision-unjudgeable",
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

    findings.extend(unused);
    findings.extend(uncovered_source_paths(
        RULE_ID,
        "decision-uncovered-path",
        files,
        source_paths,
        config_path,
    ));

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population.with_excused_lines(excused_lines)),
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

    /// Fixture text writes the markers as `@I` and `@V`, so no fixture line is
    /// itself a back-pointer that this repository's own `check` would read.
    fn expand(text: &str) -> String {
        text.replace("@I", MARKER_IMPLEMENTS)
            .replace("@V", MARKER_VERIFIES)
    }

    fn ids(content: &str) -> Vec<(usize, String)> {
        scan_back_pointers(
            std::path::Path::new("src/a.rs"),
            &expand(content),
            &prefixes(),
        )
        .into_iter()
        .map(|b| (b.line, b.id))
        .collect()
    }

    #[test]
    fn a_declared_ignore_marker_drops_only_its_own_line() {
        let text = expand("// @I ADR-1\n// @I ADR-9 skip-me\n// @I ADR-2\n");
        let path = std::path::Path::new("src/a.rs");
        let scan =
            scan_marker_excusing(path, &text, &prefixes(), MARKER_IMPLEMENTS, Some("skip-me"));
        let kept: Vec<usize> = scan.kept.iter().map(|p| p.line).collect();
        assert_eq!(kept, vec![1, 3]);
        assert_eq!(scan.excused.iter().copied().collect::<Vec<_>>(), vec![2]);
        // No marker declared: identical to the plain scan.
        let plain = scan_marker_excusing(path, &text, &prefixes(), MARKER_IMPLEMENTS, None);
        assert_eq!(plain.kept, scan_back_pointers(path, &text, &prefixes()));
        assert!(plain.excused.is_empty());
    }

    #[test]
    fn a_marker_on_a_line_without_a_back_pointer_is_unused() {
        let text = expand("// @I ADR-1 skip-me\nfn a() {} // skip-me\n");
        let unused = unused_marker_lines(
            &text,
            &prefixes(),
            std::path::Path::new("a.rs"),
            "skip-me",
            &[MARKER_IMPLEMENTS],
        );
        assert_eq!(unused, vec![2]);
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
            "    # @I ADR-1",
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
    fn a_marker_that_is_not_a_comment_marker_is_not_read() {
        for line in [
            "Implements: ADR-1",
            "// see Implements: ADR-1",
            "// implements: ADR-1",
            "// Implements:",
            "// Implements: the thing",
            "// Implements: ADR-x",
            "let s = \"Implements: ADR-1\";",
            "let x = 1; Implements: ADR-1",
            "a * Implements: ADR-1",
            "see http://x Implements: ADR-1 no",
        ] {
            assert!(ids(line).is_empty(), "{line}");
        }
    }

    #[test]
    fn a_marker_after_code_on_the_same_line_is_read_observed_failing() {
        for line in [
            "let x = 1; // @I ADR-1",
            "let x = 1; /// @I ADR-1",
            "x = 1  # @I ADR-1",
            "SELECT 1 -- @I ADR-1",
            "foo(); /* @I ADR-1 */",
            "<p>x</p> <!-- @I ADR-1 -->",
        ] {
            assert_eq!(ids(line), vec![(1, "ADR-1".to_string())], "{line}");
        }
    }

    #[test]
    fn an_id_list_continues_past_a_parenthetical_note_observed_failing() {
        let want = vec![(1, "ADR-1".to_string()), (1, "ADR-2".to_string())];
        assert_eq!(ids("// Implements: ADR-1 (the note), ADR-2"), want);
        assert_eq!(ids("// Implements: ADR-1 (a, b), ADR-2"), want);
        assert_eq!(ids("// Implements: ADR-1 -- the note, ADR-2"), want);
        assert_eq!(ids("let a = 1; // @I ADR-1 (n), ADR-2."), want);
        assert_eq!(ids("// Implements: ADR-1 ADR-2"), want);
    }

    #[test]
    fn reading_stops_at_a_segment_that_does_not_open_with_an_id() {
        assert_eq!(
            ids("// Implements: ADR-1 and ADR-2"),
            vec![(1, "ADR-1".to_string())]
        );
        assert_eq!(
            ids("// Implements: ADR-1 (note), see ADR-2"),
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
            content: Some(expand(content)),
        }
    }

    #[test]
    fn a_back_pointer_resolves_to_the_cited_record_by_number_observed_failing() {
        let records = vec![adr("docs/adr/ADR-0012-x.md"), adr("docs/adr/ADR-13-y.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("src/a.rs", "// Implements: ADR-12\n")];
        let got = back_pointer_evidence(&files, &["src".to_string()], &[], &config(), &index);
        assert_eq!(
            got.code,
            HashSet::from([PathBuf::from("docs/adr/ADR-0012-x.md")])
        );
        assert!(got.test.is_empty());
    }

    fn tiers(files: &[SourceFile], tests: &[&str]) -> BackPointerEvidence {
        let records = vec![adr("docs/adr/ADR-12-x.md"), adr("docs/adr/ADR-13-y.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let tests: Vec<String> = tests.iter().map(|t| t.to_string()).collect();
        back_pointer_evidence(files, &["src".to_string()], &tests, &config(), &index)
    }

    #[test]
    fn a_verifies_comment_in_a_test_file_is_test_tier_and_an_implements_elsewhere_is_code_observed_failing(
    ) {
        let files = vec![
            file("src/tests/a.rs", "// Verifies: ADR-12\n"),
            file("src/b.rs", "// Implements: ADR-13\n"),
        ];
        let got = tiers(&files, &["src/tests"]);
        assert_eq!(
            got.test,
            HashSet::from([PathBuf::from("docs/adr/ADR-12-x.md")])
        );
        assert_eq!(
            got.code,
            HashSet::from([PathBuf::from("docs/adr/ADR-13-y.md")])
        );
    }

    #[test]
    fn one_record_can_be_cited_at_both_tiers_observed_failing() {
        let files = vec![
            file("src/tests/a.rs", "// Verifies: ADR-12\n"),
            file("src/b.rs", "// Implements: ADR-12\n"),
        ];
        let got = tiers(&files, &["src/tests"]);
        assert_eq!(got.test, got.code);
        assert_eq!(got.test.len(), 1);
    }

    #[test]
    fn a_verifies_comment_outside_the_test_class_or_with_no_class_declared_is_not_read() {
        let files = vec![file("src/b.rs", "// Verifies: ADR-12\n")];
        assert!(tiers(&files, &["src/tests"]).test.is_empty());
        let files = vec![file("src/tests/a.rs", "// Verifies: ADR-12\n")];
        assert!(tiers(&files, &[]).test.is_empty());
        assert!(tiers(&files, &[]).code.is_empty());
    }

    #[test]
    fn a_verifies_comment_reads_the_same_widened_id_list() {
        let files = vec![file(
            "src/tests/a.rs",
            "let x = 1; // @V ADR-12 (note), ADR-13\n",
        )];
        assert_eq!(tiers(&files, &["src/tests"]).test.len(), 2);
    }

    #[test]
    fn a_verifies_citation_of_no_record_and_an_uncovered_test_path_are_findings() {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("src/tests/a.rs", "// Verifies: ADR-12, ADR-99\n")];
        let (_, findings) = embodiment_back_pointer_resolution(
            &files,
            &["src".to_string()],
            &["src/tests".to_string(), "src/gone".to_string()],
            &config(),
            std::path::Path::new(".urzua/config.yaml"),
            &index,
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings.iter().any(|f| f.message.contains("ADR-99")));
        assert!(findings
            .iter()
            .any(|f| f.message.contains("test path 'src/gone'")));
    }

    #[test]
    fn a_file_outside_source_paths_is_not_evidence() {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("other/a.rs", "// Implements: ADR-12\n")];
        let got = back_pointer_evidence(&files, &["src".to_string()], &[], &config(), &index);
        assert!(got.code.is_empty() && got.test.is_empty());
    }

    #[test]
    fn a_back_pointer_to_no_record_is_an_error_observed_failing() {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        let files = vec![file("src/a.rs", "ok\n// Implements: ADR-12, ADR-99\n")];
        let (exec, findings) = embodiment_back_pointer_resolution(
            &files,
            &["src".to_string()],
            &[],
            &config(),
            std::path::Path::new(".urzua/config.yaml"),
            &index,
        );
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
            &[],
            &config(),
            std::path::Path::new(".urzua/config.yaml"),
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
        let (exec, findings) = embodiment_back_pointer_resolution(
            &files,
            &["src".to_string()],
            &[],
            &config(),
            std::path::Path::new(".urzua/config.yaml"),
            &index,
        );
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
            &format!("---\nStatus: '{status}'\n---\n# t\n"),
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
            std::path::Path::new(".urzua/config.yaml"),
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
    fn the_gate_judges_every_id_of_a_noted_list_and_a_trailing_marker_observed_failing() {
        let records = vec![
            adr_with("docs/adr/ADR-1-a.md", "Accepted"),
            adr_with("docs/adr/ADR-2-b.md", "Proposed"),
            adr_with("docs/adr/ADR-3-c.md", "Proposed"),
        ];
        let files = vec![file(
            "src/a.rs",
            "// @I ADR-1 (note), ADR-2\nlet x = 1; // @I ADR-3\n",
        )];
        let (_, findings) = gate(&records, &files, &[], None, &declared_config());
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings[0].message.contains("ADR-2") && findings[0].line == Some(1));
        assert!(findings[1].message.contains("ADR-3") && findings[1].line == Some(2));
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

    #[test]
    fn a_file_the_diff_did_not_change_is_not_out_of_scope_observed_failing() {
        // Narrowing to the diff is not the configuration failing to reach a
        // file: reporting it as out of scope made `config.scope-matches-nothing`
        // warn on every `--base` run that touched a source file.
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Accepted")];
        let files = vec![
            file("src/a.rs", "// Implements: ADR-1\n"),
            file("src/b.rs", "// Implements: ADR-1\n"),
        ];
        let changed = vec![PathBuf::from("src/b.rs")];
        let (exec, _) = gate(&records, &files, &[], Some(&changed), &declared_config());
        assert_eq!(exec.population.unwrap().out_of_scope(), 0);
    }

    #[test]
    fn a_changed_file_outside_the_declared_paths_is_still_out_of_scope() {
        let records = vec![adr_with("docs/adr/ADR-1-a.md", "Accepted")];
        let files = vec![file("elsewhere/a.rs", "// Implements: ADR-1\n")];
        let changed = vec![PathBuf::from("elsewhere/a.rs")];
        let (exec, _) = gate(&records, &files, &[], Some(&changed), &declared_config());
        assert_eq!(exec.population.unwrap().out_of_scope(), 1);
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

    // ---- Violation and evidence are separate matchers (`RFC-11` section 3, `ADR-80`) ----

    fn config_with_published(paths: &[&str], exclude: &[&str], level: &str) -> Config {
        let mut c = config();
        let yaml = format!(
            "level: {level}\nboundaries:\n  - paths: {paths:?}\n    kind: published\n    reason: ships to users\n    severity: error\n    exclude: {exclude:?}\n"
        );
        let yaml = if exclude.is_empty() {
            yaml.replace("    exclude: []\n", "")
        } else {
            yaml
        };
        c.rules.insert(
            RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE.to_string(),
            yaml_serde::from_str(&yaml).unwrap(),
        );
        c
    }

    fn cited_by(config: &Config, files: &[SourceFile]) -> BackPointerEvidence {
        let records = vec![adr("docs/adr/ADR-12-x.md")];
        let (index, _) = build_index_reporting_collisions(&records);
        back_pointer_evidence(files, &["src".to_string()], &[], config, &index)
    }

    #[test]
    fn a_marker_inside_a_published_boundary_is_not_evidence() {
        let files = [
            file("src/pub/a.rs", "// Implements: ADR-12\n"),
            file("src/internal/b.rs", "// Implements: ADR-12\n"),
        ];
        let cfg = config_with_published(&["src/pub"], &[], "error");
        let got = cited_by(&cfg, &files);
        assert_eq!(got.code.len(), 1, "the internal file is still evidence");
        // Only the published file: nothing counts.
        let only_published = [files[0].clone()];
        assert!(cited_by(&cfg, &only_published).code.is_empty());
        // Without the boundary the same text is evidence.
        assert_eq!(cited_by(&config(), &only_published).code.len(), 1);
    }

    #[test]
    fn a_path_an_exclude_excuses_does_not_become_evidence_by_being_excused() {
        let files = [file("src/pub/fixtures/a.rs", "// Implements: ADR-12\n")];
        let cfg = config_with_published(&["src/pub"], &["src/pub/fixtures"], "error");
        assert!(cited_by(&cfg, &files).code.is_empty());
    }

    #[test]
    fn prose_citing_a_record_is_not_evidence_and_a_marker_is_not_the_only_violation() {
        let files = [file("src/a.rs", "// see ADR-12 for why\n")];
        assert!(
            cited_by(&config(), &files).code.is_empty(),
            "prose is not evidence"
        );
        let prefixes = declared_prefixes(&config());
        assert!(
            cited_identifiers("// see ADR-12 for why", &prefixes).len() == 1,
            "the boundary matcher reads prose"
        );
        assert!(scan_back_pointers(
            std::path::Path::new("a.rs"),
            "// see ADR-12 for why",
            &prefixes
        )
        .is_empty());
    }

    #[test]
    fn a_disabled_boundary_rule_skips_nothing() {
        let files = [file("src/pub/a.rs", "// Implements: ADR-12\n")];
        let mut cfg = config_with_published(&["src/pub"], &[], "off");
        cfg.rules
            .get_mut(RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
            .unwrap()
            .level = crate::config::RuleLevel::Off;
        assert_eq!(cited_by(&cfg, &files).code.len(), 1);
    }
}
