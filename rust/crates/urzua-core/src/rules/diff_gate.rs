//! Rules over the set of paths a `--base` run changed (`MILE-124`, `MILE-142`).
//!
//! The core stays pure: the caller passes the changed paths as data, or `None`
//! when the run has no diff. A diff rule on a run with none reports
//! `not-applicable`, which is neither a pass nor a skip it hides.

use super::*;
use crate::config::RelationRole;
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

/// What the decision gate (`MILE-133`) found among the diff's changed records.
struct GateEvidence {
    /// Whether any changed record releases the gate.
    released: bool,
    /// Changed decision records not at a decided status, as `ID (Status)`,
    /// named in the finding so the author sees what is close.
    undecided: Vec<String>,
}

/// Reads the changed records against the gate's three declarations: a decision
/// record at a decided status, a record of a fix type, or a record whose link
/// field names a decision at a decided status. A deleted record is not in
/// `records`, so it satisfies nothing.
fn gate_evidence(
    paths: &[&PathBuf],
    records: &[Record],
    index: &RecordIndex,
    opts: &SourceRequiresRecordOpts,
    config: &Config,
) -> GateEvidence {
    let status_of = |record: &Record| -> String {
        let field = relation_field_name(config, &record.record_type, RelationRole::Status);
        record
            .header
            .get(field)
            .map(str::trim)
            .unwrap_or("")
            .to_string()
    };
    let decided = |record: &Record| -> bool {
        opts.decision_types
            .get(&record.record_type)
            .is_some_and(|statuses| statuses.contains(&status_of(record)))
    };
    let mut evidence = GateEvidence {
        released: false,
        undecided: Vec::new(),
    };
    for path in paths {
        let Some(record) = records.iter().find(|r| &r.path == *path) else {
            continue;
        };
        if opts.fix_types.contains(&record.record_type) || decided(record) {
            evidence.released = true;
        } else if opts.decision_types.contains_key(&record.record_type) {
            let id = record_id(record).unwrap_or_else(|| record.path.display().to_string());
            evidence
                .undecided
                .push(format!("{id} ({})", status_of(record)));
        }
        let Some(type_config) = config.record_types.get(&record.record_type) else {
            continue;
        };
        for field in &opts.link_fields {
            let declared = type_config
                .pointer_fields
                .iter()
                .flatten()
                .any(|f| f == field)
                || type_config
                    .narrative_fields
                    .iter()
                    .flatten()
                    .any(|f| f == field);
            if !declared {
                continue;
            }
            let Ok(references) = declared_field_references(record, field) else {
                continue;
            };
            let linked = references.iter().any(|reference| {
                index
                    .get(&crate::values::RecordId::new(reference))
                    .is_some_and(|target| decided(target))
            });
            if linked {
                evidence.released = true;
            }
        }
    }
    evidence
}

/// A diff that changes declared source paths but no governed record
/// (`MILE-124`). A changed path matching `exclude_paths` is not source (a test
/// file beside the code, `MILE-158`) and is left unexamined, so a diff
/// that touches only excluded files is not a trigger. Population: every
/// changed path, examined when it is source or a record.
///
/// With `decision_types` declared it is the decision gate (`MILE-133`): the
/// diff must instead change a decision record at a decided status, a record
/// linking one through a `link_fields` field, or a record of a `fix_types`
/// type. Without it, any governed record satisfies the rule as before.
pub fn source_requires_record(
    changed: Option<&[PathBuf]>,
    opts: &SourceRequiresRecordOpts,
    config: &Config,
    records: &[Record],
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DIFF_SOURCE_REQUIRES_RECORD;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let paths = sorted(changed);
    let gated = !opts.decision_types.is_empty();
    let evidence = gated.then(|| gate_evidence(&paths, records, index, opts, config));
    let record_changed = match &evidence {
        Some(e) => e.released,
        None => paths.iter().any(|p| is_governed_record(p, config)),
    };
    let mut findings = Vec::new();
    let population = census(PopulationUnit::ChangedPath, paths, |path| {
        if is_governed_record(path, config) {
            return Outcome::Examined;
        }
        if !under_any(path, &opts.source_paths) {
            return Outcome::Absent;
        }
        // Set aside on purpose, so `Absent` (eligible, not examined) and not
        // `OutOfScope`, which `config.scope-matches-nothing` reads as a
        // configuration the rule cannot reach.
        if under_any(path, &opts.exclude_paths) {
            return Outcome::Absent;
        }
        if !record_changed {
            findings.push(match &evidence {
                Some(e) => gate_finding(path, e, opts),
                None => finding(
                    RULE_ID,
                    (*path).clone(),
                    format!(
                        "source path changed, but no governed record changed in the same diff; \
                         change or add the record that governs {}",
                        path.display()
                    ),
                    // The record's type is not knowable from a source path.
                    ActionSource::new("source-without-record"),
                ),
            });
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

/// The decision gate's finding for one source path: what would release it,
/// and the changed decision records that are not yet decided.
fn gate_finding(path: &Path, evidence: &GateEvidence, opts: &SourceRequiresRecordOpts) -> Finding {
    let decided: Vec<String> = opts
        .decision_types
        .iter()
        .map(|(t, statuses)| format!("{t}: {}", statuses.join(", ")))
        .collect();
    let mut options = vec![format!(
        "a decision record at a decided status ({})",
        decided.join("; ")
    )];
    if !opts.link_fields.is_empty() {
        options.push(format!(
            "a record whose {} names one",
            opts.link_fields.join(" or ")
        ));
    }
    if !opts.fix_types.is_empty() {
        options.push(format!("a {} record", opts.fix_types.join(" or ")));
    }
    let mut message = format!(
        "source path changed, but the diff changes none of: {}",
        options.join("; ")
    );
    if !evidence.undecided.is_empty() {
        message.push_str(&format!(
            "; it does change {}, not yet at a decided status",
            evidence.undecided.join(", ")
        ));
    }
    let mut action = ActionSource::new("source-without-decision");
    // A type to add is determinable only when exactly one holds decisions.
    if let (1, Some(only)) = (opts.decision_types.len(), opts.decision_types.keys().next()) {
        action = action.record_type(only.clone());
    }
    finding(
        RULE_DIFF_SOURCE_REQUIRES_RECORD,
        path.to_path_buf(),
        message,
        action,
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

    /// The rule with only path options declared (no decision gate), over no
    /// record contents.
    fn req(
        changed: Option<&[PathBuf]>,
        source_paths: &[String],
        exclude_paths: &[String],
        config: &Config,
    ) -> (RuleExecution, Vec<Finding>) {
        let opts = SourceRequiresRecordOpts {
            source_paths: source_paths.to_vec(),
            exclude_paths: exclude_paths.to_vec(),
            ..Default::default()
        };
        let (index, _) = crate::rules::build_index_reporting_collisions(&[]);
        source_requires_record(changed, &opts, config, &[], &index)
    }

    fn paths(p: &[&str]) -> Vec<PathBuf> {
        p.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn no_diff_is_not_applicable_for_both_rules() {
        let src = vec!["src".to_string()];
        let (e, f) = req(None, &src, &[], &config());
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
        assert!(req(Some(&changed), &src, &[], &config()).1.is_empty());
        let nested = paths(&["src/a.rs", &format!("{dir}/sub/ADR-1-x.md")]);
        assert_eq!(req(Some(&nested), &src, &[], &config()).1.len(), 1);
    }

    #[test]
    fn a_diff_touching_only_excluded_files_is_not_a_trigger_observed_failing() {
        let src = vec!["src".to_string()];
        let exclude = vec!["**/*.test.*".to_string(), "src/tests".to_string()];
        let only_tests = paths(&["src/a.test.ts", "src/deep/b.test.ts", "src/tests/c.rs"]);
        let (exec, findings) = req(Some(&only_tests), &src, &exclude, &config());
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!(
            (pop.eligible(), pop.examined(), pop.out_of_scope()),
            (3, 0, 0)
        );
        // Code beside the tests still triggers, and names only the code file.
        let mixed = paths(&["src/a.test.ts", "src/a.ts"]);
        let (_, findings) = req(Some(&mixed), &src, &exclude, &config());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, PathBuf::from("src/a.ts"));
    }

    #[test]
    fn a_glob_source_path_selects_by_pattern_and_a_prefix_config_is_unchanged() {
        let changed = paths(&["pkg/a/src/x.rs", "pkg/a/docs/y.md"]);
        let glob = vec!["pkg/*/src".to_string()];
        assert_eq!(req(Some(&changed), &glob, &[], &config()).1.len(), 1);
        let prefix = vec!["pkg".to_string()];
        assert_eq!(req(Some(&changed), &prefix, &[], &config()).1.len(), 2);
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

    // ---------------------------------------------------------- MILE-133

    fn gate_config() -> Config {
        let mut adr = super::super::tests::type_config(None);
        adr.dir = "docs/adr".to_string();
        adr.prefix = Some("ADR".to_string());
        let mut bug = super::super::tests::type_config(None);
        bug.dir = "docs/bugs".to_string();
        bug.prefix = Some("BUG".to_string());
        let mut mile = super::super::tests::type_config(None);
        mile.dir = "docs/milestones".to_string();
        mile.prefix = Some("MILE".to_string());
        mile.pointer_fields = Some(vec!["Implements".to_string()]);
        mile.narrative_fields = Some(vec![]);
        super::super::tests::config_with_types(vec![
            ("adr", adr),
            ("bug", bug),
            ("milestone", mile),
        ])
    }

    fn gate_opts(single_type: bool) -> SourceRequiresRecordOpts {
        let mut decision_types = std::collections::BTreeMap::new();
        decision_types.insert("adr".to_string(), vec!["Accepted".to_string()]);
        if !single_type {
            decision_types.insert("milestone".to_string(), vec!["Done".to_string()]);
        }
        SourceRequiresRecordOpts {
            source_paths: vec!["src".to_string()],
            decision_types,
            fix_types: vec!["bug".to_string()],
            link_fields: vec!["Implements".to_string()],
            ..Default::default()
        }
    }

    fn gate_records() -> Vec<Record> {
        let make = |path: &str, ty: &str, prefix: &str, header: &str| {
            super::super::tests::record_with_prefix(
                path,
                ty,
                prefix,
                &format!("---\n{header}\n---\n# t\n"),
            )
        };
        vec![
            make("docs/adr/ADR-1-a.md", "adr", "ADR", "Status: Accepted"),
            make("docs/adr/ADR-2-b.md", "adr", "ADR", "Status: Proposed"),
            make("docs/bugs/BUG-1-c.md", "bug", "BUG", "Status: Fixed"),
            make(
                "docs/milestones/MILE-1-d.md",
                "milestone",
                "MILE",
                "Status: Planned\nImplements: ADR-1",
            ),
            make(
                "docs/milestones/MILE-2-e.md",
                "milestone",
                "MILE",
                "Status: Planned\nImplements: ADR-2",
            ),
            make(
                "docs/milestones/MILE-3-f.md",
                "milestone",
                "MILE",
                "Status: Planned\nImplements: ADR-404",
            ),
        ]
    }

    fn gate(changed: &[&str], opts: &SourceRequiresRecordOpts) -> (RuleExecution, Vec<Finding>) {
        let records = gate_records();
        let (index, _) = crate::rules::build_index_reporting_collisions(&records);
        let changed = paths(changed);
        source_requires_record(Some(&changed), opts, &gate_config(), &records, &index)
    }

    #[test]
    fn a_decided_decision_a_fix_or_a_link_to_a_decided_decision_releases_the_gate() {
        let opts = gate_opts(false);
        for released_by in [
            "docs/adr/ADR-1-a.md",
            "docs/bugs/BUG-1-c.md",
            "docs/milestones/MILE-1-d.md",
        ] {
            let (_, findings) = gate(&["src/a.rs", released_by], &opts);
            assert!(findings.is_empty(), "{released_by}: {findings:?}");
        }
    }

    #[test]
    fn an_undecided_decision_an_unresolved_link_and_a_link_to_an_undecided_one_do_not_observed_failing(
    ) {
        let opts = gate_opts(false);
        for held_by in [
            "docs/adr/ADR-2-b.md",
            "docs/milestones/MILE-2-e.md",
            "docs/milestones/MILE-3-f.md",
        ] {
            let (_, findings) = gate(&["src/a.rs", held_by], &opts);
            assert_eq!(findings.len(), 1, "{held_by}");
            assert_eq!(findings[0].file, PathBuf::from("src/a.rs"));
        }
        let (_, findings) = gate(&["src/a.rs"], &opts);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn the_message_names_what_would_release_the_gate_and_the_near_miss() {
        let (_, findings) = gate(&["src/a.rs", "docs/adr/ADR-2-b.md"], &gate_opts(false));
        let message = &findings[0].message;
        assert!(message.contains("adr: Accepted"), "{message}");
        assert!(message.contains("Implements"), "{message}");
        assert!(message.contains("a bug record"), "{message}");
        assert!(message.contains("ADR-2 (Proposed)"), "{message}");
    }

    #[test]
    fn a_deleted_record_does_not_release_the_gate() {
        // The deleted path is in the changed set but in no loaded record.
        let (_, findings) = gate(&["src/a.rs", "docs/adr/ADR-9-gone.md"], &gate_opts(false));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn an_excluded_path_is_still_set_aside_by_the_gate() {
        let mut opts = gate_opts(false);
        opts.exclude_paths = vec!["src/tests".to_string()];
        let (exec, findings) = gate(&["src/tests/t.rs"], &opts);
        assert!(findings.is_empty());
        assert_eq!(exec.population.unwrap().examined(), 0);
    }

    #[test]
    fn one_decision_type_names_the_record_to_add_and_several_do_not() {
        let single = gate(&["src/a.rs"], &gate_opts(true)).1;
        assert_eq!(
            single[0].action_source.target.record_type.as_deref(),
            Some("adr")
        );
        let several = gate(&["src/a.rs"], &gate_opts(false)).1;
        assert_eq!(several[0].action_source.target.record_type, None);
        assert_eq!(several[0].action_source.variant, "source-without-decision");
    }

    #[test]
    fn without_decision_types_any_governed_record_still_releases_the_rule() {
        let mut opts = gate_opts(false);
        opts.decision_types.clear();
        let (_, findings) = gate(&["src/a.rs", "docs/adr/ADR-2-b.md"], &opts);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
