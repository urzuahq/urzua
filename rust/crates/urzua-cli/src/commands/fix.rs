//! `urzua fix` (SPEC-8): detect mode only (ADR-0015 §3) is read-only, safe
//! in CI. Apply mode (`--ids`, `--by`, `--force`) writes computed values
//! back, gated hard.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, FixFailure, FixReport, FixStatus, Notice, NoticeSeverity};

use crate::session::Session;
use crate::{emit, NOTICE_IDENTITY};

/// The `urzua fix` command line, gathered so `run` stays one argument wide.
pub struct FixArgs {
    pub paths: Vec<PathBuf>,
    pub stdin_path: Option<PathBuf>,
    pub tier: u8,
    pub apply: bool,
    pub ids: Vec<String>,
    pub by: Option<String>,
    pub actor: urzua_core::fix::ActorKind,
    pub force: bool,
    pub rebaseline: bool,
    pub reason: Option<String>,
}

pub fn run(config_path: Option<PathBuf>, args: FixArgs) -> ExitCode {
    let FixArgs {
        paths,
        stdin_path,
        tier,
        apply,
        ids,
        by,
        actor,
        force,
        rebaseline,
        reason,
    } = args;
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

    // `--force`, `--by` and `--actor` belong to a write; without `--apply` they
    // would be silently ignored, so they are a usage error (`--ids` instead
    // narrows the dry-run report).
    if !apply && (force || by.is_some() || actor != urzua_core::fix::ActorKind::Unspecified) {
        return emit(&CouldNotRun::from(
            "urzua fix --force, --by and --actor belong to --apply: without it nothing is written, so they would be ignored (--ids alone narrows the dry-run report)",
        ));
    }

    if rebaseline && stdin_path.is_some() {
        return emit(&CouldNotRun::from(
            "urzua fix --rebaseline works on files in the tree: it cannot be combined with --stdin-path",
        ));
    }
    if !rebaseline && reason.is_some() {
        return emit(&CouldNotRun::from(
            "urzua fix --reason belongs to --rebaseline: a Tier 1 repair records its computation, not a reason",
        ));
    }
    let reason = match (rebaseline && apply, reason.as_deref().map(str::trim)) {
        (true, None | Some("")) => {
            return emit(&CouldNotRun::from(
                "urzua fix --rebaseline --apply: pass --reason <why the evidence still holds>; a re-pin without a reason is a rubber stamp",
            ))
        }
        (_, Some(r)) if r.contains('|') || r.contains(['\n', '\r']) => {
            return emit(&CouldNotRun::from(
                "--reason must be one line without '|' (it is written into a revision-log table row)",
            ))
        }
        (_, r) => r.unwrap_or_default().to_string(),
    };

    if stdin_path.is_some()
        && (apply || force || !ids.is_empty() || by.is_some() || !paths.is_empty())
    {
        return emit(&CouldNotRun::from(
            "urzua fix --stdin-path judges proposed content and writes nothing: it cannot be combined with PATHS, --apply, --ids, --by or --force",
        ));
    }
    let Session {
        repo_root,
        config,
        discovered,
        mut records,
        ..
    } = match Session::open(config_path) {
        Ok(session) => session,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    // `PATHS` narrow what is examined, drifted and repaired (`MILE-159`); the
    // corpus a back-pointer resolves against stays whole. A path that matches
    // no record is a failure naming it, never a silent skip.
    let mut proposed: Option<String> = None;
    let (scoped, unmatched) = if let Some(stdin_path) = &stdin_path {
        // The proposed content replaces the on-disk record (or adds a new one)
        // for this one path; nothing else is examined or drifted.
        let rel = match crate::discovery::relative_to_root(&repo_root, stdin_path) {
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
    // verdict from an empty drift set (`MILE-159`, as `check` gates it). The
    // `Status` repair (`MILE-183`) is the other thing `fix` writes, and needs
    // no drift, so `status.delivery-agreement` being on is enough to proceed.
    let enabled = |rule: &str| {
        config
            .rules
            .get(rule)
            .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off)
    };
    if !enabled(urzua_core::rules::RULE_EMBODIMENT_CONSISTENCY)
        && !enabled(urzua_core::rules::RULE_STATUS_DELIVERY_AGREEMENT)
    {
        return emit(&CouldNotRun::from(
            "urzua fix repairs the Embodiment field that embodiment.consistency judges and the Status that status.delivery-agreement judges, and both rules are off or undeclared in the configuration: nothing to repair",
        ));
    }
    if rebaseline {
        return run_rebaseline(
            &repo_root,
            &config,
            &records,
            &scoped,
            unmatched,
            apply,
            &ids,
            force,
            by,
            actor,
            &reason,
            &|| {},
        );
    }
    let drift = if !enabled(urzua_core::rules::RULE_EMBODIMENT_CONSISTENCY) {
        Default::default()
    } else {
        match &proposed {
            Some(content) => crate::discovery::compute_drifted_records_for_content(
                &repo_root, &scoped, &config, content,
            ),
            None => crate::discovery::compute_drifted_records(&repo_root, &scoped, &config),
        }
    };
    let mut drift_notices = crate::discovery::drift_notices(drift.shallow, &drift.git_problems);
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
    // The derived-blocked records (`ADR-96` section 7): `fix` writes no
    // `progress` status to one. Computed only when a type is opted in.
    let blocked = if scoped.iter().any(|r| r.items.is_some()) {
        urzua_core::rules::derived_blocked_records(&records, &config)
    } else {
        Default::default()
    };
    let (population, repairs, unevidenced) = crate::file_texts::with_locator_env(
        &repo_root,
        &config,
        &discovered.paths,
        &records,
        |env| {
            let ctx = urzua_core::rules::DeliveryContext {
                back_pointed: &back_pointed,
                env,
                blocked: &blocked,
            };
            let notices = urzua_core::fix::unevidenced_successes(&scoped, &config, &ctx);
            let (population, repairs) = urzua_core::fix::detect_repairs(
                &scoped,
                &config,
                &drift.drifted,
                &drift.fresh,
                &ctx,
            );
            (population, repairs, notices)
        },
    );
    drift_notices.extend(unevidenced);

    if !apply {
        let (repairs, unmatched) = narrow_to_ids(
            repairs,
            |r| r.record.display().to_string(),
            &ids,
            unmatched,
            "matched no pending repair (an entry is a record's repo-relative path, e.g. docs/adr/ADR-1-title.md)",
        );
        return emit(&build_fix_report(
            population,
            repairs,
            unmatched,
            None,
            false,
            drift_notices,
        ));
    }

    let identity = match resolve_attestation(by.as_deref(), actor, &repo_root, &config) {
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
        let record_type = records
            .iter()
            .find(|r| r.path == repair.record)
            .map(|r| r.record_type.as_str());
        // A status an `authority` gate names is a decider's, never computed from
        // evidence and written (`ADR-70` amendment, `ADR-89` section 4).
        if let (Some(t), Some(authority)) = (record_type, config.authority.as_ref()) {
            let status_field = urzua_core::rules::relation_field_name(
                &config,
                t,
                urzua_core::config::RelationRole::Status,
            );
            if repair.field == status_field
                && authority
                    .status_gates(t, &repair.computed_value)
                    .next()
                    .is_some()
            {
                failed.push((
                    repair.record.clone(),
                    format!(
                        "refused: '{}' is a {status_field} an authority gate names, so only a decider sets it and fix never writes it",
                        repair.computed_value
                    ),
                ));
                continue;
            }
        }
        let log_id = record_type
            .map(|t| config.revision_log_id(t))
            .unwrap_or_default();
        let log_shape = record_type
            .map(|t| config.revision_log_shape(t))
            .unwrap_or_default();
        let log_required = record_type.is_none_or(|t| config.revision_log_required(t));
        match urzua_core::fix::apply_repair(
            &fresh_content,
            &repair,
            &today,
            &identity.attestation,
            &log_id,
            &log_shape,
            log_required,
        ) {
            Ok(new_content) => {
                match urzua_io::write_atomic_in(&repo_root, &full_path, &new_content) {
                    Ok(()) => applied.push(repair),
                    Err(e) => failed.push((repair.record.clone(), format!("could not write: {e}"))),
                }
            }
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    emit(&build_fix_report(
        population,
        applied,
        failed,
        Some(identity),
        true,
        drift_notices,
    ))
}

/// The identity an apply writes under: the attestation the revision rows
/// carry, the report's account of it, and the notice for a replaced `--by`.
struct ResolvedAttestation {
    attestation: urzua_core::fix::Attestation,
    report: urzua_core::report::AttestationReport,
    warning: Option<String>,
}

/// Resolve who attests under the configuration's `attestation` (`ADR-89`
/// section 5, `MILE-201`). Runs before the lock and any write, so a refusal
/// (`require_verified` with no `gh` answer) writes nothing.
fn resolve_attestation(
    by: Option<&str>,
    actor: urzua_core::fix::ActorKind,
    repo_root: &std::path::Path,
    config: &urzua_core::config::Config,
) -> Result<ResolvedAttestation, String> {
    use urzua_core::fix::IdentitySource as Source;
    let policy = urzua_io::IdentityPolicy {
        asserted: config.attestation.identity == urzua_core::config::AttestationIdentity::Asserted,
        require_verified: config.attestation.require_verified,
    };
    let resolved = urzua_io::resolve_identity_with(by, repo_root, policy)?;
    let source = match resolved.source {
        urzua_io::IdentitySource::GhVerified => Source::GhVerified,
        urzua_io::IdentitySource::Asserted => Source::Asserted,
        urzua_io::IdentitySource::GitConfig => Source::GitConfig,
    };
    let mut attestation = urzua_core::fix::Attestation::new(&resolved.name, source, actor);
    attestation.verified_login = resolved.verified_login.clone();
    let report = urzua_core::report::AttestationReport {
        name: resolved.name,
        source,
        actor,
        replaced_by: resolved.replaced_by,
        verified_login: resolved.verified_login,
    };
    Ok(ResolvedAttestation {
        attestation,
        report,
        warning: resolved.warning,
    })
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
        match crate::discovery::relative_to_root(repo_root, p) {
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
    identity: Option<ResolvedAttestation>,
    ran_apply: bool,
    extra_notices: Vec<Notice>,
) -> FixReport {
    build_report(
        population,
        repairs,
        Vec::new(),
        Vec::new(),
        failed,
        identity,
        ran_apply,
        extra_notices,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    population: urzua_core::report::Population,
    repairs: Vec<urzua_core::fix::Repair>,
    rebaselines: Vec<urzua_core::fix::Rebaseline>,
    failed_pins: Vec<urzua_core::fix::FailedPin>,
    failed: Vec<(PathBuf, String)>,
    identity: Option<ResolvedAttestation>,
    ran_apply: bool,
    extra_notices: Vec<Notice>,
) -> FixReport {
    let status = if population.examined() == 0 && !ran_apply && failed.is_empty() {
        FixStatus::NotRun
    } else if !failed.is_empty() || !failed_pins.is_empty() {
        FixStatus::PartialFailure
    } else if repairs.is_empty() && rebaselines.is_empty() {
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
        rebaselines,
        failed_pins,
        failed: failed
            .into_iter()
            .map(|(record, error)| FixFailure { record, error })
            .collect(),
        identity: identity.as_ref().map(|i| i.report.clone()),
        notices: identity
            .and_then(|i| i.warning)
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

// Implements: ADR-67 (attributed re-baselining of a pin)
/// `urzua fix --rebaseline` (`MILE-181`): detect lists the records whose
/// pinned evidence changed; apply re-pins the chosen ones. Selection, identity,
/// lock and re-read-before-write are exactly Tier 1 apply's, so the two modes
/// share one set of gates. A record is the selector (`--ids` takes its
/// repo-relative path), and all of its stale pins move together.
/// The dry-run counterpart of `--apply`'s `--ids` selection: keep only the
/// entries an `--ids` value names, and report an `--ids` value that named none
/// as a failure, as `--apply` does. No `--ids` keeps everything.
fn narrow_to_ids<T>(
    items: Vec<T>,
    key: impl Fn(&T) -> String,
    ids: &[String],
    mut failed: Vec<(PathBuf, String)>,
    unmatched_reason: &str,
) -> (Vec<T>, Vec<(PathBuf, String)>) {
    if ids.is_empty() {
        return (items, failed);
    }
    for id in ids {
        if !items.iter().any(|i| key(i) == *id) {
            failed.push((
                PathBuf::from(id),
                format!("--ids entry '{id}' {unmatched_reason}"),
            ));
        }
    }
    let kept = items
        .into_iter()
        .filter(|i| ids.contains(&key(i)))
        .collect();
    (kept, failed)
}

#[allow(clippy::too_many_arguments)]
fn run_rebaseline(
    repo_root: &std::path::Path,
    config: &urzua_core::config::Config,
    records: &[urzua_core::record::Record],
    scoped: &[urzua_core::record::Record],
    unmatched: Vec<(PathBuf, String)>,
    apply: bool,
    ids: &[String],
    force: bool,
    by: Option<String>,
    actor: urzua_core::fix::ActorKind,
    reason: &str,
    after_lock: &dyn Fn(),
) -> ExitCode {
    // The pin hashes written come from this detect pass, so for `--apply` the
    // lock is taken first: evidence edited between an earlier detect and the
    // lock would otherwise be hashed stale and attested as current.
    let identity = if apply {
        match resolve_attestation(by.as_deref(), actor, repo_root, config) {
            Ok(id) => Some(id),
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    } else {
        None
    };
    let _lock = if apply {
        match urzua_io::FixLock::acquire(repo_root) {
            Ok(lock) => Some(lock),
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    } else {
        None
    };
    after_lock();
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    let read = |locator: &urzua_core::locator::Locator| {
        crate::pins::read_pin(repo_root, config, &texts, locator)
    };
    let (population, found, failed_pins) =
        urzua_core::fix::detect_rebaselines(scoped, config, &read);
    let Some(identity) = identity else {
        let (found, unmatched) = narrow_to_ids(
            found,
            |r| r.record.display().to_string(),
            ids,
            unmatched,
            "matched no record with a stale @sha256: pin (an entry is a record's repo-relative path; only pinned locators are re-baselined)",
        );
        return emit(&build_report(
            population,
            Vec::new(),
            found,
            failed_pins,
            unmatched,
            None,
            false,
            Vec::new(),
        ));
    };
    let today = urzua_io::today();
    let mut failed = unmatched;
    if !force {
        for id in ids {
            if !found.iter().any(|r| r.record.display().to_string() == *id) {
                failed.push((
                    PathBuf::from(id),
                    format!(
                        "--ids entry '{id}' matched no record with a stale @sha256: pin (an entry is a record's repo-relative path; only pinned locators are re-baselined)"
                    ),
                ));
            }
        }
    }
    let mut applied = Vec::new();
    for rb in found
        .into_iter()
        .filter(|r| force || ids.contains(&r.record.display().to_string()))
    {
        let full_path = repo_root.join(&rb.record);
        let fresh = match urzua_io::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                failed.push((rb.record.clone(), format!("could not re-read: {e}")));
                continue;
            }
        };
        let log_id = records
            .iter()
            .find(|r| r.path == rb.record)
            .map(|r| config.revision_log_id(&r.record_type))
            .unwrap_or_default();
        let log_shape = records
            .iter()
            .find(|r| r.path == rb.record)
            .map(|r| config.revision_log_shape(&r.record_type))
            .unwrap_or_default();
        match urzua_core::fix::apply_rebaseline(
            &fresh,
            &rb,
            &today,
            &identity.attestation,
            reason,
            &log_id,
            &log_shape,
        ) {
            Ok(new) => match urzua_io::write_atomic_in(repo_root, &full_path, &new) {
                Ok(()) => applied.push(rb),
                Err(e) => failed.push((rb.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((rb.record.clone(), e)),
        }
    }
    emit(&build_report(
        population,
        Vec::new(),
        applied,
        failed_pins,
        failed,
        Some(identity),
        true,
        Vec::new(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, Workspace};

    fn git(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git should run");
        assert!(status.success(), "git {args:?} failed");
    }

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The evidence is hashed under the lock: a file edited after the
    /// re-baseline was started but before the lock was held is pinned at the
    /// text it has then, not the text an earlier read saw.
    #[test]
    fn rebaseline_apply_hashes_the_evidence_as_it_is_under_the_lock() {
        let v1 = "fn a() {}\n";
        let v2 = "fn a() { let _ = 1; }\n";
        let v3 = "fn a() { let _ = 3; }\n";
        let dir = Scratch(
            std::env::temp_dir().join(format!("urzua-fix-lock-race-{}", std::process::id())),
        );
        let _ = std::fs::remove_dir_all(&dir.0);
        std::fs::create_dir_all(dir.0.join("docs/adr")).unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::create_dir_all(dir.0.join(".urzua")).unwrap();
        git(&dir.0, &["init", "-q"]);
        git(&dir.0, &["config", "user.email", "t@example.com"]);
        git(&dir.0, &["config", "user.name", "t"]);
        std::fs::write(
            dir.0.join(".urzua/config.yaml"),
            "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
        )
        .unwrap();
        let record = format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs@sha256:{}'\n---\n# ADR-1: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n",
            urzua_io::sha256_hex_of_str(v1)
        );
        std::fs::write(dir.0.join("docs/adr/ADR-1-a.md"), record).unwrap();
        std::fs::write(dir.0.join("src/a.rs"), v1).unwrap();
        git(&dir.0, &["add", "-A"]);
        git(&dir.0, &["commit", "-q", "-m", "fixture"]);
        std::fs::write(dir.0.join("src/a.rs"), v2).unwrap();
        git(&dir.0, &["commit", "-qam", "edit"]);

        let root = dir.0.canonicalize().unwrap();
        let config_path = root.join(".urzua/config.yaml");
        let config = crate::discovery::load_config(&config_path).unwrap();
        let session = Session::load(
            Workspace {
                repo_root: root.clone(),
                config_path,
                config,
            },
            |_, _, _| Ok(()),
        )
        .unwrap();
        let evidence = root.join("src/a.rs");
        run_rebaseline(
            &root,
            &session.config,
            &session.records,
            &session.records,
            Vec::new(),
            true,
            &[],
            true,
            Some("ana".to_string()),
            urzua_core::fix::ActorKind::Unspecified,
            "checked",
            &|| std::fs::write(&evidence, v3).unwrap(),
        );
        let written = std::fs::read_to_string(root.join("docs/adr/ADR-1-a.md")).unwrap();
        assert!(
            written.contains(&format!("@sha256:{}", urzua_io::sha256_hex_of_str(v3))),
            "{written}"
        );
        assert!(
            !written.contains(&urzua_io::sha256_hex_of_str(v2)),
            "{written}"
        );
    }
}
