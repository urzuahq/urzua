use super::*;
use std::collections::BTreeSet;

/// Rule: a source comment that cites a record which does not exist.
///
/// The mirror of `embodiment.locator-exists`: a locator names a file, and this
/// names a record from the file's side. A comment citing an id (a declared type
/// prefix, a hyphen and a number: `ADR-12`) is a claim that the record exists;
/// one whose id resolves to no record is reported with the file and line. Only
/// existence is judged, never currency, so a citation of a superseded or
/// retired record is not reported.
///
/// The population is the source files under `back_pointer_paths` whose
/// extension is one of `extensions` (the caller read only those); a declared
/// path covering no such file is itself a finding, as for the back-pointer
/// rules. Comments are read exactly as the back-pointer rules read them. A
/// citation `embodiment.back-pointer-resolution` already judges (an
/// `Implements:` back-pointer, or a `Verifies:` one in its test class) is left
/// to that rule, so one line is not reported twice. An id matched by
/// `external_ids` (`citation.resolution`'s option) is not a record. A line
/// carrying the declared `ignore_marker` is neither reported nor counted, the
/// marker's lines are disclosed in the population's `excused_lines`, and one
/// that excuses nothing is a finding.
pub fn code_dangling_bug_reference(
    files: &[SourceFile],
    source_paths: &[String],
    extensions: &[String],
    config: &Config,
    config_path: &std::path::Path,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CODE_DANGLING_BUG_REFERENCE;
    let prefixes = declared_prefixes(config);
    let setting = config.rules.get(RULE_ID);
    let external: Vec<ExternalId> = setting
        .and_then(|s| s.external_ids.as_ref())
        .into_iter()
        .flatten()
        .filter_map(|e| parse_external_id(e, &prefixes).ok())
        .collect();
    let judge = Judge {
        prefixes: &prefixes,
        ignore: setting.and_then(|s| s.ignore_marker.as_deref()),
        external: &external,
        // What the resolution rule judges, where it is enabled.
        resolution: config
            .rules
            .get(RULE_EMBODIMENT_BACK_POINTER_RESOLUTION)
            .filter(|s| s.level != crate::config::RuleLevel::Off),
        index,
        leaders: setting.and_then(|s| s.code_comment_prefixes.as_ref()),
    };
    let mut findings = Vec::new();
    let mut excused_lines = 0;

    let population = census(PopulationUnit::File, files.iter().collect(), |file| {
        let wanted_extension = file
            .path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.iter().any(|x| x == e));
        if !wanted_extension
            || !source_paths
                .iter()
                .any(|e| path_is_published(&file.path, e))
        {
            return Outcome::OutOfScope;
        }
        let Some(content) = &file.content else {
            return Outcome::Unreadable;
        };
        let (found, excused) = judge.file(file, content);
        findings.extend(found);
        excused_lines += excused;
        Outcome::Examined
    });

    findings.extend(uncovered_source_paths(
        RULE_ID,
        "dangling-reference-uncovered-path",
        files,
        source_paths,
        config_path,
    ));

    (
        RuleExecution::ran(
            RULE_ID,
            population.with_excused_lines(excused_lines),
            Vec::new(),
        ),
        findings,
    )
}

/// What judging one file reads: the declared prefixes, marker and external
/// ids, the resolution rule's settings and the record index.
struct Judge<'a> {
    prefixes: &'a [String],
    ignore: Option<&'a str>,
    external: &'a [ExternalId],
    resolution: Option<&'a crate::config::RuleSetting>,
    index: &'a RecordIndex<'a>,
    /// The declared comment leaders per extension, where declared.
    leaders: Option<&'a std::collections::BTreeMap<String, Vec<String>>>,
}

impl Judge<'_> {
    /// The findings of one file and the number of lines the marker excused.
    fn file(&self, file: &SourceFile, content: &str) -> (Vec<Finding>, usize) {
        const RULE_ID: &str = RULE_CODE_DANGLING_BUG_REFERENCE;
        let judged_elsewhere = judged_by_resolution(self.resolution, file, content, self.prefixes);
        let lines: Vec<&str> = content.lines().collect();
        let declared: Option<&[String]> = self.leaders.and_then(|m| {
            let ext = file.path.extension()?.to_str()?;
            m.get(ext).map(Vec::as_slice)
        });
        let citations = scan_comment_citations(content, self.prefixes, declared);
        let mut findings = Vec::new();
        let mut excused = BTreeSet::new();
        for citation in &citations {
            if self
                .ignore
                .is_some_and(|m| line_carries_ignore_marker(lines[citation.line - 1], m))
            {
                excused.insert(citation.line);
                continue;
            }
            if judged_elsewhere.contains(&(citation.line, citation.id.clone()))
                || self.external.iter().any(|e| e.matches(&citation.id))
                || self
                    .index
                    .contains_key(&crate::values::RecordId::new(&citation.id))
            {
                continue;
            }
            findings.push(
                Finding::new(
                    RULE_ID,
                    FindingSeverity::Error,
                    file.path.clone(),
                    ActionSource::new("dangling-reference"),
                    format!("comment cites {}, which resolves to no record", citation.id),
                )
                .at(citation.line),
            );
        }
        if let Some(marker) = self.ignore {
            let cited: BTreeSet<usize> = citations.iter().map(|c| c.line).collect();
            for (i, line) in lines.iter().enumerate() {
                if line_carries_ignore_marker(line, marker) && !cited.contains(&(i + 1)) {
                    findings.push(unused_marker_finding(
                        RULE_ID,
                        "dangling-reference-marker-unused",
                        &file.path,
                        i + 1,
                        marker,
                    ));
                }
            }
        }
        (findings, excused.len())
    }
}

/// The `(line, id)` pairs `embodiment.back-pointer-resolution` judges in `file`:
/// `Implements:` back-pointers under its paths, and `Verifies:` ones in its
/// test class. Empty when that rule is off or does not cover the file.
fn judged_by_resolution(
    resolution: Option<&crate::config::RuleSetting>,
    file: &SourceFile,
    content: &str,
    prefixes: &[String],
) -> BTreeSet<(usize, String)> {
    let mut out = BTreeSet::new();
    let Some(setting) = resolution else {
        return out;
    };
    let covered = setting
        .back_pointer_paths
        .iter()
        .flatten()
        .any(|e| path_is_published(&file.path, e));
    if !covered {
        return out;
    }
    let tests = setting.test_paths.clone().unwrap_or_default();
    let mut markers = vec![MARKER_IMPLEMENTS];
    if is_test_file(&file.path, &tests) {
        markers.push(MARKER_VERIFIES);
    }
    for marker in markers {
        for p in scan_marker(&file.path, content, prefixes, marker) {
            out.insert((p.line, p.id));
        }
    }
    out
}
