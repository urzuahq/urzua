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
            .any(|t| path.parent() == Some(Path::new(&t.dir)))
}

fn under_any(path: &Path, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| path_is_published(path, p))
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

fn finding(rule: &str, file: PathBuf, message: String) -> Finding {
    Finding {
        rule: rule.to_string(),
        severity: FindingSeverity::Error,
        file,
        line: None,
        message,
        waived: None,
        declared_severity: None,
    }
}

fn sorted(changed: &[PathBuf]) -> Vec<&PathBuf> {
    let mut paths: Vec<&PathBuf> = changed.iter().collect();
    paths.sort();
    paths.dedup();
    paths
}

/// A diff that changes declared source paths but no governed record
/// (`MILE-124`). Population: every changed path, examined when it is source or
/// a record.
pub fn source_requires_record(
    changed: Option<&[PathBuf]>,
    source_paths: &[String],
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
        if !record_changed {
            findings.push(finding(
                RULE_ID,
                (*path).clone(),
                format!(
                    "source path changed, but no governed record changed in the same diff; \
                     change or add the record that governs {}",
                    path.display()
                ),
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
        let (e, f) = source_requires_record(None, &src, &config());
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
        assert!(source_requires_record(Some(&changed), &src, &config())
            .1
            .is_empty());
        let nested = paths(&["src/a.rs", &format!("{dir}/sub/ADR-1-x.md")]);
        assert_eq!(
            source_requires_record(Some(&nested), &src, &config())
                .1
                .len(),
            1
        );
    }
}
