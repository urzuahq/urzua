//! `urzua doctor` (SPEC-15): reports on the tool's own configuration and
//! invocation health, not on record content -- `check` validates records;
//! whether *itself* is correctly invoked is a different question with a
//! different failure mode. An unrecognized config key is an error here, not
//! a warning: the alternative is a typo silently disabling a rule.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, Notice, Report};

use crate::discovery::{find_repo_root, load_config};
use crate::emit;

#[derive(serde::Serialize)]
struct DoctorCheck {
    check: String,
    status: DoctorStatus,
    message: String,
    /// The git hook this check is about (`hook-wired` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    layer: Option<String>,
    /// The layer's wiring state, machine-readable (`hook-wired` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<HookState>,
}

/// How one local hook layer is wired (ADR-71: wiring is verifiable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum HookState {
    NotInstalled,
    InstalledRunningUrzua,
    InstalledNotRunningUrzua,
    /// Present but without the execute bit, which git silently ignores.
    InstalledNotExecutable,
}

/// The local layers ADR-71 names; the editor layer is ADR-69's.
const HOOK_LAYERS: [&str; 2] = ["pre-commit", "pre-push"];

/// The directory git runs hooks from: `core.hooksPath` (relative paths are
/// relative to the working tree root) or the repository's own hooks dir.
fn hooks_dir(repo_root: &std::path::Path) -> Result<PathBuf, String> {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(repo_root)
            .output()
            .map_err(|e| format!("could not run git: {e}"))
    };
    let configured = git(&["config", "--get", "core.hooksPath"])?;
    let value = String::from_utf8_lossy(&configured.stdout)
        .trim()
        .to_string();
    if configured.status.success() && !value.is_empty() {
        let p = PathBuf::from(&value);
        return Ok(if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        });
    }
    let out = git(&["rev-parse", "--git-path", "hooks"])?;
    if !out.status.success() {
        return Err(format!(
            "git could not locate the hooks directory: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let p = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Ok(if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    })
}

fn hook_state(path: &std::path::Path) -> Result<HookState, String> {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HookState::NotInstalled),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    if !meta.is_file() {
        return Ok(HookState::NotInstalled);
    }
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    };
    // Windows has no execute bit; git runs the hook through its shell.
    #[cfg(not(unix))]
    let executable = true;
    if !executable {
        return Ok(HookState::InstalledNotExecutable);
    }
    let content =
        std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let content = String::from_utf8_lossy(&content);
    Ok(if invokes_urzua(&content, None, false) {
        HookState::InstalledRunningUrzua
    } else {
        HookState::InstalledNotRunningUrzua
    })
}

/// Whether `content` runs urzua: a command position, not a word in a comment
/// or an `echo`. A line's `#` comment is dropped, and a line that only prints
/// (`echo`, `printf`) runs nothing. The executable is any path ending in
/// `urzua`, followed by a subcommand (`subcommand` when given). With
/// `accept_action`, a `uses:` line naming urzua's reusable gate action counts
/// too. The engine knows no repository's wrapper script or make target, so a
/// repository that wraps the call is seen only through the call itself.
fn invokes_urzua(content: &str, subcommand: Option<&str>, accept_action: bool) -> bool {
    // Variables assigned a path that ends in `urzua` (`bin="$root/.../urzua"`),
    // so `"$bin" check` is seen as the call it is.
    let mut binaries: Vec<String> = Vec::new();
    for line in content.lines() {
        let l = line.trim_start().trim_start_matches("export ");
        if let Some((name, value)) = l.split_once('=') {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && value.rsplit('/').next() == Some("urzua")
            {
                binaries.push(name.to_string());
            }
        }
    }
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let code = match line.find(" #") {
            Some(at) => &line[..at],
            None => line,
        };
        let tokens: Vec<&str> = code
            .split(|c: char| c.is_whitespace() || "\"'();&|`".contains(c))
            .filter(|t| !t.is_empty())
            .collect();
        if matches!(tokens.first(), Some(&"echo") | Some(&"printf")) {
            continue;
        }
        if accept_action && tokens.iter().any(|t| t.contains("actions/urzua-gate")) {
            return true;
        }
        for pair in tokens.windows(2) {
            let executable = pair[0].rsplit('/').next() == Some("urzua")
                || binaries
                    .iter()
                    .any(|b| pair[0] == format!("${b}") || pair[0] == format!("${{{b}}}"));
            let wanted = match subcommand {
                Some(sub) => pair[1] == sub,
                None => pair[1].starts_with(|c: char| c.is_ascii_alphabetic()),
            };
            if executable && wanted {
                return true;
            }
        }
    }
    false
}

/// What is wrong with the local git hook layers, as plain messages: the
/// session-start check of the harness reads the same wiring `doctor` does.
pub(crate) fn git_hook_problems(repo_root: &std::path::Path) -> Vec<String> {
    hook_checks(repo_root)
        .into_iter()
        .filter(|c| c.status != DoctorStatus::Ok)
        .map(|c| c.message)
        .collect()
}

fn hook_checks(repo_root: &std::path::Path) -> Vec<DoctorCheck> {
    let dir = match hooks_dir(repo_root) {
        Ok(d) => d,
        Err(message) => {
            return HOOK_LAYERS
                .iter()
                .map(|layer| DoctorCheck {
                    check: "hook-wired".to_string(),
                    status: DoctorStatus::Error,
                    message: format!("{layer}: {message} -- cannot tell whether the hook is wired"),
                    layer: Some((*layer).to_string()),
                    state: None,
                })
                .collect();
        }
    };
    HOOK_LAYERS
        .iter()
        .map(|layer| {
            let path = dir.join(layer);
            let (status, state, message) = match hook_state(&path) {
                Err(message) => (DoctorStatus::Error, None, message),
                Ok(state) => {
                    let (status, message) = match state {
                        HookState::InstalledRunningUrzua => (
                            DoctorStatus::Ok,
                            format!("{} runs urzua", path.display()),
                        ),
                        HookState::NotInstalled => (
                            DoctorStatus::Warn,
                            format!(
                                "no {layer} hook at {} -- add one that runs `urzua check`; CI remains the guarantee (ADR-71)",
                                path.display()
                            ),
                        ),
                        HookState::InstalledNotRunningUrzua => (
                            DoctorStatus::Warn,
                            format!("{} does not run urzua", path.display()),
                        ),
                        HookState::InstalledNotExecutable => (
                            DoctorStatus::Warn,
                            format!("{} is not executable, so git ignores it", path.display()),
                        ),
                    };
                    (status, Some(state), message)
                }
            };
            DoctorCheck {
                check: "hook-wired".to_string(),
                status,
                message,
                layer: Some((*layer).to_string()),
                state,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum DoctorStatus {
    Ok,
    Warn,
    Error,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    status: DoctorStatus,
    checks: Vec<DoctorCheck>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    /// Not serialized -- `run` has three real exit codes (2 = config
    /// missing, 1 = config parse error or any check `Error`, 0 = otherwise),
    /// but `status` alone can't distinguish the first two (both are
    /// `DoctorStatus::Error`). Set explicitly by whichever branch builds the
    /// report, so `exit_code()` never has to (mis)derive it from `status`.
    #[serde(skip)]
    exit_code: u8,
}

impl Report for DoctorReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// Emits one report, JSON, unconditionally (ADR-23) -- BUG-4 was `doctor`
/// printing `[OK]`/`[WARN]`/`[ERROR]` lines instead, the one command that
/// hadn't caught up with every other command's output contract.
pub fn run(config: Option<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut checks = Vec::new();
    // The global `--config`, as every other command reads it: reporting on the
    // default file instead would be a silent no-op.
    let config_path = config.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));

    if !config_path.exists() {
        checks.push(DoctorCheck {
            check: "config-exists".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Error,
            message: format!(
                "{} does not exist -- run `urzua init` to adopt this corpus",
                config_path.display()
            ),
        });
        return emit(&DoctorReport {
            status: DoctorStatus::Error,
            checks,
            notices: Vec::new(),
            exit_code: 2,
        });
    }
    checks.push(DoctorCheck {
        check: "config-exists".to_string(),
        layer: None,
        state: None,
        status: DoctorStatus::Ok,
        message: format!("{} exists", config_path.display()),
    });

    let config = match load_config(&config_path) {
        Ok(c) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                layer: None,
                state: None,
                status: DoctorStatus::Ok,
                // The harness table keeps its unrecognized keys as problems
                // rather than refusing to load (`ADR-89`); `harness-config`
                // below names them, so this line must not deny them.
                message: match c.harness.as_ref().map_or(0, |h| h.problems.len()) {
                    0 => "config parses (no unrecognized keys)".to_string(),
                    n => format!(
                        "config parses ({n} unrecognized harness {}, named under harness-config)",
                        if n == 1 { "entry" } else { "entries" }
                    ),
                },
            });
            c
        }
        Err(e) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                layer: None,
                state: None,
                status: DoctorStatus::Error,
                message: format!("config does not parse: {e}"),
            });
            return emit(&DoctorReport {
                status: DoctorStatus::Error,
                checks,
                notices: Vec::new(),
                exit_code: 1,
            });
        }
    };

    if config.record_types.is_empty() {
        checks.push(DoctorCheck {
            check: "record-types-declared".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Error,
            message: "no record_types declared -- check will never examine anything".to_string(),
        });
    }

    // Every rule is opt-in (ADR-53): a config enabling none makes `check`
    // exit 2 not-run, so "healthy" here would be the silent no-op doctor
    // exists to catch.
    if config
        .rules
        .values()
        .all(|r| r.level == urzua_core::config::RuleLevel::Off)
    {
        checks.push(DoctorCheck {
            check: "rules-enabled".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Error,
            message:
                "no rule is enabled under `rules:` -- check will report not-run and examine nothing"
                    .to_string(),
        });
    }

    let type_names = config.sorted_type_names();
    for name in type_names {
        let cfg = &config.record_types[name];
        for dir in cfg.all_dirs() {
            if repo_root.join(dir).is_dir() {
                checks.push(DoctorCheck {
                    check: "record-type-dir".to_string(),
                    layer: None,
                    state: None,
                    status: DoctorStatus::Ok,
                    message: format!("record type '{name}' -> {dir} exists"),
                });
            } else {
                checks.push(DoctorCheck {
                    check: "record-type-dir".to_string(),
                    layer: None,
                    state: None,
                    status: DoctorStatus::Error,
                    message: format!(
                        "record type '{name}' declares dir '{dir}', which does not exist"
                    ),
                });
            }
        }
        if cfg.required_fields.is_empty() {
            checks.push(DoctorCheck {
                check: "record-type-required-fields".to_string(),
                layer: None,
                state: None,
                status: DoctorStatus::Warn,
                message: format!(
                    "record type '{name}' has no required_fields -- field.quality and field.pending will never fire for it"
                ),
            });
        }
    }

    // Every workflow, not one by name. Reading `ci.yml` alone reported that a
    // repository was unwired when its invocation lived in a differently named
    // workflow (BUG-91).
    //
    // No error swallowed. "No workflow invokes the checker" and "the workflows
    // could not be read" are different answers, and collapsing them reports
    // confidently about a directory never opened -- the defect this check was
    // just fixed for, one level down. Only an absent directory is a warning.
    let workflows_dir = repo_root.join(".github/workflows");
    let ci_wired: Result<bool, String> = match std::fs::read_dir(&workflows_dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("could not read {}: {e}", workflows_dir.display())),
        Ok(entries) => {
            let mut found = false;
            let mut failure: Option<String> = None;
            for entry in entries {
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(e) => {
                        failure = Some(format!(
                            "could not read an entry of {}: {e}",
                            workflows_dir.display()
                        ));
                        break;
                    }
                };
                if !path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
                    continue;
                }
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        // `urzua-gate` is urzua's own reusable action
                        // (MILE-180), which runs `urzua check` for the caller.
                        if invokes_urzua(&content, Some("check"), true) {
                            found = true;
                            break;
                        }
                    }
                    Err(e) => {
                        failure = Some(format!("could not read {}: {e}", path.display()));
                        break;
                    }
                }
            }
            match failure {
                Some(message) => Err(message),
                None => Ok(found),
            }
        }
    };
    match &ci_wired {
        Err(message) => checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Error,
            message: format!(
                "{message} -- cannot tell whether the checker is wired in, which is not the same as it not being"
            ),
        }),
        Ok(true) => checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Ok,
            message: "a workflow invokes `urzua check` or the urzua-gate action".to_string(),
        }),
        Ok(false) => checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            layer: None,
            state: None,
            status: DoctorStatus::Warn,
            message: format!(
                "no workflow in {} invokes `urzua check` or the urzua-gate action -- a check that exists but is never run reports nothing to anyone",
                workflows_dir.display()
            ),
        }),
    }

    // Local hook layers (ADR-71, MILE-179). Warn, not error, when absent:
    // a local layer is a convenience and only CI is the guarantee, so a clone
    // without hooks is reported but does not fail doctor. A hook directory
    // that cannot be read is an error: not-knowing is not the same as absent.
    checks.extend(hook_checks(&repo_root));

    // The agent harness (ADR-89), when the configuration declares one.
    for (level, check, message) in crate::harness::doctor_checks(&repo_root, &config) {
        checks.push(DoctorCheck {
            check: check.to_string(),
            layer: None,
            state: None,
            status: match level {
                crate::harness::Level::Ok => DoctorStatus::Ok,
                crate::harness::Level::Warn => DoctorStatus::Warn,
            },
            message,
        });
    }

    let has_error = checks.iter().any(|c| c.status == DoctorStatus::Error);
    let has_warn = checks.iter().any(|c| c.status == DoctorStatus::Warn);
    let status = if has_error {
        DoctorStatus::Error
    } else if has_warn {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Ok
    };

    emit(&DoctorReport {
        status,
        checks,
        notices: Vec::new(),
        exit_code: if has_error { 1 } else { 0 },
    })
}
