//! `urzua hooks install|status|uninstall|run` (`ADR-71` amendment of
//! 2026-11-16, `MILE-220`): wires the pre-commit and pre-push git hooks
//! `doctor` asks for, wherever the repository keeps its hooks, and runs the
//! configured check from them.
//!
//! A hook file urzua did not write is extended with a marked block, never
//! replaced, and only where the block is certain to run; `uninstall` removes
//! only that block, or a file that held nothing else. Nothing is written
//! through a symlink or outside the repository and its git directory.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{FailOn, HookOnNotRun, HooksDecl};
use urzua_core::report::{CouldNotRun, NextAction, Notice, NoticeSeverity, Report};

use crate::emit;
use crate::session::Workspace;

pub(crate) const HOOKS: [&str; 2] = ["pre-commit", "pre-push"];
pub(crate) const BEGIN: &str = "# >>> urzua-hooks begin";
pub(crate) const END: &str = "# <<< urzua-hooks end";
pub(crate) const INSTALL: &str = "urzua hooks install";
const FRAMEWORK_CONFIG: &str = ".pre-commit-config.yaml";
const SHEBANG: &str = "#!/bin/sh\n";
const INSTALL_DOC: &str = "https://github.com/urzuahq/urzua/blob/main/docs/reference/install.md";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// `hooks run` exits with this when urzua did not run and the policy is
/// `block`, so the block can tell it from a blocking finding (1) and from an
/// older urzua's usage error (2).
const NOT_RUN_BLOCK: u8 = 3;

/// Where the repository's git hooks live, and how they are run.
#[derive(Debug, Clone)]
pub(crate) enum Layout {
    /// Hook files git runs from one directory.
    Files {
        /// `default` (`.git/hooks`), `hooks-path` (`core.hooksPath`) or `husky`.
        kind: &'static str,
        dir: PathBuf,
        /// Why urzua will not write there, when it will not.
        unwritable: Option<String>,
        /// husky v9 runs `.husky/<hook>` through `sh` from its `.husky/_`
        /// stubs, so the file needs no execute bit there.
        needs_exec: bool,
    },
    /// The pre-commit framework: hooks are declared in its config and its own
    /// script in the git hooks directory runs them.
    Framework { config: PathBuf, git_hooks: PathBuf },
}

impl Layout {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Layout::Files { kind, .. } => kind,
            Layout::Framework { .. } => "pre-commit-framework",
        }
    }
}

fn git(repo_root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    urzua_io::git::GitRun::new()
        .args(args)
        .cwd(repo_root)
        .output()
        .map_err(|e| format!("could not run git: {}", e.into_io()))
}

fn git_path(repo_root: &Path, args: &[&str]) -> Result<PathBuf, String> {
    let out = git(repo_root, args)?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(absolute(
        repo_root,
        String::from_utf8_lossy(&out.stdout).trim(),
    ))
}

fn absolute(repo_root: &Path, p: &str) -> PathBuf {
    let p = PathBuf::from(p);
    if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    }
}

/// `path` with every symlink resolved: the nearest existing ancestor is
/// canonicalized and the rest appended, so a directory not yet created still
/// resolves through a symlinked parent.
fn resolve(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut cur = path.to_path_buf();
    loop {
        if let Ok(c) = cur.canonicalize() {
            return rest.into_iter().rev().fold(c, |acc, part| acc.join(part));
        }
        match (cur.file_name().map(|n| n.to_os_string()), cur.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                cur = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// The places urzua may write a hook: the working tree and the git
/// directory, which for a linked worktree or a submodule lives elsewhere.
fn allowed_roots(repo_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![resolve(repo_root)];
    for args in [
        ["rev-parse", "--absolute-git-dir"],
        ["rev-parse", "--git-common-dir"],
    ] {
        if let Ok(p) = git_path(repo_root, &args) {
            roots.push(resolve(&p));
        }
    }
    roots
}

/// Why urzua will not write in `dir`: it resolves outside the allowed roots,
/// or a file cannot be created there (probed: permission bits alone miss
/// read-only mounts).
fn write_problem(dir: &Path, roots: &[PathBuf]) -> Option<String> {
    let real = resolve(dir);
    if !roots.iter().any(|r| real.starts_with(r)) {
        return Some(format!(
            "{} resolves to {}, outside the repository and its git directory; urzua did not set it and will not write there",
            dir.display(),
            real.display()
        ));
    }
    if !dir.exists() {
        return None;
    }
    let probe = dir.join(".urzua-hooks-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            None
        }
        Err(e) => Some(format!("{} cannot be written: {e}", dir.display())),
    }
}

pub(crate) fn detect(repo_root: &Path) -> Result<Layout, String> {
    let roots = allowed_roots(repo_root);
    let configured = git(repo_root, &["config", "--get", "core.hooksPath"])?;
    let value = String::from_utf8_lossy(&configured.stdout)
        .trim()
        .to_string();
    if configured.status.success() && !value.is_empty() {
        let dir = absolute(repo_root, &value);
        let rel = dir.strip_prefix(repo_root).ok().map(Path::to_path_buf);
        if let Some(rel) = rel.filter(|r| r.starts_with(".husky")) {
            let husky = repo_root.join(".husky");
            return Ok(Layout::Files {
                kind: "husky",
                unwritable: write_problem(&husky, &roots),
                needs_exec: rel == Path::new(".husky"),
                dir: husky,
            });
        }
        return Ok(Layout::Files {
            kind: "hooks-path",
            unwritable: write_problem(&dir, &roots),
            dir,
            needs_exec: true,
        });
    }
    let git_hooks = git_path(repo_root, &["rev-parse", "--git-path", "hooks"])?;
    if repo_root.join(FRAMEWORK_CONFIG).is_file() {
        return Ok(Layout::Framework {
            config: repo_root.join(FRAMEWORK_CONFIG),
            git_hooks,
        });
    }
    if repo_root.join(".husky").is_dir() {
        let dir = repo_root.join(".husky");
        return Ok(Layout::Files {
            kind: "husky",
            unwritable: write_problem(&dir, &roots),
            dir,
            needs_exec: false,
        });
    }
    Ok(Layout::Files {
        kind: "default",
        unwritable: write_problem(&git_hooks, &roots),
        dir: git_hooks,
        needs_exec: true,
    })
}

/// How one hook is wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HookState {
    NotInstalled,
    InstalledRunningUrzua,
    InstalledNotRunningUrzua,
    /// Present but without the execute bit, which git silently ignores.
    InstalledNotExecutable,
    /// The call to urzua sits after an unconditional top-level `exit` or
    /// `exec`, so it never runs.
    InstalledUnreachable,
    /// The block carries a not-run policy the configuration no longer names.
    InstalledStalePolicy,
    /// More than one marked block.
    InstalledDuplicated,
}

impl HookState {
    fn status_word(self) -> &'static str {
        match self {
            HookState::InstalledRunningUrzua => "ok",
            HookState::NotInstalled => "missing",
            HookState::InstalledNotRunningUrzua => "not-wired",
            HookState::InstalledNotExecutable => "not-executable",
            HookState::InstalledUnreachable => "unreachable",
            HookState::InstalledStalePolicy => "stale-policy",
            HookState::InstalledDuplicated => "duplicated",
        }
    }
}

/// One hook's wiring, as `doctor`, `hooks status` and the session-start check
/// all report it.
#[derive(Debug, Clone)]
pub(crate) struct Wiring {
    pub hook: &'static str,
    pub path: PathBuf,
    pub state: HookState,
    pub message: String,
    /// The one runnable command that wires it, when there is one.
    pub fix: Option<String>,
}

/// The line a hook urzua will not write must carry to run urzua.
pub(crate) fn shell_line(hook: &str) -> String {
    format!("\"${{URZUA_BIN:-urzua}}\" hooks run {hook} || exit $?")
}

/// `s` as one shell word, single-quoted.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn executable(meta: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    // Windows has no execute bit; git runs the hook through its shell.
    #[cfg(not(unix))]
    {
        let _ = meta;
        true
    }
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(String::from_utf8_lossy(&b).into_owned())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read {}: {e}", path.display())),
    }
}

/// Every hook's wiring in `layout`, or why it cannot be read.
pub(crate) fn wiring(layout: &Layout, decl: &HooksDecl) -> Vec<Result<Wiring, String>> {
    HOOKS
        .iter()
        .map(|hook| match layout {
            Layout::Files {
                dir,
                unwritable,
                needs_exec,
                ..
            } => file_wiring(
                hook,
                &dir.join(hook),
                *needs_exec,
                unwritable.as_deref(),
                decl,
            ),
            Layout::Framework { config, git_hooks } => {
                framework_wiring(hook, config, git_hooks, decl)
            }
        })
        .collect()
}

fn wired(
    hook: &'static str,
    path: &Path,
    state: HookState,
    message: String,
    fix: Option<String>,
) -> Wiring {
    Wiring {
        hook,
        path: path.to_path_buf(),
        state,
        message,
        fix,
    }
}

/// The repair for a hook urzua may write: `INSTALL`, or, where it will not
/// write, no command and the line to add in the message.
fn repair(
    hook: &str,
    path: &Path,
    unwritable: Option<&str>,
    message: String,
) -> (String, Option<String>) {
    match unwritable {
        Some(why) => (
            format!(
                "{message}; {why}, so add `{}` to {}",
                shell_line(hook),
                path.display()
            ),
            None,
        ),
        None => (message, Some(INSTALL.to_string())),
    }
}

fn file_wiring(
    hook: &'static str,
    path: &Path,
    needs_exec: bool,
    unwritable: Option<&str>,
    decl: &HooksDecl,
) -> Result<Wiring, String> {
    let meta = match std::fs::metadata(path) {
        Ok(m) if m.is_file() => Some(m),
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    let Some(meta) = meta else {
        let (message, fix) = repair(
            hook,
            path,
            unwritable,
            format!(
                "no {hook} hook at {} -- `{INSTALL}` adds one that runs urzua; CI remains the guarantee (ADR-71)",
                path.display()
            ),
        );
        return Ok(wired(hook, path, HookState::NotInstalled, message, fix));
    };
    if needs_exec && !executable(&meta) {
        return Ok(wired(
            hook,
            path,
            HookState::InstalledNotExecutable,
            format!(
                "{} is not executable, so git ignores it -- make it executable, then run `{INSTALL}`",
                path.display()
            ),
            Some(format!(
                "chmod +x {} && {INSTALL}",
                shell_quote(&path.to_string_lossy())
            )),
        ));
    }
    let content = read_optional(path)?.unwrap_or_default();
    let (state, message) = content_state(path, &content, decl);
    let fix = match state {
        HookState::InstalledRunningUrzua => None,
        HookState::InstalledDuplicated => Some(format!("urzua hooks uninstall && {INSTALL}")),
        _ => {
            let (_, fix) = repair(hook, path, unwritable, String::new());
            return Ok(wired(
                hook,
                path,
                state,
                repair(hook, path, unwritable, message).0,
                fix,
            ));
        }
    };
    Ok(wired(hook, path, state, message, fix))
}

/// What a hook file's content says about its wiring.
fn content_state(path: &Path, content: &str, decl: &HooksDecl) -> (HookState, String) {
    let shown = path.display();
    let blocks = content.matches(BEGIN).count();
    if blocks > 1 {
        return (
            HookState::InstalledDuplicated,
            format!(
                "{shown} has {blocks} urzua blocks -- run `urzua hooks uninstall && {INSTALL}`"
            ),
        );
    }
    let at = if blocks == 1 {
        content.lines().position(|l| l.contains(BEGIN))
    } else {
        invocation_line(content, None, false)
    };
    let Some(at) = at else {
        return (
            HookState::InstalledNotRunningUrzua,
            format!("{shown} does not run urzua -- `{INSTALL}` adds a marked block that does, after what is there"),
        );
    };
    if shell_hook(content).is_ok() {
        if let Some(stop) = unconditional_stop_before(content, at) {
            return (
                HookState::InstalledUnreachable,
                format!(
                    "{shown} calls urzua on line {}, after the unconditional `{}` on line {}, so it never runs -- move the call before that line, or remove the block and run `{INSTALL}`",
                    at + 1,
                    stop.1,
                    stop.0 + 1
                ),
            );
        }
    }
    if blocks == 1 {
        if let Some(baked) = baked_policy(content) {
            if baked != decl.on_not_run.as_str() {
                return (
                    HookState::InstalledStalePolicy,
                    format!(
                        "{shown} was written with on_not_run: {baked}, but the configuration says {} -- run `{INSTALL}` to rewrite it",
                        decl.on_not_run.as_str()
                    ),
                );
            }
        }
    }
    (
        HookState::InstalledRunningUrzua,
        format!("{shown} runs urzua"),
    )
}

/// The not-run policy written into a block's first line.
fn baked_policy(content: &str) -> Option<&str> {
    let line = content.lines().find(|l| l.contains(BEGIN))?;
    let rest = line.split("on_not_run: ").nth(1)?;
    rest.split(|c: char| !c.is_ascii_alphabetic()).next()
}

/// A framework hook is wired when its config runs urzua at that stage and the
/// framework's own script is installed for that hook type.
fn framework_wiring(
    hook: &'static str,
    config: &Path,
    git_hooks: &Path,
    decl: &HooksDecl,
) -> Result<Wiring, String> {
    let text = read_optional(config)?.unwrap_or_default();
    let declared = text.lines().any(|l| {
        !l.trim_start().starts_with('#')
            && l.contains(&format!("hooks run {hook}"))
            && l.contains("urzua")
    });
    if !declared {
        return Ok(wired(
            hook,
            config,
            HookState::InstalledNotRunningUrzua,
            format!(
                "{} declares no hook that runs urzua at {hook} -- `{INSTALL} --write-config` adds the stanza",
                config.display()
            ),
            Some(format!("{INSTALL} --write-config")),
        ));
    }
    if let Some(baked) = baked_policy(&text).filter(|b| *b != decl.on_not_run.as_str()) {
        return Ok(wired(
            hook,
            config,
            HookState::InstalledStalePolicy,
            format!(
                "{} carries the urzua stanza written with on_not_run: {baked}, but the configuration says {} -- run `urzua hooks uninstall && {INSTALL} --write-config`",
                config.display(),
                decl.on_not_run.as_str()
            ),
            Some(format!("urzua hooks uninstall && {INSTALL} --write-config")),
        ));
    }
    let script = git_hooks.join(hook);
    if !script.is_file() {
        return Ok(wired(
            hook,
            &script,
            HookState::NotInstalled,
            format!(
                "{} declares urzua at {hook}, but the pre-commit framework is not installed for it (no {}) -- run `pre-commit install --hook-type {hook}`",
                config.display(),
                script.display()
            ),
            Some(format!("pre-commit install --hook-type {hook}")),
        ));
    }
    Ok(wired(
        hook,
        config,
        HookState::InstalledRunningUrzua,
        format!("{} runs urzua at {hook}", config.display()),
        None,
    ))
}

/// A token naming the urzua executable: a path ending in `urzua`, a
/// `${VAR:-<path ending in urzua>}` default, or one of `binaries`.
fn names_urzua(token: &str, binaries: &[String]) -> bool {
    let inner = token
        .strip_prefix("${")
        .and_then(|t| t.strip_suffix('}'))
        .and_then(|t| t.split_once(":-"))
        .map_or(token, |(_, default)| default);
    inner.rsplit('/').next() == Some("urzua")
        || binaries
            .iter()
            .any(|b| token == format!("${b}") || token == format!("${{{b}}}"))
}

/// Variables assigned a path that ends in `urzua` (`bin="$root/.../urzua"`),
/// so `"$bin" check` is seen as the call it is.
fn urzua_variables(content: &str) -> Vec<String> {
    let mut binaries = Vec::new();
    for line in content.lines() {
        let l = line.trim_start().trim_start_matches("export ");
        if let Some((name, value)) = l.split_once('=') {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && names_urzua(value, &[])
            {
                binaries.push(name.to_string());
            }
        }
    }
    binaries
}

/// Whether one line runs urzua: a command position, not a word in a comment
/// or an `echo`. In another language's call (`execFileSync("urzua",
/// ["hooks", "run", ...])`) the arguments are separated by commas and
/// brackets; there `urzua hooks run` must appear in that order.
fn line_invokes(
    line: &str,
    binaries: &[String],
    subcommand: Option<&str>,
    accept_action: bool,
) -> bool {
    if line.trim_start().starts_with('#') {
        return false;
    }
    let code = line.find(" #").map_or(line, |at| &line[..at]);
    let split = |extra: &str| -> Vec<String> {
        code.split(|c: char| c.is_whitespace() || "\"'();&|`".contains(c) || extra.contains(c))
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    };
    let tokens = split("");
    if matches!(tokens.first().map(String::as_str), Some("echo" | "printf")) {
        return false;
    }
    if accept_action && tokens.iter().any(|t| t.contains("actions/urzua-gate")) {
        return true;
    }
    let direct = tokens.windows(2).any(|pair| {
        let wanted = match subcommand {
            Some(sub) => pair[1] == sub,
            None => pair[1].starts_with(|c: char| c.is_ascii_alphabetic()),
        };
        names_urzua(&pair[0], binaries) && wanted
    });
    direct
        || split(",[]{}")
            .windows(3)
            .any(|w| names_urzua(&w[0], binaries) && w[1] == "hooks" && w[2] == "run")
}

/// The 0-based line of the first call to urzua in `content`.
fn invocation_line(content: &str, subcommand: Option<&str>, accept_action: bool) -> Option<usize> {
    let binaries = urzua_variables(content);
    content
        .lines()
        .position(|l| line_invokes(l, &binaries, subcommand, accept_action))
}

/// Whether `content` runs urzua. The engine knows no repository's wrapper
/// script or make target, so a repository that wraps the call is seen only
/// through the call itself. With `accept_action`, a `uses:` line naming
/// urzua's reusable gate action counts too.
pub(crate) fn invokes_urzua(content: &str, subcommand: Option<&str>, accept_action: bool) -> bool {
    invocation_line(content, subcommand, accept_action).is_some()
}

fn words(code: &str) -> Vec<&str> {
    code.split(|c: char| c.is_whitespace() || ";&|()`".contains(c))
        .filter(|t| !t.is_empty())
        .collect()
}

/// The lines of a shell hook that are commands: blank and comment lines, and
/// the marked block, dropped. (0-based index, code without a trailing comment.)
fn command_lines(content: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (i, line) in content.lines().enumerate() {
        if line.contains(BEGIN) {
            in_block = true;
        }
        let skip = in_block || line.trim().is_empty() || line.trim_start().starts_with('#');
        if line.contains(END) {
            in_block = false;
        }
        if !skip {
            out.push((i, line.find(" #").map_or(line, |at| &line[..at])));
        }
    }
    out
}

/// Nesting change of one line: `if`/`case`/loops and `{` open, `fi`/`esac`/
/// `done` and `}` close.
fn depth_delta(code: &str) -> i32 {
    let mut d = 0;
    for w in code.split(|c: char| c.is_whitespace() || c == ';') {
        match w {
            "if" | "case" | "while" | "until" | "for" | "{" => d += 1,
            "fi" | "esac" | "done" | "}" => d -= 1,
            _ if w.ends_with('{') => d += 1,
            _ if w.starts_with('}') => d -= 1,
            _ => {}
        }
    }
    d
}

/// The first unconditional top-level `exit` or `exec` before line `at`, as
/// (0-based line, word).
fn unconditional_stop_before(content: &str, at: usize) -> Option<(usize, &'static str)> {
    let mut depth = 0;
    for (i, code) in command_lines(content) {
        if i >= at {
            break;
        }
        if depth == 0 {
            match words(code).first() {
                Some(&"exit") => return Some((i, "exit")),
                Some(&"exec") => return Some((i, "exec")),
                _ => {}
            }
        }
        depth += depth_delta(code);
    }
    None
}

/// Where the block goes in a hook urzua did not write: after the last
/// command, or before it when it is a top-level `exit` or `exec`. `Err` names
/// the line that could end the hook before the block, or the compound command
/// it would follow, when no place is certain to run.
fn insertion_point(content: &str) -> Result<usize, (usize, String)> {
    let lines = command_lines(content);
    let Some(&(last, last_code)) = lines.last() else {
        return Ok(content.lines().count());
    };
    for &(i, code) in &lines {
        let w = words(code);
        if w.first() == Some(&"trap") {
            return Err((i, "a `trap` can end the hook on its own".to_string()));
        }
        let stops = w.iter().any(|t| *t == "exit" || *t == "exec");
        let final_stop = i == last && matches!(w.first(), Some(&"exit") | Some(&"exec"));
        if final_stop {
            return Ok(i);
        }
        if stops {
            return Err((
                i,
                "an `exit` or `exec` here can end the hook before the block".to_string(),
            ));
        }
    }
    let ends_compound = words(last_code)
        .last()
        .is_some_and(|t| matches!(*t, "fi" | "done" | "esac") || t.ends_with('}'));
    if ends_compound {
        return Err((last, "the hook ends in a compound command".to_string()));
    }
    Ok(content.lines().count())
}

/// The marked shell block a hook runs urzua from. The binary is found as the
/// harness finds it: `URZUA_BIN`, then `PATH`. `hooks run` reads the not-run
/// policy from the configuration when it runs; `on_not_run` here is the
/// policy for the cases it cannot ask urzua: no binary, or one without
/// `hooks run`.
pub(crate) fn shell_block(hook: &str, on_not_run: HookOnNotRun) -> String {
    let verb = if hook == "pre-push" { "push" } else { "commit" };
    let stop = match on_not_run {
        HookOnNotRun::Block => format!(
            "echo \"{hook}: to {verb} without the check once: git {verb} --no-verify\" >&2; exit 1"
        ),
        HookOnNotRun::Warn => ":".to_string(),
    };
    format!(
        "{BEGIN} (on_not_run: {policy}; written by `{INSTALL}`; `urzua hooks uninstall` removes it)\n\
urzua_bin=\"${{URZUA_BIN:-}}\"\n\
[ -n \"$urzua_bin\" ] || urzua_bin=\"$(command -v urzua 2>/dev/null || true)\"\n\
if [ -z \"$urzua_bin\" ]; then\n\
\x20 echo \"{hook}: urzua did not run (not installed: URZUA_BIN unset, not on PATH), so the records were NOT checked; install it: {INSTALL_DOC} -- CI will check them\" >&2\n\
\x20 {stop}\n\
else\n\
\x20 urzua_out=\"$(\"$urzua_bin\" hooks run {hook} 2>&1 >/dev/null)\"\n\
\x20 urzua_code=$?\n\
\x20 case \"$urzua_code\" in\n\
\x20   0|1|{NOT_RUN_BLOCK}) [ -z \"$urzua_out\" ] || printf '%s\\n' \"$urzua_out\" >&2 ;;\n\
\x20   *) echo \"{hook}: urzua did not run: $urzua_bin has no \\`hooks run\\` (exit $urzua_code); upgrade urzua to {VERSION} or later. The records were NOT checked -- CI will check them\" >&2\n\
\x20      {stop} ;;\n\
\x20 esac\n\
\x20 case \"$urzua_code\" in 1|{NOT_RUN_BLOCK}) exit 1 ;; esac\n\
fi\n\
{END}",
        policy = on_not_run.as_str()
    )
}

/// `s` as a YAML single-quoted scalar.
fn yaml_single(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The framework hook's entry: urzua's own not-run message when the binary is
/// missing, not the framework's "executable not found".
fn framework_entry(hook: &str, on_not_run: HookOnNotRun) -> String {
    let exit = match on_not_run {
        HookOnNotRun::Block => 1,
        HookOnNotRun::Warn => 0,
    };
    let script = format!(
        "b=\"${{URZUA_BIN:-urzua}}\"; if command -v \"$b\" >/dev/null 2>&1; then exec \"$b\" hooks run {hook}; fi; echo \"{hook}: urzua did not run (not installed), so the records were NOT checked; install it: {INSTALL_DOC} -- CI will check them\" >&2; exit {exit}"
    );
    yaml_single(&format!("sh -c {}", shell_quote(&script)))
}

/// The `repo: local` stanza for the pre-commit framework, at `indent`, with
/// its markers.
pub(crate) fn framework_stanza(indent: &str, on_not_run: HookOnNotRun) -> String {
    let mut out = format!(
        "{indent}{BEGIN} (on_not_run: {}; written by `{INSTALL} --write-config`)\n",
        on_not_run.as_str()
    );
    out.push_str(&format!("{indent}- repo: local\n{indent}  hooks:\n"));
    for hook in HOOKS {
        out.push_str(&format!(
            "{indent}    - id: urzua-{hook}\n\
{indent}      name: urzua ({hook})\n\
{indent}      entry: {}\n\
{indent}      language: system\n\
{indent}      pass_filenames: false\n\
{indent}      always_run: true\n\
{indent}      stages: [{hook}]\n",
            framework_entry(hook, on_not_run)
        ));
    }
    out.push_str(&format!("{indent}{END}"));
    out
}

/// (start, end) of the first marked block in `text`, `end` past the end
/// marker's line.
fn span(text: &str) -> Result<Option<(usize, usize)>, String> {
    match (text.find(BEGIN), text.find(END)) {
        (None, None) => Ok(None),
        (Some(b), Some(e)) if b < e => {
            let start = text[..b].rfind('\n').map_or(0, |i| i + 1);
            let mut end = e + END.len();
            if text[end..].starts_with('\n') {
                end += 1;
            }
            Ok(Some((start, end)))
        }
        _ => Err(format!(
            "has a '{BEGIN}' without a later '{END}' (or the reverse); fix the markers by hand, urzua will not guess"
        )),
    }
}

/// `Err` with the interpreter a hook's shebang names, for a non-shell one.
fn shell_hook(content: &str) -> Result<(), String> {
    let Some(first) = content.lines().next().filter(|l| l.starts_with("#!")) else {
        return Ok(());
    };
    let words: Vec<&str> = first[2..].split_whitespace().collect();
    let interp = match words.as_slice() {
        [env, name, ..] if env.ends_with("/env") => *name,
        [path, ..] => path.rsplit('/').next().unwrap_or(path),
        [] => return Ok(()),
    };
    if ["sh", "bash", "dash", "zsh", "ksh", "ash"].contains(&interp) {
        Ok(())
    } else {
        Err(interp.to_string())
    }
}

/// The one line a non-shell hook needs to run urzua, in its language.
pub(crate) fn foreign_line(interp: &str, hook: &str) -> String {
    if interp.starts_with("node") || interp == "bun" || interp == "deno" {
        format!(
            "require(\"child_process\").execFileSync(process.env.URZUA_BIN || \"urzua\", [\"hooks\", \"run\", \"{hook}\"], {{ stdio: \"inherit\" }});"
        )
    } else if interp.starts_with("python") {
        format!(
            "import os, subprocess, sys; r = subprocess.run([os.environ.get(\"URZUA_BIN\", \"urzua\"), \"hooks\", \"run\", \"{hook}\"]).returncode; r and sys.exit(r)"
        )
    } else {
        format!(
            "run `urzua hooks run {hook}` (URZUA_BIN, else PATH) and exit non-zero when it does"
        )
    }
}

/// `content` without any marked block.
fn remove_blocks(content: &str) -> Result<String, String> {
    let mut out = content.to_string();
    while let Some((b, e)) = span(&out)? {
        out = format!("{}{}", &out[..b], &out[e..]);
    }
    Ok(out)
}

/// `content` with `block` set: every existing block removed and one put in
/// its place, or inserted at `insertion_point`. `Err` is the refusal.
fn place_block(content: &str, block: &str, hook: &str) -> Result<String, String> {
    if let Some((b, _)) = span(content)? {
        let rest = remove_blocks(content)?;
        return Ok(format!("{}{block}\n{}", &rest[..b], &rest[b..]));
    }
    let at = insertion_point(content).map_err(|(line, why)| {
        format!(
            "{why} (line {}: `{}`), so a block urzua appends might never run; add this line where it runs on every path, before line {}: {}",
            line + 1,
            content.lines().nth(line).unwrap_or("").trim(),
            line + 1,
            shell_line(hook)
        )
    })?;
    let lines: Vec<&str> = content.lines().collect();
    let mut out = String::new();
    for l in &lines[..at] {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(block);
    out.push('\n');
    for l in &lines[at..] {
        out.push_str(l);
        out.push('\n');
    }
    Ok(out)
}

/// One file the command looked at, and what became of it.
#[derive(serde::Serialize)]
struct Item {
    hook: String,
    path: String,
    /// install: `created`, `extended`, `updated`, `unchanged`, `wired`,
    /// `would-...`; status: `ok`, `missing`, `not-wired`, `not-executable`,
    /// `unreachable`, `stale-policy`, `duplicated`; uninstall: `removed`,
    /// `block-removed`, `absent`, `not-ours`.
    state: String,
}

#[derive(serde::Serialize)]
struct HooksReport {
    status: &'static str,
    action: &'static str,
    dry_run: bool,
    layout: &'static str,
    items: Vec<Item>,
    /// The stanza for `.pre-commit-config.yaml`, when the layout is the
    /// pre-commit framework and the config does not yet run urzua.
    #[serde(skip_serializing_if = "Option::is_none")]
    stanza: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    problems: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    #[serde(skip)]
    next: Option<NextAction>,
    #[serde(skip)]
    exit: u8,
}

impl Report for HooksReport {
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

impl HooksReport {
    fn new(action: &'static str, dry_run: bool, layout: &Layout) -> Self {
        HooksReport {
            status: "ok",
            action,
            dry_run,
            layout: layout.kind(),
            items: Vec::new(),
            stanza: None,
            problems: Vec::new(),
            notices: Vec::new(),
            next: None,
            exit: 0,
        }
    }

    fn item(&mut self, hook: &str, path: &Path, state: &str) {
        self.items.push(Item {
            hook: hook.to_string(),
            path: path.display().to_string(),
            state: state.to_string(),
        });
    }

    fn notice(&mut self, severity: NoticeSeverity, message: String) {
        self.notices.push(Notice {
            severity,
            subject: "hooks".to_string(),
            message,
            detail: None,
        });
    }

    fn next(&mut self, command: String, reason: &str) {
        if self.next.is_none() {
            self.next = Some(NextAction {
                command,
                reason: reason.to_string(),
            });
        }
    }

    /// Nothing was written because the run refused.
    fn refuse(&mut self, why: String) {
        self.status = "not-run";
        self.exit = 2;
        self.problems.push(why);
    }
}

fn not_run(message: impl Into<String>) -> ExitCode {
    emit(&CouldNotRun::from(message.into()))
}

fn open(config: Option<PathBuf>) -> Result<(Workspace, Layout), String> {
    let ws = Workspace::open(&[PathBuf::from(".")], config).map_err(|e| e.message)?;
    let layout = detect(&ws.repo_root)?;
    Ok((ws, layout))
}

pub fn run_status(config: Option<PathBuf>) -> ExitCode {
    let (ws, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    emit(&status(&layout, &ws.config.hooks))
}

fn status(layout: &Layout, decl: &HooksDecl) -> HooksReport {
    let mut report = HooksReport::new("status", false, layout);
    for w in wiring(layout, decl) {
        match w {
            Err(m) => report.refuse(m),
            Ok(w) => {
                report.item(w.hook, &w.path, w.state.status_word());
                if w.state != HookState::InstalledRunningUrzua {
                    report.problems.push(w.message);
                    if let Some(cmd) = w.fix {
                        report.next(cmd, "wires the git hooks that do not run urzua");
                    }
                    if report.exit == 0 {
                        report.status = "drift";
                        report.exit = 1;
                    }
                }
            }
        }
    }
    report
}

pub fn run_install(config: Option<PathBuf>, dry_run: bool, write_config: bool) -> ExitCode {
    let (ws, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    emit(&install(&layout, &ws.config.hooks, dry_run, write_config))
}

fn install(layout: &Layout, decl: &HooksDecl, dry_run: bool, write_config: bool) -> HooksReport {
    let mut report = HooksReport::new("install", dry_run, layout);
    match layout {
        Layout::Framework { config, git_hooks } => {
            install_framework(&mut report, decl, config, git_hooks, dry_run, write_config)
        }
        Layout::Files {
            kind,
            dir,
            unwritable,
            needs_exec,
        } => {
            if *kind == "husky" && *needs_exec {
                report.notice(
                    NoticeSeverity::Info,
                    "core.hooksPath is .husky, so git runs these files directly; they are written executable".to_string(),
                );
            }
            let target = Target {
                dir,
                unwritable: unwritable.as_deref(),
                needs_exec: *needs_exec,
            };
            install_files(&mut report, decl, &target, dry_run)
        }
    }
    report
}

/// Where hook files are written.
struct Target<'a> {
    dir: &'a Path,
    unwritable: Option<&'a str>,
    needs_exec: bool,
}

/// The plan for one hook file: its new content and what the write is.
struct Write {
    hook: &'static str,
    path: PathBuf,
    content: String,
    state: &'static str,
}

/// Why urzua will not write `path` itself: it is a symlink, wherever it points.
fn symlink_problem(hook: &str, path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_symlink() {
        return None;
    }
    let target = std::fs::read_link(path)
        .map(|t| t.display().to_string())
        .unwrap_or_else(|_| "an unreadable target".to_string());
    Some(format!(
        "{} is a symlink to {target}; urzua never writes through a symlink, so add this line to the hook it points at: {}",
        path.display(),
        shell_line(hook)
    ))
}

/// What to write for one hook, `Ok(None)` when nothing changes, `Err` the
/// refusal.
fn plan_hook(
    report: &mut HooksReport,
    hook: &'static str,
    path: &Path,
    block: &str,
    target: &Target,
) -> Result<Option<(String, &'static str)>, String> {
    if let Some(why) = symlink_problem(hook, path) {
        return Err(why);
    }
    let Some(content) = read_optional(path)? else {
        return Ok(Some((format!("{SHEBANG}{block}\n"), "created")));
    };
    if target.needs_exec {
        if let Ok(meta) = std::fs::metadata(path) {
            if !executable(&meta) {
                return Err(format!(
                    "{} is not executable, so git ignores it; urzua will not turn on a hook someone left off -- make it executable (`chmod +x {}`) or remove it, then run `{INSTALL}` again",
                    path.display(),
                    shell_quote(&path.to_string_lossy())
                ));
            }
        }
    }
    if content.contains(BEGIN) || content.contains(END) {
        let new =
            place_block(&content, block, hook).map_err(|m| format!("{} {m}", path.display()))?;
        return Ok((new != content).then_some((new, "updated")));
    }
    if invokes_urzua(&content, None, false) {
        report.item(hook, path, "wired");
        return Ok(None);
    }
    if let Err(interp) = shell_hook(&content) {
        return Err(format!(
            "{} is a {interp} script, so urzua will not add a shell block to it; add this line to it: {}",
            path.display(),
            foreign_line(&interp, hook)
        ));
    }
    let new = place_block(&content, block, hook).map_err(|m| format!("{}: {m}", path.display()))?;
    Ok(Some((new, "extended")))
}

fn install_files(report: &mut HooksReport, decl: &HooksDecl, target: &Target, dry_run: bool) {
    let mut writes: Vec<Write> = Vec::new();
    for hook in HOOKS {
        let path = target.dir.join(hook);
        let before = report.items.len();
        match plan_hook(
            report,
            hook,
            &path,
            &shell_block(hook, decl.on_not_run),
            target,
        ) {
            Err(why) => report.refuse(why),
            Ok(None) if report.items.len() == before => report.item(hook, &path, "unchanged"),
            Ok(None) => {}
            Ok(Some((content, state))) => match target.unwritable {
                Some(why) => report.refuse(format!(
                    "{why}; add this line to {} to run urzua from it: {}",
                    path.display(),
                    shell_line(hook)
                )),
                None => writes.push(Write {
                    hook,
                    path,
                    content,
                    state,
                }),
            },
        }
    }
    let refused = report.exit != 0;
    for w in writes {
        if refused || dry_run {
            report.item(w.hook, &w.path, &would(w.state));
            continue;
        }
        if let Err(e) = write_hook(&w.path, &w.content) {
            report.refuse(format!("could not write {}: {e}", w.path.display()));
            continue;
        }
        report.item(w.hook, &w.path, w.state);
    }
}

fn would(state: &str) -> String {
    match state {
        "created" => "would-create".to_string(),
        "extended" => "would-extend".to_string(),
        "updated" => "would-update".to_string(),
        "removed" => "would-remove".to_string(),
        "block-removed" => "would-remove-block".to_string(),
        other => format!("would-{other}"),
    }
}

fn write_hook(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// The indentation of the `repos:` list items when `repos:` is the config's
/// last top-level key (so an item appended at the end belongs to it), or why
/// the stanza cannot be appended safely.
fn repos_indent(text: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = lines.iter().position(|l| l.trim_end() == "repos:") else {
        return Err("it has no block-style `repos:` key".to_string());
    };
    let rest = &lines[at + 1..];
    if rest
        .iter()
        .any(|l| !l.is_empty() && !l.starts_with(' ') && !l.starts_with('-') && !l.starts_with('#'))
    {
        return Err("`repos:` is not its last top-level key".to_string());
    }
    Ok(rest
        .iter()
        .find_map(|l| {
            let t = l.trim_start();
            t.starts_with("- ")
                .then(|| l[..l.len() - t.len()].to_string())
        })
        .unwrap_or_else(|| "  ".to_string()))
}

fn install_framework(
    report: &mut HooksReport,
    decl: &HooksDecl,
    config: &Path,
    git_hooks: &Path,
    dry_run: bool,
    write_config: bool,
) {
    let text = match read_optional(config) {
        Ok(t) => t.unwrap_or_default(),
        Err(m) => return report.refuse(m),
    };
    let declared = text.contains(BEGIN)
        || HOOKS
            .iter()
            .all(|h| text.contains(&format!("hooks run {h}")) && text.contains("urzua"));
    if declared {
        report.item("pre-commit-framework", config, "wired");
    } else if !append_stanza(report, decl, config, &text, dry_run, write_config) {
        return;
    }
    let layout = Layout::Framework {
        config: config.to_path_buf(),
        git_hooks: git_hooks.to_path_buf(),
    };
    for w in wiring(&layout, decl).into_iter().flatten() {
        if matches!(
            w.state,
            HookState::NotInstalled | HookState::InstalledStalePolicy
        ) {
            report.notice(NoticeSeverity::Warning, w.message);
        }
    }
    let missing: Vec<String> = HOOKS
        .iter()
        .filter(|h| !git_hooks.join(h).is_file())
        .map(|h| format!("--hook-type {h}"))
        .collect();
    if !missing.is_empty() {
        report.next(
            format!("pre-commit install {}", missing.join(" ")),
            "installs the pre-commit framework's own git hooks, which run the stanza",
        );
    }
}

/// Puts the stanza in the report and, with `write_config`, appends it.
/// `false` when the run stops here.
fn append_stanza(
    report: &mut HooksReport,
    decl: &HooksDecl,
    config: &Path,
    text: &str,
    dry_run: bool,
    write_config: bool,
) -> bool {
    let indent = repos_indent(text);
    report.stanza = Some(framework_stanza(
        indent.as_deref().unwrap_or("  "),
        decl.on_not_run,
    ));
    if !write_config {
        report.item("pre-commit-framework", config, "stanza-needed");
        report.notice(
            NoticeSeverity::Warning,
            format!(
                "this repository uses the pre-commit framework, so no hook file is written; add `stanza` under `repos:` in {}, or run `{INSTALL} --write-config` to append it",
                config.display()
            ),
        );
        report.next(
            format!("{INSTALL} --write-config"),
            "appends the urzua stanza to .pre-commit-config.yaml",
        );
        return false;
    }
    if let Some(why) = symlink_problem("pre-commit", config) {
        report.refuse(why);
        return false;
    }
    let indent = match indent {
        Ok(i) => i,
        Err(why) => {
            report.refuse(format!(
                "{}: {why}, so urzua will not edit it; add `stanza` under `repos:` by hand",
                config.display()
            ));
            return false;
        }
    };
    let mut new = text.to_string();
    if !new.ends_with('\n') {
        new.push('\n');
    }
    new.push_str(&framework_stanza(&indent, decl.on_not_run));
    new.push('\n');
    if dry_run {
        report.item("pre-commit-framework", config, "would-extend");
    } else if let Err(e) = std::fs::write(config, new) {
        report.refuse(format!("could not write {}: {e}", config.display()));
        return false;
    } else {
        report.item("pre-commit-framework", config, "extended");
    }
    true
}

pub fn run_uninstall(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let (_, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    let mut report = HooksReport::new("uninstall", dry_run, &layout);
    let framework = matches!(layout, Layout::Framework { .. });
    let files: Vec<(&str, PathBuf)> = match &layout {
        Layout::Framework { config, .. } => vec![("pre-commit-framework", config.clone())],
        Layout::Files { dir, .. } => HOOKS.iter().map(|h| (*h, dir.join(h))).collect(),
    };
    for (hook, path) in files {
        uninstall_one(&mut report, hook, &path, framework, dry_run);
    }
    emit(&report)
}

fn uninstall_one(
    report: &mut HooksReport,
    hook: &str,
    path: &Path,
    framework: bool,
    dry_run: bool,
) {
    let content = match read_optional(path) {
        Ok(Some(c)) => c,
        Ok(None) => return report.item(hook, path, "absent"),
        Err(m) => return report.refuse(m),
    };
    if !content.contains(BEGIN) && !content.contains(END) {
        return report.item(hook, path, "not-ours");
    }
    if let Some(why) = symlink_problem(hook, path) {
        return report.refuse(why.replace("add this line to", "remove the urzua block from"));
    }
    let rest = match remove_blocks(&content) {
        Ok(r) => r,
        Err(m) => return report.refuse(format!("{} {m}", path.display())),
    };
    let delete = !framework && (rest.trim().is_empty() || rest == SHEBANG);
    let state = if delete { "removed" } else { "block-removed" };
    if dry_run {
        return report.item(hook, path, &would(state));
    }
    let done = if delete {
        std::fs::remove_file(path)
    } else {
        std::fs::write(path, rest)
    };
    match done {
        Ok(()) => report.item(hook, path, state),
        Err(e) => report.refuse(format!("could not change {}: {e}", path.display())),
    }
}

/// `urzua hooks run <hook>`: what the installed block calls. Runs the
/// configured check and prints a summary a person or agent can act on to
/// stderr. Exit 0 when clean or not run under `warn`, 1 on blocking findings,
/// `NOT_RUN_BLOCK` when not run under `block`.
pub fn run_hook(config: Option<PathBuf>, hook: String) -> ExitCode {
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => {
            return not_run_hook(
                &hook,
                &format!("the configuration could not be read: {}", e.message),
                HookOnNotRun::default(),
            )
        }
    };
    let decl = &ws.config.hooks;
    let Some(args) = decl.args(&hook) else {
        return not_run(format!(
            "'{hook}' is not a hook urzua runs; the hooks are {}",
            HOOKS.join(", ")
        ));
    };
    let shown = format!("urzua {}", args.join(" "));
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            return not_run_hook(
                &hook,
                &format!("cannot find this binary: {e}"),
                decl.on_not_run,
            )
        }
    };
    let mut cmd = std::process::Command::new(exe);
    if let Some(c) = &config {
        cmd.arg("--config").arg(c);
    }
    let out = match cmd.args(args).current_dir(&ws.repo_root).output() {
        Ok(o) => o,
        Err(e) => {
            return not_run_hook(
                &hook,
                &format!("{shown} could not start: {e}"),
                decl.on_not_run,
            )
        }
    };
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    match out.status.code() {
        Some(0) => {
            eprintln!("{hook}: {shown}: no blocking findings");
            ExitCode::SUCCESS
        }
        Some(1) => blocked(&hook, &shown, &report, ws.config.fail_on),
        _ => {
            let why = report
                .get("message")
                .or_else(|| report.get("error"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string());
            not_run_hook(
                &hook,
                &format!("{shown} could not run: {why}"),
                decl.on_not_run,
            )
        }
    }
}

fn blocked(hook: &str, shown: &str, report: &serde_json::Value, fail_on: FailOn) -> ExitCode {
    let blocking = blocking_findings(report, fail_on);
    eprintln!(
        "{hook}: {shown}: {} blocking finding(s); this {} was stopped",
        blocking.len(),
        if hook == "pre-push" { "push" } else { "commit" }
    );
    for line in &blocking {
        eprintln!("  {line}");
    }
    if let Some(next) = report.get("next_action").and_then(|v| v.as_str()) {
        eprintln!("{hook}: suggested: {next}");
    }
    eprintln!("{hook}: the full report: {shown}");
    ExitCode::from(1)
}

fn not_run_hook(hook: &str, why: &str, on_not_run: HookOnNotRun) -> ExitCode {
    let verb = if hook == "pre-push" { "push" } else { "commit" };
    eprintln!(
        "{hook}: urzua did not run ({why}), so the records were NOT checked -- CI will check them"
    );
    match on_not_run {
        HookOnNotRun::Block => {
            eprintln!(
                "{hook}: fix the cause, or {verb} without the check once: git {verb} --no-verify"
            );
            ExitCode::from(NOT_RUN_BLOCK)
        }
        HookOnNotRun::Warn => ExitCode::SUCCESS,
    }
}

/// `rule file:line: message` for each unwaived finding at or above `fail_on`.
fn blocking_findings(report: &serde_json::Value, fail_on: FailOn) -> Vec<String> {
    let Some(findings) = report.get("findings").and_then(|f| f.as_array()) else {
        return Vec::new();
    };
    findings
        .iter()
        .filter(|f| f.get("waived").is_none_or(|w| w.is_null()))
        .filter(|f| match f.get("severity").and_then(|s| s.as_str()) {
            Some("error") => true,
            Some("warning") => fail_on == FailOn::Warning,
            _ => false,
        })
        .map(|f| {
            let s = |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("");
            let at = match f.get("line").and_then(|l| l.as_u64()) {
                Some(l) => format!("{}:{l}", s("file")),
                None => s("file").to_string(),
            };
            format!("{} {at}: {}", s("rule"), s("message"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trailing_exec_or_exit_keeps_the_block_before_it() {
        let hook = "npx lint-staged\nexec node scripts/check.js \"$@\"\n";
        let out = place_block(hook, "B", "pre-commit").unwrap();
        assert_eq!(
            out,
            "npx lint-staged\nB\nexec node scripts/check.js \"$@\"\n"
        );
        let out = place_block("#!/bin/sh\necho a\nexit 0\n", "B", "pre-commit").unwrap();
        assert_eq!(out, "#!/bin/sh\necho a\nB\nexit 0\n");
    }

    #[test]
    fn the_block_goes_after_existing_content_and_is_replaced_in_place() {
        let once = place_block("#!/bin/sh\nmake lint\n", "B1", "pre-commit").unwrap();
        assert_eq!(once, "#!/bin/sh\nmake lint\nB1\n");
        let block = format!("{BEGIN}\nx\n{END}");
        let first = place_block("make lint\n", &block, "pre-commit").unwrap();
        let again = place_block(&first, &block, "pre-commit").unwrap();
        assert_eq!(first, again);
        assert_eq!(remove_blocks(&first).unwrap(), "make lint\n");
    }

    #[test]
    fn a_hook_that_can_leave_early_has_no_insertion_point() {
        for (hook, line) in [
            ("if x; then exec y; fi\n", 0),
            ("f() { exec y; }\nf\n", 0),
            ("trap 'exit 1' EXIT\nx\n", 0),
            ("a\nexit 0\nb\n", 1),
            ("if x; then\n  y\nfi\n", 2),
            ("a || exit 1\n", 0),
        ] {
            assert_eq!(insertion_point(hook).map_err(|e| e.0), Err(line), "{hook}");
        }
    }

    #[test]
    fn an_unconditional_stop_is_found_only_at_top_level() {
        assert_eq!(
            unconditional_stop_before("exit 0\nB\n", 1),
            Some((0, "exit"))
        );
        assert_eq!(
            unconditional_stop_before("if x; then\n  exit 0\nfi\nB\n", 3),
            None
        );
        assert_eq!(unconditional_stop_before("echo exit\nB\n", 1), None);
    }

    #[test]
    fn non_shell_shebangs_are_named() {
        assert_eq!(shell_hook("#!/usr/bin/env node\n"), Err("node".to_string()));
        assert_eq!(
            shell_hook("#!/usr/bin/python3\n"),
            Err("python3".to_string())
        );
        assert!(shell_hook("#!/usr/bin/env bash\n").is_ok());
        assert!(shell_hook("npx lint-staged\n").is_ok());
    }

    #[test]
    fn printed_lines_are_recognised() {
        assert!(invokes_urzua(&shell_line("pre-commit"), None, false));
        assert!(invokes_urzua(
            &foreign_line("node", "pre-commit"),
            None,
            false
        ));
        assert!(invokes_urzua(
            &foreign_line("python3", "pre-push"),
            None,
            false
        ));
        assert!(!invokes_urzua("console.log(\"urzua, check\")", None, false));
    }

    #[test]
    fn the_stanza_follows_the_repos_indentation() {
        assert_eq!(repos_indent("repos:\n-   repo: x\n").unwrap(), "");
        assert_eq!(repos_indent("repos:\n  - repo: x\n").unwrap(), "  ");
        assert!(repos_indent("repos:\n  - repo: x\nci:\n  a: 1\n").is_err());
        assert!(repos_indent("repos: []\n").is_err());
    }

    #[test]
    fn blocking_findings_respect_fail_on_and_waivers() {
        let r = serde_json::json!({"findings": [
            {"rule": "a", "severity": "error", "file": "x.md", "line": 3, "message": "m"},
            {"rule": "b", "severity": "warning", "file": "y.md", "message": "w"},
            {"rule": "c", "severity": "error", "file": "z.md", "message": "z", "waived": "W-1"}
        ]});
        assert_eq!(blocking_findings(&r, FailOn::Error), vec!["a x.md:3: m"]);
        assert_eq!(blocking_findings(&r, FailOn::Warning).len(), 2);
    }
}
