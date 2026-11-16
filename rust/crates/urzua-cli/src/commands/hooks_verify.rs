//! `urzua hooks verify [<hook>] [--run]` (`ADR-71` amendment of 2026-11-16,
//! `MILE-220`): proves by behaviour, not by reading text, that a hook reaches
//! urzua. With `--run` each hook runs once as git would run it, with a probe in
//! its environment; any urzua the hook starts records that it was reached and
//! exits 0 without checking. Everything else in the hook runs for real.
//!
//! The probe cannot turn a real check off: urzua honours it only when the
//! process that set it is a running `urzua hooks verify --run` among its own
//! ancestors, and the probe's directory holds the random token that run made.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::HookManaged;
use urzua_core::report::{CouldNotRun, NextAction, Notice, NoticeSeverity, Report};

use super::hooks::{random_hex, HOOKS, INSTALL};
use crate::emit;
use crate::session::Workspace;

pub(crate) const PROBE_VAR: &str = "URZUA_HOOKS_PROBE";

/// Called first by every urzua run: when a valid probe is set, records that
/// urzua was reached and returns the exit to end the run with.
pub fn probe_if_requested() -> Option<ExitCode> {
    let value = std::env::var(PROBE_VAR).ok()?;
    match valid_probe(&value) {
        Some(dir) => {
            use std::io::Write as _;
            let recorded = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("reached"))
                .and_then(|mut f| f.write_all(b"reached\n"));
            Some(if recorded.is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        None => {
            eprintln!("urzua: ignoring {PROBE_VAR}: it was not set by a running `urzua hooks verify --run`");
            None
        }
    }
}

/// `<pid>:<token>:<dir>`, honoured only when `pid` is an ancestor of this
/// process running `urzua hooks verify --run` and `<dir>/token` holds `token`.
fn valid_probe(value: &str) -> Option<PathBuf> {
    let mut parts = value.splitn(3, ':');
    let pid: u32 = parts.next()?.parse().ok()?;
    let token = parts.next()?;
    let dir = PathBuf::from(parts.next()?);
    if token.len() < 32 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if std::fs::read_to_string(dir.join("token")).ok()?.trim() != token {
        return None;
    }
    (is_ancestor(pid) && is_verify_run(pid)).then_some(dir)
}

fn parent_of(pid: u32) -> Option<u32> {
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        // The command name may hold spaces and parentheses: read after the last `)`.
        let rest = &stat[stat.rfind(')')? + 1..];
        return rest.split_whitespace().nth(1)?.parse().ok();
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn is_ancestor(pid: u32) -> bool {
    let mut current = std::process::id();
    for _ in 0..64 {
        match parent_of(current) {
            Some(p) if p == pid => return true,
            Some(p) if p > 1 => current = p,
            _ => return false,
        }
    }
    false
}

fn is_verify_run(pid: u32) -> bool {
    let args: Vec<String> = match std::fs::read(format!("/proc/{pid}/cmdline")) {
        Ok(raw) => raw
            .split(|b| *b == 0)
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect(),
        Err(_) => std::process::Command::new("ps")
            .args(["-o", "command=", "-p", &pid.to_string()])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    };
    let named_urzua = args
        .first()
        .is_some_and(|a| a.rsplit('/').next().is_some_and(|n| n.starts_with("urzua")));
    let at = args.iter().position(|a| a == "hooks");
    named_urzua
        && at.is_some_and(|i| args.get(i + 1).is_some_and(|a| a == "verify"))
        && args.iter().any(|a| a == "--run")
}

/// The directory git runs hooks from: `core.hooksPath`, else the repository's
/// own hooks directory.
fn run_dir(repo_root: &Path) -> Result<PathBuf, String> {
    let git = |args: &[&str]| {
        urzua_io::git::GitRun::new()
            .args(args)
            .cwd(repo_root)
            .output()
            .map_err(|e| format!("could not run git: {}", e.into_io()))
    };
    let configured = git(&["config", "--get", "core.hooksPath"])?;
    let value = String::from_utf8_lossy(&configured.stdout)
        .trim()
        .to_string();
    let raw = if configured.status.success() && !value.is_empty() {
        value
    } else {
        let out = git(&["rev-parse", "--git-path", "hooks"])?;
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let p = PathBuf::from(raw);
    Ok(if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    })
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
    note: String,
}

#[derive(serde::Serialize)]
struct VerifyReport {
    status: &'static str,
    action: &'static str,
    run: bool,
    hooks: Vec<HookRun>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    #[serde(skip)]
    next: Option<NextAction>,
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
        self.next.clone()
    }
}

fn managed_word(m: HookManaged) -> &'static str {
    match m {
        HookManaged::Block => "block",
        HookManaged::Declared => "declared",
    }
}

pub fn run_verify(config: Option<PathBuf>, hook: Option<String>, run: bool) -> ExitCode {
    let ws = match Workspace::open(&[PathBuf::from(".")], config) {
        Ok(ws) => ws,
        Err(e) => return emit(&CouldNotRun::from(e.message)),
    };
    let hooks: Vec<&str> = match &hook {
        Some(h) if HOOKS.contains(&h.as_str()) => vec![h.as_str()],
        Some(h) => {
            return emit(&CouldNotRun::from(format!(
                "'{h}' is not a hook urzua verifies; the hooks are {}",
                HOOKS.join(", ")
            )))
        }
        None => HOOKS.to_vec(),
    };
    let dir = match run_dir(&ws.repo_root) {
        Ok(d) => d,
        Err(m) => return emit(&CouldNotRun::from(m)),
    };
    let mut report = VerifyReport {
        status: "ok",
        action: "verify",
        run,
        hooks: Vec::new(),
        notices: Vec::new(),
        next: None,
        exit: 0,
    };
    if !run {
        explain(&mut report, &ws, &hooks, &dir);
    } else if let Err(m) = run_all(&mut report, &ws, &hooks, &dir) {
        return emit(&CouldNotRun::from(m));
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
            note: format!(
                "with --run: runs {} once from {}, as git would, and reports whether it reaches urzua",
                dir.join(hook).display(),
                ws.repo_root.display()
            ),
        });
    }
    report.notices.push(Notice {
        severity: NoticeSeverity::Info,
        subject: "hooks".to_string(),
        message: "nothing was run. With --run each hook runs for real: everything in it other than urzua's check happens (a formatter rewrites files, other checks run); any urzua it starts records that it was reached and exits 0 without checking".to_string(),
        detail: None,
    });
    report.next = Some(NextAction::runs_as_printed(
        "urzua hooks verify --run",
        "runs each hook once to prove it reaches urzua",
    ));
}

/// A probe directory only this run can name: created fresh, owner-only, with
/// a random token the honouring urzua checks.
fn probe_dir() -> Result<(PathBuf, String), String> {
    let token = random_hex();
    let dir = std::env::temp_dir().join(format!("urzua-hooks-verify-{}", random_hex()));
    std::fs::create_dir(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    std::fs::write(dir.join("token"), &token)
        .map_err(|e| format!("could not write the probe token: {e}"))?;
    Ok((dir, token))
}

fn run_all(
    report: &mut VerifyReport,
    ws: &Workspace,
    hooks: &[&str],
    dir: &Path,
) -> Result<(), String> {
    let (probe, token) = probe_dir()?;
    let value = format!("{}:{token}:{}", std::process::id(), probe.display());
    for hook in hooks {
        let managed = ws.config.hooks.managed(hook);
        let run = run_one(
            &probe,
            &value,
            &ws.repo_root,
            &dir.join(hook),
            hook,
            managed,
        );
        if run.reached != Some(true) {
            report.status = "unreached";
            report.exit = 1;
            if managed == HookManaged::Block && report.next.is_none() {
                report.next = Some(NextAction::runs_as_printed(
                    INSTALL,
                    "adds the managed block, which runs urzua first",
                ));
            }
        }
        report.hooks.push(run);
    }
    let _ = std::fs::remove_dir_all(&probe);
    report.notices.push(Notice {
        severity: NoticeSeverity::Warning,
        subject: "hooks".to_string(),
        message: "each hook ran for real: everything in it other than urzua's check happened (formatters, other checks, their changes to files); urzua itself only recorded that it was reached".to_string(),
        detail: None,
    });
    Ok(())
}

fn run_one(
    probe: &Path,
    value: &str,
    root: &Path,
    hook_path: &Path,
    hook: &str,
    managed: HookManaged,
) -> HookRun {
    let mut result = HookRun {
        hook: hook.to_string(),
        path: hook_path.display().to_string(),
        managed: managed_word(managed),
        reached: Some(false),
        exit_code: None,
        note: String::new(),
    };
    let _ = std::fs::remove_file(probe.join("reached"));
    if !hook_path.is_file() {
        result.note = format!(
            "there is no hook at {}, so git runs nothing",
            hook_path.display()
        );
        return result;
    }
    let mut cmd = std::process::Command::new(hook_path);
    if hook == "pre-push" {
        cmd.args(["origin", "origin"]);
    }
    let out = cmd
        .current_dir(root)
        .env(PROBE_VAR, value)
        .stdin(std::process::Stdio::null())
        .output();
    match out {
        Err(e) => result.note = format!("git could not run it either: {e}"),
        Ok(o) => {
            result.exit_code = o.status.code();
            let reached = probe.join("reached").is_file();
            result.reached = Some(reached);
            result.note = if reached {
                "the hook reached urzua; the rest of it ran for real".to_string()
            } else {
                format!(
                    "the hook ran (exit {}) without reaching urzua",
                    o.status
                        .code()
                        .map_or("signal".to_string(), |c| c.to_string())
                )
            };
        }
    }
    result
}
