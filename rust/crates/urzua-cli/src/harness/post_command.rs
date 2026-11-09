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
//! Git does not see a gitignored file, so a protected path that is ignored
//! (a local settings file under a protected directory) would never show in `git status`. The
//! harness state therefore also keeps a content hash of every file under a
//! protected path (`.urzua/cache/protected-hashes.json`, seeded at session
//! start) and reports any file whose hash differs, that is new, or that is gone,
//! whether or not git shows it. A change is reported by the first check that
//! sees it; the baseline then moves, so the same change is not reported again.
//!
//! Cost: `git status` and one `git rev-parse` are the gate. A clean tree at the
//! remembered commit whose protected files hash as they did ends before the
//! configuration is parsed (the state holds the protected entries and the hash
//! of the configuration file they came from), as does a tree whose changes are
//! not governed records, and a record whose content hash is the one already
//! reported is not checked again. The memory is `.urzua/cache/post-command.json`
//! and the protected baseline is `protected-hashes.json` (derived, ignored,
//! never authoritative).
//!
//! Every state in which it cannot judge is stated as `allow-unjudged` with
//! "NOT judged": a missing `git`, a working tree git does not know, a
//! configuration that does not load, a `check` that cannot run. A missing check
//! is never a quiet pass.

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use urzua_core::config::Config;
use urzua_core::report::{CheckReport, Finding, FindingSeverity, Notice, Report, ReportStatus};
use urzua_io::git::GitRun;

use super::run::{key, live, to_json};
use crate::commands::check::{check_paths, judge_proposed};
use crate::emit;
use crate::session::Workspace;

const GUARDRAIL: &str = "post-command";
const P: &str = "urzua harness (post-command)";
const CACHE: &str = ".urzua/cache/post-command.json";
const HEAD_FILE: &str = ".urzua/cache/post-command-head";
const PROTECTED_FILE: &str = ".urzua/cache/protected-hashes.json";

#[derive(Deserialize, Default)]
struct Request {
    cwd: Option<String>,
    #[serde(default)]
    project_dir: Option<String>,
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
    // A command that ran inside a nested repository (`git init sub && cd sub`)
    // is still the governed repository's session.
    if config.is_none() {
        super::run::enter_governed_cwd(request.project_dir.as_deref());
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
    let mut tree = match changed_paths() {
        Ok(c) => c,
        Err(why) => return PostVerdict::unjudged(&why),
    };
    // A file git was told to stop looking at is not in `git status` at all.
    tree.extend(top.map(hidden_changes).unwrap_or_default());
    tree.sort();
    tree.dedup();
    if tree.is_empty() && committed.is_empty() && protected_unchanged(top, config.as_deref()) {
        forget_all(top);
        return PostVerdict::quiet(format!("{P}: git shows no changed file; nothing to check"));
    }

    let Workspace {
        repo_root,
        config_path,
        config: loaded,
    } = match open_governed(config) {
        Ok(ws) => ws,
        Err(verdict) => return *verdict,
    };

    // Protected files git does not show (ignored ones), by content hash.
    let scan = scan_protected(&repo_root, &config_path, &loaded, true);
    let paths = changed_path_map(&repo_root, tree, committed, scan.changes);
    let Selected {
        fresh,
        protected,
        remembered,
    } = select_unseen(&repo_root, &loaded, &paths);
    if fresh.is_empty() && protected.is_empty() {
        write_memory(&repo_root, &remembered);
        let mut v = PostVerdict::quiet(format!(
            "{P}: no governed record changed since the last check; no check ran"
        ));
        if let Some(note) = &scan.note {
            v.verdict = "allow-with-warnings";
            v.message.push_str(&format!(" {note}"));
        }
        return v;
    }

    // A gated move is judged on the same records, against the commit the
    // command started from: the command has already run, so this reports and
    // never blocks. It does not depend on `check` being able to run.
    let gated: Vec<String> = fresh
        .iter()
        .filter_map(|rel| super::gate::gated_move_report(&repo_root, &loaded, Path::new(rel), base))
        .collect();

    let report = match run_check(config, &fresh, gated.is_empty() && protected.is_empty()) {
        Ok(r) => r,
        Err(verdict) => return *verdict,
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
        if let Some(note) = &scan.note {
            v.verdict = "allow-with-warnings";
            v.message.push_str(&format!(" {note}"));
        }
        return v;
    }
    v.verdict = "allow-with-warnings";
    v.message = findings_message(&FindingsSummary {
        checked: fresh.len(),
        new: &new,
        old: &old,
        gated: &gated,
        protected: &protected,
        note: scan.note.as_deref(),
    });
    v
}

/// Opens the workspace the command ran in. An `Err` is the verdict to
/// return: the configuration does not load, the guardrail is off, or the
/// repository root cannot be entered.
fn open_governed(config: &Option<PathBuf>) -> Result<Workspace, Box<PostVerdict>> {
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => {
            return Err(Box::new(PostVerdict::unjudged(&format!(
                "the configuration does not load: {}",
                e.message
            ))))
        }
    };
    if !ws
        .config
        .harness
        .as_ref()
        .is_some_and(|h| h.enabled(GUARDRAIL))
    {
        return Err(Box::new(PostVerdict::quiet(format!(
            "{P}: the {GUARDRAIL} guardrail is not enabled under harness.guardrails; nothing was checked"
        ))));
    }
    if std::env::set_current_dir(&ws.repo_root).is_err() {
        return Err(Box::new(PostVerdict::unjudged(
            "the repository root cannot be entered",
        )));
    }
    Ok(ws)
}

/// Runs `check` on the records to judge. `Ok(None)` when none are, or when
/// `check` could not run but a gated move or protected change is still
/// reported; an `Err` is the verdict to return.
fn run_check(
    config: &Option<PathBuf>,
    fresh: &[String],
    nothing_else_to_report: bool,
) -> Result<Option<CheckReport>, Box<PostVerdict>> {
    if fresh.is_empty() {
        return Ok(None);
    }
    match check_paths(config.clone(), fresh.iter().map(PathBuf::from).collect()) {
        Ok(r) if r.status != ReportStatus::NotRun => Ok(Some(r)),
        Ok(_) if nothing_else_to_report => Err(Box::new(PostVerdict::unjudged(
            "check could not run: no enabled rule examined the records",
        ))),
        Err(e) if nothing_else_to_report => Err(Box::new(PostVerdict::unjudged(&format!(
            "check could not run: {}",
            e.message
        )))),
        _ => Ok(None),
    }
}

/// Every path the command may have changed: the working tree's (a deleted
/// file flagged), the files committed since the last check, and the protected
/// files git does not show.
fn changed_path_map(
    repo_root: &Path,
    tree: Vec<(String, bool)>,
    committed: Vec<String>,
    protected_changes: Vec<(String, bool)>,
) -> BTreeMap<String, bool> {
    let mut paths: BTreeMap<String, bool> = BTreeMap::new();
    for (rel, deleted) in tree {
        paths.insert(rel, deleted);
    }
    for rel in committed {
        let gone = !repo_root.join(&rel).exists();
        paths.entry(rel).or_insert(gone);
    }
    for (rel, deleted) in protected_changes {
        paths.entry(rel).or_insert(deleted);
    }
    paths
}

/// The changed paths the memory has not seen at their current content.
struct Selected {
    /// Governed records to check.
    fresh: Vec<String>,
    /// One report line per protected path changed.
    protected: Vec<String>,
    /// The memory to write once judged: every candidate with its hash.
    remembered: BTreeMap<String, String>,
}

fn select_unseen(repo_root: &Path, loaded: &Config, paths: &BTreeMap<String, bool>) -> Selected {
    // Governed, still on disk, with the content hash the memory is keyed by;
    // and protected paths, a deleted one included.
    let hash_of = |rel: &str| match std::fs::read(repo_root.join(rel)) {
        Ok(bytes) => urzua_io::sha256_hex_of_str(&String::from_utf8_lossy(&bytes)),
        // Unreadable: always checked, so `check` says why it cannot judge it.
        Err(_) => String::new(),
    };
    let mut candidates: BTreeMap<String, String> = BTreeMap::new();
    let mut protected_candidates: BTreeMap<String, (String, bool)> = BTreeMap::new();
    for (rel, deleted) in paths {
        let path = Path::new(rel);
        if super::gate::protected_change_report(loaded, path, *deleted).is_some() {
            let hash = if *deleted {
                "deleted".to_string()
            } else {
                hash_of(rel)
            };
            protected_candidates.insert(rel.clone(), (hash, *deleted));
        }
        if !*deleted
            && crate::discovery::is_governed(loaded, path)
            && repo_root.join(path).is_file()
        {
            candidates.insert(rel.clone(), hash_of(rel));
        }
    }
    let seen = read_memory(repo_root);
    let fresh: Vec<String> = candidates
        .iter()
        .filter(|(p, h)| h.is_empty() || seen.get(*p) != Some(*h))
        .map(|(p, _)| p.clone())
        .collect();
    let protected: Vec<String> = protected_candidates
        .iter()
        .filter(|(p, (h, _))| h.is_empty() || seen.get(*p) != Some(h))
        .filter_map(|(p, (_, deleted))| {
            super::gate::protected_change_report(loaded, Path::new(p), *deleted)
        })
        .collect();
    let mut remembered = candidates;
    for (p, (h, _)) in &protected_candidates {
        remembered.insert(p.clone(), h.clone());
    }
    Selected {
        fresh,
        protected,
        remembered,
    }
}

/// What the final message of a run that found something reports.
struct FindingsSummary<'a> {
    checked: usize,
    new: &'a [&'a Finding],
    old: &'a [&'a Finding],
    gated: &'a [String],
    protected: &'a [String],
    note: Option<&'a str>,
}

fn findings_message(summary: &FindingsSummary) -> String {
    let (new, old, gated, protected) = (summary.new, summary.old, summary.gated, summary.protected);
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
        summary.checked,
        summary.new.len()
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
    if let Some(note) = summary.note {
        m.push_str(&format!("\n{note}"));
    }
    if !new.is_empty() {
        m.push_str(&format!("\n{}", describe(new)));
    }
    if !old.is_empty() {
        m.push_str(&format!(
            "\nAlready failing at HEAD before this command (listed, not new):\n{}",
            describe(old)
        ));
    }
    m
}

/// The repository root and the commit `HEAD` names, in one `git` call. Either
/// is `None` when git cannot say (no repository, no commit yet); the working
/// tree is still examined then.
fn top_and_head() -> (Option<PathBuf>, Option<String>) {
    let Ok(out) = GitRun::new()
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
    let out = GitRun::new()
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

/// The content hash of every file under a protected path, as of the last
/// check. `config_sig` is the hash of the configuration file the entries were
/// read from, so an unchanged configuration lets the next check skip parsing it.
#[derive(Serialize, Deserialize, Default)]
struct ProtectedState {
    config_sig: String,
    /// Whether the `protected-paths` guardrail was on.
    enabled: bool,
    entries: Vec<String>,
    files: BTreeMap<String, String>,
    /// The walk hit its file cap, so some protected files are not hashed.
    #[serde(default)]
    truncated: bool,
}

fn read_protected(top: &Path) -> Option<ProtectedState> {
    serde_json::from_str(&std::fs::read_to_string(top.join(PROTECTED_FILE)).ok()?).ok()
}

fn write_protected(top: &Path, state: &ProtectedState) {
    let Ok(text) = serde_json::to_string(state) else {
        return;
    };
    let path = top.join(PROTECTED_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|t| t == text) {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = urzua_io::write_atomic(&path, &text);
}

fn hash_files(root: &Path, files: &[String]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|rel| {
            // Unreadable (a dangling link, no permission) is its own state.
            let hash = urzua_io::sha256_hex_of_file(&root.join(rel))
                .unwrap_or_else(|| "unreadable".to_string());
            (rel.clone(), hash)
        })
        .collect()
}

fn config_signature(top: &Path, config: Option<&Path>) -> Option<String> {
    let path = config.map_or_else(
        || crate::discovery::default_config_path(top),
        Path::to_path_buf,
    );
    urzua_io::sha256_hex_of_file(&path)
}

/// The cheap check that ends a clean tree: the state exists, the configuration
/// is the one it was read from, and every protected file hashes as it did and
/// none is new or gone. Anything else (no state yet included) is a full check.
fn protected_unchanged(top: Option<&Path>, config: Option<&Path>) -> bool {
    let Some(top) = top else { return false };
    let Some(state) = read_protected(top) else {
        return false;
    };
    if config_signature(top, config).as_deref() != Some(state.config_sig.as_str()) {
        return false;
    }
    if !state.enabled {
        return true;
    }
    if state.truncated {
        return false;
    }
    let (found, truncated) = super::gate::walk_entries(top, &state.entries);
    !truncated && hash_files(top, &found) == state.files
}

struct ProtectedScan {
    /// Files that differ from the baseline, with whether each is gone.
    changes: Vec<(String, bool)>,
    /// A stated limit of this scan, for the report.
    note: Option<String>,
}

/// Hashes every file under a protected path and, when `report` is set, returns
/// those that differ from the stored baseline; either way the baseline becomes
/// what is on disk now. With no baseline yet nothing can be said to differ.
/// When the configuration itself changed, only files under the entries the old
/// baseline was read with are compared: new entries adopt their files silently
/// (the configuration is a protected path itself, and its change is reported).
fn scan_protected(root: &Path, config_path: &Path, config: &Config, report: bool) -> ProtectedScan {
    let mut scan = ProtectedScan {
        changes: Vec::new(),
        note: None,
    };
    let Some(sig) = urzua_io::sha256_hex_of_file(config_path) else {
        return scan;
    };
    let on = config
        .harness
        .as_ref()
        .is_some_and(|h| h.enabled("protected-paths"));
    let entries = if on {
        super::gate::protected_entries(config)
    } else {
        Vec::new()
    };
    let (found, truncated) = super::gate::walk_entries(root, &entries);
    let files = hash_files(root, &found);
    let old = read_protected(root);
    if report && on {
        if let Some(old) = old.filter(|o| o.enabled) {
            let same_config = old.config_sig == sig;
            let covered = |rel: &str| {
                same_config
                    || old
                        .entries
                        .iter()
                        .any(|e| urzua_core::rules::path_matches_entry(Path::new(rel), e))
            };
            for (rel, hash) in &files {
                if covered(rel) && old.files.get(rel) != Some(hash) {
                    scan.changes.push((rel.clone(), false));
                }
            }
            for rel in old.files.keys() {
                if !files.contains_key(rel) && !root.join(rel).exists() {
                    scan.changes.push((rel.clone(), true));
                }
            }
        }
    }
    if truncated {
        scan.note = Some(
            "The protected-path hash scan stopped at its file cap, so some files under a protected path were NOT hashed.".to_string(),
        );
    }
    write_protected(
        root,
        &ProtectedState {
            config_sig: sig,
            enabled: on,
            entries,
            files,
            truncated,
        },
    );
    scan
}

/// Records the protected files' hashes as the baseline the next post-command
/// check compares with. The session-start check calls it.
pub(super) fn seed_protected(root: &Path, config_path: &Path, config: &Config) {
    scan_protected(root, config_path, config, false);
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
    let out = GitRun::new()
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "-z",
            "--no-renames",
            "--untracked-files=all",
        ])
        .output()
        .map_err(|e| format!("git could not be run: {}", e.into_io()))?;
    if !out.status.success() {
        return Err(format!(
            "git status failed, so this is not a working tree git knows: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(parse_porcelain_all(&out.stdout))
}

/// The files whose change `git status` cannot show because the index says to
/// ignore it (`git update-index --assume-unchanged` or `--skip-worktree`), found
/// by comparing the working file with the blob the index holds, with whether
/// each is gone. A file that is not on disk and is only skip-worktree is not a
/// change (a sparse checkout leaves such files out). Empty when git cannot say.
fn hidden_changes(top: &Path) -> Vec<(String, bool)> {
    let git = |args: &[&str]| {
        GitRun::new()
            .cwd(top)
            .arg("--no-optional-locks")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
    };
    let Some(listed) = git(&["ls-files", "-v", "-z"]) else {
        return Vec::new();
    };
    // `git ls-files -v`: a lowercase tag is assume-unchanged, `S` is skip-worktree.
    let flagged: Vec<(String, bool)> = listed
        .stdout
        .split(|b| *b == 0)
        .filter(|e| e.len() > 2 && e[1] == b' ' && (e[0] == b'S' || e[0].is_ascii_lowercase()))
        .map(|e| {
            (
                String::from_utf8_lossy(&e[2..]).to_string(),
                e[0] != b'S' && e[0] != b's',
            )
        })
        .collect();
    if flagged.is_empty() {
        return Vec::new();
    }
    let Some(staged) = git(&["ls-files", "-s", "-z"]) else {
        return Vec::new();
    };
    // `<mode> <oid> <stage>\t<path>`
    let index: BTreeMap<String, String> = staged
        .stdout
        .split(|b| *b == 0)
        .filter_map(|e| {
            let text = String::from_utf8_lossy(e).to_string();
            let (meta, path) = text.split_once('\t')?;
            Some((path.to_string(), meta.split(' ').nth(1)?.to_string()))
        })
        .collect();
    let mut changes = Vec::new();
    let mut present: Vec<&String> = Vec::new();
    for (path, absent_is_change) in &flagged {
        if path.contains('\n') {
            continue;
        }
        if std::fs::symlink_metadata(top.join(path)).is_ok() {
            present.push(path);
        } else if *absent_is_change {
            changes.push((path.clone(), true));
        }
    }
    if present.is_empty() {
        return changes;
    }
    let paths = present
        .iter()
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    // If git cannot be run, or fails, no hash is known and each file counts as
    // changed, the safe direction.
    let hashed = GitRun::new()
        .cwd(top)
        .args(["--no-optional-locks", "hash-object", "--stdin-paths"])
        .stdin(paths.as_bytes())
        .discard_stderr()
        .output()
        .ok()
        .filter(|o| o.status.success());
    let oids: Vec<String> = hashed
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    for (i, path) in present.iter().enumerate() {
        if oids.len() != present.len() || index.get(*path) != oids.get(i) {
            changes.push(((*path).clone(), false));
        }
    }
    changes
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
    let out = GitRun::new()
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
