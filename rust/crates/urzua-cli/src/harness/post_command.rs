//! `urzua harness run post-command` (`ADR-78` amendment, `ADR-89`, `MILE-202`):
//! after any Bash command, run `check` on the records git shows as changed and
//! report what it found to the agent.
//!
//! It never reads the command. The changed set is `git status --porcelain -z`
//! (modified, added and untracked files alike), the governed records among them
//! go to one in-process `check PATHS` call (a named untracked file is examined,
//! `BUG-24`), and nothing is staged. It reports and never blocks: the command
//! has already run, the exit code is always 0, and pre-commit and CI stay the
//! hard gate (`ADR-71`).
//!
//! It also reports a changed protected path (an `authority` path gate, with the
//! `protected-paths` guardrail on) the same way, and it covers commits the
//! command made: the commit the last check saw is remembered in
//! `.urzua/cache/post-command-head` (seeded at session start), and the files
//! that changed between it and `HEAD` are examined with the working tree's, so
//! `sed ... && git commit -am x` in one command is not invisible. A record is
//! then compared with that commit, not with `HEAD`.
//!
//! Cost: `git status` and one `git rev-parse` are the gate. A clean tree at the
//! remembered commit, or a tree whose changes are not governed records, ends
//! before the configuration is parsed, and a record whose content hash is the
//! one already reported is not checked again. The memory is
//! `.urzua/cache/post-command.json` (derived, ignored, never authoritative).
//!
//! Every state in which it cannot judge is stated as `allow-unjudged` with
//! "NOT judged": a missing `git`, a working tree git does not know, a
//! configuration that does not load, a `check` that cannot run. A missing check
//! is never a quiet pass.

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::{Deserialize, Serialize};
use urzua_core::report::{Finding, FindingSeverity, Notice, Report, ReportStatus};

use super::run::{key, live, to_json};
use crate::commands::check::{check_paths, judge_proposed};
use crate::emit;
use crate::session::Workspace;

const GUARDRAIL: &str = "post-command";
const P: &str = "urzua harness (post-command)";
const CACHE: &str = ".urzua/cache/post-command.json";
const HEAD_FILE: &str = ".urzua/cache/post-command-head";

#[derive(Deserialize, Default)]
struct Request {
    cwd: Option<String>,
}

#[derive(Serialize)]
struct PostVerdict {
    verdict: &'static str,
    message: String,
    /// Unwaived findings the changed records have now and did not have at `HEAD`.
    findings: Vec<serde_json::Value>,
    /// Unwaived findings the same record already had at `HEAD`.
    pre_existing: Vec<serde_json::Value>,
    /// Records the command moved to a status the `authority` table gates
    /// (reported as errors of their own; never blocked).
    gated_moves: Vec<String>,
    /// Protected paths (an `authority` path gate) the command changed, one
    /// report line each; reported, never blocked.
    protected_changes: Vec<String>,
    /// The records `check` ran on this time. Empty when no check ran.
    checked: Vec<String>,
}

impl PostVerdict {
    fn quiet(message: impl Into<String>) -> Self {
        PostVerdict {
            verdict: "allow",
            message: message.into(),
            findings: Vec::new(),
            pre_existing: Vec::new(),
            gated_moves: Vec::new(),
            protected_changes: Vec::new(),
            checked: Vec::new(),
        }
    }

    fn unjudged(why: &str) -> Self {
        PostVerdict {
            verdict: "allow-unjudged",
            message: format!(
                "{P}: the command's effect on records was NOT judged ({why}). Nothing was blocked and no success is claimed; run 'urzua check' before committing."
            ),
            findings: Vec::new(),
            pre_existing: Vec::new(),
            gated_moves: Vec::new(),
            protected_changes: Vec::new(),
            checked: Vec::new(),
        }
    }
}

impl Report for PostVerdict {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

pub fn run_post_command(config: Option<PathBuf>) -> ExitCode {
    emit(&judge(config))
}

fn judge(config: Option<PathBuf>) -> PostVerdict {
    let mut input = String::new();
    let request: Request = match std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())
        .and_then(|_| serde_json::from_str(&input).map_err(|e| e.to_string()))
    {
        Ok(r) => r,
        Err(e) => return PostVerdict::unjudged(&format!("the request was unreadable: {e}")),
    };
    if let Some(cwd) = &request.cwd {
        if std::env::set_current_dir(cwd).is_err() {
            return PostVerdict::unjudged(&format!("the request's cwd '{cwd}' cannot be entered"));
        }
    }

    // The commit the last check saw, and the files committed since: a command
    // that committed its own change leaves a clean tree, so the working tree
    // alone cannot show it.
    let (top, head) = top_and_head();
    let last = top.as_deref().and_then(read_head);
    let committed = match (&last, &head) {
        (Some(l), Some(h)) if l != h => committed_since(l, h),
        _ => None,
    };
    let base = match (&committed, &last) {
        (Some(_), Some(l)) => l.clone(),
        _ => "HEAD".to_string(),
    };
    let v = judge_changes(
        &config,
        top.as_deref(),
        &base,
        committed.unwrap_or_default(),
    );
    // The range is consumed once it has been judged; an unjudged run keeps it.
    if v.verdict != "allow-unjudged" {
        if let (Some(t), Some(h)) = (&top, &head) {
            write_head(t, h);
        }
    }
    v
}

fn judge_changes(
    config: &Option<PathBuf>,
    top: Option<&Path>,
    base: &str,
    committed: Vec<String>,
) -> PostVerdict {
    // The gate: git's own view of the working tree, before any configuration.
    let tree = match changed_paths() {
        Ok(c) => c,
        Err(why) => return PostVerdict::unjudged(&why),
    };
    if tree.is_empty() && committed.is_empty() {
        forget_all(top);
        return PostVerdict::quiet(format!("{P}: git shows no changed file; nothing to check"));
    }

    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => {
            return PostVerdict::unjudged(&format!(
                "the configuration does not load: {}",
                e.message
            ))
        }
    };
    let Workspace {
        repo_root,
        config: loaded,
        ..
    } = ws;
    if !loaded
        .harness
        .as_ref()
        .is_some_and(|h| h.enabled(GUARDRAIL))
    {
        return PostVerdict::quiet(format!(
            "{P}: the {GUARDRAIL} guardrail is not enabled under harness.guardrails; nothing was checked"
        ));
    }
    if std::env::set_current_dir(&repo_root).is_err() {
        return PostVerdict::unjudged("the repository root cannot be entered");
    }

    // Every path the command may have changed: the working tree's (a deleted
    // file flagged) and the files committed since the last check.
    let mut paths: BTreeMap<String, bool> = BTreeMap::new();
    for (rel, deleted) in tree {
        paths.insert(rel, deleted);
    }
    for rel in committed {
        let gone = !repo_root.join(&rel).exists();
        paths.entry(rel).or_insert(gone);
    }

    // Governed, still on disk, with the content hash the memory is keyed by;
    // and protected paths, a deleted one included.
    let hash_of = |rel: &str| match std::fs::read(repo_root.join(rel)) {
        Ok(bytes) => urzua_io::sha256_hex_of_str(&String::from_utf8_lossy(&bytes)),
        // Unreadable: always checked, so `check` says why it cannot judge it.
        Err(_) => String::new(),
    };
    let mut candidates: BTreeMap<String, String> = BTreeMap::new();
    let mut protected_candidates: BTreeMap<String, (String, bool)> = BTreeMap::new();
    for (rel, deleted) in &paths {
        let path = Path::new(rel);
        if super::gate::protected_change_report(&loaded, path, *deleted).is_some() {
            let hash = if *deleted {
                "deleted".to_string()
            } else {
                hash_of(rel)
            };
            protected_candidates.insert(rel.clone(), (hash, *deleted));
        }
        if !*deleted
            && crate::discovery::is_governed(&loaded, path)
            && repo_root.join(path).is_file()
        {
            candidates.insert(rel.clone(), hash_of(rel));
        }
    }
    let seen = read_memory(&repo_root);
    let fresh: Vec<String> = candidates
        .iter()
        .filter(|(p, h)| h.is_empty() || seen.get(*p) != Some(*h))
        .map(|(p, _)| p.clone())
        .collect();
    let protected: Vec<String> = protected_candidates
        .iter()
        .filter(|(p, (h, _))| h.is_empty() || seen.get(*p) != Some(h))
        .filter_map(|(p, (_, deleted))| {
            super::gate::protected_change_report(&loaded, Path::new(p), *deleted)
        })
        .collect();
    let mut remembered = candidates.clone();
    for (p, (h, _)) in &protected_candidates {
        remembered.insert(p.clone(), h.clone());
    }
    if fresh.is_empty() && protected.is_empty() {
        write_memory(&repo_root, &remembered);
        return PostVerdict::quiet(format!(
            "{P}: no governed record changed since the last check; no check ran"
        ));
    }

    // A gated move is judged on the same records, against the commit the
    // command started from: the command has already run, so this reports and
    // never blocks. It does not depend on `check` being able to run.
    let gated: Vec<String> = fresh
        .iter()
        .filter_map(|rel| super::gate::gated_move_report(&repo_root, &loaded, Path::new(rel), base))
        .collect();

    let report = if fresh.is_empty() {
        None
    } else {
        match check_paths(config.clone(), fresh.iter().map(PathBuf::from).collect()) {
            Ok(r) if r.status != ReportStatus::NotRun => Some(r),
            Ok(_) if gated.is_empty() && protected.is_empty() => {
                return PostVerdict::unjudged(
                    "check could not run: no enabled rule examined the records",
                )
            }
            Err(e) if gated.is_empty() && protected.is_empty() => {
                return PostVerdict::unjudged(&format!("check could not run: {}", e.message))
            }
            _ => None,
        }
    };
    let all = report.as_ref().map(live).unwrap_or_default();
    let (old, new) = partition_by_head(config, &all, base);
    write_memory(&repo_root, &remembered);

    let mut v = PostVerdict::quiet(String::new());
    v.checked = fresh.clone();
    v.findings = to_json(&new);
    v.pre_existing = to_json(&old);
    v.gated_moves = gated.clone();
    v.protected_changes = protected.clone();
    if new.is_empty() && old.is_empty() && gated.is_empty() && protected.is_empty() {
        v.message = format!(
            "{P}: checked {} changed record(s); no findings",
            fresh.len()
        );
        return v;
    }
    v.verdict = "allow-with-warnings";
    let errors = new
        .iter()
        .filter(|f| f.severity == FindingSeverity::Error)
        .count();
    let mut notes = String::new();
    if !gated.is_empty() {
        notes.push_str(&format!(", and {} gated move(s)", gated.len()));
    }
    if !protected.is_empty() {
        notes.push_str(&format!(
            ", and {} protected path change(s)",
            protected.len()
        ));
    }
    let mut m = format!(
        "{P}: urzua checked {} record(s) the command changed (git status and any commit it made, untracked included) and found {} finding(s), {errors} of them errors{notes}. The command had already run, so this was not prevented and nothing was blocked; pre-commit and CI remain the hard gate.",
        fresh.len(),
        new.len()
    );
    if !gated.is_empty() {
        m.push_str(&format!(
            "\nGated move(s) to a status the authority table reserves for a decider:\n{}",
            gated.join("\n")
        ));
    }
    if !protected.is_empty() {
        m.push_str(&format!(
            "\nProtected path(s) the authority table reserves for a decider:\n{}",
            protected.join("\n")
        ));
    }
    if !new.is_empty() {
        m.push_str(&format!("\n{}", describe(&new)));
    }
    if !old.is_empty() {
        m.push_str(&format!(
            "\nAlready failing at HEAD before this command (listed, not new):\n{}",
            describe(&old)
        ));
    }
    v.message = m;
    v
}

/// The repository root and the commit `HEAD` names, in one `git` call. Either
/// is `None` when git cannot say (no repository, no commit yet); the working
/// tree is still examined then.
fn top_and_head() -> (Option<PathBuf>, Option<String>) {
    let Ok(out) = Command::new("git")
        .args([
            "--no-optional-locks",
            "rev-parse",
            "--show-toplevel",
            "HEAD",
        ])
        .output()
    else {
        return (None, None);
    };
    if !out.status.success() {
        return (None, None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let top = lines.next().map(PathBuf::from);
    let head = lines.next().map(|h| h.trim().to_string());
    (top, head.filter(|h| !h.is_empty()))
}

/// The files that differ between `last` and `head` (a deleted one included),
/// or `None` when git cannot compare them (a commit that is gone after a
/// rewrite): the working tree is judged against `HEAD` then.
fn committed_since(last: &str, head: &str) -> Option<Vec<String>> {
    let out = Command::new("git")
        .args([
            "--no-optional-locks",
            "diff",
            "--name-only",
            "-z",
            "--no-renames",
            last,
            head,
        ])
        .output()
        .ok()?;
    out.status.success().then(|| {
        out.stdout
            .split(|b| *b == 0)
            .filter(|n| !n.is_empty())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .collect()
    })
}

fn read_head(top: &Path) -> Option<String> {
    let text = std::fs::read_to_string(top.join(HEAD_FILE)).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn write_head(top: &Path, head: &str) {
    if read_head(top).as_deref() == Some(head) {
        return;
    }
    let path = top.join(HEAD_FILE);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = urzua_io::write_atomic(&path, head);
}

/// Remembers `HEAD` as the commit the next post-command check starts from. The
/// session-start check calls it so the first command of a session is covered.
pub(super) fn seed_head(repo_root: &Path) {
    let prev = std::env::current_dir().ok();
    if std::env::set_current_dir(repo_root).is_ok() {
        if let (Some(top), Some(head)) = top_and_head() {
            write_head(&top, &head);
        }
    }
    if let Some(p) = prev {
        let _ = std::env::set_current_dir(p);
    }
}

/// The files git shows as changed: modified, added, renamed (as a delete and
/// an add) and untracked, each file named (`-uall`), with whether it was
/// deleted, staged or not. An error is a stated reason the changed set could
/// not be read.
fn changed_paths() -> Result<Vec<(String, bool)>, String> {
    let out = Command::new("git")
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "-z",
            "--no-renames",
            "--untracked-files=all",
        ])
        .output()
        .map_err(|e| format!("git could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git status failed, so this is not a working tree git knows: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(parse_porcelain_all(&out.stdout))
}

/// Parses `git status --porcelain=v1 -z --no-renames`: `XY path` entries ended
/// by NUL, each with whether the file is deleted.
fn parse_porcelain_all(raw: &[u8]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for entry in raw.split(|b| *b == 0) {
        if entry.len() < 4 || entry[2] != b' ' {
            continue;
        }
        let deleted = entry[0] == b'D' || entry[1] == b'D';
        out.push((String::from_utf8_lossy(&entry[3..]).to_string(), deleted));
    }
    out.sort();
    out.dedup();
    out
}

/// The same, with deleted entries dropped: the files a record check can read.
#[cfg(test)]
fn parse_porcelain(raw: &[u8]) -> Vec<String> {
    parse_porcelain_all(raw)
        .into_iter()
        .filter(|(_, deleted)| !deleted)
        .map(|(p, _)| p)
        .collect()
}

/// Splits the findings into those the record already had at `base` (`HEAD`, or the commit the last check saw; judged
/// from its committed text, as the editor hook judges the copy on disk) and
/// those it did not. A record with no copy at `HEAD`, or a baseline that cannot
/// be judged, leaves its findings new.
fn partition_by_head<'a>(
    config: &Option<PathBuf>,
    all: &[&'a Finding],
    base: &str,
) -> (Vec<&'a Finding>, Vec<&'a Finding>) {
    let mut baselines: BTreeMap<PathBuf, HashSet<String>> = BTreeMap::new();
    let files: HashSet<PathBuf> = all.iter().map(|f| f.file.clone()).collect();
    for file in files {
        let Some(text) = head_text(&file, base) else {
            continue;
        };
        if let Ok(r) = judge_proposed(config.clone(), file.clone(), text) {
            if matches!(r.status, ReportStatus::Ok | ReportStatus::FindingsPresent) {
                baselines.insert(file, live(&r).into_iter().map(key).collect());
            }
        }
    }
    all.iter()
        .copied()
        .partition(|f| baselines.get(&f.file).is_some_and(|b| b.contains(&key(f))))
}

fn head_text(rel: &Path, base: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["--no-optional-locks", "show"])
        .arg(format!("{base}:{}", rel.to_string_lossy()))
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// One finding per line with its file, line, severity, rule and message.
fn describe(findings: &[&Finding]) -> String {
    findings
        .iter()
        .map(|f| {
            let severity = serde_json::to_value(f.severity)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            format!(
                "  {}:{}: {severity} [{}] {}",
                f.file.display(),
                f.line.map_or("-".to_string(), |l| l.to_string()),
                f.rule,
                f.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A clean tree remembers nothing: the memory file nearest above the current
/// directory is removed, so a record edited the same way again is reported
/// again.
fn forget_all(top: Option<&Path>) {
    if let Some(top) = top {
        let _ = std::fs::remove_file(top.join(CACHE));
        return;
    }
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    for dir in cwd.ancestors() {
        let file = dir.join(CACHE);
        if file.is_file() {
            let _ = std::fs::remove_file(file);
            return;
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Memory {
    records: BTreeMap<String, String>,
}

/// The hashes last reported, or none when the memory is absent or unreadable
/// (the cost is a repeated report, never a missed one).
fn read_memory(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(CACHE))
        .ok()
        .and_then(|t| serde_json::from_str::<Memory>(&t).ok())
        .map(|m| m.records)
        .unwrap_or_default()
}

/// Remembers exactly the records that are changed now, so one that is reverted
/// is forgotten and reported again if it is edited the same way later. A write
/// that fails costs only a repeated report.
fn write_memory(root: &Path, candidates: &BTreeMap<String, String>) {
    let records: BTreeMap<String, String> = candidates
        .iter()
        .filter(|(_, h)| !h.is_empty())
        .map(|(p, h)| (p.clone(), h.clone()))
        .collect();
    if records == read_memory(root) {
        return;
    }
    let path = root.join(CACHE);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(&Memory { records }) {
        let _ = urzua_io::write_atomic(&path, &text);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_porcelain;

    #[test]
    fn porcelain_z_names_modified_added_and_untracked_files_and_drops_deleted_ones() {
        let raw = b" M docs/adr/a.md\0A  docs/adr/b.md\0?? docs/adr/c.md\0 D docs/adr/d.md\0D  docs/adr/e.md\0AD docs/adr/f.md\0MM with space.md\0";
        assert_eq!(
            parse_porcelain(raw),
            [
                "docs/adr/a.md",
                "docs/adr/b.md",
                "docs/adr/c.md",
                "with space.md"
            ]
        );
    }

    #[test]
    fn an_empty_or_short_status_names_nothing() {
        assert!(parse_porcelain(b"").is_empty());
        assert!(parse_porcelain(b"?? \0x\0").is_empty());
    }
}
