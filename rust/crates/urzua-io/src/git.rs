//! The one place this workspace starts a `git` process.
//!
//! Every caller (discovery, blame, the harness, `new`, `doctor`, `fix`) builds a
//! [`GitRun`] and reads the process's [`Output`] or its [`Child`]. The runner
//! owns only the mechanics: the arguments, working directory, environment and
//! stdin it was given, in the order it was given them, and how the pipes are
//! wired. What a spawn failure or a non-zero exit is *called* in a message stays
//! with each caller, because those wordings are part of the tool's output.
//!
//! A test (`git_is_spawned_in_one_place`, in the CLI's integration tests) fails
//! when any other production source file starts git directly.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

/// Why a [`GitRun::output`] call produced no [`Output`]: git did not start, or
/// it started and its result could not be collected. Callers that word the two
/// differently match on it; the rest turn either into the `io::Error`.
#[derive(Debug)]
pub enum RunError {
    Spawn(std::io::Error),
    Wait(std::io::Error),
}

impl RunError {
    /// The underlying I/O error, whichever step failed.
    pub fn into_io(self) -> std::io::Error {
        match self {
            RunError::Spawn(e) | RunError::Wait(e) => e,
        }
    }
}

/// One `git` invocation, described and not yet started.
///
/// Arguments are passed through exactly as added: the runner inserts none of its
/// own (no `--no-optional-locks`, no `-C`, no `--literal-pathspecs`), and it
/// changes no environment variable the caller did not name.
#[derive(Default)]
pub struct GitRun {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    set_env: Vec<(String, String)>,
    unset_env: Vec<String>,
    stdin: Option<Vec<u8>>,
    discard_stderr: bool,
}

impl GitRun {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one argument.
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Appends arguments, in order.
    pub fn args<S: AsRef<OsStr>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }

    /// Appends `-C <dir>` (git's own change-directory option) at this point in
    /// the argument list.
    pub fn dash_c(self, dir: &Path) -> Self {
        self.arg("-C").arg(dir)
    }

    /// Runs git with `dir` as its working directory (the process's own when not
    /// given).
    pub fn cwd(mut self, dir: &Path) -> Self {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    /// Sets an environment variable for this process only.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.set_env.push((key.to_string(), value.to_string()));
        self
    }

    /// Removes an environment variable for this process only.
    pub fn env_remove(mut self, key: &str) -> Self {
        self.unset_env.push(key.to_string());
        self
    }

    /// Feeds `bytes` on git's stdin (it is closed when they are written). Without
    /// this, stdin is `/dev/null`, as for `Command::output`.
    pub fn stdin(mut self, bytes: &[u8]) -> Self {
        self.stdin = Some(bytes.to_vec());
        self
    }

    /// Throws git's stderr away instead of capturing it.
    pub fn discard_stderr(mut self) -> Self {
        self.discard_stderr = true;
        self
    }

    /// The command with its arguments, directory and environment, and nothing
    /// else set. This is the only `Command::new("git")` in production code.
    fn command(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.args(&self.args);
        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
        for (key, value) in &self.set_env {
            cmd.env(key, value);
        }
        for key in &self.unset_env {
            cmd.env_remove(key);
        }
        cmd
    }

    /// Runs git to completion and collects its stdout and stderr. The stdin
    /// bytes are written on their own thread, so a full output pipe cannot block
    /// them. A non-zero exit is an `Ok` output: reading its status is the
    /// caller's job.
    pub fn output(self) -> Result<Output, RunError> {
        let mut cmd = self.command();
        cmd.stdout(Stdio::piped())
            .stderr(if self.discard_stderr {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            });
        let mut child = cmd.spawn().map_err(RunError::Spawn)?;
        let input = self.stdin.as_deref().zip(child.stdin.take());
        std::thread::scope(|scope| {
            if let Some((bytes, mut pipe)) = input {
                scope.spawn(move || {
                    // A broken pipe means git exited early; its status says why.
                    let _ = pipe.write_all(bytes);
                });
            }
            child.wait_with_output().map_err(RunError::Wait)
        })
    }

    /// Starts git with no stdin and the given stdout and stderr, for a caller
    /// that supervises the process itself (a bounded `fetch`).
    pub fn spawn(self, stdout: Stdio, stderr: Stdio) -> std::io::Result<Child> {
        let mut cmd = self.command();
        cmd.stdin(Stdio::null()).stdout(stdout).stderr(stderr);
        cmd.spawn()
    }
}

/// git's stderr, lossily decoded and trimmed: the text callers put after
/// "failed:".
pub fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

/// stdout of `git -C <repo_root> <args>`: `could not run git: <error>` when git
/// does not start, `git <first argument> failed: <stderr>` on a non-zero exit.
pub fn stdout_in(repo_root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = GitRun::new()
        .dash_c(repo_root)
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {}", e.into_io()))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(format!("git {} failed: {}", args[0], stderr_text(&out)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("urzua-git-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn stdin_bytes_reach_git() {
        let out = GitRun::new()
            .args(["hash-object", "--stdin"])
            .stdin(b"hello\n")
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn a_variable_is_set_for_the_one_process() {
        let out = GitRun::new()
            .args(["var", "GIT_AUTHOR_IDENT"])
            .env("GIT_AUTHOR_NAME", "Zed")
            .env("GIT_AUTHOR_EMAIL", "z@example.test")
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).starts_with("Zed <z@example.test>"));
    }

    #[test]
    fn a_non_zero_exit_is_an_output_not_an_error() {
        let dir = scratch("exit");
        let out = GitRun::new()
            .cwd(&dir)
            .env(
                "GIT_CEILING_DIRECTORIES",
                dir.parent().unwrap().to_str().unwrap(),
            )
            .args(["rev-parse", "--show-toplevel"])
            .discard_stderr()
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(out.stderr.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_working_directory_is_a_spawn_error() {
        let missing = std::env::temp_dir().join("urzua-git-no-such-dir-for-the-runner");
        let err = GitRun::new()
            .cwd(&missing)
            .args(["status"])
            .output()
            .unwrap_err();
        assert!(matches!(err, RunError::Spawn(_)), "{err:?}");
    }

    #[test]
    fn stdout_in_words_a_failure_by_the_first_argument_and_the_stderr() {
        let dir = scratch("stdout-in");
        let missing = dir.join("absent");
        let e = stdout_in(&missing, &["rev-parse", "HEAD"]).unwrap_err();
        assert!(e.starts_with("git rev-parse failed: "), "{e}");
        assert!(e.contains("absent"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
