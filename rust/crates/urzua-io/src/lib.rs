//! Filesystem and git discovery, plus file reading. The impure half of the
//! split `urzua-core` describes in its own doc comment — see
//! ADR-0005 for why this is a separate crate, and ADR-0006 for why it shells
//! out to `git` rather than linking libgit2.
//!
//! Implements: ADR-0005, ADR-0006

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

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
    #[error("`git {args}` output was not valid UTF-8")]
    InvalidUtf8 { args: String },
}

/// The set of files git considers relevant, and how they were discovered.
/// Never `Silent` — a caller that cannot enumerate git state must know that,
/// rather than receiving an empty list indistinguishable from "no files."
#[derive(Debug, Clone)]
pub struct DiscoveredFiles {
    pub paths: Vec<PathBuf>,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// The union of `git ls-files` (tracked) and staged files.
    GitTracked,
}

/// Discover files git tracks or has staged, at or below `repo_root`. Never a
/// raw filesystem walk: an untracked scratch file must never appear here, or
/// a single stray file changes what every run examines.
pub fn discover_tracked_files(repo_root: &Path) -> Result<DiscoveredFiles, DiscoveryError> {
    let tracked = run_git(repo_root, &["ls-files"])?;
    let staged = run_git(repo_root, &["diff", "--name-only", "--cached"])?;

    let mut paths: Vec<PathBuf> = tracked
        .lines()
        .chain(staged.lines())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    paths.sort();
    paths.dedup();

    Ok(DiscoveredFiles {
        paths,
        source: DiscoverySource::GitTracked,
    })
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

/// RFC-0002 §2's identity tiering, reused by `urzua fix --apply` (ADR-0019):
/// an explicit `--by` wins outright; else `gh api user`; else `git config
/// user.name`. Every applied write needs a real identity, so this returns
/// an error rather than a placeholder when none of the three resolve.
pub fn resolve_identity(explicit: Option<&str>, repo_root: &Path) -> Result<String, String> {
    if let Some(by) = explicit {
        return Ok(by.to_string());
    }

    if let Ok(output) = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
    {
        if output.status.success() {
            let login = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !login.is_empty() {
                return Ok(login);
            }
        }
    }

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

    Err("could not resolve an identity (gh api user, git config user.name both unavailable) -- pass --by explicitly".to_string())
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
}
