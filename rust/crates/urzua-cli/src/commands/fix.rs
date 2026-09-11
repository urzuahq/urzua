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

    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    let (examined, repairs) = urzua_core::fix::detect_repairs(&records);

    if !apply {
        return emit(&build_fix_report(
            examined,
            repairs,
            Vec::new(),
            None,
            false,
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
    let mut failed = Vec::new();
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
        match urzua_core::fix::apply_repair(&fresh_content, &repair, &today, &identity.name) {
            Ok(new_content) => match std::fs::write(&full_path, new_content) {
                Ok(()) => applied.push(repair),
                Err(e) => failed.push((repair.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    emit(&build_fix_report(
        examined,
        applied,
        failed,
        identity.warning,
        true,
    ))
}

/// `ran_apply` matters because the two modes disagree on what `examined ==
/// 0` should mean for the exit code: detect mode treats it as `NotRun`
/// (exit 2, nothing to report on); apply mode's exit code was always keyed
/// only on `failed.is_empty()`, independent of `examined` -- an empty/
/// undiscovered corpus with nothing to apply and nothing to fail on
/// exits 0, not 2, matching the pre-existing behavior this refactor must
/// not silently change.
fn build_fix_report(
    examined: usize,
    repairs: Vec<urzua_core::fix::Repair>,
    failed: Vec<(PathBuf, String)>,
    warning: Option<String>,
    ran_apply: bool,
) -> FixReport {
    let status = if examined == 0 && !ran_apply {
        FixStatus::NotRun
    } else if !failed.is_empty() {
        FixStatus::PartialFailure
    } else if repairs.is_empty() {
        FixStatus::Ok
    } else {
        FixStatus::RepairsAvailable
    };

    FixReport {
        status,
        records_examined: examined,
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
            .collect(),
    }
}
