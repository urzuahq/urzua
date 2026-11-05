//! Rules over the set of paths a `--base` run changed (`MILE-124`, `MILE-142`).
//!
//! The core stays pure: the caller passes the changed paths as data, or `None`
//! when the run has no diff. A diff rule on a run with none reports
//! `not-applicable`, which is neither a pass nor a skip it hides.

use super::*;
use crate::report::{census, Outcome};
use std::path::Path;

/// Whether `path` is a governed record: a record-shaped file directly in a
/// configured type's `dir`. The narrowest reading of "a record changed" -- a
/// file in a subdirectory no type claims is not one (`type.record-outside-declared-dir`).
fn is_governed_record(path: &Path, config: &Config) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    crate::record::is_governed_record_filename(name)
        && config
            .record_types
            .values()
            .any(|t| t.owns_dir(path.parent()) && !t.excludes(name))
}

/// Whether `path` is covered by any entry: a prefix, or a glob (`MILE-158`).
fn under_any(path: &Path, entries: &[String]) -> bool {
    entries.iter().any(|e| path_matches_entry(path, e))
}

fn not_applicable(rule: &str) -> (RuleExecution, Vec<Finding>) {
    (
        RuleExecution {
            rule: rule.to_string(),
            population: None,
            status: RuleStatus::NotApplicable,
            examined_records: Vec::new(),
        },
        Vec::new(),
    )
}

fn finding(rule: &str, file: PathBuf, message: String, action_source: ActionSource) -> Finding {
    Finding {
        rule: rule.to_string(),
        severity: FindingSeverity::Error,
        file,
        line: None,
        message,
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source,
    }
}

fn sorted(changed: &[PathBuf]) -> Vec<&PathBuf> {
    let mut paths: Vec<&PathBuf> = changed.iter().collect();
    paths.sort();
    paths.dedup();
    paths
}

/// A diff that changes declared source paths but no governed record
/// (`MILE-124`). A changed path matching `exclude_paths` is not source (a test
/// file beside the code, `MILE-158`) and is left unexamined, so a diff
/// that touches only excluded files is not a trigger. Population: every
/// changed path, examined when it is source or a record.
pub fn source_requires_record(
    changed: Option<&[PathBuf]>,
    source_paths: &[String],
    exclude_paths: &[String],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DIFF_SOURCE_REQUIRES_RECORD;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let paths = sorted(changed);
    let record_changed = paths.iter().any(|p| is_governed_record(p, config));
    let mut findings = Vec::new();
    let population = census(PopulationUnit::ChangedPath, paths, |path| {
        if is_governed_record(path, config) {
            return Outcome::Examined;
        }
        if !under_any(path, source_paths) {
            return Outcome::Absent;
        }
        // Set aside on purpose, so `Absent` (eligible, not examined) and not
        // `OutOfScope`, which `config.scope-matches-nothing` reads as a
        // configuration the rule cannot reach.
        if under_any(path, exclude_paths) {
            return Outcome::Absent;
        }
        if !record_changed {
            findings.push(finding(
                RULE_ID,
                (*path).clone(),
                format!(
                    "source path changed, but no governed record changed in the same diff; \
                     change or add the record that governs {}",
                    path.display()
                ),
                // The record's type is not knowable from a source path.
                ActionSource::new("source-without-record"),
            ));
        }
        Outcome::Examined
    });
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

/// A diff that changes both a governed record and application source
/// (`MILE-142`). One finding, naming the first path of each kind.
pub fn source_and_records_mixed(
    changed: Option<&[PathBuf]>,
    application_paths: &[String],
    exclude_paths: &[String],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DIFF_SOURCE_AND_RECORDS_MIXED;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let is_application = |p: &Path| {
        !is_governed_record(p, config)
            && under_any(p, application_paths)
            && !under_any(p, exclude_paths)
    };
    let paths = sorted(changed);
    let first_record = paths.iter().find(|p| is_governed_record(p, config));
    let first_source = paths.iter().find(|p| is_application(p));
    let mut findings = Vec::new();
    if let (Some(record), Some(source)) = (first_record, first_source) {
        findings.push(finding(
            RULE_ID,
            (*record).clone(),
            format!(
                "this diff changes both a governed record ({}) and application source ({}); \
                 consider splitting it",
                record.display(),
                source.display()
            ),
            ActionSource::new("source-and-records-mixed"),
        ));
    }
    let population = census(PopulationUnit::ChangedPath, paths, |path| {
        if is_governed_record(path, config) || is_application(path) {
            Outcome::Examined
        } else {
            Outcome::Absent
        }
    });
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

    fn config() -> Config {
        let adr = super::super::tests::type_config(None);
        super::super::tests::config_with_types(vec![("adr", adr)])
    }

    fn paths(p: &[&str]) -> Vec<PathBuf> {
        p.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn no_diff_is_not_applicable_for_both_rules() {
        let src = vec!["src".to_string()];
        let (e, f) = source_requires_record(None, &src, &[], &config());
        assert_eq!(e.status, RuleStatus::NotApplicable);
        assert!(e.population.is_none() && f.is_empty());
        let (e, f) = source_and_records_mixed(None, &src, &[], &config());
        assert_eq!(e.status, RuleStatus::NotApplicable);
        assert!(f.is_empty());
    }

    #[test]
    fn the_record_dir_is_whatever_the_config_declares() {
        let dir = config().record_types["adr"].dir.clone();
        let record = format!("{dir}/ADR-1-x.md");
        let src = vec!["src".to_string()];
        let changed = paths(&["src/a.rs", &record]);
        assert!(source_requires_record(Some(&changed), &src, &[], &config())
            .1
            .is_empty());
        let nested = paths(&["src/a.rs", &format!("{dir}/sub/ADR-1-x.md")]);
        assert_eq!(
            source_requires_record(Some(&nested), &src, &[], &config())
                .1
                .len(),
            1
        );
    }

    #[test]
    fn a_diff_touching_only_excluded_files_is_not_a_trigger_observed_failing() {
        let src = vec!["src".to_string()];
        let exclude = vec!["**/*.test.*".to_string(), "src/tests".to_string()];
        let only_tests = paths(&["src/a.test.ts", "src/deep/b.test.ts", "src/tests/c.rs"]);
        let (exec, findings) = source_requires_record(Some(&only_tests), &src, &exclude, &config());
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.out_of_scope()),
            (3, 0, 0)
        );
        // Code beside the tests still triggers, and names only the code file.
        let mixed = paths(&["src/a.test.ts", "src/a.ts"]);
        let (_, findings) = source_requires_record(Some(&mixed), &src, &exclude, &config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("src/a.ts"));
    }

    #[test]
    fn a_glob_source_path_selects_by_pattern_and_a_prefix_config_is_unchanged() {
        let changed = paths(&["pkg/a/src/x.rs", "pkg/a/docs/y.md"]);
        let glob = vec!["pkg/*/src".to_string()];
        assert_eq!(
            source_requires_record(Some(&changed), &glob, &[], &config())
                .1
                .len(),
            1
        );
        let prefix = vec!["pkg".to_string()];
        assert_eq!(
            source_requires_record(Some(&changed), &prefix, &[], &config())
                .1
                .len(),
            2
        );
    }

    #[test]
    fn exclude_globs_also_narrow_the_mixed_rule() {
        let dir = config().record_types["adr"].dir.clone();
        let changed = paths(&["src/a.test.ts", &format!("{dir}/ADR-1-x.md")]);
        let app = vec!["src".to_string()];
        let (_, findings) = source_and_records_mixed(Some(&changed), &app, &[], &config());
        assert_eq!(findings.len(), 1);
        let exclude = vec!["**/*.test.*".to_string()];
        let (_, findings) = source_and_records_mixed(Some(&changed), &app, &exclude, &config());
        assert!(findings.is_empty(), "{findings:?}");
    }
}
