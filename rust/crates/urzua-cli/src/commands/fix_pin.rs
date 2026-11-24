//! `urzua fix --pin`: the attested way to give a locator its first pin, to move
//! a pin to another locator (`--replace OLD=NEW`) and to drop a pinned locator
//! (`--remove OLD`). Each write appends a row `diff.pin-changed-unattested`
//! reads, so a pull request that changes evidence this way passes and one that
//! types a hash by hand does not.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::config::Config;
use urzua_core::fix::pin::PinnedEntry;
use urzua_core::fix::{Attestation, PinRead};
use urzua_core::locator::{parse_locator, Anchor, Locator};
use urzua_core::record::Record;
use urzua_core::report::{AttestationReport, FixFailure, NextAction, Notice, Report};
use urzua_core::reverify::{apply_reverify, review_locators, FirstPin, ReverifyPlan};

use super::evidence_log::{evidence_log, with_evidence_log, EvidenceLog, LogHistory};
use super::fix::{refuse, repo_relative_ids, resolve_attestation, validate_reason, Refusal};
use crate::emit;
use crate::session::Session;

const REASON_PLACEHOLDER: &str = "<why this evidence carries the claim>";

pub(crate) struct PinArgs<'a> {
    pub apply: bool,
    pub ids: &'a [String],
    pub by: Option<&'a str>,
    pub actor: urzua_core::fix::ActorKind,
    pub reason: Option<&'a str>,
    pub replace: Option<&'a str>,
    pub remove: Option<&'a str>,
    pub base: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct PinEntryReport {
    record: PathBuf,
    action: &'static str,
    pins: Vec<String>,
    /// `code:` and `test:` locators left unpinned, each with why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    not_pinned: Vec<String>,
    /// Whether the record has no revision log and the write creates one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    creates_revision_log: bool,
}

#[derive(serde::Serialize)]
struct PinReport {
    status: &'static str,
    records: Vec<PinEntryReport>,
    failed: Vec<FixFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<AttestationReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    #[serde(skip)]
    failure_action: Option<NextAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision_log_base: Option<urzua_core::report::RevisionLogBase>,
}

impl Report for PinReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(u8::from(!self.failed.is_empty()))
    }
    fn next_action(&self) -> Option<NextAction> {
        if let Some(next) = &self.failure_action {
            return Some(next.clone());
        }
        if self.status != "pins-available" {
            return None;
        }
        let ids: Vec<String> = self
            .records
            .iter()
            .map(|r| r.record.display().to_string())
            .collect();
        let joined = ids.join(",");
        Some(
            NextAction::runs_as_printed(
                [
                    "urzua",
                    "fix",
                    "--pin",
                    "--apply",
                    "--ids",
                    joined.as_str(),
                    "--actor",
                    "agent",
                    "--reason",
                    REASON_PLACEHOLDER,
                ],
                format!(
                    "{} record(s) have unpinned code or test locators",
                    ids.len()
                ),
            )
            .with_input(
                REASON_PLACEHOLDER,
                "--reason",
                "why the pinned code or test carries the record's claim, one line without '|'",
            ),
        )
    }
}

/// An `--ids` entry that names none of `selected`, as a failure naming it.
fn unknown_ids(ids: &[String], selected: &[&Record]) -> Vec<FixFailure> {
    ids.iter()
        .filter(|i| !selected.iter().any(|r| r.path == PathBuf::from(i)))
        .map(|i| FixFailure {
            record: PathBuf::from(i),
            error: "names no record (an entry is a record's repo-relative path)".to_string(),
        })
        .collect()
}

pub(crate) fn run(config_path: Option<PathBuf>, args: PinArgs<'_>) -> Result<ExitCode, Refusal> {
    if (args.replace.is_some() || args.remove.is_some()) && args.ids.len() != 1 {
        return Err(refuse(
            "urzua fix --pin --replace/--remove changes one record: name it with --ids <record>",
        ));
    }
    if args.replace.is_some() && args.remove.is_some() {
        return Err(refuse(
            "urzua fix --pin takes --replace or --remove, not both",
        ));
    }
    let reason = validate_reason(
        args.apply,
        args.reason,
        "urzua fix --pin --apply: pass --reason <why this evidence carries the claim>; a pin without a reason attests nothing",
    )
    .map_err(refuse)?;
    let session = Session::open(config_path).map_err(refuse)?;
    let (repo_root, config) = (&session.repo_root, &session.config);
    let ids = repo_relative_ids(repo_root, args.ids);
    let args = PinArgs { ids: &ids, ..args };
    if args.apply {
        config
            .attestation
            .reason
            .check(&reason)
            .map_err(|e| refuse(format!("urzua fix --pin --apply: {e}")))?;
    }
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
    let texts = crate::file_texts::FileTexts::new(repo_root.clone(), config);
    let read = |l: &Locator| crate::pins::read_pin(repo_root, config, &texts, l);
    let selected: Vec<&Record> = session
        .records
        .iter()
        .filter(|r| args.ids.is_empty() || args.ids.iter().any(|i| r.path == PathBuf::from(i)))
        .collect();
    let mut failed = unknown_ids(args.ids, &selected);
    let mut done = Vec::new();
    let mut notices: Vec<Notice> = Vec::new();
    let history = LogHistory::resolve(repo_root, args.base)
        .map_err(|e| refuse(format!("urzua fix --pin: {e}")))?;
    let write = identity.as_ref().map(|w| Write {
        history: &history,
        session: &session,
        attestation: &w.attestation,
        reason: &reason,
    });
    let mut failure_action = None;
    for record in selected {
        let dispatch = |write: Option<&Write<'_>>| match (args.replace, args.remove) {
            (Some(spec), _) => replace(record, config, &read, spec, write),
            (_, Some(old)) => remove(record, config, old, write),
            _ => first_pins(record, config, &read, write, !args.ids.is_empty()),
        };
        let outcome = planned_outcome(
            &history,
            config,
            record,
            &dispatch,
            write.as_ref(),
            &mut failure_action,
        );
        if let Ok(Some(entry)) = &outcome {
            notices.extend(uncommitted_notices(repo_root, entry));
        }
        match outcome {
            Ok(Some(entry)) => done.push(entry),
            Ok(None) => {}
            Err(error) => failed.push(FixFailure {
                record: record.path.clone(),
                error,
            }),
        }
    }
    let created = done.iter().any(|e| e.creates_revision_log);
    Ok(emit(&PinReport {
        status: report_status(args.apply, done.is_empty(), failed.is_empty()),
        records: done,
        failed,
        identity: identity.map(|i| i.report),
        notices: notices.into_iter().chain(history.notice(created)).collect(),
        failure_action,
        revision_log_base: history.report(created),
    }))
}

fn report_status(apply: bool, none_done: bool, none_failed: bool) -> &'static str {
    match (apply, none_done, none_failed) {
        (_, _, false) => "partial-failure",
        (true, _, true) => "applied",
        (false, true, true) => "ok",
        (false, false, true) => "pins-available",
    }
}

type PinOutcome = Result<Option<PinEntryReport>, String>;

/// One record's outcome. The listing runs first, writing nothing, so a record
/// the log plan refuses is never written and is refused the same way in both
/// modes; the refusal's command, if any, fills `failure_action` once.
fn planned_outcome(
    history: &LogHistory<'_>,
    config: &Config,
    record: &Record,
    dispatch: &dyn Fn(Option<&Write<'_>>) -> PinOutcome,
    write: Option<&Write<'_>>,
    failure_action: &mut Option<NextAction>,
) -> PinOutcome {
    let entry = match dispatch(None) {
        Ok(Some(entry)) => entry,
        other => return other,
    };
    let content =
        urzua_io::read_to_string(&history.repo_root().join(&record.path)).unwrap_or_default();
    let plan = evidence_log(
        history,
        config,
        Some(&record.record_type),
        &record.path,
        &content,
        "write",
    );
    let creates = match plan {
        EvidenceLog::Refused(error, next) => {
            if failure_action.is_none() {
                *failure_action = next;
            }
            return Err(error);
        }
        plan => matches!(plan, EvidenceLog::Create(_)),
    };
    let written = match write {
        Some(w) => dispatch(Some(w))?,
        None => Some(entry),
    };
    Ok(written.map(|e| PinEntryReport {
        creates_revision_log: creates,
        ..e
    }))
}

struct Write<'a> {
    history: &'a LogHistory<'a>,
    session: &'a Session,
    attestation: &'a Attestation,
    reason: &'a str,
}

impl Write<'_> {
    /// Writes `edit(content)` to the record, refused under a path gate.
    fn apply(
        &self,
        record: &Record,
        edit: impl FnOnce(&str, (&str, &Attestation, &str)) -> Result<String, String>,
    ) -> Result<(), String> {
        let config = &self.session.config;
        if let Some(refusal) = urzua_core::authority::path_gate_refusal(
            config.authority.as_ref(),
            &record.path,
            Some(self.attestation),
        ) {
            return Err(refusal);
        }
        let full = self.session.repo_root.join(&record.path);
        let fresh =
            urzua_io::read_to_string(&full).map_err(|e| format!("could not re-read: {e}"))?;
        let (fresh, created) = with_evidence_log(
            self.history,
            config,
            Some(&record.record_type),
            &record.path,
            &fresh,
            "write",
        )?;
        let today = urzua_io::today();
        let reason = if created {
            format!("{} {}", self.reason, self.history.row_note())
        } else {
            self.reason.to_string()
        };
        let new = edit(&fresh, (&today, self.attestation, &reason))?;
        urzua_io::write_atomic_in(&self.session.repo_root, &full, &new)
            .map_err(|e| format!("could not write: {e}"))
    }
}

fn log_shape(
    config: &Config,
    record: &Record,
) -> (
    urzua_core::config::RevisionLogId,
    urzua_core::config::RevisionLogShape,
) {
    (
        config.revision_log_id(&record.record_type),
        config.revision_log_shape(&record.record_type),
    )
}

/// Pins every unpinned `code:` and `test:` locator of the record. A record with
/// none is skipped when listing everything and failed when named.
fn first_pins(
    record: &Record,
    config: &Config,
    read: &dyn Fn(&Locator) -> PinRead,
    write: Option<&Write<'_>>,
    named: bool,
) -> Result<Option<PinEntryReport>, String> {
    let review = review_locators(record, config, &["code".into(), "test".into()], read);
    let pins = FirstPin::from_review(&review.locators);
    let not_pinned: Vec<String> = review
        .locators
        .iter()
        .filter(|l| !l.pinned && l.check == urzua_core::reverify::LocatorCheck::Unresolved)
        .filter(|l| ["code", "test"].contains(&l.category.as_str()))
        .map(|l| {
            format!(
                "{}: {}",
                l.locator,
                l.reason.as_deref().unwrap_or("does not resolve")
            )
        })
        .collect();
    if pins.is_empty() {
        return match (named, not_pinned.is_empty()) {
            (_, false) => Err(format!(
                "nothing could be pinned: {}",
                not_pinned.join("; ")
            )),
            (true, true) => Err("no unpinned code or test locator".to_string()),
            (false, true) => Ok(None),
        };
    }
    if let Some(w) = write {
        let (id, shape) = log_shape(config, record);
        let plan = ReverifyPlan {
            review: &review,
            pins: &pins,
            repins: &[],
            renewal: None,
        };
        w.apply(record, |text, (today, att, reason)| {
            apply_reverify(text, &plan, today, att, reason, &id, &shape)
        })?;
    }
    Ok(Some(PinEntryReport {
        record: record.path.clone(),
        action: "pin",
        pins: pins
            .iter()
            .map(|p| format!("{}@sha256:{}", p.display, p.hash))
            .collect(),
        not_pinned,
        creates_revision_log: false,
    }))
}

/// The pinned entry of `record`'s evidence whose locator is `name`.
fn pinned_entry(record: &Record, config: &Config, name: &str) -> Result<PinnedEntry, String> {
    let field = urzua_core::rules::relation_field_name(
        config,
        &record.record_type,
        urzua_core::config::RelationRole::EmbodimentLocator,
    );
    let text = urzua_core::rules::evidence_text(record, field).unwrap_or_default();
    let found: Vec<PinnedEntry> = urzua_core::locator::realized_by_locators(&text)
        .into_iter()
        .filter_map(|e| {
            let l = e.parsed.ok()?;
            match (&l.anchor, l.display() == name) {
                (Some(Anchor::Pin(h)), true) => Some(PinnedEntry {
                    category: e.category.clone(),
                    spelling: l.spelling.clone(),
                    hash: h.clone(),
                }),
                _ => None,
            }
        })
        .collect();
    match found.first() {
        Some(e) if found.iter().all(|f| f == e) => Ok(e.clone()),
        Some(_) => Err(format!("'{name}' is pinned at more than one hash: re-pin with `urzua fix --rebaseline` first")),
        None => Err(format!("the record has no pinned locator '{name}' (name it as `path` or `path#scope`, without the pin)")),
    }
}

fn replace(
    record: &Record,
    config: &Config,
    read: &dyn Fn(&Locator) -> PinRead,
    spec: &str,
    write: Option<&Write<'_>>,
) -> Result<Option<PinEntryReport>, String> {
    let (old_name, new_text) = spec
        .split_once('=')
        .ok_or("--replace takes OLD=NEW: the pinned locator and the one that replaces it")?;
    let old = pinned_entry(record, config, old_name.trim())?;
    let new_text = new_text.trim();
    let (category, spelling) = match new_text.split_once(':') {
        Some((c, s)) if ["code", "test"].contains(&c) => (c.to_string(), s),
        _ => (old.category.clone(), new_text),
    };
    let locator =
        parse_locator(spelling).map_err(|e| format!("'{spelling}' is not a locator: {e}"))?;
    if locator.anchor.is_some() {
        return Err(format!(
            "name the new locator without a pin: '{}'",
            locator.spelling
        ));
    }
    let hash = match read(&locator) {
        PinRead::Current { hash, .. } => hash,
        PinRead::Failed(why) => return Err(why),
        PinRead::Unreadable => return Err(format!("{} cannot be read", locator.path)),
    };
    let new = PinnedEntry {
        category,
        spelling: locator.spelling.clone(),
        hash,
    };
    if let Some(w) = write {
        let (id, shape) = log_shape(config, record);
        w.apply(record, |text, ctx| {
            urzua_core::fix::pin::apply_replace(text, &old, &new, ctx, &id, &shape)
        })?;
    }
    Ok(Some(PinEntryReport {
        record: record.path.clone(),
        action: "replace",
        pins: vec![old.text(), new.text()],
        not_pinned: Vec::new(),
        creates_revision_log: false,
    }))
}

fn remove(
    record: &Record,
    config: &Config,
    old_name: &str,
    write: Option<&Write<'_>>,
) -> Result<Option<PinEntryReport>, String> {
    let old = pinned_entry(record, config, old_name.trim())?;
    if let Some(w) = write {
        let (id, shape) = log_shape(config, record);
        w.apply(record, |text, ctx| {
            urzua_core::fix::pin::apply_remove(text, &old, ctx, &id, &shape)
        })?;
    }
    Ok(Some(PinEntryReport {
        record: record.path.clone(),
        action: "remove",
        pins: vec![old.text()],
        not_pinned: Vec::new(),
        creates_revision_log: false,
    }))
}

/// A pin is of the working tree at the time of pinning: a pinned file with
/// uncommitted changes is named, so no one commits a pin that matches no
/// committed content by accident.
fn uncommitted_notices(repo_root: &std::path::Path, entry: &PinEntryReport) -> Vec<Notice> {
    let skip = usize::from(entry.action == "replace");
    entry
        .pins
        .iter()
        .skip(skip)
        .filter_map(|p| {
            let spelling = p.split("@sha256:").next()?;
            let spelling = spelling.split_once(':').map_or(spelling, |(c, rest)| {
                if c == "code" || c == "test" { rest } else { spelling }
            });
            let path = parse_locator(spelling).ok()?.path;
            let out = urzua_io::git::stdout_in(repo_root, &["status", "--porcelain", "--", &path]).ok()?;
            (!out.is_empty()).then(|| {
                Notice::new(
                    urzua_core::report::NoticeSeverity::Warning,
                    "pin-uncommitted".to_string(),
                    format!(
                        "{path} has uncommitted changes: the pin is of the working tree now; commit the file with the record so the pin matches committed content"
                    ),
                )
            })
        })
        .collect()
}
