//! A person's approval of an agent's gated move, given through the agent host
//! (`ADR-102`, amendment of 2026-11-17).
//!
//! With `acceptance.in_session: ask`, the editor hook answers an agent's move
//! into a gated status with the host's ask-the-person decision instead of a
//! refusal. Before it answers, it rewrites the proposed file to carry the
//! receipt (the status's required date fields and a revision-log row), so the
//! person approves the edit and its receipt as one write and no commit can fall
//! between the two. It also keeps a pending approval under the git directory,
//! keyed by the record and naming the host's id for the call, the status left,
//! the status entered, the blob the edit starts from and the approved bytes.
//! When the host reports that very call landed (its post-edit event, which runs
//! only after an approved tool call, and whose transcript must show the call),
//! the approval is confirmed. The local commit check accepts an uncommitted
//! gated move only with a confirmed, unexpired, unused approval for exactly
//! those bytes on that base; the first check that finds the bytes committed
//! marks it used.
//!
//! What this is not: proof. The hooks run as the agent's own user, so a shell
//! command can produce every one of these files (by writing them, or by piping
//! made-up payloads to the hook), and the person a receipt names is whoever
//! `gh` or `git config` reported. The edit tools cannot write the git
//! directory, and the post-command check reports any change under the
//! approvals directory made during a shell command. The pull request check with
//! the actors file is the control a repository relies on; CI never reads this.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use urzua_core::config::Config;
use urzua_core::rules::SessionApproval;

pub(crate) const PENDING: &str = "pending";
pub(crate) const CONFIRMED: &str = "confirmed";
pub(crate) const CONSUMED: &str = "consumed";
/// The blob id kept for a record that did not exist at `HEAD`.
pub(crate) const NO_BLOB: &str = "none";
const SEEN: &str = "approvals.seen";

/// One approval as it is kept on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Approval {
    /// The record's path from the repository root.
    pub path: String,
    pub record_id: String,
    /// The status the edit leaves; `None` for a record filed at the gated one.
    pub from: Option<String>,
    /// The status the approved edit enters.
    pub status: String,
    /// The git blob id of the record at `HEAD` when the person was asked.
    pub base_blob: String,
    /// The sha256 of the whole file as the approved edit writes it, receipt included.
    pub body_sha256: String,
    pub session: String,
    /// The host's id for the edit call.
    pub call: String,
    /// The decider login the session's identity matched.
    pub identity: String,
    /// Where that name was read: `gh` or `git config`. Not a verification.
    pub identity_source: String,
    pub created_unix: i64,
    pub expires_unix: i64,
    pub state: String,
}

/// The wall clock, never `--as-of`: an approval's life is real time.
pub(crate) fn now() -> i64 {
    urzua_io::wall_clock_unix()
}

fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = urzua_io::git::GitRun::new()
        .dash_c(dir)
        .args(args)
        .discard_stderr()
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

fn git_path(dir: &Path, args: &[&str]) -> Option<PathBuf> {
    let raw = String::from_utf8(git(dir, args)?).ok()?;
    let p = PathBuf::from(raw.trim_end_matches('\n'));
    let p = if p.is_absolute() { p } else { dir.join(p) };
    p.canonicalize().ok()
}

/// `<git common dir>/urzua`, only when the common directory is the
/// repository's own: its git directory, or the directory a linked worktree's
/// git directory sits in (`<common>/worktrees/<name>`). A `commondir` file
/// pointed anywhere else is refused, so writing it cannot move the approvals.
pub(crate) fn git_state_dir(repo_root: &Path) -> Option<PathBuf> {
    let git_dir = git_path(repo_root, &["rev-parse", "--absolute-git-dir"])?;
    let common = git_path(repo_root, &["rev-parse", "--git-common-dir"])?;
    let own = common == git_dir
        || (git_dir.parent().and_then(Path::file_name) == Some("worktrees".as_ref())
            && git_dir.parent().and_then(Path::parent) == Some(common.as_path()));
    own.then(|| common.join("urzua"))
}

fn approvals_dir(repo_root: &Path) -> Option<PathBuf> {
    git_state_dir(repo_root).map(|d| d.join("approvals"))
}

fn file_for(dir: &Path, rel: &str) -> PathBuf {
    dir.join(format!("{}.json", urzua_io::sha256_hex_of_str(rel)))
}

fn read(path: &Path) -> Option<Approval> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn write(path: &Path, approval: &Approval) -> Result<(), String> {
    let text = serde_json::to_string_pretty(approval).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    urzua_io::write_atomic(path, &(text + "\n")).map_err(|e| e.to_string())
}

/// The blob id of `rel` at `HEAD`, or [`NO_BLOB`].
pub(crate) fn head_blob(repo_root: &Path, rel: &str) -> String {
    git(
        repo_root,
        &["rev-parse", "-q", "--verify", &format!("HEAD:{rel}")],
    )
    .and_then(|o| String::from_utf8(o).ok())
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| NO_BLOB.to_string())
}

/// Keeps `approval` as the one approval for its record, replacing any earlier one.
pub(crate) fn write_pending(repo_root: &Path, approval: &Approval) -> Result<(), String> {
    let dir = approvals_dir(repo_root).ok_or(
        "the repository's own git directory was not found (or its common directory points outside it)",
    )?;
    write(&file_for(&dir, &approval.path), approval)
}

/// Whether the host's transcript records `call` as a tool call. It raises the
/// cost of a made-up post-edit payload; the transcript is the user's file and
/// a shell can write it too.
fn transcript_shows(transcript: Option<&Path>, call: &str) -> bool {
    let Some(text) = transcript.and_then(|t| std::fs::read_to_string(t).ok()) else {
        return false;
    };
    text.lines().any(|line| {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            return false;
        };
        let mut found = false;
        walk(&v, &mut |o| {
            found |= o.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                && o.get("id").and_then(|i| i.as_str()) == Some(call);
        });
        found
    })
}

fn walk(v: &serde_json::Value, f: &mut dyn FnMut(&serde_json::Map<String, serde_json::Value>)) {
    match v {
        serde_json::Value::Object(o) => {
            f(o);
            o.values().for_each(|x| walk(x, f));
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, f)),
        _ => {}
    }
}

/// Confirms the pending approval for the host's call `call` once the file on
/// disk is the approved one and the transcript shows the call. `None` when no
/// pending approval names the call; `Err` when one does and does not hold (it
/// is dropped).
pub(crate) fn confirm(
    repo_root: &Path,
    call: &str,
    transcript: Option<&Path>,
) -> Option<Result<Approval, String>> {
    let dir = approvals_dir(repo_root)?;
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let Some(mut approval) = read(&entry.path()) else {
            continue;
        };
        if approval.call != call || approval.state != PENDING {
            continue;
        }
        let on_disk = urzua_io::sha256_hex_of_file(&repo_root.join(&approval.path));
        let why = if on_disk.as_deref() != Some(approval.body_sha256.as_str()) {
            Some(format!(
                "{} is not the file the person approved",
                approval.path
            ))
        } else if !transcript_shows(transcript, call) {
            Some("the host's transcript does not show this edit call".to_string())
        } else {
            None
        };
        if let Some(why) = why {
            let _ = std::fs::remove_file(entry.path());
            return Some(Err(format!("{why}, so the approval was dropped")));
        }
        approval.state = CONFIRMED.to_string();
        return Some(write(&entry.path(), &approval).map(|_| approval));
    }
    None
}

/// What the local state says about the uncommitted record `record_id` at
/// `status` whose file is `rel`, against the blob `head` it changes.
pub(crate) fn judge_local(
    repo_root: &Path,
    rel: &str,
    record_id: &str,
    status: &str,
    head: &str,
    now: i64,
) -> Option<SessionApproval> {
    let path = approvals_dir(repo_root).map(|d| file_for(&d, rel))?;
    if !path.exists() {
        return None;
    }
    let Some(a) = read(&path) else {
        return Some(SessionApproval::Invalid(
            "the approval kept for it could not be read".into(),
        ));
    };
    let on_disk = urzua_io::sha256_hex_of_file(&repo_root.join(rel));
    Some(if a.path != rel {
        SessionApproval::Invalid(format!("the approval kept for it names {}", a.path))
    } else if a.record_id != record_id {
        SessionApproval::Invalid(format!(
            "the approval names the record {}, not {record_id}",
            a.record_id
        ))
    } else if !urzua_core::authority::same_status(&a.status, status) {
        SessionApproval::Invalid(format!(
            "the approval is for a move to '{}', and the record is at '{status}'",
            a.status
        ))
    } else if a.state == CONSUMED {
        SessionApproval::Invalid("the approval was already used by a commit".into())
    } else if a.base_blob != head {
        SessionApproval::Invalid(
            "the approval was given for an edit of another version of the record (another branch or commit)".into(),
        )
    } else if on_disk.as_deref() != Some(a.body_sha256.as_str()) {
        SessionApproval::Invalid(
            "the record changed after the person approved it (its bytes no longer match the approved edit)".into(),
        )
    } else if now < a.created_unix || now > a.expires_unix {
        SessionApproval::Invalid(
            "the approval is outside its time window (expired, or dated after now)".into(),
        )
    } else if a.state != CONFIRMED {
        SessionApproval::Pending(
            "the approved edit is on disk but the host has not reported that edit call as landed, so whether a person approved it is not known".into(),
        )
    } else {
        SessionApproval::Valid(format!(
            "an approval recorded in an agent session names {} (the name as read from {}, not verified); it is a local record that CI does not rely on",
            a.identity, a.identity_source
        ))
    })
}

/// Marks used every confirmed approval whose bytes `HEAD` now holds.
fn consume_committed(repo_root: &Path) {
    let Some(dir) = approvals_dir(repo_root) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(mut a) = read(&entry.path()) else {
            continue;
        };
        if a.state != CONFIRMED {
            continue;
        }
        let committed = git(
            repo_root,
            &["cat-file", "blob", &format!("HEAD:{}", a.path)],
        )
        .map(|b| urzua_io::sha256_hex_of_bytes(&b));
        if committed.as_deref() == Some(a.body_sha256.as_str()) {
            a.state = CONSUMED.to_string();
            let _ = write(&entry.path(), &a);
        }
    }
}

fn uncommitted(repo_root: &Path, rel: &str) -> bool {
    git(repo_root, &["status", "--porcelain", "-z", "--", rel]).is_none_or(|o| !o.is_empty())
}

/// The approvals the local commit check reads, only with
/// `acceptance.in_session: ask`, for each changed record whose working-tree or
/// index copy differs from `HEAD`. The caller asks only on a run with no
/// actors file: CI's evidence is the actors file, never this state.
pub(crate) fn for_check(
    repo_root: &Path,
    config: &Config,
    records: &[urzua_core::record::Record],
    changed: Option<&[PathBuf]>,
) -> Option<std::collections::HashMap<PathBuf, SessionApproval>> {
    let harness = config.harness.as_ref()?;
    if !harness.enabled("acceptance") || harness.in_session() != urzua_core::harness::InSession::Ask
    {
        return None;
    }
    consume_committed(repo_root);
    let changed = changed?;
    let now = now();
    let mut out = std::collections::HashMap::new();
    for record in records.iter().filter(|r| changed.contains(&r.path)) {
        let rel = record.path.to_string_lossy().to_string();
        if !uncommitted(repo_root, &rel) {
            continue;
        }
        let field = urzua_core::rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::Status,
        );
        let Some(status) = urzua_core::authority::gate_status(&record.header, field) else {
            continue;
        };
        let id = urzua_core::rules::record_id(record).unwrap_or_default();
        let head = head_blob(repo_root, &rel);
        if let Some(found) = judge_local(repo_root, &rel, &id, status, &head, now) {
            out.insert(record.path.clone(), found);
        }
    }
    Some(out)
}

/// Every approval file and its text.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            Some((name, std::fs::read_to_string(e.path()).ok()?))
        })
        .collect()
}

/// The edit tool calls the transcript shows ran without an error: their ids
/// and the file each named.
fn edits_that_ran(transcript: Option<&Path>) -> Vec<(String, String)> {
    let Some(text) = transcript.and_then(|t| std::fs::read_to_string(t).ok()) else {
        return Vec::new();
    };
    let mut uses = Vec::new();
    let mut ok = std::collections::HashSet::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        walk(&v, &mut |o| match o.get("type").and_then(|t| t.as_str()) {
            Some("tool_use")
                if matches!(
                    o.get("name").and_then(|n| n.as_str()),
                    Some("Write" | "Edit" | "MultiEdit")
                ) =>
            {
                if let (Some(id), Some(file)) = (
                    o.get("id").and_then(|i| i.as_str()),
                    o.get("input")
                        .and_then(|i| i.get("file_path"))
                        .and_then(|f| f.as_str()),
                ) {
                    uses.push((id.to_string(), file.to_string()));
                }
            }
            Some("tool_result") if o.get("is_error").and_then(|e| e.as_bool()) != Some(true) => {
                if let Some(id) = o.get("tool_use_id").and_then(|i| i.as_str()) {
                    ok.insert(id.to_string());
                }
            }
            _ => {}
        });
    }
    uses.retain(|(id, _)| ok.contains(id));
    uses
}

/// Whether a change from `old` to `new` is one the hooks make: a pending or
/// confirmed approval for an edit call the transcript shows ran on that file,
/// or a confirmed approval marked used by a commit.
fn hook_made(old: Option<&str>, new: &str, edits: &[(String, String)]) -> bool {
    let Ok(a) = serde_json::from_str::<Approval>(new) else {
        return false;
    };
    if let Some(o) = old.and_then(|o| serde_json::from_str::<Approval>(o).ok()) {
        let mut used = o.clone();
        used.state = CONSUMED.to_string();
        if o.state == CONFIRMED && used == a {
            return true;
        }
    }
    a.state != CONSUMED
        && edits
            .iter()
            .any(|(id, file)| *id == a.call && file.ends_with(&format!("/{}", a.path)))
}

/// After a shell command: one line for each approval created, changed or
/// removed since the last post-command check that the hooks did not make (see
/// [`hook_made`]). Only this check records what it saw, so an approval made by
/// piping a payload to the hook commands is reported like one written by hand.
/// It rests on the host's transcript, which the user's shell can also write:
/// it makes a forgery visible and costlier, it does not prevent one.
pub(crate) fn shell_changes(repo_root: &Path, transcript: Option<&Path>) -> Vec<String> {
    let Some(dir) = approvals_dir(repo_root) else {
        return Vec::new();
    };
    let Some(state) = dir.parent() else {
        return Vec::new();
    };
    let seen: BTreeMap<String, String> = std::fs::read_to_string(state.join(SEEN))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let now = snapshot(&dir);
    if now.is_empty() && seen.is_empty() {
        return Vec::new();
    }
    let edits = edits_that_ran(transcript);
    let mut lines = Vec::new();
    for (name, text) in &now {
        let old = seen.get(name).map(String::as_str);
        if old == Some(text.as_str()) || hook_made(old, text, &edits) {
            continue;
        }
        let act = if old.is_none() { "created" } else { "changed" };
        lines.push(format!(
            "  {} [acceptance] error: the command {act} an approval of a gated move that no edit call of this session made; approvals are written only by the agent harness's hooks during an edit call, so a shell command wrote this one. Do not commit the record it names on its strength; CI's actors file decides.",
            dir.join(name).display()
        ));
    }
    // The hooks drop only a pending approval that did not hold.
    let removed = seen.iter().filter(|(n, t)| {
        !now.contains_key(*n)
            && serde_json::from_str::<Approval>(t).map_or(true, |a| a.state != PENDING)
    });
    for (name, _) in removed {
        lines.push(format!(
            "  {} [acceptance] error: the command removed an approval of a gated move.",
            dir.join(name).display()
        ));
    }
    if let Ok(text) = serde_json::to_string(&now) {
        let _ = urzua_io::write_atomic(&state.join(SEEN), &text);
    }
    lines
}

/// The record's title: its first `# ` heading.
pub(crate) fn title_of(content: &str) -> String {
    content
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or_default()
        .to_string()
}

/// `text` as it may appear in a prompt: control characters, bidirectional
/// controls, quotes and line breaks dropped, runs of space collapsed, at most
/// `max` characters.
pub(crate) fn sanitize(text: &str, max: usize) -> String {
    let bidi = |c: char| matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}');
    let quote = |c: char| {
        "\"'`\u{2018}\u{2019}\u{201A}\u{201B}\u{201C}\u{201D}\u{201E}\u{201F}\u{00AB}\u{00BB}\u{2039}\u{203A}".contains(c)
    };
    let kept: String = text
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| {
            !c.is_control()
                && !bidi(*c)
                && !quote(*c)
                && !matches!(
                    c,
                    '\u{2028}' | '\u{2029}' | '\u{FEFF}' | '\u{200B}'..='\u{200D}'
                )
        })
        .collect();
    let collapsed = kept
        .split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let cut: String = collapsed.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Who the session says is at the keyboard: the `gh` login, else
/// `git config user.name`. Either is whatever the user's environment reports.
pub(crate) struct Approver {
    pub name: String,
    pub source: &'static str,
}

pub(crate) fn approver(repo_root: &Path, config: &Config) -> Result<Approver, String> {
    let policy = urzua_io::IdentityPolicy {
        asserted: config.attestation.identity == urzua_core::config::AttestationIdentity::Asserted,
        require_verified: config.attestation.require_verified,
    };
    let r = urzua_io::resolve_identity_with(None, repo_root, policy)?;
    Ok(match (r.source, r.verified_login) {
        (urzua_io::IdentitySource::GhVerified, _) => Approver {
            name: r.name,
            source: "gh",
        },
        (_, Some(login)) => Approver {
            name: login,
            source: "gh",
        },
        _ => Approver {
            name: r.name,
            source: "git config",
        },
    })
}

/// `content` with the receipt of an approved move: each field the entered
/// status requires that is absent or blank and that `date.field-valid`
/// declares a date gets `today`, and the revision log gets the row
/// `narrative`. The engine names no field: both come from the configuration.
pub(crate) fn with_receipt(
    config: &Config,
    record_type: &str,
    content: &str,
    narrative: &str,
    to: &str,
    today: &str,
) -> Result<String, String> {
    let mut text = content.to_string();
    let date_fields: Vec<String> = config
        .rules
        .get(urzua_core::rules::RULE_DATE_FIELD_VALID)
        .and_then(|s| s.date_fields.clone())
        .unwrap_or_default();
    if let Some(decl) = config
        .record_types
        .get(record_type)
        .and_then(|t| t.status_decl())
    {
        let header = header_values(&text);
        let value = |f: &str| header.iter().find(|(k, _)| k == f).map(|(_, v)| v.clone());
        let blank = |v: &str| matches!(v.trim(), "" | "null" | "~");
        let missing: Vec<String> = decl
            .required_for(to, &value)
            .into_iter()
            .filter(|t| t.on_status && date_fields.iter().any(|d| d == t.field))
            .filter(|t| value(t.field).is_none_or(|v| blank(&v)))
            .map(|t| t.field.to_string())
            .collect();
        for field in missing {
            text = set_header_field(&text, &field, today)?;
        }
    }
    let log_id = config.revision_log_id(record_type);
    let shape = config.revision_log_shape(record_type);
    urzua_core::fix::append_revision_row(&text, &log_id, &shape, today, narrative)
}

/// The `key: value` lines of the front matter, as written.
fn header_values(text: &str) -> Vec<(String, String)> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Vec::new();
    }
    lines
        .take_while(|l| l.trim_end() != "---")
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// `text` with `field: value` in its front matter: a blank existing line is
/// filled, else the field is added before the closing `---`.
fn set_header_field(text: &str, field: &str, value: &str) -> Result<String, String> {
    let mut out: Vec<String> = text.split('\n').map(str::to_string).collect();
    if out.first().map(|l| l.trim_end()) != Some("---") {
        return Err(format!(
            "the record has no front matter to write '{field}:' in"
        ));
    }
    for i in 1..out.len() {
        let line = out[i].trim_end().to_string();
        if line == "---" {
            out.insert(i, format!("{field}: {value}"));
            return Ok(out.join("\n"));
        }
        if let Some(v) = line.strip_prefix(field).and_then(|r| r.strip_prefix(':')) {
            if matches!(v.trim(), "" | "null" | "~") {
                out[i] = format!("{field}: {value}");
                return Ok(out.join("\n"));
            }
        }
    }
    Err(format!(
        "the record's front matter is not closed, so '{field}:' cannot be written"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_field_is_filled_when_blank_and_added_when_absent() {
        let t = "---\nStatus: Accepted\nAccepted-on: null\n---\n# T\n";
        assert_eq!(
            set_header_field(t, "Accepted-on", "2026-11-17").unwrap(),
            "---\nStatus: Accepted\nAccepted-on: 2026-11-17\n---\n# T\n"
        );
        let t = "---\nStatus: Accepted\n---\n# T\n";
        assert_eq!(
            set_header_field(t, "Accepted-on", "2026-11-17").unwrap(),
            "---\nStatus: Accepted\nAccepted-on: 2026-11-17\n---\n# T\n"
        );
        assert!(set_header_field("# T\n", "X", "1").is_err());
    }

    #[test]
    fn a_title_loses_controls_bidi_quotes_and_breaks_and_is_capped() {
        let raw = "ADR-1: Fix \"typo\"\r\u{1b}[2J\u{202E}moves\nStatus 'x'".to_string()
            + &"a".repeat(3000);
        let t = sanitize(&raw, 80);
        assert!(t.chars().count() <= 80, "{t}");
        assert!(
            !t.chars()
                .any(|c| c.is_control() || "\"'\u{202E}".contains(c)),
            "{t:?}"
        );
        assert!(t.starts_with("ADR-1: Fix typo [2Jmoves Status x"), "{t}");
    }
}
