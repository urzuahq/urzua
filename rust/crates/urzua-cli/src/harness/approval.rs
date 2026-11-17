//! A person's approval of an agent's gated move, given through the agent host
//! (`ADR-102`, amendment of 2026-11-17).
//!
//! With `acceptance.in_session: ask`, the editor hook answers an agent's move
//! into a gated status with the host's ask-the-person decision instead of a
//! refusal. Before it answers, it rewrites the proposed file to carry the
//! receipt (the status's required date fields and a revision-log row naming the
//! person), so the person approves the edit and its receipt as one write and no
//! commit can fall between the two. It also writes a pending approval under the
//! git directory, keyed by the record and naming the host's id for the call.
//! When the host reports that very call landed (its post-edit event, which runs
//! only after the person approved and the tool ran), the approval is confirmed.
//! The local commit check accepts a gated move only with a confirmed approval
//! for that record, that status and those exact bytes, before it expires.
//!
//! Why an agent cannot forge one: the approvals live under `.git/urzua`, which
//! the protected-paths guardrail keeps from the agent's edit tools, and the git
//! directory is in no commit, so CI never reads them. A shell command can write
//! there as it can write any file the user can; such a command's status move is
//! what the post-command check reports to the person, and CI still judges the
//! pull request by its actors file. The approval proves, locally, that a person
//! answered the host's prompt for this move in this session; it proves who only
//! as far as its identity source does.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use urzua_core::config::Config;
use urzua_core::rules::SessionApproval;

pub(crate) const PENDING: &str = "pending";
pub(crate) const CONFIRMED: &str = "confirmed";

/// One approval as it is kept on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Approval {
    /// The record's path from the repository root.
    pub path: String,
    pub record_id: String,
    /// The status the approved edit enters.
    pub status: String,
    /// The sha256 of the whole file as the approved edit writes it, receipt included.
    pub body_sha256: String,
    pub session: String,
    /// The host's id for the edit call.
    pub call: String,
    pub identity: String,
    pub identity_source: String,
    pub created_unix: i64,
    pub expires_unix: i64,
    pub state: String,
}

/// `<git common dir>/urzua`, the harness's own state beside the repository,
/// which no commit carries.
pub(crate) fn git_state_dir(repo_root: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--git-common-dir"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8(out.stdout).ok()?;
    let dir = PathBuf::from(dir.trim_end_matches('\n'));
    let dir = if dir.is_absolute() {
        dir
    } else {
        repo_root.join(dir)
    };
    Some(dir.join("urzua"))
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

/// Keeps `approval` as the one approval for its record, replacing any earlier one.
pub(crate) fn write_pending(repo_root: &Path, approval: &Approval) -> Result<(), String> {
    let dir = approvals_dir(repo_root).ok_or("the repository's git directory was not found")?;
    write(&file_for(&dir, &approval.path), approval)
}

/// Confirms the pending approval for the host's call `call` once the file on
/// disk is the approved one. `None` when no pending approval names the call;
/// `Err` when one does and the file is not what was approved (it is dropped).
pub(crate) fn confirm(repo_root: &Path, call: &str) -> Option<Result<Approval, String>> {
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
        if on_disk.as_deref() != Some(approval.body_sha256.as_str()) {
            let _ = std::fs::remove_file(entry.path());
            return Some(Err(format!(
                "{} is not the file the person approved, so the approval was dropped",
                approval.path
            )));
        }
        approval.state = CONFIRMED.to_string();
        return Some(write(&entry.path(), &approval).map(|_| approval));
    }
    None
}

/// What the local state says about the record `record_id` at `status` whose
/// file is `rel`, or `None` when no approval names that file.
pub(crate) fn judge_local(
    repo_root: &Path,
    rel: &str,
    record_id: &str,
    status: &str,
    now: i64,
) -> Option<SessionApproval> {
    let path = file_for(&approvals_dir(repo_root)?, rel);
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
    } else if on_disk.as_deref() != Some(a.body_sha256.as_str()) {
        SessionApproval::Invalid(
            "the record changed after the person approved it (its bytes no longer match the approved edit)".into(),
        )
    } else if now > a.expires_unix {
        SessionApproval::Invalid(format!(
            "the approval expired {} minute(s) ago",
            (now - a.expires_unix) / 60 + 1
        ))
    } else if a.state != CONFIRMED {
        SessionApproval::Pending(
            "the approved edit is on disk but the host has not reported that edit call as landed, so whether a person approved it is not known".into(),
        )
    } else {
        SessionApproval::Valid(format!(
            "approved in an agent session by {} (identity: {}), session {}",
            a.identity, a.identity_source, a.session
        ))
    })
}

/// The approvals the local commit check reads for the changed records: only
/// with `acceptance.in_session: ask`. The caller asks only on a run with no
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
    let changed = changed?;
    let now = urzua_io::now_unix();
    let mut out = std::collections::HashMap::new();
    for record in records.iter().filter(|r| changed.contains(&r.path)) {
        let field = urzua_core::rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::Status,
        );
        let Some(status) = urzua_core::authority::gate_status(&record.header, field) else {
            continue;
        };
        let Some(id) = urzua_core::rules::record_id(record) else {
            continue;
        };
        let rel = record.path.to_string_lossy();
        if let Some(found) = judge_local(repo_root, &rel, &id, status, now) {
            out.insert(record.path.clone(), found);
        }
    }
    Some(out)
}

/// The record's title: its first `# ` heading.
pub(crate) fn title_of(content: &str) -> String {
    content
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .unwrap_or_default()
}

/// Who approves, as far as the host can tell: the identity attestations use.
pub(crate) struct Approver {
    pub name: String,
    pub source: &'static str,
    /// The `gh` login, when `gh` answered.
    pub login: Option<String>,
}

pub(crate) fn approver(repo_root: &Path, config: &Config) -> Result<Approver, String> {
    let policy = urzua_io::IdentityPolicy {
        asserted: config.attestation.identity == urzua_core::config::AttestationIdentity::Asserted,
        require_verified: config.attestation.require_verified,
    };
    let r = urzua_io::resolve_identity_with(None, repo_root, policy)?;
    let (source, login) = match r.source {
        urzua_io::IdentitySource::GhVerified => ("gh-verified", Some(r.name.clone())),
        urzua_io::IdentitySource::Asserted => ("asserted", r.verified_login.clone()),
        urzua_io::IdentitySource::GitConfig => ("git-config", r.verified_login.clone()),
    };
    Ok(Approver {
        name: r.name,
        source,
        login,
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
    fn the_title_is_the_first_heading() {
        assert_eq!(
            title_of("---\nA: b\n---\n# 10 — A title\n\n## x\n"),
            "10 — A title"
        );
    }
}
