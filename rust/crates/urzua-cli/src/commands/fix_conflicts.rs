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
use super::merge_pins::Unverified;
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
    } else if applied || found.iter().all(|c| c.outcome == ConflictOutcome::Resolved) {
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

/// What a record's type declares that a resolution reads, owned.
struct TypeShape {
    evidence_field: String,
    version_field: Option<String>,
    log_id: urzua_core::config::RevisionLogId,
    log_shape: urzua_core::config::RevisionLogShape,
    bump_classes: Vec<String>,
    items: Option<urzua_core::rules::ItemsReader>,
}

impl TypeShape {
    fn of(config: &Config, path: &Path) -> Self {
        let t = record_type(config, path).unwrap_or_default();
        let bump = config.record_types[t].version_bump();
        TypeShape {
            evidence_field: urzua_core::rules::relation_field_name(
                config,
                t,
                RelationRole::EmbodimentLocator,
            )
            .to_string(),
            version_field: bump.map(|b| b.field.trim().to_string()),
            log_id: config.revision_log_id(t),
            log_shape: config.revision_log_shape(t).for_evidence(),
            bump_classes: bump
                .map(|b| {
                    b.classes
                        .iter()
                        .map(|c| urzua_core::config::strip_emphasis(c).to_string())
                        .collect()
                })
                .unwrap_or_default(),
            items: urzua_core::rules::ItemsReader::of(config, t),
        }
    }

    fn shape(&self) -> Shape<'_> {
        Shape {
            evidence_field: &self.evidence_field,
            version_field: self.version_field.as_deref(),
            log_id: &self.log_id,
            log_shape: &self.log_shape,
            bump_classes: &self.bump_classes,
            items: self.items.as_ref(),
        }
    }
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
            waits_on: Vec::new(),
        };
        let working = urzua_io::read_to_string(&self.repo_root.join(path))
            .map_err(|e| format!("could not read: {e}"))?;
        if !conflicts::has_conflict_markers(&working) {
            return Ok(self.without_markers(path, &working, refused));
        }
        let owned = TypeShape::of(self.config, path);
        let shape = owned.shape();
        let Some((base, ours, theirs)) = self.stages(path) else {
            return Ok(refused("git has no merge-base, ours and theirs version of this record (added or deleted on one side)".into()));
        };
        let heads = super::merge_pins::heads(self.repo_root);
        if let Ok(h) = &heads {
            if let Some(why) = self.forged_stage(path, h, &ours, &theirs) {
                return Ok(refused(why));
            }
        }
        let stages = Stages {
            base: &base,
            ours: &ours,
            theirs: &theirs,
            incoming_is_theirs: heads.as_ref().map_or(true, |h| h.incoming_is_theirs),
        };
        let prepared = match conflicts::prepare(&stages, &shape) {
            Ok(p) => p,
            Err(e) => return Ok(refused(format!("could not read the record's sides: {e}"))),
        };
        if let Some(why) = &prepared.item_mismatch {
            return Ok(refused(why.clone()));
        }
        let merged = match merge_file(&prepared)? {
            Ok(m) => m,
            Err(near) => return Ok(refused(format!(
                "conflicts outside the evidence pins, the version and the appended revision-log rows (near: {near}{}); resolve it by hand",
                in_item(&ours, &shape, &near)
            ))),
        };
        let recomputed = match self.recompute(path, &prepared, &heads) {
            Ok(r) => r,
            Err(c) => return Ok(*c),
        };
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
            waits_on: Vec::new(),
        })
    }

    /// The merged hash of each pin both sides moved, by its index; a refused
    /// conflict when one cannot be recomputed.
    fn recompute(
        &self,
        path: &Path,
        prepared: &conflicts::Prepared,
        heads: &Result<super::merge_pins::Heads, String>,
    ) -> Result<Vec<(usize, String)>, Box<Conflict>> {
        let refused = |reason: String| Conflict {
            record: path.to_path_buf(),
            kind: ConflictKind::Pin,
            outcome: ConflictOutcome::Refused,
            reason: Some(reason),
            pins: Vec::new(),
            version: None,
            rows_kept: 0,
            waits_on: Vec::new(),
        };
        let mut recomputed = Vec::new();
        let mut waits_on: Vec<PathBuf> = Vec::new();
        for (i, pin) in prepared.pins.iter().enumerate() {
            if !pin.needs_recompute() {
                continue;
            }
            match self.merged_hash(path, pin, heads) {
                Ok(h) => recomputed.push((i, h)),
                Err(Unverified::Unmerged(file)) => {
                    if !waits_on.contains(&file) {
                        waits_on.push(file);
                    }
                }
                Err(Unverified::Other(e)) => return Err(Box::new(refused(e))),
            }
        }
        if !waits_on.is_empty() {
            let files: Vec<String> = waits_on.iter().map(|p| p.display().to_string()).collect();
            return Err(Box::new(Conflict {
                waits_on,
                ..refused(format!(
                    "a pinned file is still unmerged ({}): its merged content does not exist yet, so the pin cannot be recomputed. Resolve and stage that file first (git add), then re-run urzua fix --conflicts",
                    files.join(", ")
                ))
            }));
        }
        Ok(recomputed)
    }

    /// A record git lists as unmerged whose working file has no markers: one
    /// this command already resolved (a merge row neither side has), or one
    /// resolved by hand.
    fn without_markers(
        &self,
        path: &Path,
        working: &str,
        refused: impl Fn(String) -> Conflict,
    ) -> Conflict {
        let merge_rows = |text: &str| -> Vec<String> {
            text.lines()
                .filter(|l| conflicts::is_machine_row(l))
                .map(str::to_string)
                .collect()
        };
        let sides: Vec<String> = self
            .stages(path)
            .map(|(_, o, t)| [merge_rows(&o), merge_rows(&t)].concat())
            .unwrap_or_default();
        if merge_rows(working).iter().any(|r| !sides.contains(r)) {
            return Conflict {
                outcome: ConflictOutcome::Resolved,
                reason: Some("already resolved by fix --conflicts; stage it with git add".into()),
                ..refused(String::new())
            };
        }
        refused("the working file carries no conflict markers: it was resolved by hand; stage it with git add".into())
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

    /// The merged pin hash when it is git's merge of two attested states;
    /// the refusal names the condition that failed and the ordinary re-pin.
    fn merged_hash(
        &self,
        path: &Path,
        pin: &conflicts::PinSides,
        heads: &Result<super::merge_pins::Heads, String>,
    ) -> Result<String, Unverified> {
        let why = match heads {
            Ok(heads) => super::merge_pins::verified_hash(
                self.repo_root,
                self.config,
                self.unmerged,
                heads,
                pin,
            ),
            Err(e) => Err(Unverified::Other(e.clone())),
        };
        why.map_err(|e| match e {
            Unverified::Unmerged(f) => Unverified::Unmerged(f),
            Unverified::Other(e) => Unverified::Other(format!(
                "the pin on '{}' ({}) cannot be recomputed on merge: {e}. Resolve the record by hand and give that pin an ordinary re-pin with a person's reason: urzua fix --rebaseline --apply --ids {} --by <you> --reason \"<what changed in the pinned file>\"",
                pin.spelling,
                pin.site,
                path.display()
            )),
        })
    }

    /// Why stage 2 or 3 is not the record as committed at the commit git
    /// names for that side: index stages written by hand would let the merge
    /// row claim a re-pin a side never made.
    fn forged_stage(
        &self,
        path: &Path,
        heads: &super::merge_pins::Heads,
        ours: &str,
        theirs: &str,
    ) -> Option<String> {
        for (stage, side, commit, text) in [
            (2, "our commit (HEAD)", &heads.ours, ours),
            (3, "their commit", &heads.theirs, theirs),
        ] {
            let committed =
                super::merge_pins::show(self.repo_root, &format!("{commit}:{}", path.display()))
                    .ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned());
            if committed.as_deref() != Some(text) {
                return Some(format!(
                    "index stage {stage} of this record is not the record at {side}: the stage was written by hand, so what that side committed cannot be told; resolve it by hand"
                ));
            }
        }
        None
    }
}

/// Which Done-means item a conflicting line lies in, for the refusal.
fn in_item(ours: &str, shape: &Shape<'_>, near: &str) -> String {
    let Some(reader) = shape.items else {
        return String::new();
    };
    let near = near.trim();
    if near.is_empty() {
        return String::new();
    }
    let header = if ours.starts_with("---") {
        ours.lines()
            .skip(1)
            .position(|l| l.trim_end() == "---")
            .map_or(0, |i| i + 2)
    } else {
        0
    };
    let lines: Vec<&str> = ours.lines().collect();
    reader
        .items(ours, header)
        .items
        .iter()
        .find(|item| {
            [
                Some(item.line),
                item.evidence.as_ref().map(|c| c.line),
                item.waiver.as_ref().map(|w| w.line),
            ]
            .into_iter()
            .flatten()
            .any(|n| lines.get(n - 1).is_some_and(|l| l.trim().starts_with(near)))
        })
        .map(|item| {
            format!(
                ", in Done means item {} '{}'",
                item.n,
                item.text.chars().take(60).collect::<String>()
            )
        })
        .unwrap_or_default()
}

/// The three-way merge of the normalised texts: `Err` with the first
/// conflicting line when it does not merge cleanly.
fn merge_file(prepared: &conflicts::Prepared) -> Result<Result<String, String>, String> {
    let (clean, out) = super::merge_pins::merge_three(
        prepared.ours.as_bytes(),
        prepared.base.as_bytes(),
        prepared.theirs.as_bytes(),
    )?;
    let text = String::from_utf8_lossy(&out).into_owned();
    Ok(if clean {
        Ok(text)
    } else {
        Err(conflicts::first_conflict_line(&text).unwrap_or_default())
    })
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
