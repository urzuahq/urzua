//! The impure half of `urzua new`'s open-pull-request check (`ADR-93`, `MILE-89`):
//! the fetch of pull-request head refs into `refs/urzua/pr/<segment>/*`, the
//! numbering lock, and the scan of sibling worktrees. All plain `git`; nothing
//! here uses `gh` or an API.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use urzua_core::config::NewDecl;

/// The namespace the fetched refs live under; no default refspec, `git branch`
/// or `git log` shows it.
pub(crate) const PR_NAMESPACE: &str = "refs/urzua/pr";

/// Why a fetch did not happen.
pub(crate) enum FetchFailure {
    /// The remote is missing, or the fetch failed.
    Offline(String),
    /// The fetch was killed at the deadline.
    Timeout,
}

impl FetchFailure {
    pub(crate) fn reason(&self) -> &'static str {
        match self {
            FetchFailure::Offline(_) => "offline",
            FetchFailure::Timeout => "timeout",
        }
    }
}

/// `git fetch --no-tags --quiet <remote> '+<pattern>:refs/urzua/pr/<segment>/*'`
/// for each configured pattern (`ADR-93` clause 2). The segment is the remote's
/// name when one pattern is configured and the pattern's index when several are,
/// so two patterns that both yield `123` cannot overwrite each other. A bounded
/// wait kills a hung fetch. Git's own credential helper does any
/// authentication; a prompt is never shown.
///
/// `--filter=blob:none` is not used: it makes a clone a partial clone (it sets the
/// remote's `promisor` configuration), which a numbering command must not do to a
/// repository, and only trees are read either way.
pub(crate) fn fetch_pull_refs(repo_root: &Path, opts: &NewDecl) -> Result<(), FetchFailure> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(opts.fetch_timeout_seconds));
    let single = opts.pull_ref_patterns.len() == 1;
    for (index, pattern) in opts.pull_ref_patterns.iter().enumerate() {
        let segment = if single {
            opts.remote.clone()
        } else {
            index.to_string()
        };
        let refspec = format!("+{pattern}:{PR_NAMESPACE}/{segment}/*");
        let mut child = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(["fetch", "--no-tags", "--quiet"])
            .arg(&opts.remote)
            .arg(&refspec)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| FetchFailure::Offline(format!("could not run git: {e}")))?;
        let stderr = child.stderr.take();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut s) = stderr {
                let _ = std::io::Read::read_to_string(&mut s, &mut text);
            }
            let _ = tx.send(text);
        });
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    // The reader may be held open by a helper that outlived git;
                    // it is left to finish on its own.
                    return Err(FetchFailure::Timeout);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(e) => {
                    return Err(FetchFailure::Offline(format!(
                        "could not wait for git: {e}"
                    )))
                }
            }
        };
        if !status.success() {
            let text = rx.recv_timeout(Duration::from_secs(1)).unwrap_or_default();
            let line = text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("git fetch failed");
            return Err(FetchFailure::Offline(line.trim().to_string()));
        }
    }
    Ok(())
}

/// How many refs sit under `refs/urzua/pr`, fetched now or by an earlier run.
pub(crate) fn count_pull_refs(repo_root: &Path) -> usize {
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["for-each-ref", "--format=%(refname)", PR_NAMESPACE])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or(0, |o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count()
        })
}

/// The lock that serializes `urzua new` (`ADR-93` clause 4): a `create_new` file in
/// the git common directory, so linked worktrees share it, holding the owner's
/// pid. Removed when dropped.
pub(crate) struct NumberingLock {
    path: PathBuf,
}

impl Drop for NumberingLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub(crate) enum LockOutcome {
    Held(NumberingLock),
    /// The lock could not be taken for a reason that is not contention (no git
    /// common directory, an unwritable one): numbering goes on without it and
    /// says so.
    Unavailable(String),
    /// Another run held it for the whole wait.
    TimedOut(String),
}

fn git_common_dir(repo_root: &Path) -> Result<PathBuf, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let dir = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Ok(if dir.is_absolute() {
        dir
    } else {
        repo_root.join(dir)
    })
}

/// Whether a process with this pid exists. A pid the platform cannot be asked
/// about counts as alive: the lock then times out instead of being stolen.
fn process_alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let proc = Path::new("/proc");
    if proc.join("self").exists() {
        return proc.join(pid.to_string()).exists();
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .ok()
            .is_none_or(|s| s.success())
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Removes `path` if it still names the dead `holder` (or nobody), under a guard file
/// that only one waiter can hold at a time. A waiter that cannot take the guard
/// leaves the lock alone and retries.
fn remove_if_stale(path: &Path, holder: Option<u32>) {
    let guard = path.with_extension("lock.reap");
    // A guard left by a crashed reaper is old; the work under it takes microseconds.
    let old = std::fs::metadata(&guard)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age > Duration::from_secs(5));
    if old {
        let _ = std::fs::remove_file(&guard);
    }
    let Ok(_file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&guard)
    else {
        return;
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let unchanged = text.trim().parse::<u32>().ok() == holder;
    if unchanged && holder.is_none_or(|pid| !process_alive(pid)) {
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_file(&guard);
}

pub(crate) fn acquire_lock(repo_root: &Path, timeout_seconds: u32) -> LockOutcome {
    let dir = match git_common_dir(repo_root) {
        Ok(d) => d,
        Err(e) => return LockOutcome::Unavailable(format!("no git common directory ({e})")),
    };
    let path = dir.join("urzua-new.lock");
    let started = Instant::now();
    let deadline = Duration::from_secs(u64::from(timeout_seconds));
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let _ = std::io::Write::write_all(
                    &mut file,
                    format!("{}\n", std::process::id()).as_bytes(),
                );
                return LockOutcome::Held(NumberingLock { path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let holder = text.trim().parse::<u32>().ok();
                let young_and_empty = text.trim().is_empty()
                    && std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age < Duration::from_secs(5));
                let stale = match holder {
                    Some(pid) => !process_alive(pid),
                    None => !young_and_empty,
                };
                if stale {
                    // A lock whose owner is gone is removed (`ADR-93` clause 4). Two
                    // waiters must not both remove it: the slower one would delete
                    // the lock the faster one had just taken. So removal is guarded
                    // by a second `create_new` file, and the lock is read again
                    // under the guard; only a lock still stale is removed.
                    remove_if_stale(&path, holder);
                    continue;
                }
                if started.elapsed() >= deadline {
                    return LockOutcome::TimedOut(format!(
                        "another `urzua new` holds {} (pid {}); waited {timeout_seconds}s. If no such run exists, remove the file",
                        path.display(),
                        holder.map_or("unknown".to_string(), |p| p.to_string())
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return LockOutcome::Unavailable(format!(
                    "could not create {}: {e}",
                    path.display()
                ))
            }
        }
    }
}

/// The other worktrees of this repository, from `git worktree list --porcelain`:
/// each path, in listing order. The worktree at `repo_root` and a bare entry are
/// left out.
pub(crate) fn sibling_worktrees(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let here = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let mut siblings = Vec::new();
    let mut current: Option<PathBuf> = None;
    let mut bare = false;
    let mut flush = |current: &mut Option<PathBuf>, bare: &mut bool| {
        if let Some(path) = current.take() {
            let same = path.canonicalize().unwrap_or_else(|_| path.clone()) == here;
            if !*bare && !same {
                siblings.push(path);
            }
        }
        *bare = false;
    };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            flush(&mut current, &mut bare);
            current = Some(PathBuf::from(path));
        } else if line == "bare" {
            bare = true;
        }
    }
    flush(&mut current, &mut bare);
    Ok(siblings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_whose_owner_is_gone_is_taken_and_leaves_no_guard_behind() {
        let dir = std::env::temp_dir().join(format!("urzua-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(&dir)
            .arg("init")
            .arg("-q")
            .status()
            .unwrap()
            .success());
        let mut gone = Command::new("true").spawn().unwrap();
        gone.wait().unwrap();
        let lock = dir.join(".git/urzua-new.lock");
        std::fs::write(&lock, format!("{}\n", gone.id())).unwrap();
        match acquire_lock(&dir, 2) {
            LockOutcome::Held(held) => {
                assert_eq!(
                    std::fs::read_to_string(&lock).unwrap().trim(),
                    std::process::id().to_string()
                );
                drop(held);
            }
            _ => panic!("a lock held by a dead pid must be taken"),
        }
        assert!(!lock.exists());
        assert!(!dir.join(".git/urzua-new.lock.reap").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
