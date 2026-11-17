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

use super::gate::GateEvent;
use super::run::{key, live, to_json};
use crate::commands::check::{check_paths, judge_proposed};
use crate::emit;
use crate::session::Workspace;

const GUARDRAIL: &str = "post-command";

/// The settings every git call of this module pins. Derived caches (the
/// commit-graph and its changed-path filters) and display settings live in
/// the repository, where the judged command can write them, and each can
/// change which commits or paths these read-only queries report.
const PINNED: &[&str] = &[
    "-c",
    "core.commitGraph=false",
    "-c",
    "commitGraph.readChangedPaths=false",
    "-c",
    "log.showRoot=true",
];

/// Every git call of this module: the pinned settings, and replace refs off,
/// since a local `refs/replace` entry would change the trees and history read.
fn git_run() -> GitRun {
    GitRun::new()
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .args(PINNED)
}

/// The shallow file's bytes, empty when there is none. `None` when git
/// cannot name it.
fn shallow_content(repo_root: &Path) -> Option<Vec<u8>> {
    let out = git_run()
        .cwd(repo_root)
        .args(["--no-optional-locks", "rev-parse", "--git-path", "shallow"])
        .discard_stderr()
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(std::fs::read(repo_root.join(path)).unwrap_or_default())
}

/// Why nothing may be compared because of the shallow file: a commit in
/// `start..HEAD` is a shallow boundary (a hand-edited shallow file acts as a
/// graft), or the file changed since the baseline. A boundary in the base's
/// history, as in an ordinary shallow clone, does not count. Git reads only a
/// line's leading object id, so that is what is compared.
fn shallow_problem(
    repo_root: &Path,
    start: &str,
    prior: Option<&ProtectedState>,
) -> Option<&'static str> {
    const TRUNCATED: &str =
        "the command truncated the history (a shallow boundary in its own commits)";
    let Some(shallow) = shallow_content(repo_root) else {
        return Some(TRUNCATED);
    };
    let bounds: HashSet<String> = shallow
        .split(|b| *b == b'\n')
        .map(|l| {
            l.iter()
                .take_while(|b| b.is_ascii_hexdigit())
                .map(|b| b.to_ascii_lowercase() as char)
                .collect::<String>()
        })
        .filter(|id| id.len() >= 40)
        .collect();
    if start != "HEAD" && !bounds.is_empty() {
        let Some(out) = git_run()
            .cwd(repo_root)
            .args(["--no-optional-locks", "rev-list", &format!("{start}..HEAD")])
            .discard_stderr()
            .output()
            .ok()
            .filter(|o| o.status.success())
        else {
            return Some(TRUNCATED);
        };
        if String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|c| bounds.contains(c.trim()))
        {
            return Some(TRUNCATED);
        }
    }
    let sig = urzua_io::sha256_hex_of_bytes(&shallow);
    prior
        .and_then(|p| p.post_shallow.as_deref())
        .is_some_and(|old| old != sig)
        .then_some("the shallow file changed")
}

/// Whether replace refs or a grafts file exist. Git still honours grafts with
/// replace refs off, so either means nothing is compared with the base. A
/// shallow clone's `shallow` file is ordinary and not counted.
fn history_rewritten(repo_root: &Path) -> bool {
    let out = |args: &[&str]| {
        git_run()
            .cwd(repo_root)
            .args(args)
            .discard_stderr()
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let Some(replace) = out(&["--no-optional-locks", "for-each-ref", "refs/replace"]) else {
        return true;
    };
    let Some(grafts) = out(&[
        "--no-optional-locks",
        "rev-parse",
        "--git-path",
        "info/grafts",
    ]) else {
        return true;
    };
    !replace.is_empty() || repo_root.join(grafts).exists()
}
const P: &str = "urzua harness (post-command)";
const CACHE: &str = ".urzua/cache/post-command.json";
const HEAD_FILE: &str = ".urzua/cache/post-command-head";
const PROTECTED_FILE: &str = ".urzua/cache/protected-hashes.json";
/// How far back the reflog is read for the moves since the last check.
const REFLOG_DEPTH: usize = 200;

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
    /// What the agent is told: a short report of what it should act on, or empty.
    agent: String,
    /// What the person is told: only a gated move, a protected-path change, or
    /// a check that could not run; empty otherwise.
    person: String,
    /// Records whose ordinary findings were absorbed into the baseline unreported,
    /// because a pull, checkout, merge or rebase brought them in. A gated move or
    /// a protected change in them is still reported.
    adopted: Vec<String>,
    /// The gated paths whose content is the one on the configured base ref,
    /// reported at the configured lower level. `None` when there are none.
    base_match: Option<BaseMatch>,
}

#[derive(Serialize)]
struct BaseMatch {
    base: String,
    level: &'static str,
    paths: Vec<String>,
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
            agent: String::new(),
            person: String::new(),
            adopted: Vec::new(),
            base_match: None,
        }
    }

    fn unjudged(why: &str) -> Self {
        let message = format!(
            "{P}: the command's effect on records was NOT judged ({why}). Nothing was blocked and no success is claimed; run 'urzua check' before committing."
        );
        let mut v = PostVerdict::quiet(message.clone());
        v.verdict = "allow-unjudged";
        v.person = message;
        v.agent = format!("{P}: NOT judged; run 'urzua check' before committing.");
        v
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
    super::limits::activate(&super::limits::of(config.clone(), None));

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
    let committed = committed.unwrap_or_default();
    // The reflog is written by whoever runs git, the agent included, so it
    // chooses only whether ordinary findings are absorbed; it never keeps a
    // gate event from the report.
    let absorbed = match (&top, &last, &head) {
        (Some(t), Some(l), Some(h)) if l != h && !committed.is_empty() => {
            let elsewhere = not_own(&committed, own_files(t, l, h).as_ref());
            let refs: Vec<&String> = elsewhere.iter().collect();
            same_as(t, h, &refs, &HashSet::new())
        }
        _ => Vec::new(),
    };
    let v = judge_changes(&config, top.as_deref(), &base, committed, &absorbed);
    // The range is consumed once it has been judged; an unjudged run keeps it.
    if v.verdict != "allow-unjudged" {
        if let (Some(t), Some(h)) = (&top, &head) {
            write_head(t, h);
        }
    }
    v
}

/// The files the session's own commits between `last` and `head` changed. A
/// commit is the session's own when the `HEAD` reflog has a commit-making entry
/// for it (`commit`, its `amend`, `initial` and `merge` forms, `cherry-pick`,
/// `revert`, a rebase pick), on whatever branch it was made, so merging or
/// checking out a branch that holds it does not make it foreign. `None` when
/// git cannot say: every commit then counts as the session's own.
fn own_files(top: &Path, last: &str, head: &str) -> Option<HashSet<String>> {
    let git = |args: &[&str]| {
        git_run()
            .cwd(top)
            .arg("--no-optional-locks")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
    };
    let depth = format!("-n{REFLOG_DEPTH}");
    let reflog = git(&["reflog", "--format=%H%x00%gs", &depth, "HEAD"])?;
    let made = |subject: &str| {
        ["commit:", "commit (", "cherry-pick:", "revert:"]
            .iter()
            .any(|p| subject.starts_with(p))
            || (subject.starts_with("rebase") && subject.contains("(pick)"))
    };
    let own: HashSet<&str> = reflog
        .lines()
        .filter_map(|l| l.split_once('\0'))
        .filter(|(_, subject)| made(subject))
        .map(|(h, _)| h)
        .collect();
    let range = git(&["rev-list", &format!("{last}..{head}")])?;
    let mut files = HashSet::new();
    for commit in range.lines().filter(|c| own.contains(c)) {
        // `--cc` names, for a merge, only what the merge itself resolved.
        let changed = git(&[
            "diff-tree",
            "-r",
            "--root",
            "--cc",
            "--name-only",
            "--no-commit-id",
            commit,
        ])?;
        files.extend(
            changed
                .lines()
                .filter(|l| !l.is_empty())
                .map(str::to_string),
        );
    }
    Some(files)
}

/// The files of `committed` that did not come from the session's own commits.
fn not_own(committed: &[String], own: Option<&HashSet<String>>) -> Vec<String> {
    match own {
        Some(own) => committed
            .iter()
            .filter(|p| !own.contains(*p))
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

/// The `(mode, blob)` `rev` holds for each of `paths` that it has, from
/// `git ls-tree` with the paths as literal pathspecs. `None` when git fails.
fn base_entries(
    top: &Path,
    rev: &str,
    paths: &[&String],
) -> Option<BTreeMap<String, (String, String)>> {
    let mut args: Vec<String> = [
        "--no-optional-locks",
        "--literal-pathspecs",
        "ls-tree",
        "-r",
        "-z",
        "--full-tree",
        "--end-of-options",
        rev,
        "--",
    ]
    .iter()
    .map(|a| a.to_string())
    .collect();
    args.extend(paths.iter().map(|p| p.to_string()));
    let out = git_run()
        .cwd(top)
        .args(&args)
        .discard_stderr()
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    // `<mode> <type> <oid>\t<path>`
    let entries: BTreeMap<String, (String, String)> = out
        .stdout
        .split(|b| *b == 0)
        .filter_map(|e| {
            let text = String::from_utf8_lossy(e).to_string();
            let (meta, path) = text.split_once('\t')?;
            let mut meta = meta.split(' ');
            let mode = meta.next()?.to_string();
            let oid = meta.nth(1)?.to_string();
            Some((path.to_string(), (mode, oid)))
        })
        .collect();
    Some(entries)
}

/// The paths whose file on disk is the entry `rev` holds, the same mode and the
/// same blob of its raw bytes. Absent from both counts only for a path in
/// `deleted`; a path git named in bytes that are not UTF-8 never matches,
/// since its decoded name is not the file. Filters are not run: a
/// clean filter the agent configures could make any content hash as `rev`'s. A symlink is compared as its link text and
/// is never followed; a submodule or a directory never matches.
fn same_as(top: &Path, rev: &str, paths: &[&String], deleted: &HashSet<String>) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }
    let Some(entries) = base_entries(top, rev, paths) else {
        return Vec::new();
    };
    let mut same = Vec::new();
    let mut regular: Vec<(&String, &'static str)> = Vec::new();
    for p in paths {
        if p.contains('\u{FFFD}') {
            continue;
        }
        let Ok(meta) = top.join(p.as_str()).symlink_metadata() else {
            if deleted.contains(p.as_str()) && !entries.contains_key(p.as_str()) {
                same.push((*p).clone());
            }
            continue;
        };
        let Some((mode, oid)) = entries.get(p.as_str()) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            if mode == "120000" && link_blob(top, p).as_deref() == Some(oid.as_str()) {
                same.push((*p).clone());
            }
        } else if meta.is_file() {
            regular.push((p, file_mode(&meta)));
        }
    }
    if regular.is_empty() {
        return same;
    }
    let list = regular
        .iter()
        .map(|(p, _)| p.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let oids: Vec<String> = git_run()
        .cwd(top)
        .args([
            "--no-optional-locks",
            "hash-object",
            "--no-filters",
            "--stdin-paths",
        ])
        .stdin(list.as_bytes())
        .discard_stderr()
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if oids.len() == regular.len() {
        for ((p, mode), oid) in regular.iter().zip(&oids) {
            if entries.get(p.as_str()) == Some(&(mode.to_string(), oid.clone())) {
                same.push((*p).clone());
            }
        }
    }
    same
}

/// The tree mode git would record for a regular file.
fn file_mode(meta: &std::fs::Metadata) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 != 0 {
            return "100755";
        }
    }
    let _ = meta;
    "100644"
}

/// The blob id of a symlink's link text, as git stores a symlink.
fn link_blob(top: &Path, rel: &str) -> Option<String> {
    let target = std::fs::read_link(top.join(rel)).ok()?;
    let text = target.to_string_lossy().to_string();
    git_run()
        .cwd(top)
        .args(["--no-optional-locks", "hash-object", "--stdin"])
        .stdin(text.as_bytes())
        .discard_stderr()
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn judge_changes(
    config: &Option<PathBuf>,
    top: Option<&Path>,
    base: &str,
    committed: Vec<String>,
    absorbed: &[String],
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
        Err(verdict) => return name_gated(*verdict, top, &tree, &committed),
    };

    // The comparison rule as it was before the command, from the baseline.
    let prior = read_protected(&repo_root);

    // Protected files git does not show (ignored ones), by content hash.
    let scan = scan_protected(&repo_root, &config_path, &loaded, true);
    let paths = changed_path_map(&repo_root, tree, committed, scan.changes.clone());
    let Selected {
        fresh,
        mut protected,
        remembered,
    } = select_unseen(&repo_root, &loaded, &paths, &scan.baseline_entries);
    let prior_rule = prior.as_ref();
    protected.extend(shrink_lines(
        &repo_root,
        &config_path,
        &scan,
        (&loaded, prior_rule),
    ));
    let deletions = gated_deletions(&repo_root, &loaded, &paths, base, &scan);
    let mut remembered = remembered;
    let adopted: Vec<String> = fresh
        .iter()
        .filter(|p| absorbed.contains(*p))
        .cloned()
        .collect();
    remember_deletions(&repo_root, &paths, &deletions, &mut remembered);
    let deletions: Vec<GateEvent> = deletions.into_iter().map(|(_, e)| e).collect();
    if fresh.is_empty() && protected.is_empty() && deletions.is_empty() {
        write_memory(&repo_root, &remembered);
        return nothing_to_check(adopted, scan.note.as_deref());
    }

    // A gated move is judged on the same records, against the commit the
    // command started from: the command has already run, so this reports and
    // never blocks. It does not depend on `check` being able to run.
    let mut gated = gated_moves(&repo_root, &loaded, &fresh, base, &scan);
    gated.extend(deletions);
    let gates = base_gates(
        &repo_root,
        &config_path,
        &loaded,
        &paths,
        prior.as_ref(),
        (gated, protected, base),
    );
    let to_check: Vec<String> = fresh
        .iter()
        .filter(|p| !absorbed.contains(*p))
        .cloned()
        .collect();

    let report = match run_check(config, &to_check, gates.is_empty()) {
        Ok(r) => r,
        Err(verdict) => return *verdict,
    };
    let all = report.as_ref().map(live).unwrap_or_default();
    let (old, new) = partition_by_head(config, &all, base);
    write_memory(&repo_root, &remembered);

    let mut v = report_verdict(&to_check, &gates, (new, old), scan.note.as_deref());
    v.adopted = adopted;
    v
}

/// The comparison rule (`base`, `base_match`) the configuration sets.
fn loaded_rule(loaded: &Config) -> (String, String) {
    let harness = loaded.harness.as_ref();
    (
        harness
            .map_or(urzua_core::harness::DEFAULT_BASE, |h| h.post_command_base())
            .to_string(),
        level_name(harness.map_or(FindingSeverity::Info, |h| h.post_command_base_match()))
            .to_string(),
    )
}

/// An event when the comparison rule differs from the one the baseline
/// recorded: the configuration changed it, or, with the configuration file
/// unchanged, the baseline under `.urzua/cache` was edited. Either way a
/// command cannot lower its own report, so this is an error of its own whether
/// or not the configuration file is a protected path.
fn rule_change(
    repo_root: &Path,
    config_path: &Path,
    loaded: &Config,
    prior: Option<&ProtectedState>,
) -> Option<GateEvent> {
    let prior = prior?;
    let now = loaded_rule(loaded);
    let show = |(b, l): &(String, String)| format!("base '{b}', base_match '{l}'");
    let same_config =
        urzua_io::sha256_hex_of_file(config_path).is_some_and(|sig| sig == prior.config_sig);
    let recorded = prior.post_base.clone().zip(prior.post_level.clone());
    let act = match recorded {
        Some(r) if r == now => return None,
        Some(r) => rule_act(&show(&now), &show(&r), same_config),
        None if same_config => return None,
        None => format!(
            "changed the configuration while the baseline records no post-command comparison rule to compare with; the rule is now {}",
            show(&now)
        ),
    };
    Some(GateEvent {
        path: config_path
            .strip_prefix(repo_root)
            .unwrap_or(config_path)
            .display()
            .to_string(),
        guardrail: "post-command",
        act,
        gate: String::new(),
    })
}

fn rule_act(now: &str, recorded: &str, same_config: bool) -> String {
    if same_config {
        format!(
            "left the configuration's post-command comparison rule ({now}) different from the one the baseline under .urzua/cache recorded ({recorded}), so the baseline was edited"
        )
    } else {
        format!("changed the post-command comparison rule from {recorded} to {now}")
    }
}

/// The verdict when no governed record and no gated path changed.
fn nothing_to_check(adopted: Vec<String>, note: Option<&str>) -> PostVerdict {
    let mut v = PostVerdict::quiet(format!(
        "{P}: no governed record changed since the last check; no check ran"
    ));
    v.adopted = adopted;
    if let Some(note) = note {
        v.verdict = "allow-with-warnings";
        v.message.push_str(&format!(" {note}"));
        v.agent = format!("{P}: {note}");
    }
    v
}

/// The gated changes of one command, compared with the base ref the
/// configuration names, or the one recorded before a command that changed it.
fn base_gates(
    repo_root: &Path,
    config_path: &Path,
    loaded: &Config,
    paths: &BTreeMap<String, bool>,
    prior: Option<&ProtectedState>,
    (gated, protected, start): (Vec<GateEvent>, Vec<GateEvent>, &str),
) -> Gates {
    let harness = loaded.harness.as_ref();
    let config_rel = config_path
        .strip_prefix(repo_root)
        .unwrap_or(config_path)
        .to_string_lossy()
        .to_string();
    let config_changed = paths.contains_key(&config_rel)
        || prior.is_some_and(|p| {
            urzua_io::sha256_hex_of_file(config_path).is_some_and(|sig| sig != p.config_sig)
        });
    // A command that edits the configuration could set the comparison rule
    // that judges it (`base: HEAD`), so then the rule as it was before the
    // command is used, and the configuration file itself must match it.
    let (base_ref, level) = if config_changed {
        match prior.and_then(|p| {
            p.post_base
                .clone()
                .zip(p.post_level.as_deref().and_then(level_of))
        }) {
            Some((b, l)) => (b, l),
            None => (String::new(), FindingSeverity::Error),
        }
    } else {
        (
            harness
                .map_or(urzua_core::harness::DEFAULT_BASE, |h| h.post_command_base())
                .to_string(),
            harness.map_or(FindingSeverity::Info, |h| h.post_command_base_match()),
        )
    };
    let mut event_paths: Vec<&String> = gated.iter().chain(&protected).map(|e| &e.path).collect();
    if config_changed {
        event_paths.push(&config_rel);
    }
    let rewritten = history_rewritten(repo_root);
    let truncated = shallow_problem(repo_root, start, prior);
    let matched = if base_ref.is_empty() || rewritten || truncated.is_some() {
        None
    } else {
        let deleted: HashSet<String> = paths
            .iter()
            .filter(|(_, d)| **d)
            .map(|(p, _)| p.clone())
            .collect();
        matching_base(repo_root, (&base_ref, start), &event_paths, &deleted)
    };
    let compared = matched.is_some();
    let mut matched = matched.unwrap_or_default();
    let rule_changed = protected.iter().any(|e| e.guardrail == "post-command");
    if rule_changed || (config_changed && !matched.contains(&config_rel)) {
        matched.clear();
    }
    let (base, remote) = describe_base(repo_root, &base_ref);
    Gates {
        moves: gated,
        protected,
        matched,
        base,
        // A local ref is whatever the agent made it, so a match with it
        // lowers nothing.
        local: compared && !remote,
        rule_changed,
        level: if remote { level } else { FindingSeverity::Error },
        unresolved: (!compared).then(|| {
            if rewritten {
                "replace refs or grafts are present, so nothing was compared with the base".to_string()
            } else if let Some(why) = truncated {
                format!("{why}, so nothing was compared with the base")
            } else if base_ref.is_empty() {
                "no base ref to compare with: the command changed the configuration and the comparison rule it had before is not known".to_string()
            } else {
                format!("no base ref to compare with: '{base_ref}' does not resolve")
            }
        }),
    }
}

/// The gated changes of one command, and which of them hold exactly the
/// content of the `base` tree.
struct Gates {
    moves: Vec<GateEvent>,
    protected: Vec<GateEvent>,
    matched: HashSet<String>,
    base: String,
    /// The base is not a remote-tracking ref, so a match proves nothing.
    local: bool,
    /// The comparison rule changed, so nothing was compared.
    rule_changed: bool,
    /// Why no comparison was made, when none was.
    unresolved: Option<String>,
    level: FindingSeverity,
}

impl Gates {
    /// ", and their content is not the one on <base>" when that was compared.
    fn unlike(&self) -> String {
        if self.rule_changed {
            return " (the post-command comparison rule changed in this command, so nothing was compared with the base)".to_string();
        }
        match &self.unresolved {
            None => format!(", and their content is not the one on {}", self.base),
            Some(why) => format!(" ({why})"),
        }
    }

    /// The agent's own line when every gated change matches, never the
    /// person's text, so the translator does not drop it as a repeat.
    fn agent_match_line(&self) -> Option<String> {
        let n = self.matching().len();
        if n == 0 {
            return None;
        }
        Some(if self.local {
            format!(
                "{P}: {n} gated change(s) match {}, which proves nothing; a decider must make or approve them, and the person has been told.",
                self.base
            )
        } else if self.level == FindingSeverity::Error {
            format!(
                "{P}: {n} gated change(s) match {}; a decider must make or approve them, and the person has been told.",
                self.base
            )
        } else {
            format!(
                "{P}: {n} gated change(s) match {}; nothing to do.",
                self.base
            )
        })
    }

    fn is_empty(&self) -> bool {
        self.moves.is_empty() && self.protected.is_empty()
    }

    fn all(&self) -> impl Iterator<Item = &GateEvent> {
        self.moves.iter().chain(&self.protected)
    }

    fn matches(&self, e: &GateEvent) -> bool {
        self.matched.contains(&e.path)
    }

    /// The events reported at full weight: every one whose content differs from
    /// the base tree.
    fn differing(&self) -> Vec<&GateEvent> {
        self.all().filter(|e| !self.matches(e)).collect()
    }

    fn matching(&self) -> Vec<&GateEvent> {
        self.all().filter(|e| self.matches(e)).collect()
    }

    /// One line per event, at the level it is reported at, for the JSON lists.
    fn lines(&self, events: &[GateEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| {
                if self.matches(e) {
                    format!(
                        "{} Its content is the one on {}.",
                        e.line(level_name(self.level)),
                        self.base
                    )
                } else {
                    e.line("error")
                }
            })
            .collect()
    }

    /// The sentence for the paths that hold the base tree's content, `None`
    /// when there are none.
    fn matching_sentence(&self, alone: bool) -> Option<String> {
        let matching = self.matching();
        let n = matching.len();
        if n == 0 {
            return None;
        }
        let level = level_name(self.level);
        let names = named(matching.iter().map(|e| e.path.as_str()));
        let tail = if self.local {
            ", which the agent can move, so the match proves nothing; a decider must review the listed paths"
        } else if self.level == FindingSeverity::Error {
            "; a decider must review the listed paths"
        } else if alone {
            ", so nothing needs doing"
        } else {
            "; nothing needs doing for them"
        };
        Some(if alone {
            format!(
                "{P} {level}: the command changed {n} gated path(s) ({names}), and each now holds exactly the content on {}{tail}.",
                self.base
            )
        } else {
            format!(
                "{level}: {n} other gated path(s) ({names}) now hold exactly the content on {}{tail}.",
                self.base
            )
        })
    }

    /// What the person is told: the differing changes grouped under each gate,
    /// the gate named once, then the matching ones in one line.
    fn person(&self) -> String {
        let differing = self.differing();
        if differing.is_empty() {
            return self.matching_sentence(true).unwrap_or_default();
        }
        let mut out = format!(
            "{P}: the command changed {} path(s) the authority table reserves for a decider{}. It had already run, so nothing was prevented; CI's diff.decider-required checks who made it. A decider must make or approve these changes.",
            differing.len(),
            self.unlike()
        );
        out.push_str(&grouped(&differing));
        if let Some(rest) = self.matching_sentence(false) {
            out.push_str(&format!("\n{rest}"));
        }
        out
    }
}

/// Up to five paths by name, then how many more.
fn named<'a>(paths: impl Iterator<Item = &'a str>) -> String {
    const SHOWN: usize = 5;
    let all: Vec<&str> = paths.collect();
    let mut text = all
        .iter()
        .take(SHOWN)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if all.len() > SHOWN {
        text.push_str(&format!(" and {} more", all.len() - SHOWN));
    }
    text
}

/// Adds to a verdict that could not judge the gated paths git shows as
/// changed, by the previous baseline's protected entries and its
/// configuration file, so a broken configuration does not hide them.
fn name_gated(
    mut v: PostVerdict,
    top: Option<&Path>,
    tree: &[(String, bool)],
    committed: &[String],
) -> PostVerdict {
    if v.verdict != "allow-unjudged" {
        return v;
    }
    let Some(state) = top.and_then(read_protected) else {
        return v;
    };
    let changed: Vec<&str> = tree
        .iter()
        .map(|(p, _)| p.as_str())
        .chain(committed.iter().map(String::as_str))
        .collect();
    let mut gated: Vec<&str> = changed
        .into_iter()
        .filter(|p| {
            super::gate::baseline_protected_report(Path::new(p), false, &state.entries).is_some()
        })
        .collect();
    gated.sort();
    gated.dedup();
    if !gated.is_empty() {
        let extra = format!(
            " Gated paths the command changed, by the previous configuration: {}.",
            named(gated.into_iter())
        );
        v.message.push_str(&extra);
        v.person.push_str(&extra);
    }
    v
}

/// The events under one heading per guardrail and gate, each path a line.
fn grouped(events: &[&GateEvent]) -> String {
    let mut groups: Vec<((&str, &str), Vec<&GateEvent>)> = Vec::new();
    for e in events {
        let k = (e.guardrail, e.gate.as_str());
        match groups.iter_mut().find(|(g, _)| *g == k) {
            Some((_, list)) => list.push(e),
            None => groups.push((k, vec![e])),
        }
    }
    let mut out = String::new();
    for ((guardrail, gate), list) in groups {
        if gate.is_empty() {
            out.push_str(&format!("\n[{guardrail}] error:"));
        } else {
            out.push_str(&format!("\n[{guardrail}] error ({gate}):"));
        }
        for e in list {
            out.push_str(&format!("\n  {}: the command {}", e.path, e.act));
        }
    }
    out
}

fn level_of(name: &str) -> Option<FindingSeverity> {
    match name {
        "error" => Some(FindingSeverity::Error),
        "warning" => Some(FindingSeverity::Warning),
        "info" => Some(FindingSeverity::Info),
        _ => None,
    }
}

fn level_name(level: FindingSeverity) -> &'static str {
    match level {
        FindingSeverity::Error => "error",
        FindingSeverity::Warning => "warning",
        FindingSeverity::Info => "info",
    }
}

/// The paths of `paths` whose file on disk, hashed as git hashes it, is the
/// blob `base` holds for it, a path absent from both included. `None` when
/// `base` does not resolve to a tree.
///
/// This is advisory: `base` is a local ref, and the agent the report is about
/// can move it or commit to it. A match therefore only lowers how loudly a
/// change is reported, never whether it is; CI compares with the real remote.
fn matching_base(
    repo_root: &Path,
    (base, start): (&str, &str),
    paths: &[&String],
    deleted: &HashSet<String>,
) -> Option<HashSet<String>> {
    if paths.is_empty() {
        return Some(HashSet::new());
    }
    let tree = format!("{base}^{{tree}}");
    let resolves = git_run()
        .cwd(repo_root)
        .args(["--no-optional-locks", "rev-parse", "-q", "--verify", &tree])
        .discard_stderr()
        .output()
        .is_ok_and(|o| o.status.success());
    if !resolves {
        return None;
    }
    let mut same: HashSet<String> = same_as(repo_root, base, paths, deleted)
        .into_iter()
        .collect();
    // The working tree alone can hold the base's bytes while the index or a
    // commit holds other content, so all three must equal the base, and no
    // commit the command made that the base lacks may touch the path.
    match committed_state(repo_root, (base, start), paths) {
        Some((base_e, head_e, index_e, foreign)) => same.retain(|p| {
            base_e.get(p) == head_e.get(p)
                && base_e.get(p) == index_e.get(p)
                && !foreign.contains(p)
        }),
        None => same.clear(),
    }
    Some(same)
}

type Entries = BTreeMap<String, (String, String)>;

/// The base's, `HEAD`'s and the index's entry for each path, and the paths a
/// commit in `start..HEAD` that `base` does not contain touched. `None` when
/// git cannot say.
fn committed_state(
    repo_root: &Path,
    (base, start): (&str, &str),
    paths: &[&String],
) -> Option<(Entries, Entries, Entries, HashSet<String>)> {
    let base_e = base_entries(repo_root, base, paths)?;
    let head_e = base_entries(repo_root, "HEAD", paths)?;
    let git = |args: Vec<String>| {
        git_run()
            .cwd(repo_root)
            .args(&args)
            .discard_stderr()
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| o.stdout)
    };
    let with_paths = |mut args: Vec<String>| {
        args.push("--".to_string());
        args.extend(paths.iter().map(|p| p.to_string()));
        args
    };
    let flags = ["--no-optional-locks", "--literal-pathspecs"].map(String::from);
    let mut ls = flags.to_vec();
    ls.extend(["ls-files", "-s", "-z"].map(String::from));
    // `<mode> <oid> <stage>\t<path>`; a conflicted path never matches.
    let index_e: Entries = git(with_paths(ls))?
        .split(|b| *b == 0)
        .filter_map(|e| {
            let text = String::from_utf8_lossy(e).to_string();
            let (meta, path) = text.split_once('\t')?;
            let mut meta = meta.split(' ');
            let (mode, oid, stage) = (meta.next()?, meta.next()?, meta.next()?);
            let entry = if stage == "0" {
                (mode.to_string(), oid.to_string())
            } else {
                ("conflict".to_string(), String::new())
            };
            Some((path.to_string(), entry))
        })
        .collect();
    let mut foreign = HashSet::new();
    if start != "HEAD" {
        let mut log = flags.to_vec();
        // `--full-history --cc` so a merge commit that changed the path is listed;
        // `-m` would also list a clean merge of the base.
        log.extend(
            [
                "log",
                "--root",
                "--full-history",
                "--cc",
                "--format=",
                "--name-only",
                "-z",
            ]
            .map(String::from),
        );
        log.extend([
            format!("{start}..HEAD"),
            "--not".to_string(),
            base.to_string(),
        ]);
        foreign = git(with_paths(log))?
            .split(|b| *b == 0 || *b == b'\n')
            .filter(|n| !n.is_empty())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .collect();
    }
    Some((base_e, head_e, index_e, foreign))
}

/// The base ref for the report, as configured. A remote-tracking ref that
/// names another (`origin/HEAD`) adds the branch it points at; anything else
/// (a local branch, `HEAD`, a commit) is called a local ref, since it says
/// nothing about the default branch.
fn describe_base(repo_root: &Path, base: &str) -> (String, bool) {
    let git = |args: &[&str]| {
        git_run()
            .cwd(repo_root)
            .args(args)
            .discard_stderr()
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|t| !t.is_empty())
    };
    if base.is_empty() {
        return ("no base".to_string(), false);
    }
    let full = git(&[
        "--no-optional-locks",
        "rev-parse",
        "--symbolic-full-name",
        base,
    ]);
    if !full
        .as_deref()
        .is_some_and(|f| f.starts_with("refs/remotes/"))
    {
        return (format!("the local ref {base}"), false);
    }
    let name = match git(&["--no-optional-locks", "rev-parse", "--abbrev-ref", base]) {
        Some(short) if short != base => format!("{base} ({short})"),
        _ => base.to_string(),
    };
    (name, true)
}

/// The verdict for a judged command: the findings `check` reported on the
/// changed records, the gated moves and the protected changes.
fn report_verdict(
    fresh: &[String],
    gates: &Gates,
    (new, old): (Vec<&Finding>, Vec<&Finding>),
    note: Option<&str>,
) -> PostVerdict {
    let mut v = PostVerdict::quiet(String::new());
    v.checked = fresh.to_vec();
    v.findings = to_json(&new);
    v.pre_existing = to_json(&old);
    v.gated_moves = gates.lines(&gates.moves);
    v.protected_changes = gates.lines(&gates.protected);
    if !gates.matching().is_empty() {
        v.base_match = Some(BaseMatch {
            base: gates.base.clone(),
            level: level_name(gates.level),
            paths: gates.matching().iter().map(|e| e.path.clone()).collect(),
        });
    }
    if new.is_empty() && old.is_empty() && gates.is_empty() {
        v.message = format!(
            "{P}: checked {} changed record(s); no findings",
            fresh.len()
        );
        if let Some(note) = note {
            v.verdict = "allow-with-warnings";
            v.message.push_str(&format!(" {note}"));
            v.agent = format!("{P}: {note}");
        }
        return v;
    }
    v.agent = agent_line(&new, old.len(), gates, note);
    if !gates.is_empty() {
        v.person = gates.person();
    }
    // Every gated change is printed, a matching one included.
    v.verdict = "allow-with-warnings";
    v.message = findings_message(&FindingsSummary {
        checked: fresh.len(),
        new: &new,
        old: &old,
        gates,
        note,
    });
    v
}

/// The agent's report: the new findings to fix, one line each, and a short
/// line for a decider's change it must not present as done.
fn agent_line(new: &[&Finding], old: usize, gates: &Gates, note: Option<&str>) -> String {
    let mut parts = Vec::new();
    if !new.is_empty() {
        parts.push(format!(
            "{P}: {} new finding(s) in records the command changed:\n{}",
            new.len(),
            describe(new)
        ));
    } else if old > 0 && gates.is_empty() {
        parts.push(format!(
            "{P}: {old} finding(s) the changed records already had before this command; none is new."
        ));
    }
    let differing = gates.differing().len();
    if differing > 0 {
        parts.push(format!(
            "{P}: the command made {differing} gated change(s){}; a decider must make or approve these, and the person has been told.",
            gates.unlike()
        ));
    }
    if differing == 0 {
        parts.extend(gates.agent_match_line());
    } else if let Some(m) = gates.matching_sentence(false) {
        parts.push(format!("{P} {m}"));
    }
    if let Some(note) = note {
        parts.push(format!("{P}: {note}"));
    }
    parts.join("\n")
}

/// The report lines for what the command took out of the baseline's gates: a
/// path gate, and a status gate or the `acceptance` guardrail.
fn shrink_lines(
    repo_root: &Path,
    config_path: &Path,
    scan: &ProtectedScan,
    (loaded, prior): (&Config, Option<&ProtectedState>),
) -> Vec<GateEvent> {
    let mut lines = Vec::new();
    if !scan.shrunk.is_empty() {
        lines.push(shrink_line(repo_root, config_path, &scan.shrunk));
    }
    if !scan.status_shrunk.is_empty() {
        lines.push(status_shrink_line(
            repo_root,
            config_path,
            &scan.status_shrunk,
        ));
    }
    lines.extend(rule_change(repo_root, config_path, loaded, prior));
    lines
}

/// One report line per record the command moved to a gated status, judged
/// against the commit it started from and the baseline's status gates too.
fn gated_moves(
    repo_root: &Path,
    loaded: &Config,
    fresh: &[String],
    base: &str,
    scan: &ProtectedScan,
) -> Vec<GateEvent> {
    fresh
        .iter()
        .filter_map(|rel| {
            super::gate::gated_move_report(
                repo_root,
                loaded,
                Path::new(rel),
                base,
                &scan.baseline_status_gates,
            )
        })
        .collect()
}

/// Keeps a deletion in the memory once it is reported, and for as long as the
/// path stays deleted, so the next command does not report it again.
fn remember_deletions(
    repo_root: &Path,
    paths: &BTreeMap<String, bool>,
    reported: &[(String, GateEvent)],
    remembered: &mut BTreeMap<String, String>,
) {
    for (rel, _) in reported {
        remembered.insert(rel.clone(), "deleted".to_string());
    }
    for (rel, state) in read_memory(repo_root) {
        if state == "deleted" && paths.get(&rel) == Some(&true) {
            remembered.insert(rel, state);
        }
    }
}

/// One report line per gated record the command deleted that the memory has not
/// reported, with its path. A deletion that pairs with a record the command
/// changed (the same `Stable-Id` and record id) is a rename, not reported.
fn gated_deletions(
    repo_root: &Path,
    loaded: &Config,
    paths: &BTreeMap<String, bool>,
    base: &str,
    scan: &ProtectedScan,
) -> Vec<(String, GateEvent)> {
    let deleted: Vec<&String> = paths.iter().filter(|(_, d)| **d).map(|(p, _)| p).collect();
    if deleted.is_empty() {
        return Vec::new();
    }
    let survivors: Vec<super::gate::RecordIdentity> = paths
        .iter()
        .filter(|(_, d)| !**d)
        .filter_map(|(rel, _)| {
            let text = std::fs::read_to_string(repo_root.join(rel)).ok()?;
            super::gate::record_identity(loaded, Path::new(rel), &text)
        })
        .collect();
    let seen = read_memory(repo_root);
    deleted
        .into_iter()
        .filter(|rel| seen.get(*rel).map(String::as_str) != Some("deleted"))
        .filter_map(|rel| {
            super::gate::gated_deletion_report(
                repo_root,
                loaded,
                Path::new(rel),
                base,
                &scan.baseline_status_gates,
                &survivors,
            )
            .map(|line| (rel.clone(), line))
        })
        .collect()
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
        let gone = !rel.contains('\u{FFFD}') && !repo_root.join(&rel).exists();
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
    /// One event per protected path changed.
    protected: Vec<GateEvent>,
    /// The memory to write once judged: every candidate with its hash.
    remembered: BTreeMap<String, String>,
}

fn select_unseen(
    repo_root: &Path,
    loaded: &Config,
    paths: &BTreeMap<String, bool>,
    baseline: &[String],
) -> Selected {
    // A path is protected when the edited configuration gates it or the
    // baseline's configuration did: a gate the command removed still reports.
    let protected_report = |rel: &str, deleted: bool| {
        super::gate::protected_change_report(loaded, Path::new(rel), deleted)
            .or_else(|| super::gate::baseline_protected_report(Path::new(rel), deleted, baseline))
    };
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
        if protected_report(rel, *deleted).is_some() {
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
    let protected: Vec<GateEvent> = protected_candidates
        .iter()
        .filter(|(p, (h, _))| h.is_empty() || seen.get(*p) != Some(h))
        .filter_map(|(p, (_, deleted))| protected_report(p, *deleted))
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
    gates: &'a Gates,
    note: Option<&'a str>,
}

fn findings_message(summary: &FindingsSummary) -> String {
    let (new, old, gates) = (summary.new, summary.old, summary.gates);
    if new.is_empty() && old.is_empty() && gates.differing().is_empty() {
        let mut m = gates.matching_sentence(true).unwrap_or_default();
        if let Some(note) = summary.note {
            m.push_str(&format!("\n{note}"));
        }
        return m;
    }
    let errors = new
        .iter()
        .filter(|f| f.severity == FindingSeverity::Error)
        .count();
    let mut notes = String::new();
    let differing = gates.differing();
    if !differing.is_empty() {
        notes.push_str(&format!(
            ", and {} gated change(s) reserved for a decider",
            differing.len()
        ));
    }
    let mut m = format!(
        "{P}: urzua checked {} record(s) the command changed (git status and any commit it made, untracked included) and found {} finding(s), {errors} of them errors{notes}. The command had already run, so this was not prevented and nothing was blocked; pre-commit and CI remain the hard gate.",
        summary.checked,
        summary.new.len()
    );
    if !differing.is_empty() {
        m.push_str(&grouped(&differing));
    }
    if let Some(rest) = gates.matching_sentence(false) {
        m.push_str(&format!("\n{rest}"));
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
    let Ok(out) = git_run()
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
    let out = git_run()
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
    /// The status gates in force (empty with the `acceptance` guardrail off).
    #[serde(default)]
    status_gates: Vec<super::gate::StatusGateEntry>,
    /// The post-command comparison ref and level the configuration set.
    #[serde(default)]
    post_base: Option<String>,
    #[serde(default)]
    post_level: Option<String>,
    /// The hash of the shallow file's content (empty when there is none).
    #[serde(default)]
    post_shallow: Option<String>,
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
    /// The protected entries the previous baseline held (empty with no
    /// baseline or one taken with the guardrail off): judged beside the edited
    /// configuration's, so a gate the command removed still reports.
    baseline_entries: Vec<String>,
    /// Baseline entries the configuration no longer gates.
    shrunk: Vec<String>,
    /// The status gates the previous baseline held, judged beside the edited
    /// configuration's so a gate the command removed still reports its move.
    baseline_status_gates: Vec<super::gate::StatusGateEntry>,
    /// The `(type, status)` pairs the baseline gated and the configuration no
    /// longer does (a gate removed or narrowed, or `acceptance` switched off).
    status_shrunk: Vec<String>,
}

/// The entries the previous baseline protected that `entries` no longer does.
fn shrunk_entries(old: Option<&ProtectedState>, entries: &[String]) -> Vec<String> {
    old.filter(|o| o.enabled)
        .map(|o| {
            o.entries
                .iter()
                .filter(|e| !entries.contains(e))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn shrink_line(root: &Path, config_path: &Path, shrunk: &[String]) -> GateEvent {
    let rel = config_path.strip_prefix(root).unwrap_or(config_path);
    GateEvent {
        path: rel.display().to_string(),
        guardrail: "protected-paths",
        act: format!(
            "changed the configuration so the protected set shrank: it no longer gates {}, so those paths are not protected from now on",
            shrunk.join(", ")
        ),
        gate: String::new(),
    }
}

fn status_shrink_line(root: &Path, config_path: &Path, lost: &[String]) -> GateEvent {
    let rel = config_path.strip_prefix(root).unwrap_or(config_path);
    GateEvent {
        path: rel.display().to_string(),
        guardrail: "acceptance",
        act: format!(
            "changed the configuration so the status gates shrank: it no longer gates {} (a gate removed or narrowed, or the acceptance guardrail switched off), so those moves are not gated from now on",
            lost.join("; ")
        ),
        gate: String::new(),
    }
}

/// At session start, before the baseline is taken from the configuration as it
/// is now: the entries the previous baseline protected that it no longer gates.
pub(super) fn protected_set_shrank(root: &Path, config: &Config) -> Option<String> {
    let on = config
        .harness
        .as_ref()
        .is_some_and(|h| h.enabled("protected-paths"));
    let entries = if on {
        super::gate::protected_entries(config)
    } else {
        Vec::new()
    };
    let state = read_protected(root);
    let shrunk = shrunk_entries(state.as_ref(), &entries);
    let lost = state
        .map(|o| {
            super::gate::lost_status_gates(
                &o.status_gates,
                &super::gate::status_gate_entries(config),
            )
        })
        .unwrap_or_default();
    let mut parts = Vec::new();
    if !shrunk.is_empty() {
        parts.push(format!(
            "the protected set shrank since the last session: the configuration no longer gates {}, so those paths are not protected",
            shrunk.join(", ")
        ));
    }
    if !lost.is_empty() {
        parts.push(format!(
            "the status gates shrank since the last session: the configuration no longer gates {}, so those moves are not gated",
            lost.join("; ")
        ));
    }
    (!parts.is_empty()).then(|| {
        format!(
            "{} (review the change to the configuration)",
            parts.join("; ")
        )
    })
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
        baseline_entries: Vec::new(),
        shrunk: Vec::new(),
        baseline_status_gates: Vec::new(),
        status_shrunk: Vec::new(),
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
    let status_gates = super::gate::status_gate_entries(config);
    let old = read_protected(root);
    if report {
        if let Some(old) = &old {
            scan.status_shrunk = super::gate::lost_status_gates(&old.status_gates, &status_gates);
            scan.baseline_status_gates = old.status_gates.clone();
        }
        if let Some(old) = old.filter(|o| o.enabled) {
            compare_baseline(root, config_path, &old, &entries, (&files, &sig), &mut scan);
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
            status_gates,
            post_base: Some(
                config
                    .harness
                    .as_ref()
                    .map_or(urzua_core::harness::DEFAULT_BASE, |h| h.post_command_base())
                    .to_string(),
            ),
            post_level: Some(
                level_name(
                    config
                        .harness
                        .as_ref()
                        .map_or(FindingSeverity::Info, |h| h.post_command_base_match()),
                )
                .to_string(),
            ),
            post_shallow: shallow_content(root).map(|s| urzua_io::sha256_hex_of_bytes(&s)),
        },
    );
    scan
}

/// Compares the files now with the baseline. Files under an entry only the
/// baseline had are hashed too, so a gate the command removed and a file it
/// changed in the same command both report; the configuration file is a change
/// of its own whenever its hash differs from the baseline's.
fn compare_baseline(
    root: &Path,
    config_path: &Path,
    old: &ProtectedState,
    entries: &[String],
    now: (&BTreeMap<String, String>, &str),
    scan: &mut ProtectedScan,
) {
    let (files, sig) = now;
    let same_config = old.config_sig == sig;
    scan.baseline_entries = old.entries.clone();
    scan.shrunk = shrunk_entries(Some(old), entries);
    let removed: Vec<String> = scan.shrunk.clone();
    let (extra_found, _) = super::gate::walk_entries(root, &removed);
    let mut all = files.clone();
    for (rel, hash) in hash_files(root, &extra_found) {
        all.entry(rel).or_insert(hash);
    }
    let covered = |rel: &str| {
        same_config
            || old
                .entries
                .iter()
                .any(|e| urzua_core::rules::path_matches_entry(Path::new(rel), e))
    };
    for (rel, hash) in &all {
        if covered(rel) && old.files.get(rel) != Some(hash) {
            scan.changes.push((rel.clone(), false));
        }
    }
    for rel in old.files.keys() {
        if !all.contains_key(rel) && !root.join(rel).exists() {
            scan.changes.push((rel.clone(), true));
        }
    }
    if !same_config {
        if let Ok(rel) = config_path.strip_prefix(root) {
            scan.changes
                .push((rel.to_string_lossy().replace('\\', "/"), false));
        }
    }
}

/// Records the protected files' hashes as the baseline the next post-command
/// check compares with. The session-start check calls it.
pub(super) fn seed_protected(root: &Path, config_path: &Path, config: &Config) {
    scan_protected(root, config_path, config, false);
    // A new session starts from what is on disk: what an earlier session was
    // told is not remembered, so the same change made again is reported again.
    forget_all(Some(root));
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
    let out = git_run()
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
        git_run()
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
    let hashed = git_run()
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
    let out = git_run()
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
