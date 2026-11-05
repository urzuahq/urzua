//! Filesystem and git discovery, plus file reading. The impure half of the
//! split `urzua-core` describes in its own doc comment — see
//! ADR-0005 for why this is a separate crate, and ADR-0006 for why it shells
//! out to `git` rather than linking libgit2.
//!
//! Implements: ADR-0005, ADR-0006

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("could not run `git {args}` in {dir}: {source}")]
    Spawn {
        args: String,
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("`git {args}` in {dir} exited with {status}: {stderr}")]
    NonZeroExit {
        args: String,
        dir: PathBuf,
        status: std::process::ExitStatus,
        stderr: String,
    },
    #[error("{base:?} is not a valid base ref: a ref may not begin with `-`")]
    InvalidRef { base: String },
    #[error("there is no merge base between {base:?} and HEAD: the histories are unrelated")]
    NoMergeBase { base: String },
    #[error("`git {args}` output was not valid UTF-8")]
    InvalidUtf8 { args: String },
}

/// The set of files git considers relevant, and how they were discovered.
/// Never `Silent` — a caller that cannot enumerate git state must know that,
/// rather than receiving an empty list indistinguishable from "no files."
#[derive(Debug, Clone)]
pub struct DiscoveredFiles {
    pub paths: Vec<PathBuf>,
    /// Paths staged for deletion. They are still tracked, so they appear in
    /// `paths`, and they are legitimately not on disk. Carried separately
    /// because without it a caller cannot tell a staged deletion from a file
    /// someone deleted and has not staged, and skipping both let a record
    /// vanish from the corpus with the run still reported clean (BUG-82).
    pub staged_deletions: std::collections::HashSet<PathBuf>,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// The union of `git ls-files` (tracked) and staged files.
    GitTracked,
    /// Explicit paths on argv, read as given (`BUG-24`): git-tracked status
    /// is not consulted at all, unlike `GitTracked`'s default sweep.
    Argv,
    /// Only the records changed against a base ref (`MILE-123`): the tracked
    /// set is still what is indexed, the diff is what is reported on.
    GitDiff,
}

/// Discover files git tracks or has staged, at or below `repo_root`. Never a
/// raw filesystem walk: an untracked scratch file must never appear here, or
/// a single stray file changes what every run examines.
pub fn discover_tracked_files(repo_root: &Path) -> Result<DiscoveredFiles, DiscoveryError> {
    // `-z`, so paths arrive NUL-separated and verbatim. With `core.quotepath`
    // at its default git C-quotes any path holding a non-ASCII byte -- the
    // name arrives wrapped in quotes with the bytes escaped, its parent never
    // equals the declared `dir`, and the record drops out of the corpus with
    // no rule reporting it (BUG-87).
    let tracked = run_git(repo_root, &["ls-files", "-z"])?;
    // `--name-status`, one subprocess instead of two separate `--name-only`
    // calls: the deleted set is always a subset of the staged set for every
    // status but a rename/copy, so a second call for it was re-deriving what
    // this one call's own status codes already say. `-z` pairs each entry as
    // `<status>\0<path>\0`, except `R`/`C` (rename/copy), which git always
    // reports as `<status>\0<old-path>\0<new-path>\0` -- three fields, not
    // two (verified against real git output, not assumed from the docs). A
    // rename's old name is deliberately not counted as a staged deletion
    // here, matching the two-call version's own behavior: `--diff-filter=D`
    // never matched a rename either, since git classifies it `R`, not `D`.
    let staged = run_git(repo_root, &["diff", "--name-status", "--cached", "-z"])?;

    let mut staged_paths = Vec::new();
    let mut staged_deletions = std::collections::HashSet::new();
    let mut fields = staged.split('\0').filter(|f| !f.is_empty());
    while let Some(status) = fields.next() {
        let is_rename_or_copy = status.starts_with('R') || status.starts_with('C');
        let Some(first_path) = fields.next() else {
            break;
        };
        let path = if is_rename_or_copy {
            // The old name is the extra field a rename/copy carries; the
            // new name is the one that matters to a caller reading the
            // working tree today.
            let Some(new_path) = fields.next() else {
                break;
            };
            new_path
        } else {
            first_path
        };
        let path = PathBuf::from(path);
        if status.starts_with('D') {
            staged_deletions.insert(path.clone());
        }
        staged_paths.push(path);
    }

    let mut paths: Vec<PathBuf> = tracked
        .split('\0')
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .chain(staged_paths)
        .collect();
    paths.sort();
    paths.dedup();

    Ok(DiscoveredFiles {
        paths,
        staged_deletions,
        source: DiscoverySource::GitTracked,
    })
}

/// What differs between the merge base of a ref and the working tree
/// (`MILE-123`).
#[derive(Debug, Clone)]
pub struct GitChanges {
    /// The merge base commit, so a caller can read a deleted file as it was.
    pub merge_base: String,
    /// Every path that differs, deletions included, repository-relative.
    pub changed: std::collections::HashSet<PathBuf>,
    /// The subset of `changed` that no longer exists in the working tree.
    pub deleted: std::collections::HashSet<PathBuf>,
    /// The subset of `changed` that did not exist at the merge base. With
    /// `--no-renames` a renamed file is a deletion plus an addition, so the
    /// new name is in here.
    pub added: std::collections::HashSet<PathBuf>,
}

/// Every path that differs between the merge base of `base` and `HEAD` and
/// the working tree, repository-relative (`MILE-123`). Merge base, not the
/// ref's tip: a pull request must not be charged for what its base branch
/// gained since the fork. The working tree, not `HEAD`: staged and unstaged
/// edits to tracked files are changes a local run must see. Untracked files
/// are not reported, matching the tracked-only discovery (`ADR-6`).
///
/// Renames are reported as a deletion plus an addition (`--no-renames`), so
/// the new name is always listed on its own. A deleted path is listed too,
/// and also in `deleted`: a record that was deleted is no longer in the
/// tree, but whatever referred to it is affected by the change.
pub fn changed_paths_since(repo_root: &Path, base: &str) -> Result<GitChanges, DiscoveryError> {
    // A ref beginning with `-` would be read by git as an option.
    if base.starts_with('-') {
        return Err(DiscoveryError::InvalidRef {
            base: base.to_string(),
        });
    }
    // `merge-base` exits 1 with nothing on stderr when the two commits share
    // no history; every other failure (an unknown ref, 128) keeps git's own
    // message, which names the ref.
    let merge_base = match run_git(repo_root, &["merge-base", base, "HEAD"]) {
        Ok(out) => out,
        Err(DiscoveryError::NonZeroExit { status, stderr, .. })
            if status.code() == Some(1) && stderr.trim().is_empty() =>
        {
            return Err(DiscoveryError::NoMergeBase {
                base: base.to_string(),
            })
        }
        Err(e) => return Err(e),
    };
    let merge_base = merge_base.trim().to_string();
    let diff = run_git(
        repo_root,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
            &merge_base,
            "--",
        ],
    )?;
    let mut changed = std::collections::HashSet::new();
    let mut deleted = std::collections::HashSet::new();
    let mut added = std::collections::HashSet::new();
    let mut fields = diff.split('\0').filter(|f| !f.is_empty());
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let path = PathBuf::from(path);
        if status.starts_with('D') {
            deleted.insert(path.clone());
        }
        if status.starts_with('A') {
            added.insert(path.clone());
        }
        changed.insert(path);
    }
    Ok(GitChanges {
        merge_base,
        changed,
        deleted,
        added,
    })
}

/// A file as it was at `commit`: how a deleted record is read once it is
/// gone from the tree.
pub fn read_at_commit(
    repo_root: &Path,
    commit: &str,
    path: &Path,
) -> Result<String, DiscoveryError> {
    let spec = format!("{commit}:{}", path.display());
    run_git(repo_root, &["show", &spec])
}

/// Read a file's content as UTF-8. The only I/O `urzua-core`'s parser needs;
/// `urzua-core` itself never touches a `Path`.
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Today's date, `YYYY-MM-DD`, local time. The one piece of wall-clock I/O
/// waiver expiry needs (ADR-0011) -- `urzua-core`'s expiry comparison itself
/// takes this as a plain string argument and stays pure.
pub fn today() -> String {
    chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

const GH_AUTH_TIMEOUT: Duration = Duration::from_secs(5);

/// `resolve_identity`'s result plus any non-fatal notice about it. Structured
/// rather than a bare `eprintln!` so a caller can fold `warning` into its own
/// JSON report instead of it landing as unparseable stderr prose.
pub struct ResolvedIdentity {
    pub name: String,
    pub warning: Option<String>,
}

/// Identity tiering (RFC-0002 §2, amended by ADR-0031 in this repo's own
/// records): `gh api user` wins even over an explicit `--by` (surfaced via
/// `warning` on divergence, never silently discarding `--by`); `--by` wins
/// over `git config user.name`. Errors rather than defaulting when none of
/// the three resolve.
pub fn resolve_identity(
    explicit: Option<&str>,
    repo_root: &Path,
) -> Result<ResolvedIdentity, String> {
    if let Some(login) = gh_authenticated_login() {
        let warning = match explicit {
            Some(by) if by != login => Some(format!(
                "--by '{by}' differs from the authenticated gh login '{login}' -- using '{login}'"
            )),
            _ => None,
        };
        return Ok(ResolvedIdentity {
            name: login,
            warning,
        });
    }

    if let Some(by) = explicit {
        return Ok(ResolvedIdentity {
            name: by.to_string(),
            warning: None,
        });
    }

    if let Ok(output) = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(repo_root)
        .output()
    {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Ok(ResolvedIdentity {
                    name,
                    warning: None,
                });
            }
        }
    }

    Err("could not resolve an identity (gh api user unavailable, git config user.name unset) -- pass --by explicitly".to_string())
}

/// `Command::output()` blocks indefinitely if `gh` hangs -- an
/// unauthenticated interactive prompt, a stalled network call. Bounded so a
/// hung `gh` falls through to the next identity tier instead of hanging
/// `fix --apply` forever (RFC-0002's undesigned timeout question).
fn gh_authenticated_login() -> Option<String> {
    let mut child = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + GH_AUTH_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut stdout = String::new();
                child.stdout.take()?.read_to_string(&mut stdout).ok()?;
                let login = stdout.trim().to_string();
                return if login.is_empty() { None } else { Some(login) };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// A corpus-level lock for `urzua fix --apply` (RFC-0008 §4): apply is
/// single-process, and a second concurrent apply must fail loud rather than
/// interleave writes. `create_new` is atomic -- two processes racing to
/// create the same file, only one succeeds.
pub struct FixLock {
    path: PathBuf,
}

impl FixLock {
    pub fn acquire(repo_root: &Path) -> Result<Self, String> {
        let dir = repo_root.join(".urzua/cache");
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let path = dir.join("fix.lock");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| {
                format!(
                    "{} already exists -- another `urzua fix --apply` may be running; remove it if that's not the case",
                    path.display()
                )
            })?;
        Ok(FixLock { path })
    }
}

impl Drop for FixLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The commit that last touched a specific 1-indexed line (RFC-0005 tier 1),
/// via `git blame`. Scoped to the line, not the whole file, so an unrelated
/// edit elsewhere in a record can't silently reset its own staleness clock
/// (ADR-0032). `Ok(None)` for anything blame can't resolve -- no history,
/// an untracked file -- never an error a caller has to specially handle.
pub fn commit_for_line(
    repo_root: &Path,
    path: &Path,
    line: usize,
) -> Result<Option<String>, String> {
    commit_for_line_in(repo_root, path, line, None)
}

/// As [`commit_for_line`], blaming `contents` (proposed text, not yet on
/// disk) instead of the working-tree file when given. A line the proposal
/// changes or adds is attributed to no commit: git reports an all-zero hash,
/// which the caller reads as "touched now" (`MILE-159`).
pub fn commit_for_line_in(
    repo_root: &Path,
    path: &Path,
    line: usize,
    contents: Option<&str>,
) -> Result<Option<String>, String> {
    use std::io::Write;
    let mut cmd = Command::new("git");
    cmd.args(["blame", "-L", &format!("{line},{line}"), "--porcelain"]);
    if contents.is_some() {
        cmd.args(["--contents", "-"]);
    }
    cmd.arg("--")
        .arg(path)
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(if contents.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        });
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("could not run git blame on {}: {e}", path.display()))?;
    if let (Some(text), Some(mut stdin)) = (contents, child.stdin.take()) {
        // A broken pipe means git exited early (no history); its status says why.
        let _ = stdin.write_all(text.as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("could not run git blame on {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // No history for the file or line (untracked, staged-new, unborn
        // HEAD) is the ordinary "nothing to date" answer; anything else is a
        // git failure the caller must be able to report (`MILE-159`).
        if is_no_history_stderr(&stderr) {
            return Ok(None);
        }
        return Err(format!(
            "git blame on {} failed: {}",
            path.display(),
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().next())
        .map(|s| s.to_string()))
}

/// The commit that last touched a whole file -- used for locator paths,
/// which (unlike a header field) aren't a single line (ADR-0032).
pub fn last_commit_for_path(repo_root: &Path, path: &Path) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H", "--"])
        .arg(path)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git log on {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_no_history_stderr(&stderr) {
            return Ok(None);
        }
        return Err(format!(
            "git log on {} failed: {}",
            path.display(),
            stderr.trim()
        ));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// The newest commit that last touched any of `paths` -- a directory or glob
/// locator's drift anchor (`BUG-176`). Paths are literal (never read as
/// pathspec magic) and are passed in batches, so a locator matching many files
/// does not exceed the argument limit; the newest of the batches' answers is
/// chosen by commit date.
pub fn last_commit_for_paths(
    repo_root: &Path,
    paths: &[PathBuf],
) -> Result<Option<String>, String> {
    let mut shas: Vec<String> = Vec::new();
    for batch in paths.chunks(400) {
        let output = Command::new("git")
            .args(["--literal-pathspecs", "log", "-1", "--format=%H", "--"])
            .args(batch)
            .current_dir(repo_root)
            .output()
            .map_err(|e| format!("could not run git log: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if is_no_history_stderr(&stderr) {
                return Ok(None);
            }
            return Err(format!("git log failed: {}", stderr.trim()));
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !sha.is_empty() && !shas.contains(&sha) {
            shas.push(sha);
        }
    }
    if shas.len() <= 1 {
        return Ok(shas.pop());
    }
    let output = Command::new("git")
        .args(["log", "--no-walk=sorted", "-n", "1", "--format=%H"])
        .args(&shas)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git log: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// True if `ancestor` came strictly before `descendant` in history.
/// `Ok(false)` when git says it is not an ancestor (exit 1) or the commits
/// are the same. Any other git outcome (a commit the repository does not
/// have, as in a shallow clone) is an `Err`, never a quiet "no drift"
/// (`MILE-159`).
pub fn commit_strictly_before(
    repo_root: &Path,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, String> {
    if ancestor == descendant {
        return Ok(false);
    }
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git merge-base: {e}"))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "git merge-base --is-ancestor {} {} failed: {}",
            &ancestor[..ancestor.len().min(12)],
            &descendant[..descendant.len().min(12)],
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// Whether the repository is a shallow clone, which hides the history the
/// drift computation reads (`MILE-159`). `Err` when git cannot say.
pub fn is_shallow_repository(repo_root: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-shallow-repository"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --is-shallow-repository failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim() == "true")
}

/// git's wording for "this path or ref has no history here", which is an
/// answer, not a failure.
fn is_no_history_stderr(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("no such path")
        || s.contains("does not have any commits")
        || s.contains("unknown revision")
        || s.contains("bad revision")
        || s.contains("ambiguous argument 'head'")
}

/// Committer time of `commit`, in seconds since the epoch (`MILE-129`).
/// `None` when git cannot say, so a grace window or claim age never fires on
/// a commit it could not date.
pub fn commit_unix_time(repo_root: &Path, commit: &str) -> Option<i64> {
    let output = Command::new("git")
        .args(["show", "-s", "--format=%ct", commit])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Seconds since the epoch, now (`MILE-129`).
pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Lowercase hex SHA-256 of the file's bytes (`MILE-129`), or `None` when it
/// cannot be read (a missing locator is `embodiment.locator-exists`' finding,
/// not a hash mismatch).
pub fn sha256_hex_of_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Lowercase hex SHA-256 of a string's UTF-8 bytes (`ADR-74`: the pinned
/// bytes of a scoped span).
pub fn sha256_hex_of_str(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String, DiscoveryError> {
    let args_str = args.join(" ");
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|source| DiscoveryError::Spawn {
            args: args_str.clone(),
            dir: dir.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(DiscoveryError::NonZeroExit {
            args: args_str,
            dir: dir.to_path_buf(),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    String::from_utf8(output.stdout).map_err(|_| DiscoveryError::InvalidUtf8 { args: args_str })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serializes PATH-stubbing tests -- `std::env::set_var("PATH", ...)` is
    /// process-wide and Rust runs tests in parallel threads.
    static PATH_MUTEX: Mutex<()> = Mutex::new(());

    fn init_repo(dir: &Path) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git command should run")
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "test"]);
    }

    #[test]
    fn discovers_only_tracked_and_staged_files_never_a_raw_walk() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("tracked.md"), "tracked").unwrap();
        Command::new("git")
            .args(["add", "tracked.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        // The untracked scratch file is the planted violation: a raw
        // filesystem walk would find it, and the check must not.
        fs::write(tmp.join("scratch.md"), "untracked scratch file").unwrap();

        let result = discover_tracked_files(&tmp).expect("discovery should succeed");
        let names: Vec<_> = result
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"tracked.md".to_string()));
        assert!(
            !names.contains(&"scratch.md".to_string()),
            "untracked scratch file must never be discovered: {names:?}"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    /// `BUG-147`'s own regression: merging the staged/deleted discovery into
    /// one `--name-status` call has to handle a staged rename correctly --
    /// `git diff --name-status` reports it as `R100\0old\0new\0`, three
    /// fields, not the two every other status line carries. A naive
    /// two-fields-per-record parser would misalign on exactly this input.
    #[test]
    fn a_staged_rename_is_discovered_under_its_new_name_only_observed_failing() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-rename-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);
        // Pin rename detection locally: a developer's global `diff.renames=false`
        // would otherwise report this move as `D`+`A` and fail the test.
        Command::new("git")
            .args(["config", "diff.renames", "true"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        // Identical, substantial content: git's default rename detection
        // needs enough similarity to classify this as `R`, not a plain `D`
        // followed by an unrelated `A`.
        let body = "line one\nline two\nline three\nline four\nline five\n".repeat(4);
        fs::write(tmp.join("old-name.md"), &body).unwrap();
        Command::new("git")
            .args(["add", "old-name.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        Command::new("git")
            .args(["mv", "old-name.md", "new-name.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        let result = discover_tracked_files(&tmp).expect("discovery should succeed");
        let names: Vec<_> = result
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        assert!(
            names.contains(&"new-name.md".to_string()),
            "the renamed file's new name must be discovered: {names:?}"
        );
        assert!(
            !names.contains(&"old-name.md".to_string()),
            "a rename's old name must not linger as a phantom entry: {names:?}"
        );
        assert!(
            !result
                .staged_deletions
                .contains(&PathBuf::from("old-name.md")),
            "a rename is not a deletion -- git classifies it R, not D: {:?}",
            result.staged_deletions
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_missing_git_repo_is_a_reported_error_not_a_silent_empty_list() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-notgit-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();

        let result = discover_tracked_files(&tmp);
        assert!(
            result.is_err(),
            "a directory with no git repo must error, not return an empty list silently"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    /// A stub `gh` ahead of the real PATH, always exiting non-zero -- lets
    /// `resolve_identity`'s gh-unavailable fallback be tested deterministically
    /// without depending on whether this machine actually has `gh` authenticated.
    fn prepend_failing_gh_to_path() -> (PathBuf, String) {
        prepend_stub_gh_to_path("exit 1\n")
    }

    /// A stub `gh` ahead of the real PATH that succeeds and reports `login`
    /// -- lets the gh-wins-even-over-explicit-`--by` tier be tested
    /// deterministically, the mirror of `prepend_failing_gh_to_path`.
    fn prepend_successful_gh_to_path(login: &str) -> (PathBuf, String) {
        prepend_stub_gh_to_path(&format!("echo '{login}'\nexit 0\n"))
    }

    fn prepend_stub_gh_to_path(script_body: &str) -> (PathBuf, String) {
        let stub_dir = std::env::temp_dir().join(format!(
            "urzua-io-fake-gh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&stub_dir).unwrap();
        let stub_gh = stub_dir.join("gh");
        fs::write(&stub_gh, format!("#!/bin/sh\n{script_body}")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&stub_gh, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", stub_dir.display(), original_path));
        (stub_dir, original_path)
    }

    #[test]
    fn resolve_identity_falls_through_to_git_config_when_gh_is_unavailable() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = std::env::temp_dir().join(format!("urzua-io-identity-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(None, &tmp);

        std::env::set_var("PATH", original_path);
        fs::remove_dir_all(&tmp).ok();
        fs::remove_dir_all(&stub_dir).ok();

        let result = result.unwrap();
        assert_eq!(
            result.name, "test",
            "with gh unavailable, git config user.name must win over nothing"
        );
        assert!(result.warning.is_none());
    }

    /// gh unavailable, git config unset, explicit --by given -> --by
    /// resolves. Doesn't cover --by vs. a present git config -- see
    /// `resolve_identity_prefers_explicit_by_over_git_config`.
    #[test]
    fn resolve_identity_never_lets_explicit_by_override_a_verified_gh_login() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = std::env::temp_dir().join(format!("urzua-io-identity-by-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        // Isolates from this machine's real global git config.
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_SYSTEM", "/dev/null");

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::remove_var("GIT_CONFIG_GLOBAL");
        std::env::remove_var("GIT_CONFIG_SYSTEM");
        std::env::set_var("PATH", original_path);
        fs::remove_dir_all(&tmp).ok();
        fs::remove_dir_all(&stub_dir).ok();

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn resolve_identity_prefers_explicit_by_over_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp =
            std::env::temp_dir().join(format!("urzua-io-identity-by2-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);
        fs::remove_dir_all(&tmp).ok();
        fs::remove_dir_all(&stub_dir).ok();

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn resolve_identity_lets_gh_win_over_a_differing_explicit_by() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");

        let tmp = std::env::temp_dir().join(format!("urzua-io-identity-gh-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);
        fs::remove_dir_all(&tmp).ok();
        fs::remove_dir_all(&stub_dir).ok();

        let result = result.unwrap();
        assert_eq!(result.name, "gh-login");
        assert!(result.warning.unwrap().contains("explicit-name"));
    }

    fn commit_all(dir: &Path, message: &str) -> String {
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", message])
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    }

    #[test]
    fn commit_strictly_before_is_true_only_in_the_real_commit_order() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-order-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("a.txt"), "1").unwrap();
        let first = commit_all(&tmp, "first");
        fs::write(tmp.join("a.txt"), "2").unwrap();
        let second = commit_all(&tmp, "second");

        assert!(
            commit_strictly_before(&tmp, &first, &second).unwrap(),
            "first really did happen before second"
        );
        assert!(
            !commit_strictly_before(&tmp, &second, &first).unwrap(),
            "second did not happen before first"
        );
        assert!(
            !commit_strictly_before(&tmp, &first, &first).unwrap(),
            "a commit is never strictly before itself"
        );
        // A commit the repository does not have is a git failure, not
        // "no drift" (`MILE-159`).
        let missing = "0123456789abcdef0123456789abcdef01234567";
        assert!(
            commit_strictly_before(&tmp, missing, &second).is_err(),
            "an unknown commit must surface as an error"
        );
        assert!(!is_shallow_repository(&tmp).unwrap());

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_commit_for_path_tracks_the_most_recent_touch() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-lastcommit-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("a.txt"), "1").unwrap();
        fs::write(tmp.join("b.txt"), "1").unwrap();
        let first = commit_all(&tmp, "add both");
        fs::write(tmp.join("a.txt"), "2").unwrap();
        let second = commit_all(&tmp, "touch a only");

        assert_eq!(
            last_commit_for_path(&tmp, &tmp.join("a.txt")).unwrap(),
            Some(second)
        );
        assert_eq!(
            last_commit_for_path(&tmp, &tmp.join("b.txt")).unwrap(),
            Some(first)
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn commit_for_line_is_scoped_to_the_line_not_the_whole_file() {
        let tmp = std::env::temp_dir().join(format!("urzua-io-blame-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("doc.md"), "line one\nline two\n").unwrap();
        let first = commit_all(&tmp, "add both lines");
        fs::write(tmp.join("doc.md"), "line one\nline two CHANGED\n").unwrap();
        let second = commit_all(&tmp, "change line two only");

        assert_eq!(
            commit_for_line(&tmp, &tmp.join("doc.md"), 1).unwrap(),
            Some(first),
            "line 1 was never touched by the second commit"
        );
        assert_eq!(
            commit_for_line(&tmp, &tmp.join("doc.md"), 2).unwrap(),
            Some(second),
            "line 2 was changed by the second commit"
        );

        fs::remove_dir_all(&tmp).ok();
    }
}
