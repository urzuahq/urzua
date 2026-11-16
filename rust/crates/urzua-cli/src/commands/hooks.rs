//! `urzua hooks install|status|uninstall|run` (`ADR-71` amendment of
//! 2026-11-16, `MILE-220`): wires the pre-commit and pre-push git hooks
//! `doctor` asks for, wherever the repository keeps its hooks, and runs the
//! configured check from them.
//!
//! A hook file urzua did not write is extended with a marked block, never
//! replaced; `uninstall` removes only that block, or a file that held nothing
//! else.

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

fn absolute(repo_root: &Path, p: &str) -> PathBuf {
    let p = PathBuf::from(p);
    if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    }
}

/// `path` with `.` and `..` resolved lexically, so a `core.hooksPath` of
/// `../elsewhere` is seen as outside the repository.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Whether urzua can create a file in `dir` (or create `dir`), probed by
/// writing and removing a file: permission bits alone miss read-only mounts.
fn write_problem(dir: &Path) -> Option<String> {
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
    let configured = git(repo_root, &["config", "--get", "core.hooksPath"])?;
    let value = String::from_utf8_lossy(&configured.stdout)
        .trim()
        .to_string();
    if configured.status.success() && !value.is_empty() {
        let dir = normalize(&absolute(repo_root, &value));
        let root = normalize(repo_root);
        let Ok(rel) = dir.strip_prefix(&root) else {
            return Ok(Layout::Files {
                kind: "hooks-path",
                unwritable: Some(format!(
                    "core.hooksPath is {}, outside the repository; urzua did not set it and will not write there",
                    dir.display()
                )),
                dir,
                needs_exec: true,
            });
        };
        if rel.starts_with(".husky") {
            return Ok(Layout::Files {
                kind: "husky",
                unwritable: write_problem(&root.join(".husky")),
                needs_exec: rel == Path::new(".husky"),
                dir: root.join(".husky"),
            });
        }
        return Ok(Layout::Files {
            kind: "hooks-path",
            unwritable: write_problem(&dir),
            dir,
            needs_exec: true,
        });
    }
    let out = git(repo_root, &["rev-parse", "--git-path", "hooks"])?;
    if !out.status.success() {
        return Err(format!(
            "git could not locate the hooks directory: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let git_hooks = absolute(repo_root, String::from_utf8_lossy(&out.stdout).trim());
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
            unwritable: write_problem(&dir),
            dir,
            needs_exec: false,
        });
    }
    Ok(Layout::Files {
        kind: "default",
        unwritable: write_problem(&git_hooks),
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
}

/// One hook's wiring, as `doctor`, `hooks status` and the session-start check
/// all report it.
#[derive(Debug, Clone)]
pub(crate) struct Wiring {
    pub hook: &'static str,
    pub path: PathBuf,
    pub state: HookState,
    pub message: String,
    /// The one command that wires it, when it is not wired.
    pub fix: Option<String>,
}

/// The line a hook urzua will not write must carry to run urzua.
pub(crate) fn shell_line(hook: &str) -> String {
    format!("\"${{URZUA_BIN:-urzua}}\" hooks run {hook} || exit $?")
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
pub(crate) fn wiring(layout: &Layout) -> Vec<Result<Wiring, String>> {
    HOOKS
        .iter()
        .map(|hook| match layout {
            Layout::Files {
                dir,
                unwritable,
                needs_exec,
                ..
            } => file_wiring(hook, &dir.join(hook), *needs_exec, unwritable.as_deref()),
            Layout::Framework { config, git_hooks } => framework_wiring(hook, config, git_hooks),
        })
        .collect()
}

fn file_wiring(
    hook: &'static str,
    path: &Path,
    needs_exec: bool,
    unwritable: Option<&str>,
) -> Result<Wiring, String> {
    let fix = |default: String| match unwritable {
        Some(_) => format!(
            "add `{}` to {} (urzua will not write there)",
            shell_line(hook),
            path.display()
        ),
        None => default,
    };
    let meta = match std::fs::metadata(path) {
        Ok(m) if m.is_file() => Some(m),
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    let Some(meta) = meta else {
        return Ok(Wiring {
            hook,
            path: path.to_path_buf(),
            state: HookState::NotInstalled,
            message: format!(
                "no {hook} hook at {} -- `{INSTALL}` adds one that runs urzua; CI remains the guarantee (ADR-71)",
                path.display()
            ),
            fix: Some(fix(INSTALL.to_string())),
        });
    };
    if needs_exec && !executable(&meta) {
        return Ok(Wiring {
            hook,
            path: path.to_path_buf(),
            state: HookState::InstalledNotExecutable,
            message: format!(
                "{} is not executable, so git ignores it -- make it executable, then run `{INSTALL}`",
                path.display()
            ),
            fix: Some(fix(format!("chmod +x {} && {INSTALL}", path.display()))),
        });
    }
    let content = read_optional(path)?.unwrap_or_default();
    if content.contains(BEGIN) || invokes_urzua(&content, None, false) {
        return Ok(Wiring {
            hook,
            path: path.to_path_buf(),
            state: HookState::InstalledRunningUrzua,
            message: format!("{} runs urzua", path.display()),
            fix: None,
        });
    }
    Ok(Wiring {
        hook,
        path: path.to_path_buf(),
        state: HookState::InstalledNotRunningUrzua,
        message: format!(
            "{} does not run urzua -- `{INSTALL}` adds a marked block that does, after what is there",
            path.display()
        ),
        fix: Some(fix(INSTALL.to_string())),
    })
}

/// A framework hook is wired when its config runs urzua at that stage and the
/// framework's own script is installed for that hook type.
fn framework_wiring(hook: &'static str, config: &Path, git_hooks: &Path) -> Result<Wiring, String> {
    let text = read_optional(config)?.unwrap_or_default();
    let declared = text.contains(BEGIN)
        || text
            .lines()
            .any(|l| l.contains(&format!("hooks run {hook}")) && invokes_urzua(l, None, false));
    let script = git_hooks.join(hook);
    if !declared {
        return Ok(Wiring {
            hook,
            path: config.to_path_buf(),
            state: HookState::InstalledNotRunningUrzua,
            message: format!(
                "{} declares no hook that runs urzua at {hook} -- `{INSTALL} --write-config` adds the stanza",
                config.display()
            ),
            fix: Some(format!("{INSTALL} --write-config")),
        });
    }
    if !script.is_file() {
        return Ok(Wiring {
            hook,
            path: script.clone(),
            state: HookState::NotInstalled,
            message: format!(
                "{} declares urzua at {hook}, but the pre-commit framework is not installed for it (no {}) -- run `pre-commit install --hook-type {hook}`",
                config.display(),
                script.display()
            ),
            fix: Some(format!("pre-commit install --hook-type {hook}")),
        });
    }
    Ok(Wiring {
        hook,
        path: config.to_path_buf(),
        state: HookState::InstalledRunningUrzua,
        message: format!("{} runs urzua at {hook}", config.display()),
        fix: None,
    })
}

/// Whether `content` runs urzua: a command position, not a word in a comment
/// or an `echo`. A line's `#` comment is dropped, and a line that only prints
/// (`echo`, `printf`) runs nothing. The executable is any path ending in
/// `urzua`, followed by a subcommand (`subcommand` when given). With
/// `accept_action`, a `uses:` line naming urzua's reusable gate action counts
/// too. The engine knows no repository's wrapper script or make target, so a
/// repository that wraps the call is seen only through the call itself.
pub(crate) fn invokes_urzua(content: &str, subcommand: Option<&str>, accept_action: bool) -> bool {
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

/// The marked shell block a hook runs urzua from. The binary is found as the
/// harness finds it: `URZUA_BIN`, then `PATH`.
pub(crate) fn shell_block(hook: &str, on_not_run: HookOnNotRun) -> String {
    let verb = if hook == "pre-push" { "push" } else { "commit" };
    let missing_exit = match on_not_run {
        HookOnNotRun::Block => "  exit 1\n",
        HookOnNotRun::Warn => "",
    };
    format!(
        "{BEGIN} (written by `{INSTALL}`; `urzua hooks uninstall` removes it)\n\
urzua_bin=\"${{URZUA_BIN:-}}\"\n\
[ -n \"$urzua_bin\" ] || urzua_bin=\"$(command -v urzua 2>/dev/null || true)\"\n\
if [ -z \"$urzua_bin\" ]; then\n\
\x20 echo \"{hook}: urzua is not installed (not on PATH, URZUA_BIN unset), so this {verb} was not checked\" >&2\n\
\x20 echo \"{hook}: install it: {INSTALL_DOC}\" >&2\n\
\x20 echo \"{hook}: or skip this hook once: git {verb} --no-verify\" >&2\n\
{missing_exit}else\n\
\x20 \"$urzua_bin\" hooks run {hook} || exit $?\n\
fi\n\
{END}"
    )
}

/// The `repo: local` stanza for the pre-commit framework, at `indent`, with
/// its markers.
pub(crate) fn framework_stanza(indent: &str) -> String {
    let mut out = format!("{indent}{BEGIN} (written by `{INSTALL} --write-config`)\n");
    out.push_str(&format!("{indent}- repo: local\n{indent}  hooks:\n"));
    for hook in HOOKS {
        out.push_str(&format!(
            "{indent}    - id: urzua-{hook}\n\
{indent}      name: urzua ({hook})\n\
{indent}      entry: urzua hooks run {hook}\n\
{indent}      language: system\n\
{indent}      pass_filenames: false\n\
{indent}      always_run: true\n\
{indent}      stages: [{hook}]\n"
        ));
    }
    out.push_str(&format!("{indent}{END}"));
    out
}

/// (start, end) of the marked block in `text`, `end` past the end marker's line.
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

/// The shell interpreter a hook's shebang names, or `None` for a non-shell one.
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

/// `content` with `block` set: replaced between its markers, or inserted after
/// the existing content and before a trailing `exec` line, which would
/// otherwise replace the shell before the block ran.
fn place_block(content: &str, block: &str) -> Result<String, String> {
    if let Some((b, e)) = span(content)? {
        return Ok(format!("{}{block}\n{}", &content[..b], &content[e..]));
    }
    let lines: Vec<&str> = content.lines().collect();
    let last = lines
        .iter()
        .rposition(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let mut out = String::new();
    match last.filter(|&i| {
        let l = lines[i].trim_start();
        l == "exec" || l.starts_with("exec ")
    }) {
        Some(i) => {
            for l in &lines[..i] {
                out.push_str(l);
                out.push('\n');
            }
            out.push_str(block);
            out.push('\n');
            for l in &lines[i..] {
                out.push_str(l);
                out.push('\n');
            }
        }
        None => {
            out.push_str(content);
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(block);
            out.push('\n');
        }
    }
    Ok(out)
}

/// `content` without the marked block.
fn remove_block(content: &str) -> Result<String, String> {
    Ok(match span(content)? {
        Some((b, e)) => format!("{}{}", &content[..b], &content[e..]),
        None => content.to_string(),
    })
}

/// One file the command looked at, and what became of it.
#[derive(serde::Serialize)]
struct Item {
    hook: String,
    path: String,
    /// install: `created`, `extended`, `updated`, `unchanged`, `wired`,
    /// `would-...`; status: `ok`, `missing`, `not-wired`, `not-executable`;
    /// uninstall: `removed`, `block-removed`, `absent`, `not-ours`.
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
    let (_, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    emit(&status(&layout))
}

fn status(layout: &Layout) -> HooksReport {
    let mut report = HooksReport::new("status", false, layout);
    for w in wiring(layout) {
        match w {
            Err(m) => report.refuse(m),
            Ok(w) => {
                let state = match w.state {
                    HookState::InstalledRunningUrzua => "ok",
                    HookState::NotInstalled => "missing",
                    HookState::InstalledNotRunningUrzua => "not-wired",
                    HookState::InstalledNotExecutable => "not-executable",
                };
                report.item(w.hook, &w.path, state);
                if let Some(cmd) = w.fix {
                    report.problems.push(w.message);
                    report.next(cmd, "wires the git hooks that do not run urzua");
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
            install_framework(&mut report, config, git_hooks, dry_run, write_config)
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
            install_files(
                &mut report,
                decl,
                dir,
                unwritable.as_deref(),
                *needs_exec,
                dry_run,
            )
        }
    }
    report
}

/// The plan for one hook file: its new content, whether urzua creates it.
struct Write {
    hook: &'static str,
    path: PathBuf,
    content: String,
    state: &'static str,
}

fn install_files(
    report: &mut HooksReport,
    decl: &HooksDecl,
    dir: &Path,
    unwritable: Option<&str>,
    needs_exec: bool,
    dry_run: bool,
) {
    let mut writes: Vec<Write> = Vec::new();
    for hook in HOOKS {
        let path = dir.join(hook);
        let block = shell_block(hook, decl.on_not_run);
        let existing = match read_optional(&path) {
            Ok(c) => c,
            Err(m) => return report.refuse(m),
        };
        let planned = match &existing {
            None => Some((format!("{SHEBANG}{block}\n"), "created")),
            Some(content) => {
                if needs_exec {
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if !executable(&meta) {
                            report.refuse(format!(
                                "{} is not executable, so git ignores it; urzua will not turn on a hook someone left off -- make it executable (`chmod +x {}`) or remove it, then run `{INSTALL}` again",
                                path.display(),
                                path.display()
                            ));
                            continue;
                        }
                    }
                }
                if content.contains(BEGIN) || content.contains(END) {
                    match place_block(content, &block) {
                        Err(m) => {
                            report.refuse(format!("{} {m}", path.display()));
                            continue;
                        }
                        Ok(new) if new == *content => None,
                        Ok(new) => Some((new, "updated")),
                    }
                } else if invokes_urzua(content, None, false) {
                    report.item(hook, &path, "wired");
                    continue;
                } else if let Err(interp) = shell_hook(content) {
                    report.refuse(format!(
                        "{} is a {interp} script, so urzua will not add a shell block to it; add this line to it: {}",
                        path.display(),
                        foreign_line(&interp, hook)
                    ));
                    continue;
                } else {
                    match place_block(content, &block) {
                        Err(m) => {
                            report.refuse(format!("{} {m}", path.display()));
                            continue;
                        }
                        Ok(new) => Some((new, "extended")),
                    }
                }
            }
        };
        match planned {
            None => report.item(hook, &path, "unchanged"),
            Some((content, state)) => {
                if let Some(why) = unwritable {
                    report.refuse(format!(
                        "{why}; add this line to {} to run urzua from it: {}",
                        path.display(),
                        shell_line(hook)
                    ));
                    continue;
                }
                writes.push(Write {
                    hook,
                    path,
                    content,
                    state,
                });
            }
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
    config: &Path,
    git_hooks: &Path,
    dry_run: bool,
    write_config: bool,
) {
    let text = match read_optional(config) {
        Ok(t) => t.unwrap_or_default(),
        Err(m) => return report.refuse(m),
    };
    let wired: Vec<_> = wiring(&Layout::Framework {
        config: config.to_path_buf(),
        git_hooks: git_hooks.to_path_buf(),
    });
    let declared = text.contains(BEGIN)
        || HOOKS
            .iter()
            .all(|h| text.contains(&format!("hooks run {h}")) && invokes_urzua(&text, None, false));
    if declared {
        report.item("pre-commit-framework", config, "wired");
    } else {
        let indent = repos_indent(&text).unwrap_or_else(|_| "  ".to_string());
        report.stanza = Some(framework_stanza(&indent));
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
            return;
        }
        let indent = match repos_indent(&text) {
            Ok(i) => i,
            Err(why) => {
                return report.refuse(format!(
                    "{}: {why}, so urzua will not edit it; add `stanza` under `repos:` by hand",
                    config.display()
                ))
            }
        };
        let mut new = text.clone();
        if !new.ends_with('\n') {
            new.push('\n');
        }
        new.push_str(&framework_stanza(&indent));
        new.push('\n');
        if dry_run {
            report.item("pre-commit-framework", config, "would-extend");
        } else if let Err(e) = std::fs::write(config, new) {
            return report.refuse(format!("could not write {}: {e}", config.display()));
        } else {
            report.item("pre-commit-framework", config, "extended");
        }
    }
    for w in wired.into_iter().flatten() {
        if w.state == HookState::NotInstalled {
            report.notice(NoticeSeverity::Warning, w.message);
        }
    }
    let missing: Vec<&str> = HOOKS
        .iter()
        .copied()
        .filter(|h| !git_hooks.join(h).is_file())
        .collect();
    if !missing.is_empty() {
        let cmd = format!(
            "pre-commit install {}",
            missing
                .iter()
                .map(|h| format!("--hook-type {h}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        report.next(
            cmd,
            "installs the pre-commit framework's own git hooks, which run the stanza",
        );
    }
}

pub fn run_uninstall(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let (_, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    let mut report = HooksReport::new("uninstall", dry_run, &layout);
    let files: Vec<(&str, PathBuf)> = match &layout {
        Layout::Framework { config, .. } => vec![("pre-commit-framework", config.clone())],
        Layout::Files { dir, .. } => HOOKS.iter().map(|h| (*h, dir.join(h))).collect(),
    };
    let framework = matches!(layout, Layout::Framework { .. });
    for (hook, path) in files {
        let content = match read_optional(&path) {
            Ok(Some(c)) => c,
            Ok(None) => {
                report.item(hook, &path, "absent");
                continue;
            }
            Err(m) => {
                report.refuse(m);
                continue;
            }
        };
        if !content.contains(BEGIN) && !content.contains(END) {
            report.item(hook, &path, "not-ours");
            continue;
        }
        let rest = match remove_block(&content) {
            Ok(r) => r,
            Err(m) => {
                report.refuse(format!("{} {m}", path.display()));
                continue;
            }
        };
        let delete = !framework && (rest.trim().is_empty() || rest == SHEBANG);
        let state = if delete { "removed" } else { "block-removed" };
        if dry_run {
            report.item(hook, &path, &would(state));
            continue;
        }
        let done = if delete {
            std::fs::remove_file(&path)
        } else {
            std::fs::write(&path, rest)
        };
        match done {
            Ok(()) => report.item(hook, &path, state),
            Err(e) => report.refuse(format!("could not change {}: {e}", path.display())),
        }
    }
    emit(&report)
}

/// `urzua hooks run <hook>`: what the installed block calls. Runs the
/// configured urzua arguments and prints a summary a person or agent can act
/// on to stderr; its exit is the hook's.
pub fn run_hook(config: Option<PathBuf>, hook: String) -> ExitCode {
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => return not_run_hook(&hook, &e.message, HookOnNotRun::default()),
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
    match out.status.code() {
        Some(0) => {
            eprintln!("{hook}: {shown}: no blocking findings");
            ExitCode::SUCCESS
        }
        Some(1) => {
            let report: serde_json::Value =
                serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
            let blocking = blocking_findings(&report, ws.config.fail_on);
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
        _ => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let report: serde_json::Value =
                serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
            let why = report
                .get("message")
                .or_else(|| report.get("error"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| stderr.trim().to_string());
            not_run_hook(
                &hook,
                &format!("{shown} could not run: {why}"),
                decl.on_not_run,
            )
        }
    }
}

fn not_run_hook(hook: &str, why: &str, on_not_run: HookOnNotRun) -> ExitCode {
    let verb = if hook == "pre-push" { "push" } else { "commit" };
    eprintln!("{hook}: urzua NOT RUN -- {why}");
    eprintln!("{hook}: this {verb} was not checked locally; CI remains the guarantee");
    match on_not_run {
        HookOnNotRun::Block => {
            eprintln!("{hook}: fix the cause, or skip this hook once: git {verb} --no-verify");
            ExitCode::from(1)
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
    fn a_trailing_exec_keeps_the_block_before_it() {
        let hook = "npx lint-staged\nexec node scripts/check.js \"$@\"\n";
        let out = place_block(hook, "B").unwrap();
        assert_eq!(
            out,
            "npx lint-staged\nB\nexec node scripts/check.js \"$@\"\n"
        );
    }

    #[test]
    fn the_block_goes_after_existing_content_and_is_replaced_in_place() {
        let once = place_block("#!/bin/sh\nmake lint\n", "B1").unwrap();
        assert_eq!(once, "#!/bin/sh\nmake lint\nB1\n");
        let block = format!("{BEGIN}\nx\n{END}");
        let first = place_block("make lint\n", &block).unwrap();
        let again = place_block(&first, &block).unwrap();
        assert_eq!(first, again);
        assert_eq!(remove_block(&first).unwrap(), "make lint\n");
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
