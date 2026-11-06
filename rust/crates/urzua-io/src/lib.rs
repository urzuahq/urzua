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
    #[error("there is no merge base between {base:?} and HEAD because this is a shallow clone and the history that joins them is not fetched: fetch full history (e.g. `git fetch --unshallow`, or `fetch-depth: 0` in CI)")]
    ShallowClone { base: String },
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
    /// Tracked record paths missing from the working tree that nothing has
    /// staged for deletion (an `rm`, or a `mv` before `git add`). The caller
    /// that notices them also adds them to `staged_deletions`, so they are
    /// treated as removed; this set is what lets the run say so rather than
    /// let a record vanish in silence (BUG-82).
    pub unstaged_deletions: std::collections::HashSet<PathBuf>,
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
        unstaged_deletions: std::collections::HashSet::new(),
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
            // A shallow clone hides the commits that join the two histories;
            // that, not unrelated histories, is the usual cause in CI.
            if is_shallow_repository(repo_root).unwrap_or(false) {
                return Err(DiscoveryError::ShallowClone {
                    base: base.to_string(),
                });
            }
            return Err(DiscoveryError::NoMergeBase {
                base: base.to_string(),
            });
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

/// A file as it was at `commit`, or `None` when the commit has no such path.
/// Absence is an answer; every other failure (a missing object, a copy that is
/// not UTF-8) is an error, so a caller cannot read "could not read" as "was
/// never there". Presence is asked of the tree (`ls-tree`), not read off git's
/// error wording, which is localised.
pub fn read_at_commit_if_present(
    repo_root: &Path,
    commit: &str,
    path: &Path,
) -> Result<Option<String>, DiscoveryError> {
    let listed = run_git(
        repo_root,
        &[
            "ls-tree",
            "--name-only",
            "-z",
            commit,
            "--",
            &path.display().to_string(),
        ],
    )?;
    if listed.is_empty() {
        return Ok(None);
    }
    read_at_commit(repo_root, commit, path).map(Some)
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

/// Where a resolved identity came from (`ADR-89` section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// The `gh` login that `gh api user` verified.
    GhVerified,
    /// `--by`, taken as given.
    Asserted,
    /// `git config user.name`.
    GitConfig,
}

/// The `attestation` configuration, as `resolve_identity_with` reads it
/// (`ADR-89` section 5). `urzua-io` does not know the configuration type, so
/// the caller passes the two decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdentityPolicy {
    /// `identity: asserted`: `--by` is taken as given, a verified login is
    /// recorded beside it.
    pub asserted: bool,
    /// `require_verified: true`: no `gh` answer is an error, never a fallback.
    pub require_verified: bool,
}

/// `resolve_identity`'s result plus any non-fatal notice about it. Structured
/// rather than a bare `eprintln!` so a caller can fold `warning` into its own
/// JSON report instead of it landing as unparseable stderr prose.
#[derive(Debug)]
pub struct ResolvedIdentity {
    pub name: String,
    pub warning: Option<String>,
    pub source: IdentitySource,
    /// The `--by` a verified login replaced, when the two differ.
    pub replaced_by: Option<String>,
    /// The login `gh` verified, recorded beside an `asserted` name.
    pub verified_login: Option<String>,
}

/// `resolve_identity_with` under the default policy (`gh-verified`, nothing
/// required): the behaviour of a repository that declares no `attestation`.
pub fn resolve_identity(
    explicit: Option<&str>,
    repo_root: &Path,
) -> Result<ResolvedIdentity, String> {
    resolve_identity_with(explicit, repo_root, IdentityPolicy::default())
}

/// Identity tiering (RFC-0002 §2, amended by ADR-0031 in this repo's own
/// records; `ADR-89` section 5 makes the source configurable): under the
/// default a `gh api user` login wins even over an explicit `--by` (surfaced
/// via `warning` and `replaced_by` on divergence, never silently discarding
/// `--by`); `--by` wins over `git config user.name`. Errors rather than
/// defaulting when none of the three resolve.
pub fn resolve_identity_with(
    explicit: Option<&str>,
    repo_root: &Path,
    policy: IdentityPolicy,
) -> Result<ResolvedIdentity, String> {
    // Checked whichever tier wins: a `--by` that would corrupt the row is
    // refused even when a gh login supersedes it.
    if let Some(by) = explicit {
        row_safe_identity(by, "--by")?;
    }
    let resolved = resolve_identity_unchecked(explicit, repo_root, policy)?;
    row_safe_identity(&resolved.name, "the resolved identity")?;
    if let Some(login) = &resolved.verified_login {
        row_safe_identity(login, "the verified gh login")?;
    }
    Ok(resolved)
}

/// The identity is written into a revision-log table row, where a `|` adds a
/// cell and a line break splits the row. Refused, as `--reason` is, rather
/// than rewritten: a silently altered name is not the one that attested.
fn row_safe_identity(name: &str, source: &str) -> Result<(), String> {
    if name.chars().any(|c| c == '|' || c.is_control()) {
        return Err(format!(
            "{source} {name:?} contains '|' or a control character such as a line break; \
             it is written into a revision-log table row. Pass --by with a plain name \
             (or fix `git config user.name`)"
        ));
    }
    Ok(())
}

fn resolve_identity_unchecked(
    explicit: Option<&str>,
    repo_root: &Path,
    policy: IdentityPolicy,
) -> Result<ResolvedIdentity, String> {
    let verified = gh_authenticated_login();

    if policy.asserted {
        // `--by` as given, the verified login beside it (`ADR-89` section 5).
        // With no `--by` there is nothing asserted: the git configuration is
        // the next tier, as it is under `gh-verified`.
        let (name, source) = match explicit {
            Some(by) => (by.to_string(), IdentitySource::Asserted),
            None => (git_config_name(repo_root)?, IdentitySource::GitConfig),
        };
        return Ok(ResolvedIdentity {
            name,
            warning: None,
            source,
            replaced_by: None,
            verified_login: verified,
        });
    }

    if let Some(login) = verified {
        let replaced_by = explicit.filter(|by| *by != login).map(str::to_string);
        let warning = replaced_by.as_ref().map(|by| {
            format!(
                "--by '{by}' differs from the authenticated gh login '{login}' -- using '{login}'"
            )
        });
        return Ok(ResolvedIdentity {
            name: login,
            warning,
            source: IdentitySource::GhVerified,
            replaced_by,
            verified_login: None,
        });
    }

    if policy.require_verified {
        return Err(
            "attestation.require_verified is true and gh could not verify a login (`gh api user` failed or timed out); nothing was written. Log in with `gh auth login`, or set `attestation.identity: asserted` for a repository whose agents attest on behalf of a named person"
                .to_string(),
        );
    }

    if let Some(by) = explicit {
        return Ok(ResolvedIdentity {
            name: by.to_string(),
            warning: None,
            source: IdentitySource::Asserted,
            replaced_by: None,
            verified_login: None,
        });
    }

    Ok(ResolvedIdentity {
        name: git_config_name(repo_root)?,
        warning: None,
        source: IdentitySource::GitConfig,
        replaced_by: None,
        verified_login: None,
    })
}

fn git_config_name(repo_root: &Path) -> Result<String, String> {
    if let Ok(output) = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(repo_root)
        .output()
    {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Ok(name);
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
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "{} already exists -- another `urzua fix --apply` may be running; remove it if that's not the case",
                        path.display()
                    )
                } else {
                    format!("could not create {}: {e}", path.display())
                }
            })?;
        Ok(FixLock { path })
    }
}

impl Drop for FixLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Replace `path` with `text` only once the whole text is on disk: written to a
/// temporary file in the same directory, then renamed over the target. A write
/// that fails (disk full) or a process killed part-way leaves the old file
/// intact instead of an empty or truncated record or configuration. An
/// existing file's permissions are kept; a symlink is written through, so the
/// link stays a link. A read-only file is refused, as a plain write would.
/// The temporary file is removed on failure.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let target = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => std::fs::canonicalize(path)?,
        _ => path.to_path_buf(),
    };
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.urzua-tmp-{}", std::process::id()));
    let result = (|| {
        // A file made read-only on purpose stays unwritable: rename would
        // replace it regardless of its own mode.
        if let Ok(meta) = std::fs::metadata(&target) {
            if meta.permissions().readonly() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "the file is read-only",
                ));
            }
        }
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if let Ok(meta) = std::fs::metadata(&target) {
            std::fs::set_permissions(&temp, meta.permissions())?;
        }
        std::fs::rename(&temp, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
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

    /// A fresh directory under the temp dir, removed on drop so a failing
    /// assertion cannot leave it behind (BUG-193).
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Scratch {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("{label}-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    #[test]
    fn discovers_only_tracked_and_staged_files_never_a_raw_walk() {
        let tmp = Scratch::new("urzua-io-test");
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
    }

    /// `BUG-147`'s own regression: merging the staged/deleted discovery into
    /// one `--name-status` call has to handle a staged rename correctly --
    /// `git diff --name-status` reports it as `R100\0old\0new\0`, three
    /// fields, not the two every other status line carries. A naive
    /// two-fields-per-record parser would misalign on exactly this input.
    #[test]
    fn a_staged_rename_is_discovered_under_its_new_name_only_observed_failing() {
        let tmp = Scratch::new("urzua-io-rename-test");
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
    }

    #[test]
    fn a_missing_git_repo_is_a_reported_error_not_a_silent_empty_list() {
        let tmp = Scratch::new("urzua-io-notgit");
        fs::create_dir_all(&tmp).unwrap();

        let result = discover_tracked_files(&tmp);
        assert!(
            result.is_err(),
            "a directory with no git repo must error, not return an empty list silently"
        );
    }

    /// A stub `gh` ahead of the real PATH, always exiting non-zero -- lets
    /// `resolve_identity`'s gh-unavailable fallback be tested deterministically
    /// without depending on whether this machine actually has `gh` authenticated.
    fn prepend_failing_gh_to_path() -> (Scratch, String) {
        prepend_stub_gh_to_path("exit 1\n")
    }

    /// A stub `gh` ahead of the real PATH that succeeds and reports `login`
    /// -- lets the gh-wins-even-over-explicit-`--by` tier be tested
    /// deterministically, the mirror of `prepend_failing_gh_to_path`.
    fn prepend_successful_gh_to_path(login: &str) -> (Scratch, String) {
        prepend_stub_gh_to_path(&format!("echo '{login}'\nexit 0\n"))
    }

    fn prepend_stub_gh_to_path(script_body: &str) -> (Scratch, String) {
        let stub_dir = Scratch::new("urzua-io-fake-gh");
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
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(None, &tmp);

        std::env::set_var("PATH", original_path);

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
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-by");
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

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn resolve_identity_prefers_explicit_by_over_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-by2");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temporary_behind() {
        let tmp = Scratch::new("urzua-io-atomic");
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("record.md");
        fs::write(&path, "old\n").unwrap();
        write_atomic(&path, "new\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let names: Vec<_> = fs::read_dir(&*tmp).unwrap().collect();
        assert_eq!(names.len(), 1, "only the record remains: {names:?}");
    }

    #[test]
    fn write_atomic_failing_part_way_keeps_the_old_file() {
        let tmp = Scratch::new("urzua-io-atomic-fail");
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("record.md");
        fs::write(&path, "old\n").unwrap();
        // The temporary name is taken by a directory, so creating it fails
        // before anything is renamed: the target must be untouched.
        let temp = tmp.join(format!(".record.md.urzua-tmp-{}", std::process::id()));
        fs::create_dir(&temp).unwrap();
        assert!(write_atomic(&path, "new\n").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_the_mode_and_writes_through_a_symlink() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = Scratch::new("urzua-io-atomic-link");
        fs::create_dir_all(&tmp).unwrap();
        let real = tmp.join("real.yaml");
        fs::write(&real, "a: 1\n").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        let link = tmp.join("link.yaml");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "a: 2\n").unwrap();
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "a: 2\n");
        assert_eq!(
            fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn resolve_identity_refuses_a_name_that_would_break_a_table_row() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-pipe");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        let pipe = resolve_identity(Some("a | b"), &tmp);
        let newline = resolve_identity(Some("a\nb"), &tmp);
        // A git user.name is checked too, not only --by.
        Command::new("git")
            .args(["config", "user.name", "x | y"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        let from_git = resolve_identity(None, &tmp);

        std::env::set_var("PATH", original_path);

        assert!(pipe.unwrap_err().contains("--by"));
        assert!(newline.is_err());
        assert!(from_git.unwrap_err().contains("resolved identity"));
    }

    #[test]
    fn resolve_identity_lets_gh_win_over_a_differing_explicit_by() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");

        let tmp = Scratch::new("urzua-io-identity-gh");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "gh-login");
        assert!(result.warning.unwrap().contains("explicit-name"));
    }

    fn quiet_repo(prefix: &str, with_git_name: bool) -> Scratch {
        let tmp = Scratch::new(prefix);
        fs::create_dir_all(&tmp).unwrap();
        if with_git_name {
            init_repo(&tmp); // sets local user.name to "test"
        } else {
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(&tmp)
                .output()
                .unwrap();
        }
        tmp
    }

    const GH_VERIFIED: IdentityPolicy = IdentityPolicy {
        asserted: false,
        require_verified: false,
    };
    const GH_REQUIRED: IdentityPolicy = IdentityPolicy {
        asserted: false,
        require_verified: true,
    };
    const ASSERTED: IdentityPolicy = IdentityPolicy {
        asserted: true,
        require_verified: false,
    };

    #[test]
    fn gh_verified_names_the_replaced_by_as_a_field_and_the_source() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-replaced", true);

        let result = resolve_identity_with(Some("typed"), &tmp, GH_VERIFIED);
        let agree = resolve_identity_with(Some("gh-login"), &tmp, GH_VERIFIED);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "gh-login");
        assert_eq!(result.source, IdentitySource::GhVerified);
        assert_eq!(result.replaced_by.as_deref(), Some("typed"));
        assert!(result.warning.unwrap().contains("typed"));
        let agree = agree.unwrap();
        assert_eq!(agree.source, IdentitySource::GhVerified);
        assert!(agree.replaced_by.is_none() && agree.warning.is_none());
    }

    #[test]
    fn gh_verified_without_a_gh_answer_records_by_as_asserted_and_git_as_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-fallback", true);

        let typed = resolve_identity_with(Some("typed"), &tmp, GH_VERIFIED);
        let git = resolve_identity_with(None, &tmp, GH_VERIFIED);

        std::env::set_var("PATH", original_path);

        assert_eq!(typed.unwrap().source, IdentitySource::Asserted);
        let git = git.unwrap();
        assert_eq!(
            (git.name.as_str(), git.source),
            ("test", IdentitySource::GitConfig)
        );
    }

    #[test]
    fn require_verified_refuses_every_fallback_when_gh_does_not_answer() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-required", true);

        let with_by = resolve_identity_with(Some("typed"), &tmp, GH_REQUIRED);
        let with_git = resolve_identity_with(None, &tmp, GH_REQUIRED);

        std::env::set_var("PATH", original_path);

        for err in [with_by.unwrap_err(), with_git.unwrap_err()] {
            assert!(
                err.contains("require_verified") && err.contains("gh"),
                "{err}"
            );
        }
    }

    #[test]
    fn require_verified_accepts_a_verified_login() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-required-ok", true);

        let result = resolve_identity_with(None, &tmp, GH_REQUIRED);

        std::env::set_var("PATH", original_path);

        assert_eq!(result.unwrap().source, IdentitySource::GhVerified);
    }

    #[test]
    fn asserted_takes_by_as_given_with_the_verified_login_beside_it() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-asserted", true);

        let result = resolve_identity_with(Some("typed"), &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "typed");
        assert_eq!(result.source, IdentitySource::Asserted);
        assert_eq!(result.verified_login.as_deref(), Some("gh-login"));
        assert!(result.replaced_by.is_none());
        assert!(result.warning.is_none());
    }

    #[test]
    fn asserted_without_a_gh_answer_has_no_verified_login_and_without_by_uses_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-asserted-nogh", true);

        let typed = resolve_identity_with(Some("typed"), &tmp, ASSERTED);
        let git = resolve_identity_with(None, &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        let typed = typed.unwrap();
        assert!(typed.verified_login.is_none());
        assert_eq!(typed.source, IdentitySource::Asserted);
        assert_eq!(git.unwrap().source, IdentitySource::GitConfig);
    }

    #[test]
    fn a_verified_login_that_would_break_a_row_is_refused_beside_asserted_too() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("a|b");
        let tmp = quiet_repo("urzua-io-id-asserted-pipe", true);

        let result = resolve_identity_with(Some("typed"), &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        assert!(result.is_err());
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
        let tmp = Scratch::new("urzua-io-order");
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
    }

    #[test]
    fn last_commit_for_path_tracks_the_most_recent_touch() {
        let tmp = Scratch::new("urzua-io-lastcommit");
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
    }

    #[test]
    fn commit_for_line_is_scoped_to_the_line_not_the_whole_file() {
        let tmp = Scratch::new("urzua-io-blame");
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
    }
}
