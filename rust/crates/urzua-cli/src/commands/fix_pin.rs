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
use urzua_core::report::{ActionInput, AttestationReport, FixFailure, NextAction, Notice, Report};
use urzua_core::reverify::{apply_reverify, review_locators, FirstPin, ReverifyPlan};

use super::fix::{refuse, resolve_attestation, validate_reason, Refusal};
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
}

#[derive(serde::Serialize)]
struct PinEntryReport {
    record: PathBuf,
    action: &'static str,
    pins: Vec<String>,
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
}

impl Report for PinReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(u8::from(!self.failed.is_empty()))
    }
    fn next_action(&self) -> Option<NextAction> {
        if self.status != "pins-available" {
            return None;
        }
        let ids: Vec<String> = self
            .records
            .iter()
            .map(|r| r.record.display().to_string())
            .collect();
        Some(NextAction {
            command: format!(
                "urzua fix --pin --apply --ids {} --actor agent --reason \"{REASON_PLACEHOLDER}\"",
                ids.join(",")
            ),
            reason: format!(
                "{} record(s) have unpinned code or test locators",
                ids.len()
            ),
            inputs: vec![ActionInput {
                placeholder: REASON_PLACEHOLDER.to_string(),
                flag: "--reason".to_string(),
                means:
                    "why the pinned code or test carries the record's claim, one line without '|'"
                        .to_string(),
            }],
        })
    }
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
    let mut failed: Vec<FixFailure> = args
        .ids
        .iter()
        .filter(|i| !selected.iter().any(|r| r.path == PathBuf::from(i)))
        .map(|i| FixFailure {
            record: PathBuf::from(i),
            error: "names no record (an entry is a record's repo-relative path)".to_string(),
        })
        .collect();
    let mut done = Vec::new();
    let write = identity.as_ref().map(|w| Write {
        session: &session,
        attestation: &w.attestation,
        reason: &reason,
    });
    for record in selected {
        let outcome = match (args.replace, args.remove) {
            (Some(spec), _) => replace(record, config, &read, spec, write.as_ref()),
            (_, Some(old)) => remove(record, config, old, write.as_ref()),
            _ => first_pins(record, config, &read, write.as_ref(), !args.ids.is_empty()),
        };
        match outcome {
            Ok(Some(entry)) => done.push(entry),
            Ok(None) => {}
            Err(error) => failed.push(FixFailure {
                record: record.path.clone(),
                error,
            }),
        }
    }
    let status = match (args.apply, done.is_empty(), failed.is_empty()) {
        (_, _, false) => "partial-failure",
        (true, _, true) => "applied",
        (false, true, true) => "ok",
        (false, false, true) => "pins-available",
    };
    Ok(emit(&PinReport {
        status,
        records: done,
        failed,
        identity: identity.map(|i| i.report),
        notices: Vec::new(),
    }))
}

struct Write<'a> {
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
        let today = urzua_io::today();
        let new = edit(&fresh, (&today, self.attestation, self.reason))?;
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
    if pins.is_empty() {
        return if named {
            Err("no unpinned code or test locator whose file can be read".to_string())
        } else {
            Ok(None)
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
    }))
}
