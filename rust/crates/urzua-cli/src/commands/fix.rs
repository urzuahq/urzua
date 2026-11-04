//! `urzua fix` (SPEC-8): detect mode only (ADR-0015 §3) is read-only, safe
//! in CI. Apply mode (`--ids`, `--by`, `--force`) writes computed values
//! back, gated hard.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, FixFailure, FixReport, FixStatus, Notice, NoticeSeverity};

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::{emit, NOTICE_IDENTITY};

pub fn run(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    stdin_path: Option<PathBuf>,
    tier: u8,
    apply: bool,
    ids: Vec<String>,
    by: Option<String>,
    force: bool,
) -> ExitCode {
    if tier != 1 {
        return emit(&CouldNotRun::from(
            "only tier 1 is implemented so far -- tiers 2 and 3 are not yet built",
        ));
    }
    if apply && ids.is_empty() && !force {
        return emit(&CouldNotRun::from(
            "urzua fix --apply: pass --ids <record,...> or --force (never the default)",
        ));
    }

    if stdin_path.is_some()
        && (apply || force || !ids.is_empty() || by.is_some() || !paths.is_empty())
    {
        return emit(&CouldNotRun::from(
            "urzua fix --stdin-path judges proposed content and writes nothing: it cannot be combined with PATHS, --apply, --ids, --by or --force",
        ));
    }
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path =
        config_path.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (mut records, _full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    // `PATHS` narrow what is examined, drifted and repaired (`MILE-159`); the
    // corpus a back-pointer resolves against stays whole. A path that matches
    // no record is a failure naming it, never a silent skip.
    let mut proposed: Option<String> = None;
    let (scoped, unmatched) = if let Some(stdin_path) = &stdin_path {
        // The proposed content replaces the on-disk record (or adds a new one)
        // for this one path; nothing else is examined or drifted.
        let rel = match relative_to_root(&repo_root, stdin_path) {
            Ok(rel) => rel,
            Err(e) => return emit(&CouldNotRun::from(e)),
        };
        let mut content = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut content) {
            return emit(&CouldNotRun::from(format!(
                "could not read the proposed content from stdin: {e}"
            )));
        }
        let record = match crate::discovery::parse_prospective_record(&config, &rel, &content) {
            Ok(r) => r,
            Err(e) => return emit(&CouldNotRun::from(e)),
        };
        records.retain(|r| r.path != rel);
        records.push(record.clone());
        proposed = Some(content);
        (vec![record], Vec::new())
    } else {
        scope_records(&repo_root, &paths, &records)
    };
    // Drift is the expensive part (several `git` subprocesses per record) and
    // its only consumer is `embodiment.consistency`; with that rule off `fix`
    // has no embodiment verdict to repair, so say so instead of computing a
    // verdict from an empty drift set (`MILE-159`, as `check` gates it).
    if !config
        .rules
        .get(urzua_core::rules::RULE_EMBODIMENT_CONSISTENCY)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off)
    {
        return emit(&CouldNotRun::from(
            "urzua fix repairs the Embodiment field that embodiment.consistency judges, and that rule is off or undeclared in the configuration: nothing to repair",
        ));
    }
    let drift = match &proposed {
        Some(content) => crate::discovery::compute_drifted_records_for_content(
            &repo_root, &scoped, &config, content,
        ),
        None => crate::discovery::compute_drifted_records(&repo_root, &scoped, &config),
    };
    let drift_notices = crate::discovery::drift_notices(drift.shallow, &drift.git_problems);
    let back_pointed =
        match crate::discovery::read_back_pointer_sources(&repo_root, &discovered, &config) {
            Ok(Some(sources)) => {
                let (index, _) = urzua_core::rules::build_index_reporting_collisions(&records);
                urzua_core::rules::back_pointer_evidence(
                    &sources.files,
                    &sources.back_pointer_paths,
                    &sources.test_paths,
                    &config,
                    &index,
                )
            }
            Ok(None) => Default::default(),
            Err(e) => return emit(&CouldNotRun::from(e)),
        };
    let (population, repairs) = urzua_core::fix::detect_repairs(
        &scoped,
        &config,
        &drift.drifted,
        &drift.fresh,
        &back_pointed,
    );

    if !apply {
        return emit(&build_fix_report(
            population,
            repairs,
            unmatched,
            None,
            false,
            drift_notices,
        ));
    }

    let identity = match urzua_io::resolve_identity(by.as_deref(), &repo_root) {
        Ok(id) => id,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let _lock = match urzua_io::FixLock::acquire(&repo_root) {
        Ok(lock) => lock,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let today = urzua_io::today();
    let selected: Vec<_> = repairs
        .into_iter()
        .filter(|r| force || ids.contains(&r.record.display().to_string()))
        .collect();

    let mut applied = Vec::new();
    let mut failed = unmatched;
    // An `--ids` entry that selected nothing is a failure naming it, not a
    // clean `ok`: the selector is matched against a record's repo-relative
    // path, and a typo or a record id (`ADR-1`) would otherwise read as
    // "checked, nothing to do" while a repair is still pending. `--force`
    // selects every repair regardless, so it has no selectors to miss.
    if !force {
        for id in &ids {
            if !selected
                .iter()
                .any(|r| r.record.display().to_string() == *id)
            {
                failed.push((
                    PathBuf::from(id),
                    format!(
                        "--ids entry '{id}' matched no pending repair (an entry is a record's repo-relative path, e.g. docs/adr/ADR-1-title.md)"
                    ),
                ));
            }
        }
    }
    for repair in selected {
        let full_path = repo_root.join(&repair.record);
        // Re-read from disk right before writing (RFC-0008 §4's
        // re-verify-before-write): apply_repair itself fails if the current
        // value it was told to replace is no longer there, which is exactly
        // the case a concurrent edit since detect ran would produce.
        let fresh_content = match urzua_io::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                failed.push((repair.record.clone(), format!("could not re-read: {e}")));
                continue;
            }
        };
        let log_id = records
            .iter()
            .find(|r| r.path == repair.record)
            .map(|r| config.revision_log_id(&r.record_type))
            .unwrap_or_default();
        match urzua_core::fix::apply_repair(
            &fresh_content,
            &repair,
            &today,
            &identity.name,
            &log_id,
        ) {
            Ok(new_content) => match std::fs::write(&full_path, new_content) {
                Ok(()) => applied.push(repair),
                Err(e) => failed.push((repair.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    emit(&build_fix_report(
        population,
        applied,
        failed,
        identity.warning,
        true,
        drift_notices,
    ))
}

/// `p` (absolute, or relative to the working directory) as a repo-relative
/// path, without requiring that it exist.
fn relative_to_root(repo_root: &std::path::Path, p: &std::path::Path) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .and_then(|c| c.canonicalize())
        .unwrap_or_default();
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let mut normal = PathBuf::new();
    for c in joined.components() {
        match c {
            std::path::Component::ParentDir => {
                normal.pop();
            }
            std::path::Component::CurDir => {}
            other => normal.push(other),
        }
    }
    normal
        .strip_prefix(repo_root)
        .map(PathBuf::from)
        .map_err(|_| format!("{} is outside the repository", p.display()))
}

/// The records `PATHS` select: a record whose repo-relative path equals a
/// path or sits under it (a directory). No paths selects every record. Each
/// path that selects nothing comes back as a failure naming it.
fn scope_records(
    repo_root: &std::path::Path,
    paths: &[PathBuf],
    records: &[urzua_core::record::Record],
) -> (Vec<urzua_core::record::Record>, Vec<(PathBuf, String)>) {
    if paths.is_empty() {
        return (records.to_vec(), Vec::new());
    }
    let mut unmatched = Vec::new();
    let mut scopes = Vec::new();
    for p in paths {
        match relative_to_root(repo_root, p) {
            Ok(rel) => scopes.push((p.clone(), rel)),
            Err(e) => unmatched.push((p.clone(), e)),
        }
    }
    for (given, rel) in &scopes {
        if !records.iter().any(|r| r.path.starts_with(rel)) {
            unmatched.push((
                given.clone(),
                format!(
                    "path '{}' matched no record (a path is a record file or a directory holding records)",
                    given.display()
                ),
            ));
        }
    }
    let selected = records
        .iter()
        .filter(|r| scopes.iter().any(|(_, rel)| r.path.starts_with(rel)))
        .cloned()
        .collect();
    (selected, unmatched)
}

/// `ran_apply` matters because the two modes disagree on what
/// `population.examined() == 0` should mean for the exit code: detect mode
/// treats it as `NotRun` (exit 2, nothing to report on); apply mode's exit
/// code was always keyed only on `failed.is_empty()`, independent of the
/// count -- an empty/undiscovered corpus with nothing to apply and nothing
/// to fail on exits 0, not 2, matching the pre-existing behavior this
/// refactor must not silently change.
fn build_fix_report(
    population: urzua_core::report::Population,
    repairs: Vec<urzua_core::fix::Repair>,
    failed: Vec<(PathBuf, String)>,
    warning: Option<String>,
    ran_apply: bool,
    extra_notices: Vec<Notice>,
) -> FixReport {
    let status = if population.examined() == 0 && !ran_apply && failed.is_empty() {
        FixStatus::NotRun
    } else if !failed.is_empty() {
        FixStatus::PartialFailure
    } else if repairs.is_empty() {
        FixStatus::Ok
    } else if ran_apply {
        FixStatus::Applied
    } else {
        FixStatus::RepairsAvailable
    };

    FixReport {
        status,
        population,
        repairs,
        failed: failed
            .into_iter()
            .map(|(record, error)| FixFailure { record, error })
            .collect(),
        notices: warning
            .into_iter()
            .map(|message| Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_IDENTITY.to_string(),
                message,
            })
            .chain(extra_notices)
            .collect(),
    }
}
