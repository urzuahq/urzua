//! `urzua fix` (SPEC-8): detect mode only (ADR-0015 §3) is read-only, safe
//! in CI. Apply mode (`--ids`, `--by`, `--force`) writes computed values
//! back, gated hard.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{Config, RuleLevel};
use urzua_core::fix::{Attestation, FailedPin, Rebaseline, Repair};
use urzua_core::record::Record;
use urzua_core::report::{
    CouldNotRun, Failure, FixFailure, FixReport, FixStatus, Notice, NoticeSeverity, Population,
};

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

/// Why a command could not run: the sentence `urzua` prints as `not-run`.
pub(crate) type Refusal = Failure;

pub(crate) fn refuse(reason: impl Into<Failure>) -> Refusal {
    reason.into()
}

pub fn run(config_path: Option<PathBuf>, args: FixArgs) -> ExitCode {
    run_checked(config_path, args).unwrap_or_else(|e| emit(&CouldNotRun::from(e)))
}

/// Whether the configuration declares `rule` at a level above `off`.
pub(crate) fn rule_enabled(config: &Config, rule: &str) -> bool {
    config
        .rules
        .get(rule)
        .is_some_and(|s| s.level != RuleLevel::Off)
}

/// `run` with every refusal as an `Err`, so each step can stop with `?`.
fn run_checked(config_path: Option<PathBuf>, args: FixArgs) -> Result<ExitCode, Refusal> {
    let reason = check_usage(&args)?;
    let Session {
        repo_root,
        config,
        discovered,
        mut records,
        ..
    } = Session::open(config_path).map_err(refuse)?;
    // `PATHS` narrow what is examined, drifted and repaired (`MILE-159`); the
    // corpus a back-pointer resolves against stays whole. A path that matches
    // no record is a failure naming it, never a silent skip.
    let (scoped, unmatched, proposed) = select_scope(&repo_root, &config, &mut records, &args)?;
    // Drift is the expensive part (several `git` subprocesses per record) and
    // its only consumer is `embodiment.consistency`; with that rule off `fix`
    // has no embodiment verdict to repair, so say so instead of computing a
    // verdict from an empty drift set (`MILE-159`, as `check` gates it). The
    // `Status` repair (`MILE-183`) is the other thing `fix` writes, and needs
    // no drift, so `status.delivery-agreement` being on is enough to proceed.
    if !rule_enabled(&config, urzua_core::rules::RULE_EMBODIMENT_CONSISTENCY)
        && !rule_enabled(&config, urzua_core::rules::RULE_STATUS_DELIVERY_AGREEMENT)
    {
        return Err(refuse(
            "urzua fix repairs the Embodiment field that embodiment.consistency judges and the Status that status.delivery-agreement judges, and both rules are off or undeclared in the configuration: nothing to repair",
        ));
    }
    if args.rebaseline {
        return Ok(run_rebaseline(RebaselineArgs {
            repo_root: &repo_root,
            config: &config,
            records: &records,
            scoped: &scoped,
            unmatched,
            apply: args.apply,
            ids: &args.ids,
            force: args.force,
            by: args.by.as_deref(),
            actor: args.actor,
            reason: &reason,
            after_lock: &|| {},
        }));
    }
    let tree = Tree {
        repo_root: &repo_root,
        config: &config,
        discovered: &discovered,
        records: &records,
    };
    let detected = detect_repairs_in(&tree, &scoped, proposed.as_deref())?;
    if !args.apply {
        let (repairs, unmatched) = narrow_to_ids(
            detected.repairs,
            |r| r.record.display().to_string(),
            &args.ids,
            unmatched,
            "matched no pending repair (an entry is a record's repo-relative path, e.g. docs/adr/ADR-1-title.md)",
        );
        return Ok(emit(&build_fix_report(
            detected.population,
            repairs,
            unmatched,
            None,
            false,
            detected.notices,
        )));
    }
    apply_repairs(&tree, &args, detected, unmatched)
}

/// The usage errors of the command line that need no repository, in the order
/// they are reported; the validated `--reason` (empty when none applies).
fn check_usage(args: &FixArgs) -> Result<String, Refusal> {
    if args.tier != 1 {
        return Err(refuse(
            "only tier 1 is implemented so far -- tiers 2 and 3 are not yet built",
        ));
    }
    if args.apply && args.ids.is_empty() && !args.force {
        return Err(refuse(
            "urzua fix --apply: pass --ids <record,...> or --force (never the default)",
        ));
    }
    // `--force`, `--by` and `--actor` belong to a write; without `--apply` they
    // would be silently ignored, so they are a usage error (`--ids` instead
    // narrows the dry-run report).
    if !args.apply
        && (args.force
            || args.by.is_some()
            || args.actor != urzua_core::fix::ActorKind::Unspecified)
    {
        return Err(refuse(
            "urzua fix --force, --by and --actor belong to --apply: without it nothing is written, so they would be ignored (--ids alone narrows the dry-run report)",
        ));
    }
    if args.rebaseline && args.stdin_path.is_some() {
        return Err(refuse(
            "urzua fix --rebaseline works on files in the tree: it cannot be combined with --stdin-path",
        ));
    }
    if !args.rebaseline && args.reason.is_some() {
        return Err(refuse(
            "urzua fix --reason belongs to --rebaseline: a Tier 1 repair records its computation, not a reason",
        ));
    }
    let reason = validate_reason(
        args.rebaseline && args.apply,
        args.reason.as_deref(),
        "urzua fix --rebaseline --apply: pass --reason <why the evidence still holds>; a re-pin without a reason is a rubber stamp",
    )
    .map_err(refuse)?;
    if args.stdin_path.is_some()
        && (args.apply
            || args.force
            || !args.ids.is_empty()
            || args.by.is_some()
            || !args.paths.is_empty())
    {
        return Err(refuse(
            "urzua fix --stdin-path judges proposed content and writes nothing: it cannot be combined with PATHS, --apply, --ids, --by or --force",
        ));
    }
    Ok(reason)
}

/// The records `fix` examines, the `PATHS` that matched none, and the proposed
/// content when `--stdin-path` supplies it.
type Scope = (Vec<Record>, Vec<(PathBuf, String)>, Option<String>);

fn select_scope(
    repo_root: &Path,
    config: &Config,
    records: &mut Vec<Record>,
    args: &FixArgs,
) -> Result<Scope, Refusal> {
    let Some(stdin_path) = &args.stdin_path else {
        let (scoped, unmatched) = scope_records(repo_root, &args.paths, records);
        return Ok((scoped, unmatched, None));
    };
    // The proposed content replaces the on-disk record (or adds a new one)
    // for this one path; nothing else is examined or drifted.
    let rel = crate::discovery::relative_to_root(repo_root, stdin_path).map_err(refuse)?;
    let mut content = String::new();
    if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut content) {
        return Err(refuse(format!(
            "could not read the proposed content from stdin: {e}"
        )));
    }
    let record =
        crate::discovery::parse_prospective_record(config, &rel, &content).map_err(refuse)?;
    records.retain(|r| r.path != rel);
    records.push(record.clone());
    Ok((vec![record], Vec::new(), Some(content)))
}

/// The loaded repository a `fix` pass reads.
struct Tree<'a> {
    repo_root: &'a Path,
    config: &'a Config,
    discovered: &'a urzua_io::DiscoveredFiles,
    records: &'a [Record],
}

/// What the detect pass found: the population, the pending repairs and the
/// notices the report carries.
struct Detected {
    population: Population,
    repairs: Vec<Repair>,
    notices: Vec<Notice>,
}

/// The detect pass of Tier 1: drift, back-pointer evidence and the repairs
/// `scoped` needs. `proposed` is the `--stdin-path` content, if any.
fn detect_repairs_in(
    tree: &Tree<'_>,
    scoped: &[Record],
    proposed: Option<&str>,
) -> Result<Detected, Refusal> {
    let Tree {
        repo_root,
        config,
        discovered,
        records,
    } = *tree;
    let drift = if !rule_enabled(config, urzua_core::rules::RULE_EMBODIMENT_CONSISTENCY) {
        Default::default()
    } else {
        match proposed {
            Some(content) => crate::discovery::compute_drifted_records_for_content(
                repo_root, scoped, config, content,
            ),
            None => crate::discovery::compute_drifted_records(repo_root, scoped, config),
        }
    };
    let mut notices = crate::discovery::drift_notices(drift.shallow, &drift.git_problems);
    let back_pointed =
        back_pointer_evidence(repo_root, discovered, config, records).map_err(refuse)?;
    // The derived-blocked records (`ADR-96` section 7): `fix` writes no
    // `progress` status to one. Computed only when a type is opted in.
    let blocked = if scoped.iter().any(|r| r.items.is_some()) {
        urzua_core::rules::derived_blocked_records(records, config)
    } else {
        Default::default()
    };
    let (population, repairs, unevidenced) =
        crate::file_texts::with_locator_env(repo_root, config, &discovered.paths, records, |env| {
            let ctx = urzua_core::rules::DeliveryContext {
                back_pointed: &back_pointed,
                env,
                blocked: &blocked,
            };
            let notices = urzua_core::fix::unevidenced_successes(scoped, config, &ctx);
            let (population, repairs) =
                urzua_core::fix::detect_repairs(scoped, config, &drift.drifted, &drift.fresh, &ctx);
            (population, repairs, notices)
        });
    notices.extend(unevidenced);
    Ok(Detected {
        population,
        repairs,
        notices,
    })
}

/// `fix --apply`: write the selected repairs, each under the lock and after a
/// re-read, and report what was applied and what failed.
fn apply_repairs(
    tree: &Tree<'_>,
    args: &FixArgs,
    detected: Detected,
    unmatched: Vec<(PathBuf, String)>,
) -> Result<ExitCode, Refusal> {
    let identity = resolve_attestation(args.by.as_deref(), args.actor, tree.repo_root, tree.config)
        .map_err(refuse)?;
    let _lock = urzua_io::FixLock::acquire(tree.repo_root).map_err(refuse)?;

    let today = urzua_io::today();
    let selected: Vec<_> = detected
        .repairs
        .into_iter()
        .filter(|r| args.force || args.ids.contains(&r.record.display().to_string()))
        .collect();

    let mut applied = Vec::new();
    let mut failed = unmatched;
    // An `--ids` entry that selected nothing is a failure naming it, not a
    // clean `ok`: the selector is matched against a record's repo-relative
    // path, and a typo or a record id (`ADR-1`) would otherwise read as
    // "checked, nothing to do" while a repair is still pending. `--force`
    // selects every repair regardless, so it has no selectors to miss.
    if !args.force {
        failed.extend(missing_ids(
            &selected,
            |r| r.record.display().to_string(),
            &args.ids,
            "matched no pending repair (an entry is a record's repo-relative path, e.g. docs/adr/ADR-1-title.md)",
        ));
    }
    for repair in selected {
        match write_repair(tree, &repair, &today, &identity.attestation) {
            Ok(()) => applied.push(repair),
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    Ok(emit(&build_fix_report(
        detected.population,
        applied,
        failed,
        Some(identity),
        true,
        detected.notices,
    )))
}

/// Writes one repair into its record; the error is the failure's message.
fn write_repair(
    tree: &Tree<'_>,
    repair: &Repair,
    today: &str,
    attestation: &Attestation,
) -> Result<(), String> {
    let (repo_root, config) = (tree.repo_root, tree.config);
    let full_path = repo_root.join(&repair.record);
    // Re-read from disk right before writing (RFC-0008 §4's
    // re-verify-before-write): apply_repair itself fails if the current
    // value it was told to replace is no longer there, which is exactly
    // the case a concurrent edit since detect ran would produce.
    let fresh_content =
        urzua_io::read_to_string(&full_path).map_err(|e| format!("could not re-read: {e}"))?;
    let record_type = tree
        .records
        .iter()
        .find(|r| r.path == repair.record)
        .map(|r| r.record_type.as_str());
    if let Some(refusal) = authority_refusal(config, record_type, repair) {
        return Err(refusal);
    }
    let log_id = record_type
        .map(|t| config.revision_log_id(t))
        .unwrap_or_default();
    let log_shape = record_type
        .map(|t| config.revision_log_shape(t))
        .unwrap_or_default();
    let log_required = record_type.is_none_or(|t| config.revision_log_required(t));
    let new_content = urzua_core::fix::apply_repair(
        &fresh_content,
        repair,
        today,
        attestation,
        &log_id,
        &log_shape,
        log_required,
    )?;
    urzua_io::write_atomic_in(repo_root, &full_path, &new_content)
        .map_err(|e| format!("could not write: {e}"))
}

/// The refusal for a repair that would write a status an `authority` gate
/// names: that is a decider's, never computed from evidence and written
/// (`ADR-70` amendment, `ADR-89` section 4).
fn authority_refusal(
    config: &Config,
    record_type: Option<&str>,
    repair: &Repair,
) -> Option<String> {
    let (t, authority) = (record_type?, config.authority.as_ref()?);
    let status_field =
        urzua_core::rules::relation_field_name(config, t, urzua_core::config::RelationRole::Status);
    let gated = authority
        .status_gates(t, &repair.computed_value)
        .next()
        .is_some();
    (repair.field == status_field && gated).then(|| {
        format!(
            "refused: '{}' is a {status_field} an authority gate names, so only a decider sets it and fix never writes it",
            repair.computed_value
        )
    })
}

/// The `--reason` of an attributed write: required (and said so with
/// `required_message`) when `required`, always one line without `|` because it
/// is written into a revision-log table row. Shared by `fix --rebaseline` and
/// `reverify`.
pub(crate) fn validate_reason(
    required: bool,
    reason: Option<&str>,
    required_message: &str,
) -> Result<String, String> {
    match (required, reason.map(str::trim)) {
        (true, None | Some("")) => Err(required_message.to_string()),
        (_, Some(r)) if r.contains('|') || r.contains(['\n', '\r']) => Err(
            "--reason must be one line without '|' (it is written into a revision-log table row)"
                .to_string(),
        ),
        (_, r) => Ok(r.unwrap_or_default().to_string()),
    }
}

/// What the back-pointer scan finds in the tree, for the delivery tiers.
pub(crate) fn back_pointer_evidence(
    repo_root: &std::path::Path,
    discovered: &urzua_io::DiscoveredFiles,
    config: &urzua_core::config::Config,
    records: &[urzua_core::record::Record],
) -> Result<urzua_core::rules::BackPointerEvidence, String> {
    Ok(
        match crate::discovery::read_back_pointer_sources(repo_root, discovered, config)? {
            Some(sources) => {
                let (index, _) = urzua_core::rules::build_index_reporting_collisions(records);
                urzua_core::rules::back_pointer_evidence(
                    &sources.files,
                    &sources.back_pointer_paths,
                    &sources.test_paths,
                    config,
                    &index,
                )
            }
            None => Default::default(),
        },
    )
}

/// The report's `failed` list from `(record, error)` pairs.
pub(crate) fn into_failures(failed: Vec<(PathBuf, String)>) -> Vec<FixFailure> {
    failed
        .into_iter()
        .map(|(record, error)| FixFailure { record, error })
        .collect()
}

/// The notice for a replaced `--by`, when the attestation carries one.
pub(crate) fn identity_notices(identity: Option<&ResolvedAttestation>) -> Vec<Notice> {
    identity
        .and_then(|i| i.warning.clone())
        .into_iter()
        .map(|message| Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_IDENTITY.to_string(),
            message,
        })
        .collect()
}

/// The identity an apply writes under: the attestation the revision rows
/// carry, the report's account of it, and the notice for a replaced `--by`.
pub(crate) struct ResolvedAttestation {
    pub(crate) attestation: urzua_core::fix::Attestation,
    pub(crate) report: urzua_core::report::AttestationReport,
    pub(crate) warning: Option<String>,
}

/// Resolve who attests under the configuration's `attestation` (`ADR-89`
/// section 5, `MILE-201`). Runs before the lock and any write, so a refusal
/// (`require_verified` with no `gh` answer) writes nothing.
pub(crate) fn resolve_attestation(
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
    population: Population,
    repairs: Vec<Repair>,
    failed: Vec<(PathBuf, String)>,
    identity: Option<ResolvedAttestation>,
    ran_apply: bool,
    extra_notices: Vec<Notice>,
) -> FixReport {
    build_report(ReportParts {
        population,
        repairs,
        rebaselines: Vec::new(),
        failed_pins: Vec::new(),
        failed,
        identity,
        ran_apply,
        extra_notices,
    })
}

/// Everything a `fix` report is made of.
struct ReportParts {
    population: Population,
    repairs: Vec<Repair>,
    rebaselines: Vec<Rebaseline>,
    failed_pins: Vec<FailedPin>,
    failed: Vec<(PathBuf, String)>,
    identity: Option<ResolvedAttestation>,
    ran_apply: bool,
    extra_notices: Vec<Notice>,
}

fn build_report(parts: ReportParts) -> FixReport {
    let ReportParts {
        population,
        repairs,
        rebaselines,
        failed_pins,
        failed,
        identity,
        ran_apply,
        extra_notices,
    } = parts;
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
        as_of: crate::as_of(),
        status,
        population,
        repairs,
        rebaselines,
        failed_pins,
        failed: into_failures(failed),
        identity: identity.as_ref().map(|i| i.report.clone()),
        notices: identity_notices(identity.as_ref())
            .into_iter()
            .chain(extra_notices)
            .collect(),
    }
}

/// An `--ids` entry that names none of `items` as a failure naming it, saying
/// `unmatched_reason`. The selector is matched against a record's
/// repo-relative path, so a typo or a record id (`ADR-1`) is reported, not read
/// as "checked, nothing to do".
fn missing_ids<T>(
    items: &[T],
    key: impl Fn(&T) -> String,
    ids: &[String],
    unmatched_reason: &str,
) -> Vec<(PathBuf, String)> {
    ids.iter()
        .filter(|id| !items.iter().any(|i| key(i) == **id))
        .map(|id| {
            (
                PathBuf::from(id),
                format!("--ids entry '{id}' {unmatched_reason}"),
            )
        })
        .collect()
}

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
    failed.extend(missing_ids(&items, &key, ids, unmatched_reason));
    let kept = items
        .into_iter()
        .filter(|i| ids.contains(&key(i)))
        .collect();
    (kept, failed)
}

const REBASELINE_UNMATCHED: &str = "matched no record with a stale @sha256: pin (an entry is a record's repo-relative path; only pinned locators are re-baselined)";

/// What `fix --rebaseline` reads and selects by.
struct RebaselineArgs<'a> {
    repo_root: &'a Path,
    config: &'a Config,
    records: &'a [Record],
    scoped: &'a [Record],
    unmatched: Vec<(PathBuf, String)>,
    apply: bool,
    ids: &'a [String],
    force: bool,
    by: Option<&'a str>,
    actor: urzua_core::fix::ActorKind,
    reason: &'a str,
    after_lock: &'a dyn Fn(),
}

/// What the re-baseline detect pass found.
struct RebaselineScan {
    population: Population,
    found: Vec<Rebaseline>,
    failed_pins: Vec<FailedPin>,
}

// Implements: ADR-67 (attributed re-baselining of a pin)
/// `urzua fix --rebaseline` (`MILE-181`): detect lists the records whose
/// pinned evidence changed; apply re-pins the chosen ones. Selection, identity,
/// lock and re-read-before-write are exactly Tier 1 apply's, so the two modes
/// share one set of gates. A record is the selector (`--ids` takes its
/// repo-relative path), and all of its stale pins move together.
fn run_rebaseline(args: RebaselineArgs<'_>) -> ExitCode {
    rebaseline_checked(args).unwrap_or_else(|e| emit(&CouldNotRun::from(e)))
}

fn rebaseline_checked(mut args: RebaselineArgs<'_>) -> Result<ExitCode, Refusal> {
    let (repo_root, config) = (args.repo_root, args.config);
    // The pin hashes written come from this detect pass, so for `--apply` the
    // lock is taken first: evidence edited between an earlier detect and the
    // lock would otherwise be hashed stale and attested as current.
    let identity = if args.apply {
        Some(resolve_attestation(args.by, args.actor, repo_root, config).map_err(refuse)?)
    } else {
        None
    };
    let _lock = if args.apply {
        Some(urzua_io::FixLock::acquire(repo_root).map_err(refuse)?)
    } else {
        None
    };
    (args.after_lock)();
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    let read = |locator: &urzua_core::locator::Locator| {
        crate::pins::read_pin(repo_root, config, &texts, locator)
    };
    let (population, found, failed_pins) =
        urzua_core::fix::detect_rebaselines(args.scoped, config, &read);
    let scan = RebaselineScan {
        population,
        found,
        failed_pins,
    };
    let unmatched = std::mem::take(&mut args.unmatched);
    let Some(identity) = identity else {
        let (found, unmatched) = narrow_to_ids(
            scan.found,
            |r| r.record.display().to_string(),
            args.ids,
            unmatched,
            REBASELINE_UNMATCHED,
        );
        return Ok(emit(&build_report(ReportParts {
            population: scan.population,
            repairs: Vec::new(),
            rebaselines: found,
            failed_pins: scan.failed_pins,
            failed: unmatched,
            identity: None,
            ran_apply: false,
            extra_notices: Vec::new(),
        })));
    };
    Ok(apply_rebaselines(&args, identity, scan, unmatched))
}

/// `fix --rebaseline --apply`: re-pin the selected records and report what
/// was applied and what failed.
fn apply_rebaselines(
    args: &RebaselineArgs<'_>,
    identity: ResolvedAttestation,
    scan: RebaselineScan,
    unmatched: Vec<(PathBuf, String)>,
) -> ExitCode {
    let today = urzua_io::today();
    let mut failed = unmatched;
    if !args.force {
        failed.extend(missing_ids(
            &scan.found,
            |r| r.record.display().to_string(),
            args.ids,
            REBASELINE_UNMATCHED,
        ));
    }
    let mut applied = Vec::new();
    for rb in scan
        .found
        .into_iter()
        .filter(|r| args.force || args.ids.contains(&r.record.display().to_string()))
    {
        match write_rebaseline(args, &rb, &today, &identity.attestation) {
            Ok(()) => applied.push(rb),
            Err(e) => failed.push((rb.record.clone(), e)),
        }
    }
    emit(&build_report(ReportParts {
        population: scan.population,
        repairs: Vec::new(),
        rebaselines: applied,
        failed_pins: scan.failed_pins,
        failed,
        identity: Some(identity),
        ran_apply: true,
        extra_notices: Vec::new(),
    }))
}

/// Re-pins one record on disk; the error is the failure's message.
fn write_rebaseline(
    args: &RebaselineArgs<'_>,
    rb: &Rebaseline,
    today: &str,
    attestation: &Attestation,
) -> Result<(), String> {
    let full_path = args.repo_root.join(&rb.record);
    let fresh =
        urzua_io::read_to_string(&full_path).map_err(|e| format!("could not re-read: {e}"))?;
    let record_type = args
        .records
        .iter()
        .find(|r| r.path == rb.record)
        .map(|r| r.record_type.as_str());
    let log_id = record_type
        .map(|t| args.config.revision_log_id(t))
        .unwrap_or_default();
    let log_shape = record_type
        .map(|t| args.config.revision_log_shape(t))
        .unwrap_or_default();
    let new = urzua_core::fix::apply_rebaseline(
        &fresh,
        rb,
        today,
        attestation,
        args.reason,
        &log_id,
        &log_shape,
    )?;
    urzua_io::write_atomic_in(args.repo_root, &full_path, &new)
        .map_err(|e| format!("could not write: {e}"))
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
        run_rebaseline(RebaselineArgs {
            repo_root: &root,
            config: &session.config,
            records: &session.records,
            scoped: &session.records,
            unmatched: Vec::new(),
            apply: true,
            ids: &[],
            force: true,
            by: Some("ana"),
            actor: urzua_core::fix::ActorKind::Unspecified,
            reason: "checked",
            after_lock: &|| std::fs::write(&evidence, v3).unwrap(),
        });
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
