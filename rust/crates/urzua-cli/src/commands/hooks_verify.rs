//! `urzua hooks verify [<hook>] [--run]` (`ADR-71` amendment of 2026-11-16,
//! `MILE-220`): proves by behaviour, not by reading text, that a hook reaches
//! urzua. With `--run` each hook runs once, for real, as git runs it, with a
//! probe directory named in its environment; any urzua the hook starts writes a
//! marker there and then does exactly what it would have done anyway. The probe
//! never changes what urzua does, so there is nothing to spoof: setting it by
//! hand only makes urzua write a marker.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitCode, Stdio};
use std::time::{Duration, Instant};

use urzua_core::config::HookManaged;
use urzua_core::report::FailureKind;
use urzua_core::report::{CouldNotRun, NextAction, Notice, NoticeSeverity, Report};

use super::hooks::{allowed_roots, random_hex, resolve, HOOKS};
use crate::emit;
use crate::session::Workspace;

pub(crate) const PROBE_VAR: &str = "URZUA_HOOKS_PROBE";
const PROBE_PREFIX: &str = "urzua-hooks-verify-";
const PUSH_REMOTE: [&str; 2] = ["urzua-verify", "urzua-verify://not-a-remote"];

/// Called first by every urzua run. When the probe names a directory owned by
/// this user, with mode 0700 and a `token` file, writes a new empty marker in
/// it. Anything else, and any failure, is ignored: the run goes on unchanged.
pub fn record_probe() {
    let Some(value) = std::env::var_os(PROBE_VAR) else {
        return;
    };
    let dir = PathBuf::from(value);
    let Ok(meta) = std::fs::symlink_metadata(&dir) else {
        return;
    };
    if !meta.file_type().is_dir() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if meta.permissions().mode() & 0o777 != 0o700 {
            return;
        }
        // `/proc/self` is owned by this process's user where it exists; a 0700
        // directory of another user could not be written anyway.
        if let Ok(me) = std::fs::metadata("/proc/self") {
            if me.uid() != meta.uid() {
                return;
            }
        }
    }
    if !std::fs::symlink_metadata(dir.join("token")).is_ok_and(|m| m.file_type().is_file()) {
        return;
    }
    let _ = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("reached-{}", std::process::id())));
}

/// The directory git runs hooks from (`core.hooksPath` read as a path, else
/// the repository's hooks directory) and the repository's git directory.
fn dirs(repo_root: &Path) -> Result<(PathBuf, PathBuf), String> {
    let git = |args: &[&str]| -> Result<String, String> {
        let out = urzua_io::git::GitRun::new()
            .args(args)
            .cwd(repo_root)
            .output()
            .map_err(|e| format!("could not run git: {}", e.into_io()))?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let abs = |p: String| {
        let p = PathBuf::from(p);
        if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        }
    };
    let configured = git(&["config", "--type=path", "--get", "core.hooksPath"])?;
    let hooks = if configured.is_empty() {
        git(&["rev-parse", "--git-path", "hooks"])?
    } else {
        configured
    };
    Ok((abs(hooks), abs(git(&["rev-parse", "--absolute-git-dir"])?)))
}

#[derive(serde::Serialize)]
struct HookRun {
    hook: String,
    path: String,
    managed: &'static str,
    /// Whether an urzua the hook started was reached; `null` without `--run`.
    reached: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    timed_out: bool,
    /// How the hook was run and with what arguments.
    #[serde(skip_serializing_if = "String::is_empty")]
    invocation: String,
    /// urzua's own lines from the hook's output: the check's outcome as the
    /// hook reported it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    urzua_output: Vec<String>,
    note: String,
}

#[derive(serde::Serialize)]
struct VerifyReport {
    status: &'static str,
    action: &'static str,
    run: bool,
    hooks: Vec<HookRun>,
    /// Set with `status` `not-run`: why the run refused (the published agent CLI contract).
    #[serde(
        rename = "error_kinds",
        skip_serializing_if = "Option::is_none",
        serialize_with = "urzua_core::report::optional_error_kinds"
    )]
    error_kind: Option<FailureKind>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    problems: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    #[serde(skip)]
    exit: u8,
}

impl Report for VerifyReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit)
    }
    fn next_action(&self) -> Option<NextAction> {
        None
    }
}

impl VerifyReport {
    fn notice(&mut self, severity: NoticeSeverity, message: String) {
        self.notices.push(Notice {
            severity,
            subject: "hooks".to_string(),
            message,
            detail: None,
        });
    }
}

fn managed_word(m: HookManaged) -> &'static str {
    match m {
        HookManaged::Block => "block",
        HookManaged::Declared => "declared",
    }
}

pub fn run_verify(
    config: Option<PathBuf>,
    hook: Option<String>,
    run: bool,
    allow_outside: bool,
) -> ExitCode {
    let ws = match Workspace::open(&[PathBuf::from(".")], config) {
        Ok(ws) => ws,
        Err(e) => return emit(&CouldNotRun::from_failure(e)),
    };
    let hooks: Vec<&str> = match &hook {
        Some(h) if HOOKS.contains(&h.as_str()) => vec![h.as_str()],
        Some(h) => {
            return emit(&CouldNotRun::new(
                FailureKind::Usage,
                format!(
                    "'{h}' is not a hook urzua verifies; the hooks are {}",
                    HOOKS.join(", ")
                ),
            ))
        }
        None => HOOKS.to_vec(),
    };
    let (hooks_dir, git_dir) = match dirs(&ws.repo_root) {
        Ok(d) => d,
        Err(m) => return emit(&CouldNotRun::new(FailureKind::Environment, m)),
    };
    let mut report = VerifyReport {
        status: "ok",
        action: "verify",
        run,
        hooks: Vec::new(),
        problems: Vec::new(),
        error_kind: None,
        notices: Vec::new(),
        exit: 0,
    };
    let real = resolve(&hooks_dir);
    let outside = !allowed_roots(&ws.repo_root)
        .iter()
        .any(|r| real.starts_with(r));
    if !run {
        explain(&mut report, &ws, &hooks, &hooks_dir);
    } else if outside && !allow_outside {
        report.status = "not-run";
        report.error_kind = Some(FailureKind::Environment);
        report.exit = 2;
        report.problems.push(format!(
            "git runs this repository's hooks from {}, outside the repository and its git directory; nothing was run -- pass --allow-outside to run hooks from there",
            real.display()
        ));
    } else {
        if outside {
            report.notice(
                NoticeSeverity::Warning,
                format!(
                    "hooks outside the repository were run, from {}, because --allow-outside was given; that is where git runs this repository's hooks from",
                    real.display()
                ),
            );
        }
        if let Err(m) = run_all(&mut report, &ws, &hooks, &hooks_dir, &git_dir) {
            return emit(&CouldNotRun::new(FailureKind::Environment, m));
        }
    }
    emit(&report)
}

fn explain(report: &mut VerifyReport, ws: &Workspace, hooks: &[&str], dir: &Path) {
    report.status = "explained";
    for hook in hooks {
        report.hooks.push(HookRun {
            hook: hook.to_string(),
            path: dir.join(hook).display().to_string(),
            managed: managed_word(ws.config.hooks.managed(hook)),
            reached: None,
            exit_code: None,
            timed_out: false,
            invocation: String::new(),
            urzua_output: Vec::new(),
            note: format!(
                "with --run: runs {} once from {}, as git would, and reports whether it reaches urzua",
                dir.join(hook).display(),
                ws.repo_root.display()
            ),
        });
    }
    report.notice(
        NoticeSeverity::Info,
        "nothing was run. `urzua hooks verify --run` executes the repository's hooks for real: urzua's check and everything else in them (a formatter rewrites files, other checks run); a person or agent should read the hooks before running it".to_string(),
    );
}

/// A probe directory only this run made: fresh, owner-only, with a random
/// token, removed when dropped.
struct ProbeDir(PathBuf);

impl Drop for ProbeDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn pid_alive(pid: u32) -> bool {
    if Path::new("/proc/self").exists() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Removes probe directories left by a run that was stopped by a signal
/// before it could remove its own: their process is gone.
fn remove_stale_probes(tmp: &Path) {
    let Ok(entries) = std::fs::read_dir(tmp) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix(PROBE_PREFIX)
            .and_then(|r| r.split('-').next())
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() && !pid_alive(pid) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn probe_dir() -> Result<ProbeDir, String> {
    let tmp = std::env::temp_dir();
    remove_stale_probes(&tmp);
    let dir = tmp.join(format!(
        "{PROBE_PREFIX}{}-{}",
        std::process::id(),
        random_hex()
    ));
    std::fs::create_dir(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let probe = ProbeDir(dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&probe.0, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("could not restrict {}: {e}", probe.0.display()))?;
    }
    std::fs::write(probe.0.join("token"), random_hex())
        .map_err(|e| format!("could not write the probe token: {e}"))?;
    Ok(probe)
}

fn run_all(
    report: &mut VerifyReport,
    ws: &Workspace,
    hooks: &[&str],
    hooks_dir: &Path,
    git_dir: &Path,
) -> Result<(), String> {
    let probe = probe_dir()?;
    let with_git = git_has_hook_run(&ws.repo_root);
    let timeout = Duration::from_secs(ws.config.hooks.verify_timeout_seconds);
    for hook in hooks {
        let managed = ws.config.hooks.managed(hook);
        let target = Target {
            root: &ws.repo_root,
            hook_path: hooks_dir.join(hook),
            git_dir,
            hook,
        };
        let run = run_one(&probe.0, &target, managed, with_git, timeout);
        if run.reached != Some(true) || run.timed_out {
            report.status = "unreached";
            report.exit = 1;
        }
        report.hooks.push(run);
    }
    report.notice(
        NoticeSeverity::Warning,
        "each hook ran for real: urzua's check ran with its real result, and everything else in the hook happened (formatters, other checks, their changes to files); a hook that was reached but failed has real findings or a failing command of its own".to_string(),
    );
    Ok(())
}

/// Whether the installed git has `git hook run`, which runs a hook exactly as
/// git does (its environment, `core.hooksPath`, a hook with no shebang).
fn git_has_hook_run(root: &Path) -> bool {
    urzua_io::git::GitRun::new()
        .args(["hook", "-h"])
        .cwd(root)
        .output()
        .is_ok_and(|o| {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            text.contains("git hook run")
        })
}

struct Target<'a> {
    root: &'a Path,
    hook_path: PathBuf,
    git_dir: &'a Path,
    hook: &'a str,
}

/// Starts the hook as git would, in its own process group, with no input.
fn start(target: &Target, probe: &Path, with_git: bool) -> std::io::Result<(Child, String)> {
    let push = target.hook == "pre-push";
    if with_git {
        let mut run = urzua_io::git::GitRun::new()
            .args(["hook", "run", target.hook])
            .cwd(target.root)
            .env(PROBE_VAR, &probe.to_string_lossy())
            .own_process_group();
        let mut shown = format!("git hook run {}", target.hook);
        if push {
            run = run.arg("--").args(PUSH_REMOTE);
            shown.push_str(&format!(" -- {}", PUSH_REMOTE.join(" ")));
        }
        return Ok((run.spawn(Stdio::piped(), Stdio::piped())?, shown));
    }
    let hook_path = &target.hook_path;
    let shebang = std::fs::read(hook_path).is_ok_and(|b| b.starts_with(b"#!"));
    let mut cmd = if shebang {
        std::process::Command::new(hook_path)
    } else {
        let mut sh = std::process::Command::new("sh");
        sh.arg(hook_path);
        sh
    };
    if push {
        cmd.args(PUSH_REMOTE);
    }
    cmd.env("GIT_DIR", target.git_dir);
    if target.hook == "pre-commit" {
        cmd.env("GIT_INDEX_FILE", target.git_dir.join("index"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let shown = format!(
        "{}{}{}",
        if shebang { "" } else { "sh " },
        hook_path.display(),
        if push {
            format!(" {}", PUSH_REMOTE.join(" "))
        } else {
            String::new()
        }
    );
    let child = cmd
        .current_dir(target.root)
        .env(PROBE_VAR, probe)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok((child, shown))
}

/// Waits for `child` up to `timeout`; past it, stops its whole process group.
/// (exit code, timed out, stderr).
fn supervise(mut child: Child, timeout: Duration) -> (Option<i32>, bool, String) {
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut text);
            }
            text
        })
    };
    let out = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let err = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > timeout => {
                timed_out = true;
                let _ = std::process::Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .stderr(Stdio::null())
                    .status();
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break None,
        }
    };
    let _ = out.join();
    let stderr = err.join().unwrap_or_default();
    (status.and_then(|s| s.code()), timed_out, stderr)
}

fn run_one(
    probe: &Path,
    target: &Target,
    managed: HookManaged,
    with_git: bool,
    timeout: Duration,
) -> HookRun {
    let hook = target.hook;
    let mut result = HookRun {
        hook: hook.to_string(),
        path: target.hook_path.display().to_string(),
        managed: managed_word(managed),
        reached: Some(false),
        exit_code: None,
        timed_out: false,
        invocation: String::new(),
        urzua_output: Vec::new(),
        note: String::new(),
    };
    clear_markers(probe);
    if !target.hook_path.is_file() {
        result.note = format!(
            "there is no hook at {}, so git runs nothing",
            target.hook_path.display()
        );
        return result;
    }
    let (child, shown) = match start(target, probe, with_git) {
        Ok(started) => started,
        Err(e) => {
            result.note = format!("the hook could not be started: {e}");
            return result;
        }
    };
    result.invocation = format!("{shown}, from {}, with empty stdin", target.root.display());
    let (code, timed_out, stderr) = supervise(child, timeout);
    let reached = has_marker(probe);
    result.exit_code = code;
    result.timed_out = timed_out;
    result.reached = Some(reached);
    result.urzua_output = stderr
        .lines()
        .filter(|l| l.starts_with(&format!("{hook}:")) || l.starts_with("  "))
        .take(40)
        .map(str::to_string)
        .collect();
    result.note = match (reached, timed_out) {
        (_, true) => format!(
            "the hook ran longer than hooks.verify_timeout_seconds ({}s) and was stopped with everything it started",
            timeout.as_secs()
        ),
        (true, _) => "the hook reached urzua and ran for real, urzua's check included".to_string(),
        (false, _) => format!(
            "the hook ran (exit {}) without reaching urzua",
            code.map_or("signal".to_string(), |c| c.to_string())
        ),
    };
    result
}

fn clear_markers(probe: &Path) {
    if let Ok(entries) = std::fs::read_dir(probe) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with("reached-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

fn has_marker(probe: &Path) -> bool {
    std::fs::read_dir(probe).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("reached-"))
    })
}
