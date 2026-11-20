//! The `ask` answer to an agent's gated move (`ADR-102`, amendment of
//! 2026-11-17): with `acceptance.in_session: ask`, the edit goes to the person
//! through the agent host instead of being refused, carrying its receipt.
//! Every case where the person cannot be asked stays a refusal that says what
//! to do instead.

use std::path::Path;

use serde::Deserialize;
use urzua_core::harness::InSession;

use super::super::receipt;
use super::question;
use super::{judge_record, resolved_rel, Judging, Request, Verdict};

/// What the adapter knows about the call: whether the host will show the
/// person a prompt for it, and why not when it will not.
#[derive(Deserialize, Default, Clone)]
pub(super) struct AskRequest {
    #[serde(default)]
    available: bool,
    #[serde(default)]
    why_not: String,
    /// Whether the host's question tool can be used instead where no prompt is
    /// shown (`ADR-105`): a main-session call of a tool that carries a receipt.
    #[serde(default)]
    question: bool,
}

fn refuse(mut refused: Verdict, why: &str) -> Verdict {
    refused.message = format!("{} {why}", refused.message);
    refused.extra = None;
    refused
}

/// The answer to an agent's edit the gates refused: the refusal, or, for one
/// move into a gated status under `in_session: ask` that the host can put to
/// the person, `ask` with the receipt. Nothing is stored either way.
pub(super) fn ask_or_refuse(j: &Judging<'_>, request: &Request, refused: Verdict) -> Verdict {
    let Some(mv) = refused.extra.as_deref().and_then(|e| match e {
        super::Extra::Gated { gated_move } => Some(gated_move.clone()),
        _ => None,
    }) else {
        return refused;
    };
    let asks = j
        .config
        .harness
        .as_ref()
        .is_some_and(|h| h.in_session() == InSession::Ask);
    // Where the mode asks about nothing, the call is answered exactly as
    // without it: a record filed straight at a gated status, an override named
    // for the call, or a path spelled other than the record it is folded to.
    let overridden = request
        .override_request
        .as_ref()
        .and_then(|o| o.name.clone())
        .or_else(|| std::env::var("URZUA_HOOK_OVERRIDE").ok())
        .is_some_and(|n| !n.trim().is_empty());
    if !asks || mv.from.is_none() || overridden {
        return refused;
    }
    let [path] = request.paths.as_slice() else {
        return refuse(refused, "The person is asked about one file per edit, and this call names several: make the gated move in an edit of that record alone.");
    };
    let mut why_not = String::new();
    let by_question = match request.ask.as_ref() {
        None => return refuse(refused, "This agent's adapter cannot ask the person at the keyboard about an edit, so the move stays refused; a person in the gate's group makes it outside the agent harness."),
        Some(a) if a.available => false,
        Some(a) if a.question => {
            why_not = a.why_not.clone();
            true
        }
        Some(a) => return refuse(refused, &a.why_not),
    };
    let as_written = crate::discovery::relative_to_root(j.repo_root, Path::new(path)).ok();
    if as_written.as_deref() != Some(mv.rel.as_path()) {
        return refused;
    }
    let gates = j
        .config
        .authority
        .as_ref()
        .map_or(0, |a| a.status_gates(&mv.record_type, &mv.to).count());
    if gates > 1 {
        return refuse(refused, "More than one decider group must decide this move, so one person's answer here cannot make it; it goes through a pull request the groups' members approve.");
    }
    let content = request.content_at(0).unwrap_or("").to_string();
    if by_question {
        return match by_answer(j, path, &mv, content, request) {
            Ok(v) => v,
            Err(why) => refuse(refused, &format!("{why_not} {why}")),
        };
    }
    match prepare(j, path, &mv, content) {
        Ok(v) => v,
        Err(why) => refuse(refused, &why),
    }
}

/// Config text as the prompt may show it.
fn shown(text: &str) -> String {
    receipt::sanitize_config(text, 120)
}

fn prepare(
    j: &Judging<'_>,
    path: &str,
    mv: &super::gate::GatedMove,
    content: String,
) -> Result<Verdict, String> {
    let gate = j
        .config
        .authority
        .as_ref()
        .ok_or("no authority table")?
        .status_gates(&mv.record_type, &mv.to)
        .next()
        .ok_or("the gate could not be found again")?;
    let (who, login) = decider(j, gate, mv)?;
    let (id, to, from, title) = shown_move(j, gate, mv, &content)?;
    let (field, group, gate_text) = (shown(&mv.field), shown(&mv.group), shown(&mv.gate));
    let left = mv.from.as_deref().unwrap_or_default();
    let narrative = format!(
        "{} ({gate_text}); by {login}, as the host session reported them (name read from {}); {}",
        receipt::move_phrase(&id, &mv.field, left, &to),
        who.source,
        receipt::END
    );
    let today = urzua_io::today();
    let (with, filled) =
        receipt::with_receipt(j.config, &mv.record_type, &content, &narrative, &to, &today)
            .map_err(|e| {
                format!(
            "The receipt could not be written into the record ({e}); fix the record, then retry."
        )
            })?;
    let fills = match filled.as_slice() {
        [] => String::new(),
        names => format!(
            ", and fills {} with today's date",
            names
                .iter()
                .map(|n| shown(n))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    if let Some(v) = judge_receipt(j, path, &with) {
        return Ok(v);
    }
    // The engine's sentence comes first and stands alone, built only from
    // values checked against the configuration; the record's own title, which
    // the agent wrote, follows, marked as such and sanitised.
    let reason = format!(
        "urzua: approve a gated move? {id} moves {} to {field} '{to}'. {gate_text}: only the decider group '{group}' may decide. Approving writes the whole file as shown, including any change to its body, with a revision-log row recording that the move was approved in this agent session by {login}. Approving also writes a row recording this as approved in an agent session, unverified{fills}. Decline unless you are in '{group}' and mean to make this move. The record's own title, as the agent wrote it: {}",
        from.replacen("from '", &format!("from {field} '"), 1),
        receipt::sanitize(&title, 80)
    );
    let mut v = Verdict::new("ask", Some(path), reason);
    v.extra = Some(Box::new(super::Extra::Ask { content: with }));
    Ok(v)
}

/// Who the session reports at the keyboard, and the decider login it matches.
/// The name is whatever gh or git config reports, so it is held to the gate's
/// group whatever its source, compared as reported (an invisible character
/// makes it a different name), and written as the group spells it.
fn decider(
    j: &Judging<'_>,
    gate: &urzua_core::authority::Gate,
    mv: &super::gate::GatedMove,
) -> Result<(receipt::Approver, String), String> {
    let authority = j.config.authority.as_ref().ok_or("no authority table")?;
    let who = receipt::approver(j.repo_root, j.config).map_err(|e| {
        format!("Who is at the keyboard could not be read ({e}), so the receipt could not name a decider; set git config user.name or sign in to gh, then retry.")
    })?;
    let login = authority
        .group(gate)
        .and_then(|g| g.logins.iter().find(|l| l.eq_ignore_ascii_case(&who.name)))
        .cloned();
    login.map(|l| (who_clone(&who), l)).ok_or_else(|| {
        let cleaned = receipt::sanitize(&who.name, 40);
        let note = if cleaned == who.name {
            String::new()
        } else {
            " (shown with invisible or control characters removed; the name as reported holds them, so it matches no login)".to_string()
        };
        format!(
            "The session reports '{cleaned}'{note} at the keyboard (read from {}), who is not in the decider group '{}', so the person is not asked; a member of '{}' approves the move, in their own session or outside the agent harness.",
            who.source,
            shown(&mv.group),
            shown(&mv.group)
        )
    })
}

fn who_clone(w: &receipt::Approver) -> receipt::Approver {
    receipt::Approver {
        name: w.name.clone(),
        source: w.source,
    }
}

/// The record's id, the status entered and the phrase for the status left,
/// each only in a form checked against the configuration: the id as the
/// type's prefix and a number, a status only as a value the type declares.
fn shown_move(
    j: &Judging<'_>,
    gate: &urzua_core::authority::Gate,
    mv: &super::gate::GatedMove,
    content: &str,
) -> Result<(String, String, String, String), String> {
    let record = crate::discovery::parse_prospective_record(j.config, &mv.rel, content)
        .map_err(|e| format!("The record could not be read ({e})."))?;
    let id = urzua_core::rules::record_id(&record)
        .filter(|id| {
            id.strip_prefix(&format!("{}-", record.type_prefix))
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .ok_or("The record's file name does not give an id of its type's form, so the person cannot be told which record this is; name the file as `urzua new` does, then retry.")?;
    let to = gate
        .to
        .iter()
        .find(|t| urzua_core::authority::same_status(t, &mv.to))
        .cloned()
        .unwrap_or_else(|| mv.to.clone());
    let decl = j
        .config
        .record_types
        .get(&mv.record_type)
        .and_then(|t| t.status_decl());
    let declared = |s: &str| decl.as_ref().is_some_and(|d| d.declares(s));
    if !declared(&to) {
        return Err(format!("The gate's status is not a value the type declares, so the move cannot be shown to the person; declare it under record_types.{}.statuses.", shown(&mv.record_type)));
    }
    let from = match &mv.from {
        Some(f) if declared(f) => format!("from '{f}'"),
        Some(_) => "from a status the type does not declare".to_string(),
        None => "as a new record".to_string(),
    };
    let title = urzua_core::rules::record_title(content, record.header.region).unwrap_or_default();
    Ok((id, shown(&to), from, title))
}

/// The editor hook's judgement of the edit with its receipt, when it refuses:
/// a person is never asked to approve an edit the record check would refuse.
fn judge_receipt(j: &Judging<'_>, path: &str, receipt: &str) -> Option<Verdict> {
    let harness = j.config.harness.as_ref()?;
    if !harness.enabled("editor-hook") {
        return None;
    }
    let rel = crate::discovery::relative_to_root(j.repo_root, Path::new(path)).ok()?;
    let rel = resolved_rel(j.repo_root, Path::new(path)).unwrap_or(rel);
    let rel_text = rel.to_string_lossy().to_string();
    let request = Request::default();
    let v = judge_record(
        &Judging {
            repo_root: j.repo_root,
            config_path: j.config_path,
            config: j.config,
            config_arg: j.config_arg.clone(),
            on: harness.on_unjudged("editor-hook"),
            actor: j.actor,
            rel,
            rel_text: &rel_text,
        },
        receipt.to_string(),
        &request,
    );
    (v.verdict == "block").then_some(v)
}

/// The receipt row's narrative and the record with it, for a move approved by
/// `how`.
fn with_row(
    j: &Judging<'_>,
    mv: &super::gate::GatedMove,
    content: &str,
    (id, to): (&str, &str),
    how: &str,
) -> Result<String, String> {
    let left = mv.from.as_deref().unwrap_or_default();
    let narrative = format!(
        "{} ({}); {how}; {}",
        receipt::move_phrase(id, &mv.field, left, to),
        shown(&mv.gate),
        receipt::END
    );
    receipt::with_receipt(
        j.config,
        &mv.record_type,
        content,
        &narrative,
        to,
        &urzua_io::today(),
    )
    .map(|(with, _)| with)
    .map_err(|e| {
        format!(
            "The receipt could not be written into the record ({e}); fix the record, then retry."
        )
    })
}

/// The question route (`ADR-105`): the edit consumes the person's approval of
/// this very move when one is on file, else the refusal hands the agent the
/// question to ask.
fn by_answer(
    j: &Judging<'_>,
    path: &str,
    mv: &super::gate::GatedMove,
    content: String,
    request: &Request,
) -> Result<Verdict, String> {
    let session = request
        .session
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("The host reported no session id, so the person's answer cannot be bound to this session; a person in the gate's group makes the move outside the agent harness.")?;
    let gate = j
        .config
        .authority
        .as_ref()
        .ok_or("no authority table")?
        .status_gates(&mv.record_type, &mv.to)
        .next()
        .ok_or("the gate could not be found again")?;
    let (who, login) = decider(j, gate, mv)?;
    let (id, to, _, _) = shown_move(j, gate, mv, &content)?;
    let left = mv.from.as_deref().unwrap_or_default();
    let dir = question::store(j.repo_root)
        .ok_or("The repository's git directory could not be found, so no approval can be kept.")?;
    let body = urzua_io::sha256_hex_of_str(&content);
    let rel = mv.rel.to_string_lossy().to_string();
    let how = |nonce: &str, host: Option<&str>| {
        let host = host.map_or(String::new(), |h| {
            format!(" under the host '{}'", receipt::sanitize(h, 40))
        });
        format!(
            "by {login}, answering urzua's question {nonce} in an agent session{host}, as the host session reported them (name read from {})",
            who.source
        )
    };
    if let Some(c) = question::consume(&dir, (&rel, left, &to), &body, session) {
        let with = with_row(
            j,
            mv,
            &content,
            (&id, &to),
            &how(&c.nonce, c.host.as_deref()),
        )?;
        if let Some(v) = judge_receipt(j, path, &with) {
            return Ok(v);
        }
        let mut v = Verdict::new(
            "approved",
            Some(path),
            format!("urzua harness (acceptance): the person approved {id} moving to '{to}' by answering urzua's question {}; the edit is written with its receipt row.", c.nonce),
        );
        v.extra = Some(Box::new(super::Extra::Ask { content: with }));
        return Ok(v);
    }
    let nonce: String = crate::commands::hooks::random_hex()
        .chars()
        .take(12)
        .collect();
    let with = with_row(j, mv, &content, (&id, &to), &how(&nonce, None))?;
    if let Some(v) = judge_receipt(j, path, &with) {
        return Ok(v);
    }
    let (field, gate_text, group) = (shown(&mv.field), shown(&mv.gate), shown(&mv.group));
    let (c, input) = question::issue(
        &dir,
        &nonce,
        &question::Move {
            rel: &rel,
            id: &id,
            field: &field,
            from: left,
            to: &to,
            gate: &gate_text,
            group: &group,
        },
        &body,
        session,
    )
    .map_err(|e| format!("The question could not be stored ({e})."))?;
    Err(format!(
        "Or ask the person here: call the AskUserQuestion tool on its own with exactly this input and no answers field: {input} If the person picks '{}', retry this same edit unchanged within {} minutes; it is then written with its receipt row. If they decline, do not retry. An answer is recorded only by urzua's hooks on AskUserQuestion; if the retry is refused again, the person runs `urzua harness install`.",
        c.approve,
        question::APPROVED_TTL / 60
    ))
}
