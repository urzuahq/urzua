//! `urzua fix --conflicts` (`MILE-284`): lists, and with `--apply` resolves,
//! the records a merge or rebase left unmerged. One kind of conflict is
//! resolved so far: `pin`, a record whose two sides differ only in what a
//! re-pin writes. Anything else is refused with its reason and left alone.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{Config, RelationRole};
use urzua_core::conflicts::{self, Shape, Stages};
use urzua_core::report::{
    Conflict, ConflictKind, ConflictOutcome, ConflictsReport, ConflictsStatus, CouldNotRun,
    FixFailure,
};

use super::fix::{identity_notices, refuse, resolve_attestation, Refusal, ResolvedAttestation};
use crate::emit;
use crate::session::Workspace;

pub(crate) struct ConflictsArgs<'a> {
    pub apply: bool,
    pub ids: &'a [String],
    pub force: bool,
    pub by: Option<&'a str>,
    pub actor: urzua_core::fix::ActorKind,
}

pub(crate) fn run(config_path: Option<PathBuf>, args: ConflictsArgs<'_>) -> ExitCode {
    run_checked(config_path, args).unwrap_or_else(|e| emit(&CouldNotRun::from(e)))
}

fn run_checked(config_path: Option<PathBuf>, args: ConflictsArgs<'_>) -> Result<ExitCode, Refusal> {
    // Records are not loaded: a record carrying conflict markers need not parse.
    let Workspace {
        repo_root, config, ..
    } = Workspace::open(&[], config_path).map_err(refuse)?;
    let unmerged = unmerged_paths(&repo_root).map_err(refuse)?;
    let identity = if args.apply {
        Some(resolve_attestation(args.by, args.actor, &repo_root, &config).map_err(refuse)?)
    } else {
        None
    };
    let _lock = if args.apply {
        Some(urzua_io::FixLock::acquire(&repo_root).map_err(refuse)?)
    } else {
        None
    };
    let (records, other_unmerged): (Vec<_>, Vec<_>) = unmerged
        .iter()
        .cloned()
        .partition(|p| record_type(&config, p).is_some());
    let mut failed: Vec<FixFailure> = args
        .ids
        .iter()
        .filter(|id| !records.iter().any(|r| r.display().to_string() == **id))
        .map(|id| FixFailure {
            record: PathBuf::from(id),
            error: format!("--ids entry '{id}' matched no unmerged record"),
        })
        .collect();
    let ctx = Ctx {
        repo_root: &repo_root,
        config: &config,
        unmerged: &unmerged,
        today: urzua_io::today(),
    };
    let mut found = Vec::new();
    for path in &records {
        let selected = args.force || args.ids.contains(&path.display().to_string());
        let write = identity.as_ref().filter(|_| selected);
        if args.apply && !selected {
            continue;
        }
        match ctx.resolve(path, write) {
            Ok(c) => found.push(c),
            Err(e) => failed.push(FixFailure {
                record: path.clone(),
                error: e,
            }),
        }
    }
    let status = status_of(&unmerged, &records, &found, &failed, args.apply);
    Ok(emit(&ConflictsReport {
        status,
        as_of: crate::as_of(),
        conflicts: found,
        other_unmerged,
        failed,
        identity: identity.as_ref().map(|i| i.report.clone()),
        notices: identity_notices(identity.as_ref()),
    }))
}

fn status_of(
    unmerged: &BTreeSet<PathBuf>,
    records: &[PathBuf],
    found: &[Conflict],
    failed: &[FixFailure],
    applied: bool,
) -> ConflictsStatus {
    if unmerged.is_empty() {
        ConflictsStatus::NothingConflicted
    } else if records.is_empty() {
        ConflictsStatus::NoRecordConflicts
    } else if !failed.is_empty() || found.iter().any(|c| c.outcome == ConflictOutcome::Refused) {
        ConflictsStatus::PartialFailure
    } else if applied {
        ConflictsStatus::Resolved
    } else {
        ConflictsStatus::Resolvable
    }
}

/// The paths git lists as unmerged, repo-relative.
pub(crate) fn unmerged_paths(repo_root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let out = urzua_io::git::stdout_in(repo_root, &["ls-files", "-u", "-z"])?;
    Ok(out
        .split(|b| *b == 0)
        .filter_map(|entry| {
            let entry = std::str::from_utf8(entry).ok()?;
            entry.split_once('\t').map(|(_, p)| PathBuf::from(p))
        })
        .collect())
}

pub(crate) fn record_type<'c>(config: &'c Config, path: &Path) -> Option<&'c str> {
    config
        .record_types
        .iter()
        .find(|(_, t)| crate::discovery::type_governs(t, path))
        .map(|(name, _)| name.as_str())
}

struct Ctx<'a> {
    repo_root: &'a Path,
    config: &'a Config,
    unmerged: &'a BTreeSet<PathBuf>,
    today: String,
}

impl Ctx<'_> {
    /// The record's conflict, resolved and written when `write` carries the
    /// identity; a refusal is a `Refused` conflict, an I/O failure an `Err`.
    fn resolve(
        &self,
        path: &Path,
        write: Option<&ResolvedAttestation>,
    ) -> Result<Conflict, String> {
        let refused = |reason: String| Conflict {
            record: path.to_path_buf(),
            kind: ConflictKind::Pin,
            outcome: ConflictOutcome::Refused,
            reason: Some(reason),
            pins: Vec::new(),
            version: None,
            rows_kept: 0,
        };
        let working = urzua_io::read_to_string(&self.repo_root.join(path))
            .map_err(|e| format!("could not read: {e}"))?;
        if !conflicts::has_conflict_markers(&working) {
            return Ok(refused("the working file carries no conflict markers: it was resolved by hand; stage it with git add".into()));
        }
        let t = record_type(self.config, path).unwrap_or_default();
        let type_config = &self.config.record_types[t];
        let log_id = self.config.revision_log_id(t);
        let log_shape = self.config.revision_log_shape(t);
        let shape = Shape {
            evidence_field: urzua_core::rules::relation_field_name(
                self.config,
                t,
                RelationRole::EmbodimentLocator,
            ),
            version_field: type_config.version_bump().map(|b| b.field.trim()),
            log_id: &log_id,
            log_shape: &log_shape,
        };
        let Some((base, ours, theirs)) = self.stages(path) else {
            return Ok(refused("git has no merge-base, ours and theirs version of this record (added or deleted on one side)".into()));
        };
        let stages = Stages {
            base: &base,
            ours: &ours,
            theirs: &theirs,
        };
        let prepared = match conflicts::prepare(&stages, &shape) {
            Ok(p) => p,
            Err(e) => return Ok(refused(format!("could not read the record's sides: {e}"))),
        };
        let merged = match merge_file(&prepared)? {
            Ok(m) => m,
            Err(near) => return Ok(refused(format!(
                "conflicts outside the evidence pins, the version and the appended revision-log rows (near: {near}); resolve it by hand"
            ))),
        };
        let mut recomputed = Vec::new();
        for pin in prepared.pins.iter().filter(|p| p.needs_recompute()) {
            match self.merged_hash(&pin.spelling, &pin.path) {
                Ok(h) => recomputed.push((pin.spelling.clone(), h)),
                Err(e) => return Ok(refused(e)),
            }
        }
        let label_only = urzua_core::fix::Attestation::new(
            "",
            urzua_core::fix::IdentitySource::Asserted,
            urzua_core::fix::ActorKind::Unspecified,
        );
        let attestation = write.map_or(&label_only, |w| &w.attestation);
        let resolved = match conflicts::rebuild(
            &merged,
            &prepared,
            &recomputed,
            &shape,
            &self.today,
            attestation,
        ) {
            Ok(r) => r,
            Err(e) => return Ok(refused(e)),
        };
        if write.is_some() {
            urzua_io::write_atomic_in(
                self.repo_root,
                &self.repo_root.join(path),
                &resolved.content,
            )
            .map_err(|e| format!("could not write: {e}"))?;
        }
        Ok(Conflict {
            record: path.to_path_buf(),
            kind: ConflictKind::Pin,
            outcome: if write.is_some() {
                ConflictOutcome::Resolved
            } else {
                ConflictOutcome::Resolvable
            },
            reason: None,
            pins: resolved.pins,
            version: resolved.version,
            rows_kept: resolved.rows_kept,
        })
    }

    /// The record's merge-base, ours and theirs index stages.
    fn stages(&self, path: &Path) -> Option<(String, String, String)> {
        let stage = |n: u8| {
            urzua_io::git::stdout_in(
                self.repo_root,
                &["show", &format!(":{n}:{}", path.display())],
            )
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .ok()
        };
        Some((stage(1)?, stage(2)?, stage(3)?))
    }

    /// What the pin hashes to in the merged working tree; refused while the
    /// pinned file is itself unmerged or unreadable.
    fn merged_hash(&self, spelling: &str, path: &str) -> Result<String, String> {
        let file = Path::new(path.split('#').next().unwrap_or(path));
        if self.unmerged.contains(file) {
            return Err(format!(
                "the pinned file {path} is itself unmerged: resolve it first, then run fix --conflicts again"
            ));
        }
        let locator = urzua_core::locator::parse_locator(spelling)
            .map_err(|e| format!("could not read the pin on '{spelling}': {e}"))?;
        let texts = crate::file_texts::FileTexts::new(self.repo_root.to_path_buf(), self.config);
        match crate::pins::read_pin(self.repo_root, self.config, &texts, &locator) {
            urzua_core::fix::PinRead::Current { hash, .. } => Ok(hash),
            urzua_core::fix::PinRead::Failed(reason) => Err(format!(
                "the pin on '{spelling}' cannot be recomputed: {reason}"
            )),
            urzua_core::fix::PinRead::Unreadable => Err(format!(
                "the pinned file {path} is missing or unreadable in the merged tree"
            )),
        }
    }
}

/// The three-way merge of the normalised texts: `Err` with the first
/// conflicting line when it does not merge cleanly.
fn merge_file(prepared: &conflicts::Prepared) -> Result<Result<String, String>, String> {
    let dir = std::env::temp_dir().join(format!(
        "urzua-conflicts-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create a scratch dir: {e}"))?;
    let result = (|| {
        for (name, text) in [
            ("ours", &prepared.ours),
            ("base", &prepared.base),
            ("theirs", &prepared.theirs),
        ] {
            std::fs::write(dir.join(name), text).map_err(|e| format!("could not write: {e}"))?;
        }
        let out = urzua_io::git::GitRun::new()
            .args(["merge-file", "-p", "ours", "base", "theirs"])
            .cwd(&dir)
            .output()
            .map_err(|e| format!("could not run git: {}", e.into_io()))?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        match out.status.code() {
            Some(0) => Ok(Ok(text)),
            Some(n) if n > 0 => Ok(Err(
                conflicts::first_conflict_line(&text).unwrap_or_default()
            )),
            _ => Err(format!(
                "git merge-file failed: {}",
                urzua_io::git::stderr_text(&out)
            )),
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The notice `check` and `fix --rebaseline` raise when git lists records as
/// unmerged, so `urzua fix --conflicts` becomes their next action.
pub(crate) fn unmerged_records_notice(
    repo_root: &Path,
    config: &Config,
) -> Option<urzua_core::report::Notice> {
    let records: Vec<String> = unmerged_paths(repo_root)
        .ok()?
        .into_iter()
        .filter(|p| record_type(config, p).is_some())
        .map(|p| p.display().to_string())
        .collect();
    (!records.is_empty()).then(|| urzua_core::report::Notice {
        severity: urzua_core::report::NoticeSeverity::Warning,
        subject: urzua_core::report::UNMERGED_RECORDS.to_string(),
        message: format!(
            "{} unmerged after a merge or rebase ({}); urzua fix --conflicts lists which it can resolve",
            if records.len() == 1 { "1 record is".to_string() } else { format!("{} records are", records.len()) },
            records.join(", ")
        ),
        detail: None,
    })
}
