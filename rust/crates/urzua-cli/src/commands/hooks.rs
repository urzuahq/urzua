//! `urzua hooks install|status|uninstall|run` (`ADR-71` amendment of
//! 2026-11-16, `MILE-220`): wires the pre-commit and pre-push git hooks
//! `doctor` asks for, wherever the repository keeps its hooks, and runs the
//! configured check from them.
//!
//! A hook file urzua did not write gets a marked block at its top, so urzua
//! runs first and the rest of the hook is untouched: a hook's text cannot be
//! analysed reliably to find where it ends. `uninstall` removes only that
//! block, restoring the bytes. Nothing is written through a symlink or
//! outside the repository and its git directory.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{FailOn, HookManaged, HookOnNotRun, HooksDecl};
use urzua_core::report::FailingAs;
use urzua_core::report::FailureKind;
use urzua_core::report::{CouldNotRun, Failure, NextAction, Notice, NoticeSeverity, Report};

use crate::emit;
use crate::session::Workspace;

pub(crate) const HOOKS: [&str; 2] = ["pre-commit", "pre-push"];
pub(crate) const BEGIN: &str = "# >>> urzua-hooks begin";
pub(crate) const END: &str = "# <<< urzua-hooks end";
pub(crate) const INSTALL: &str = "urzua hooks install";
const INSTALL_ARGV: [&str; 3] = ["urzua", "hooks", "install"];

fn words<const N: usize>(argv: [&str; N]) -> Vec<String> {
    argv.map(String::from).to_vec()
}
const FRAMEWORK_CONFIG: &str = ".pre-commit-config.yaml";
const SHEBANG: &str = "#!/bin/sh\n";
const INSTALL_DOC: &str = "https://github.com/urzuahq/urzua/blob/main/docs/reference/install.md";
/// The first release that contains `hooks run`, named to a user of an older
/// one. The release process keeps it right: a test fails while it is not newer
/// than the workspace version before that release ships, and, once the
/// CHANGELOG has a release mentioning `urzua hooks install`, while it is not
/// that release.
pub(crate) const FIRST_VERSION: &str = "0.4.6";
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
pub(crate) fn resolve(path: &Path) -> PathBuf {
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
pub(crate) fn allowed_roots(repo_root: &Path) -> Vec<PathBuf> {
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
    // A directory not yet created is written by creating it, so its nearest
    // existing ancestor is what must accept a new entry.
    let probed = dir
        .ancestors()
        .find(|a| a.exists())
        .unwrap_or(dir)
        .to_path_buf();
    let probe = probed.join(format!(".urzua-hooks-probe-{}", random_hex()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            None
        }
        Err(e) if probed == dir => Some(format!("{} cannot be written: {e}", dir.display())),
        Err(e) => Some(format!(
            "{} does not exist and cannot be created: {} cannot be written: {e}",
            dir.display(),
            probed.display()
        )),
    }
}

pub(crate) fn detect(repo_root: &Path) -> Result<Layout, String> {
    let roots = allowed_roots(repo_root);
    let configured = git(
        repo_root,
        &["config", "--type=path", "--get", "core.hooksPath"],
    )?;
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
    /// urzua is mentioned outside the managed block: text cannot prove it runs.
    InstalledUnmanagedCall,
    /// The block carries a not-run policy the configuration no longer names.
    InstalledStalePolicy,
    /// The block differs from what `install` writes for this hook.
    InstalledStaleBlock,
    /// More than one marked block.
    InstalledDuplicated,
    /// `managed: declared`, and the hook exists, is executable and mentions
    /// urzua: the repository's assertion, which `hooks verify --run` proves.
    Declared,
    DeclaredMissing,
    DeclaredNotExecutable,
    /// Declared, but the hook does not mention urzua anywhere.
    DeclaredNoCall,
    /// `hooks.path` names a directory, and this clone's `core.hooksPath` is
    /// not pointed at it.
    NotPointed,
    /// `hooks.path` resolves outside the repository or into its git
    /// directory, so urzua will not point the clone at it.
    UnsafeDir,
    /// A hook git would run from that directory is a symlink leaving the
    /// working tree, declared or not.
    UnsafeHook,
}

impl HookState {
    pub(crate) fn is_ok(self) -> bool {
        matches!(self, HookState::InstalledRunningUrzua | HookState::Declared)
    }

    fn status_word(self) -> &'static str {
        match self {
            HookState::InstalledRunningUrzua => "ok",
            HookState::NotInstalled => "missing",
            HookState::InstalledNotRunningUrzua => "not-wired",
            HookState::InstalledNotExecutable => "not-executable",
            HookState::InstalledUnmanagedCall => "unmanaged-call",
            HookState::InstalledStalePolicy => "stale-policy",
            HookState::InstalledStaleBlock => "stale-block",
            HookState::InstalledDuplicated => "duplicated",
            HookState::Declared => "declared",
            HookState::DeclaredMissing => "declared-missing",
            HookState::DeclaredNotExecutable => "declared-not-executable",
            HookState::DeclaredNoCall => "declared-no-call",
            HookState::NotPointed => "not-pointed",
            HookState::UnsafeDir => "unsafe-dir",
            HookState::UnsafeHook => "unsafe-hook",
        }
    }
}

/// One hook's wiring, as `doctor`, `hooks status` and the session-start check
/// all report it.
#[derive(Debug, Clone)]
pub(crate) struct Wiring {
    pub hook: String,
    pub path: PathBuf,
    pub state: HookState,
    pub message: String,
    /// The one runnable command that wires it, when there is one, as its words.
    pub fix: Option<Vec<String>>,
}

/// The one line a hook urzua will not write can carry to run urzua. Like the
/// managed block, it follows the not-run policy when urzua is missing.
pub(crate) fn shell_line(hook: &str, on_not_run: HookOnNotRun) -> String {
    let stop = match on_not_run {
        HookOnNotRun::Block => "; exit 1",
        HookOnNotRun::Warn => "",
    };
    format!(
        "if command -v \"${{URZUA_BIN:-urzua}}\" >/dev/null 2>&1; then \"${{URZUA_BIN:-urzua}}\" hooks run {hook} || exit 1; else echo \"{hook}: urzua did not run (not installed), so the records were NOT checked -- CI will check them\" >&2{stop}; fi"
    )
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

fn hooks_path_value(repo_root: &Path) -> Result<Option<String>, String> {
    let out = git(
        repo_root,
        &["config", "--type=path", "--get", "core.hooksPath"],
    )?;
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((out.status.success() && !value.is_empty()).then_some(value))
}

/// Where the effective `core.hooksPath` comes from: its scope (`local`,
/// `global`, `system`, ...) and the file that sets it, as git names them.
fn hooks_path_origin(repo_root: &Path) -> (String, String) {
    let out = git(
        repo_root,
        &[
            "config",
            "--show-scope",
            "--show-origin",
            "--get",
            "core.hooksPath",
        ],
    );
    let line = out
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut parts = line.splitn(3, '\t');
    let scope = parts.next().unwrap_or("").to_string();
    let origin = parts.next().unwrap_or("").to_string();
    (
        if scope.is_empty() {
            "unknown".to_string()
        } else {
            scope
        },
        if origin.is_empty() {
            "an unknown file".to_string()
        } else {
            origin
        },
    )
}

/// A `core.hooksPath` someone else set: urzua names it and where it is set,
/// and leaves the decision to a person.
fn conflict_message(repo_root: &Path, current: &str, configured: &str) -> String {
    let (scope, origin) = hooks_path_origin(repo_root);
    format!(
        "this clone is not pointed at {configured}, the directory hooks.path names: core.hooksPath is {current} (scope {scope}, set in {origin}), so git does not run the hooks committed in {configured}; urzua does not change a value it did not set -- a person can decide to point this clone at {configured} with `git config --local core.hooksPath {configured}`"
    )
}

/// The directory `hooks.path` names, when this clone's `core.hooksPath` does
/// not point at it, and the value it has instead.
pub(crate) struct Unpointed {
    configured: String,
    dir: PathBuf,
    current: Option<String>,
}

pub(crate) fn unpointed(repo_root: &Path, decl: &HooksDecl) -> Result<Option<Unpointed>, String> {
    let Some(configured) = &decl.path else {
        return Ok(None);
    };
    let dir = repo_root.join(configured);
    let current = hooks_path_value(repo_root)?;
    if current
        .as_deref()
        .is_some_and(|c| resolve(&absolute(repo_root, c)) == resolve(&dir))
    {
        return Ok(None);
    }
    Ok(Some(Unpointed {
        configured: configured.clone(),
        dir,
        current,
    }))
}

/// Why the directory `hooks.path` names cannot be what this clone's
/// `core.hooksPath` points at: it resolves outside the working tree, into the
/// git directory, or git would read the value as another place.
fn hooks_dir_problem(repo_root: &Path, configured: &str) -> Option<String> {
    let dir = repo_root.join(configured);
    let real = resolve(&dir);
    if let Some(where_) = escapes(repo_root, &real) {
        return Some(format!(
            "hooks.path ({configured}) resolves to {}, {where_}",
            real.display()
        ));
    }
    let key = format!("core.hooksPath={configured}");
    let seen = git(
        repo_root,
        &[
            "-c",
            &key,
            "config",
            "--type=path",
            "--get",
            "core.hooksPath",
        ],
    )
    .ok()
    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    match seen {
        Some(v) if resolve(&absolute(repo_root, &v)) == real => None,
        v => Some(format!(
            "git would read core.hooksPath={configured} as {}, not {}",
            v.unwrap_or_default(),
            real.display()
        )),
    }
}

/// Where `real` lies when it is not a committed path of the working tree.
fn escapes(repo_root: &Path, real: &Path) -> Option<&'static str> {
    if !real.starts_with(resolve(repo_root)) {
        return Some("outside the repository");
    }
    for args in [
        ["rev-parse", "--absolute-git-dir"],
        ["rev-parse", "--git-common-dir"],
    ] {
        if let Ok(g) = git_path(repo_root, &args) {
            if real.starts_with(resolve(&g)) {
                return Some("inside the git directory, which is not committed");
            }
        }
    }
    None
}

/// Each hook git would run from the directory `hooks.path` names that is a
/// symlink leaving the working tree, declared or not, with why.
fn unsafe_hooks(repo_root: &Path, configured: &str) -> Vec<(String, String)> {
    let dir = repo_root.join(configured);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let message = match link_escape(repo_root, &path)? {
                LinkEscape::Unresolvable => format!(
                    "{} is a symlink whose chain could not be resolved (more than {LINK_HOPS} links, or a loop)",
                    path.display()
                ),
                LinkEscape::Reaches(real, where_) => {
                    let what = if path.is_dir() { "a directory" } else { "a file" };
                    format!(
                        "{} is a symlink that leads to {}, {where_}, so git would run {what} the repository does not commit",
                        path.display(),
                        real.display()
                    )
                }
            };
            Some((e.file_name().to_string_lossy().into_owned(), message))
        })
        .collect();
    found.sort();
    found
}

const LINK_HOPS: usize = 40;

enum LinkEscape {
    Reaches(PathBuf, &'static str),
    Unresolvable,
}

/// Where a symlink at `path` leads when that is outside the working tree or
/// inside the git directory. An existing target is known exactly from
/// `canonicalize`; a dangling chain is walked as the kernel walks it, so a
/// `..` after a symlinked directory is taken from where that link went.
fn link_escape(repo_root: &Path, path: &Path) -> Option<LinkEscape> {
    let target = std::fs::read_link(path).ok()?;
    if let Ok(real) = std::fs::canonicalize(path) {
        return escapes(repo_root, &real).map(|w| LinkEscape::Reaches(real, w));
    }
    let start = path.parent().and_then(|p| p.canonicalize().ok())?;
    let mut hops = 1;
    match walk_link(repo_root, start, &target, &mut hops) {
        Err(e) => Some(e),
        Ok(end) => escapes(repo_root, &end).map(|w| LinkEscape::Reaches(end, w)),
    }
}

/// Resolves `target` from the canonical directory `cur`, component by
/// component, following each symlink met on the way. Below the first missing
/// component nothing can be a link, so the rest is appended as text.
fn walk_link(
    repo_root: &Path,
    mut cur: PathBuf,
    target: &Path,
    hops: &mut usize,
) -> Result<PathBuf, LinkEscape> {
    use std::path::Component;
    let parts: Vec<Component> = target.components().collect();
    for (i, c) in parts.iter().enumerate() {
        match c {
            Component::Prefix(_) | Component::CurDir => {}
            Component::RootDir => cur = PathBuf::from("/"),
            Component::ParentDir => {
                cur.pop();
            }
            Component::Normal(name) => {
                let next = cur.join(name);
                match std::fs::symlink_metadata(&next) {
                    Ok(m) if m.file_type().is_symlink() => {
                        *hops += 1;
                        if *hops > LINK_HOPS {
                            return Err(LinkEscape::Unresolvable);
                        }
                        let t = std::fs::read_link(&next).map_err(|_| LinkEscape::Unresolvable)?;
                        cur = walk_link(repo_root, cur, &t, hops)?;
                        if let Some(w) = escapes(repo_root, &cur) {
                            return Err(LinkEscape::Reaches(cur, w));
                        }
                        if !cur.exists() {
                            return Ok(append_rest(cur, &parts[i + 1..]));
                        }
                    }
                    Ok(_) => cur = next.canonicalize().unwrap_or(next),
                    Err(_) => return Ok(append_rest(next, &parts[i + 1..])),
                }
            }
        }
    }
    Ok(cur)
}

fn append_rest(mut path: PathBuf, rest: &[std::path::Component]) -> PathBuf {
    for c in rest {
        match c {
            std::path::Component::ParentDir => {
                path.pop();
            }
            std::path::Component::Normal(n) => path.push(n),
            _ => {}
        }
    }
    path
}

/// `urzua hooks uninstall`, when running it would unset this clone's
/// `core.hooksPath`.
fn uninstall_fix(repo_root: &Path) -> Option<Vec<String>> {
    set_in_own_config(repo_root).then(|| words(["urzua", "hooks", "uninstall"]))
}

fn set_in_own_config(repo_root: &Path) -> bool {
    let (_, origin) = hooks_path_origin(repo_root);
    let own = git_path(repo_root, &["rev-parse", "--git-path", "config"]).ok();
    origin
        .strip_prefix("file:")
        .zip(own)
        .is_some_and(|(o, own)| resolve(&absolute(repo_root, o)) == resolve(&own))
}

/// Every hook's wiring as the reports show it: when the clone is not pointed
/// at the directory `hooks.path` names, that, since git runs none of the
/// hooks committed there; otherwise the wiring in `layout`.
pub(crate) fn report_wiring(
    repo_root: &Path,
    layout: &Layout,
    decl: &HooksDecl,
) -> Vec<Result<Wiring, String>> {
    let guarded = unsafe_wiring(repo_root, decl);
    if guarded.iter().any(|w| w.state == HookState::UnsafeDir) {
        return guarded.into_iter().map(Ok).collect();
    }
    let mut all = pointer_wiring(repo_root, layout, decl);
    for w in guarded {
        match HOOKS.iter().position(|h| *h == w.hook) {
            Some(at) => all[at] = Ok(w),
            None => all.push(Ok(w)),
        }
    }
    all
}

/// What makes the directory `hooks.path` names unsafe for git to run hooks
/// from: the directory itself (`unsafe-dir`, one per hook) or each entry that
/// links out (`unsafe-hook`). Every report and every install path reads this
/// one list, so none can miss a guard the others apply.
pub(crate) fn unsafe_wiring(repo_root: &Path, decl: &HooksDecl) -> Vec<Wiring> {
    let Some(p) = &decl.path else {
        return Vec::new();
    };
    let pointed = matches!(unpointed(repo_root, decl), Ok(None));
    if let Some(why) = hooks_dir_problem(repo_root, p) {
        let (message, fix) = if pointed {
            (
                format!("{why}; this clone is pointed at {p}, so git runs hooks from there -- `urzua hooks uninstall` stops that"),
                uninstall_fix(repo_root),
            )
        } else {
            (
                format!("{why}; urzua will not point this clone at it -- move the hooks into a directory inside the working tree and name it in hooks.path"),
                None,
            )
        };
        return every_hook(&repo_root.join(p), HookState::UnsafeDir, &message, fix)
            .into_iter()
            .flatten()
            .collect();
    }
    let mut out = Vec::new();
    for (hook, why) in unsafe_hooks(repo_root, p) {
        let (message, fix) = if pointed {
            (
                format!("{why}; this clone is pointed at {p}, so git runs it"),
                uninstall_fix(repo_root),
            )
        } else {
            (
                format!("{why}; `{INSTALL}` refuses to point this clone at {p}"),
                None,
            )
        };
        out.push(wired(
            &hook,
            &repo_root.join(p).join(&hook),
            HookState::UnsafeHook,
            message,
            fix,
        ));
    }
    out
}

fn pointer_wiring(
    repo_root: &Path,
    layout: &Layout,
    decl: &HooksDecl,
) -> Vec<Result<Wiring, String>> {
    let off = match unpointed(repo_root, decl) {
        Ok(None) => return wiring(layout, decl),
        Ok(Some(u)) => u,
        Err(m) => return HOOKS.iter().map(|_| Err(m.clone())).collect(),
    };
    let p = &off.configured;
    let (message, fix) = match &off.current {
        None => match install_refusal(repo_root, &off, decl) {
            None => (
                format!("this clone is not pointed at {p}, the directory hooks.path names: core.hooksPath is not set, so git does not run the hooks committed there -- `{INSTALL}` points this clone at {p}"),
                Some(words(INSTALL_ARGV)),
            ),
            Some(why) => (
                format!("this clone is not pointed at {p}, the directory hooks.path names, and `{INSTALL}` would refuse: {why}"),
                None,
            ),
        },
        Some(c) => (conflict_message(repo_root, c, p), None),
    };
    every_hook(&off.dir, HookState::NotPointed, &message, fix)
}

fn every_hook(
    dir: &Path,
    state: HookState,
    message: &str,
    fix: Option<Vec<String>>,
) -> Vec<Result<Wiring, String>> {
    HOOKS
        .iter()
        .map(|hook| {
            Ok(wired(
                hook,
                &dir.join(hook),
                state,
                message.to_string(),
                fix.clone(),
            ))
        })
        .collect()
}

/// Why `install` would refuse to point this clone at `hooks.path` and wire
/// the hooks there, found by running every guard without writing anything.
fn install_refusal(repo_root: &Path, off: &Unpointed, decl: &HooksDecl) -> Option<String> {
    let (_, report) = planned(repo_root, off, decl);
    report
        .problems
        .into_iter()
        .next()
        .filter(|_| report.exit == 2)
}

/// The layout `hooks.path` gives, and the dry run of the install there.
fn planned(repo_root: &Path, off: &Unpointed, decl: &HooksDecl) -> (Layout, HooksReport) {
    let layout = Layout::Files {
        kind: "hooks-path",
        unwritable: write_problem(&off.dir, &allowed_roots(repo_root)),
        dir: off.dir.clone(),
        needs_exec: true,
    };
    let mut report = HooksReport::new("install", true, &layout);
    if refuse_unsafe(&mut report, repo_root, decl) {
        return (layout, report);
    }
    if let Layout::Files {
        dir,
        unwritable,
        needs_exec,
        ..
    } = &layout
    {
        let target = Target {
            dir,
            unwritable: unwritable.as_deref(),
            needs_exec: *needs_exec,
        };
        install_files(&mut report, decl, &target, true);
    }
    (layout, report)
}

/// Every hook's wiring in `layout`, or why it cannot be read.
pub(crate) fn wiring(layout: &Layout, decl: &HooksDecl) -> Vec<Result<Wiring, String>> {
    HOOKS
        .iter()
        .map(|hook| match layout {
            Layout::Files {
                kind,
                dir,
                unwritable,
                needs_exec,
            } => match decl.managed(hook) {
                HookManaged::Declared => {
                    declared_wiring(hook, &dir.join(hook), *needs_exec, *kind == "default")
                }
                HookManaged::Block => file_wiring(
                    hook,
                    &dir.join(hook),
                    *needs_exec,
                    unwritable.as_deref(),
                    decl,
                ),
            },
            Layout::Framework { config, git_hooks } => {
                framework_wiring(hook, config, git_hooks, decl)
            }
        })
        .collect()
}

fn wired(
    hook: &str,
    path: &Path,
    state: HookState,
    message: String,
    fix: Option<Vec<String>>,
) -> Wiring {
    Wiring {
        hook: hook.to_string(),
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
) -> (String, Option<Vec<String>>) {
    match unwritable {
        Some(why) => (
            format!(
                "{message}; {why}, so add `{}` to {}, or declare it (hooks.{hook}.managed: declared) and prove it with `urzua hooks verify --run`",
                shell_line(hook, HookOnNotRun::default()),
                path.display()
            ),
            None,
        ),
        None => (message, Some(words(INSTALL_ARGV))),
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
            Some(vec![
                "chmod".to_string(),
                "+x".to_string(),
                path.to_string_lossy().into_owned(),
            ]),
        ));
    }
    let content = read_optional(path)?.unwrap_or_default();
    let (state, message) = content_state(hook, path, &content, decl);
    let fix = match state {
        HookState::InstalledRunningUrzua => None,
        HookState::InstalledDuplicated => Some(words(INSTALL_ARGV)),
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

/// A declared hook: the repository maintains it and asserts it runs urzua.
/// Only a plainly false declaration is reported; the assertion itself is
/// proven by running the hook (`hooks verify --run`), never by its text.
fn declared_wiring(
    hook: &'static str,
    path: &Path,
    needs_exec: bool,
    git_default: bool,
) -> Result<Wiring, String> {
    let shown = path.display();
    let key = format!("hooks.{hook}.managed: declared");
    let meta = match std::fs::metadata(path) {
        Ok(m) if m.is_file() => m,
        Ok(_) | Err(_) if git_default => {
            return Ok(wired(
                hook,
                path,
                HookState::DeclaredMissing,
                format!("{key}, but there is no hook in this clone's own hooks directory, where git looks while core.hooksPath is not set -- if the repository commits its hooks in a directory, name it in hooks.path and run `{INSTALL}`; otherwise remove the declaration so `{INSTALL}` manages it"),
                None,
            ))
        }
        Ok(_) | Err(_) => {
            return Ok(wired(
                hook,
                path,
                HookState::DeclaredMissing,
                format!("{key}, but there is no hook at {shown}: the declaration is false -- write the hook, or remove the declaration so `{INSTALL}` manages it"),
                None,
            ))
        }
    };
    if needs_exec && !executable(&meta) {
        return Ok(wired(
            hook,
            path,
            HookState::DeclaredNotExecutable,
            format!("{key}, but {shown} is not executable, so git ignores it"),
            Some(vec![
                "chmod".to_string(),
                "+x".to_string(),
                path.to_string_lossy().into_owned(),
            ]),
        ));
    }
    let content = read_optional(path)?.unwrap_or_default();
    if !mentions_urzua(&content) {
        return Ok(wired(
            hook,
            path,
            HookState::DeclaredNoCall,
            format!("{key}, but {shown} does not mention urzua anywhere: the declaration is false -- add the call, or remove the declaration so `{INSTALL}` manages it"),
            None,
        ));
    }
    let leftover = if content.contains(BEGIN) {
        "; it still holds urzua's managed block, which `urzua hooks uninstall` removes"
    } else {
        ""
    };
    Ok(wired(
        hook,
        path,
        HookState::Declared,
        format!("{shown} is declared ({key}) and mentions urzua; this is the repository's assertion -- `urzua hooks verify --run`, which executes the repository's hooks, proves it by running the hook{leftover}"),
        None,
    ))
}

/// Whether a line other than a comment mentions urzua.
fn mentions_urzua(content: &str) -> bool {
    content
        .lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains("urzua"))
}

/// What a hook file's content says about its wiring. From text alone only the
/// managed block, exactly as `install` writes it for this hook, is `ok`.
fn content_state(hook: &str, path: &Path, content: &str, decl: &HooksDecl) -> (HookState, String) {
    let shown = path.display();
    let blocks = content.matches(BEGIN).count();
    if blocks > 1 {
        return (
            HookState::InstalledDuplicated,
            format!("{shown} has {blocks} urzua blocks -- run `{INSTALL}`, which writes one"),
        );
    }
    if blocks == 1 {
        let marker = Marker::read(content);
        if marker.hook != hook {
            return (
                HookState::InstalledStaleBlock,
                format!("{shown} holds the urzua block written for {}, not {hook} -- run `{INSTALL}` to rewrite it", marker.hook),
            );
        }
        let expected = remove_blocks(content)
            .map(|rest| insert_block(&rest, marker.created, decl.on_not_run, hook));
        if expected.as_deref() == Ok(content) {
            return (
                HookState::InstalledRunningUrzua,
                format!("{shown} runs urzua first"),
            );
        }
        if marker.policy != decl.on_not_run.as_str() {
            return (
                HookState::InstalledStalePolicy,
                format!(
                    "{shown} was written with on_not_run: {}, but the configuration says {} -- run `{INSTALL}` to rewrite it",
                    marker.policy,
                    decl.on_not_run.as_str()
                ),
            );
        }
        return (
            HookState::InstalledStaleBlock,
            format!("{shown} has an urzua block that differs from what `{INSTALL}` writes, or is not at the top -- run `{INSTALL}` to rewrite it"),
        );
    }
    if mentions_urzua(content) {
        return (
            HookState::InstalledUnmanagedCall,
            format!("{shown} mentions urzua outside a managed block, and reading a hook cannot prove a line runs -- run `{INSTALL}` to add the managed block at the top, or, for a hook the repository maintains itself, declare it (hooks.{hook}.managed: declared) and prove it with `urzua hooks verify --run`"),
        );
    }
    (
        HookState::InstalledNotRunningUrzua,
        format!("{shown} does not run urzua -- `{INSTALL}` adds a marked block at its top"),
    )
}

/// What a block's begin marker records.
struct Marker {
    policy: String,
    created: bool,
    hook: String,
}

impl Marker {
    fn read(content: &str) -> Marker {
        let line = content.lines().find(|l| l.contains(BEGIN)).unwrap_or("");
        let field = |name: &str| {
            line.split(&format!("{name}: "))
                .nth(1)
                .and_then(|r| r.split([';', ')']).next())
                .unwrap_or("")
                .to_string()
        };
        Marker {
            policy: field("on_not_run"),
            created: field("created") == "yes",
            hook: field("hook"),
        }
    }
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
            Some(words(["urzua", "hooks", "install", "--write-config"])),
        ));
    }
    let baked = Marker::read(&text).policy;
    if text.contains(BEGIN) && baked != decl.on_not_run.as_str() {
        return Ok(wired(
            hook,
            config,
            HookState::InstalledStalePolicy,
            format!(
                "{} carries the urzua stanza written with on_not_run: {baked}, but the configuration says {} -- run `urzua hooks uninstall`, and the next run names `{INSTALL} --write-config`",
                config.display(),
                decl.on_not_run.as_str()
            ),
            Some(words(["urzua", "hooks", "uninstall"])),
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
            Some(words(["pre-commit", "install", "--hook-type", hook])),
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

/// The first command of a script: after the shebang, blank and comment lines
/// (`#`, and `//` for a script in another language), and a husky v8 source
/// line.
fn first_command(content: &str) -> Option<&str> {
    command_start(content).1
}

/// (byte offset where the block goes, the first command's line). The block
/// goes after the shebang and leading comments, or right after a husky v8
/// source line wherever it is, so it runs once, inside husky's re-run.
fn command_start(content: &str) -> (usize, Option<&str>) {
    let mut at = 0;
    let mut first: Option<(usize, &str)> = None;
    for (i, raw) in content.split_inclusive('\n').enumerate() {
        let line = raw.trim_end_matches(['\n', '\r']);
        let t = line.trim();
        if is_husky_source(t) {
            return (at + raw.len(), first.map(|f| f.1).or(Some(line)));
        }
        let skip = (i == 0 && t.starts_with("#!"))
            || t.is_empty()
            || (t.starts_with('#') && !t.starts_with("#!"))
            || t.starts_with("//");
        if !skip && first.is_none() {
            first = Some((at, line));
        }
        at += raw.len();
    }
    match first {
        Some((offset, line)) => (offset, Some(line)),
        None => (at, None),
    }
}

/// husky v8's `. "$(dirname -- "$0")/_/husky.sh"` and its variants.
fn is_husky_source(t: &str) -> bool {
    (t.starts_with(". ") || t.starts_with("source ")) && t.contains("husky.sh")
}

/// The marked shell block a hook runs urzua from. The binary is found as the
/// harness finds it: `URZUA_BIN`, then `PATH`. Correct under `set -e`,
/// `set -u` and plain POSIX sh: every status is captured with `||`, every
/// name is prefixed `urzua_` and unset at the end. `hooks run` reads the
/// not-run policy from the configuration; the block passes its own for when
/// the configuration cannot be read, and applies it itself when urzua is
/// missing or has no `hooks run`.
pub(crate) fn shell_block(hook: &str, on_not_run: HookOnNotRun, created: bool, nl: &str) -> String {
    let verb = if hook == "pre-push" { "push" } else { "commit" };
    let policy = on_not_run.as_str();
    let stop = match on_not_run {
        HookOnNotRun::Block => format!(
            "echo \"{hook}: to {verb} without the check once: git {verb} --no-verify\" >&2; exit 1"
        ),
        HookOnNotRun::Warn => ":".to_string(),
    };
    let created = if created { "yes" } else { "no" };
    let lines = [
        format!("{BEGIN} (hook: {hook}; on_not_run: {policy}; created: {created}; written by `{INSTALL}`; `urzua hooks uninstall` removes it)"),
        "urzua_bin=\"${URZUA_BIN:-}\"".to_string(),
        "if [ -z \"$urzua_bin\" ]; then urzua_bin=\"$(command -v urzua 2>/dev/null)\" || urzua_bin=\"\"; fi".to_string(),
        "urzua_code=0".to_string(),
        "if [ -z \"$urzua_bin\" ]; then urzua_code=127; urzua_out=\"\"; else".to_string(),
        format!("  urzua_out=\"$(URZUA_HOOKS_ON_NOT_RUN={policy} \"$urzua_bin\" hooks run {hook} 2>&1 >/dev/null)\" || urzua_code=$?"),
        "fi".to_string(),
        "case \"$urzua_code\" in".to_string(),
        format!("  0|1|{NOT_RUN_BLOCK}) if [ -n \"$urzua_out\" ]; then printf '%s\\n' \"$urzua_out\" >&2; fi ;;"),
        format!("  126|127) echo \"{hook}: urzua did not run (not installed, or not executable: URZUA_BIN, then PATH), so the records were NOT checked; install it: {INSTALL_DOC} -- CI will check them\" >&2"),
        format!("    {stop} ;;"),
        format!("  *) echo \"{hook}: urzua did not run: $urzua_bin has no \\`hooks run\\` (exit $urzua_code); upgrade urzua to {FIRST_VERSION} or later. The records were NOT checked -- CI will check them\" >&2"),
        format!("    {stop} ;;"),
        "esac".to_string(),
        format!("case \"$urzua_code\" in 1|{NOT_RUN_BLOCK}) exit 1 ;; esac"),
        "unset urzua_bin urzua_out urzua_code".to_string(),
        END.to_string(),
    ];
    lines.join(nl)
}

/// `content` with the block at its top (after the shebang, leading comments
/// and a husky v8 source line), in the file's own line endings. A file that
/// ends without a newline at that point gets one before the block, recorded
/// so `remove_blocks` takes it out again.
fn insert_block(content: &str, created: bool, on_not_run: HookOnNotRun, hook: &str) -> String {
    let nl = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let (at, _) = command_start(content);
    let mut block = shell_block(hook, on_not_run, created, nl);
    let joined = at == content.len() && !content.is_empty() && !content.ends_with('\n');
    if joined {
        block = format!(
            "{nl}{}",
            block.replacen(" (hook:", " (joined: yes; hook:", 1)
        );
    }
    format!("{}{block}{nl}{}", &content[..at], &content[at..])
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

/// (start, end) of the first marked block in `text`: from the start of the
/// begin marker's line (or the newline before it, when the block recorded
/// adding one) to past the end marker's line ending.
fn span(text: &str) -> Result<Option<(usize, usize)>, String> {
    match (text.find(BEGIN), text.find(END)) {
        (None, None) => Ok(None),
        (Some(b), Some(e)) if b < e => {
            let mut start = text[..b].rfind('\n').map_or(0, |i| i + 1);
            let line_end = text[b..].find('\n').map_or(text.len(), |i| b + i);
            if text[b..line_end].contains("joined: yes") && start > 0 {
                start -= 1;
                if text[..start].ends_with('\r') {
                    start -= 1;
                }
            }
            let mut end = e + END.len();
            if text[end..].starts_with("\r\n") {
                end += 2;
            } else if text[end..].starts_with('\n') {
                end += 1;
            }
            Ok(Some((start, end)))
        }
        _ => Err(format!(
            "has a '{BEGIN}' without a later '{END}' (or the reverse); fix the markers by hand, urzua will not guess"
        )),
    }
}

/// `Err` with the interpreter a hook's shebang names, for a non-shell one;
/// a hook with no shebang is shell unless its first command reads as
/// JavaScript or Python.
fn shell_hook(content: &str) -> Result<(), String> {
    let Some(first) = content.lines().next().filter(|l| l.starts_with("#!")) else {
        let foreign = first_command(content).is_some_and(|l| {
            [
                "import ",
                "from ",
                "const ",
                "let ",
                "var ",
                "require(",
                "'use strict'",
                "\"use strict\"",
                "def ",
                "console.",
            ]
            .iter()
            .any(|p| l.trim_start().starts_with(p))
        });
        return if foreign {
            Err("non-shell".to_string())
        } else {
            Ok(())
        };
    };
    let words: Vec<&str> = first[2..].split_whitespace().collect();
    let interp = match words.as_slice() {
        // `env` takes flags (`-S`, `-i`) and `NAME=value` before the program.
        [env, rest @ ..] if env.ends_with("/env") || *env == "env" => {
            match rest
                .iter()
                .find(|w| !w.starts_with('-') && !w.contains('='))
            {
                Some(name) => name.rsplit('/').next().unwrap_or(name),
                None => return Ok(()),
            }
        }
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
            error_kind: None,
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

    fn next(&mut self, argv: Vec<String>, reason: &str) {
        if self.next.is_none() {
            self.next = Some(NextAction::runs_as_printed(argv, reason));
        }
    }

    /// Nothing was written because the run refused.
    fn refuse(&mut self, why: String) {
        self.status = "not-run";
        self.error_kind = Some(FailureKind::Environment);
        self.exit = 2;
        self.problems.push(why);
    }
}

fn not_run(failure: Failure) -> ExitCode {
    emit(&CouldNotRun::from_failure(failure))
}

fn open(config: Option<PathBuf>) -> Result<(Workspace, Layout), Failure> {
    let ws = Workspace::open(&[PathBuf::from(".")], config)?;
    let layout = detect(&ws.repo_root).failing(urzua_core::report::FailureKind::Environment)?;
    Ok((ws, layout))
}

pub fn run_status(config: Option<PathBuf>) -> ExitCode {
    let (ws, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    emit(&status(&ws.repo_root, &layout, &ws.config.hooks))
}

fn status(repo_root: &Path, layout: &Layout, decl: &HooksDecl) -> HooksReport {
    let mut report = HooksReport::new("status", false, layout);
    for w in report_wiring(repo_root, layout, decl) {
        match w {
            Err(m) => report.refuse(m),
            Ok(w) => {
                report.item(&w.hook, &w.path, w.state.status_word());
                if !w.state.is_ok() {
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
    let decl = &ws.config.hooks;
    let off = match unpointed(&ws.repo_root, decl) {
        Ok(o) => o,
        Err(m) => return not_run(Failure::environment(m)),
    };
    let Some(off) = off else {
        let mut guarded = HooksReport::new("install", dry_run, &layout);
        if refuse_unsafe(&mut guarded, &ws.repo_root, decl) {
            return emit(&guarded);
        }
        return emit(&install(&layout, decl, dry_run, write_config));
    };
    emit(&point_and_install(&ws.repo_root, &off, decl, dry_run))
}

/// Refuses, listing each unsafe hook with the state and fix `status` gives it,
/// when `unsafe_wiring` finds anything; `true` when it did.
fn refuse_unsafe(report: &mut HooksReport, repo_root: &Path, decl: &HooksDecl) -> bool {
    let guarded = unsafe_wiring(repo_root, decl);
    for w in &guarded {
        report.item(&w.hook, &w.path, w.state.status_word());
        if !report.problems.contains(&w.message) {
            report.refuse(w.message.clone());
        }
        if let Some(fix) = &w.fix {
            report.next(
                fix.clone(),
                "stops git running hooks from a place the repository does not commit",
            );
        }
    }
    !guarded.is_empty()
}

/// Points this clone's `core.hooksPath` at the directory `hooks.path` names,
/// then wires the hooks there. A value someone else set is never replaced.
fn point_and_install(
    repo_root: &Path,
    off: &Unpointed,
    decl: &HooksDecl,
    dry_run: bool,
) -> HooksReport {
    let p = &off.configured;
    if let Some(c) = &off.current {
        let (layout, _) = planned(repo_root, off, decl);
        let mut report = HooksReport::new("install", dry_run, &layout);
        report.refuse(conflict_message(repo_root, c, p));
        return report;
    }
    // Every guard runs before anything is written: a refusal leaves both
    // core.hooksPath and the hook files as they were.
    let (layout, mut plan) = planned(repo_root, off, decl);
    plan.dry_run = dry_run;
    if plan.exit == 2 {
        plan.problems = plan
            .problems
            .into_iter()
            .map(|m| format!("{m}; nothing was written and core.hooksPath was not set to {p}"))
            .collect();
        if !dry_run {
            for i in plan
                .items
                .iter_mut()
                .filter(|i| i.state.starts_with("would-"))
            {
                i.state = "not-written".to_string();
            }
        }
        return plan;
    }
    if dry_run {
        plan.notice(NoticeSeverity::Info, shared_note(p));
        plan.items.insert(
            0,
            Item {
                hook: "core.hooksPath".to_string(),
                path: off.dir.display().to_string(),
                state: "would-point".to_string(),
            },
        );
        declared_problems(&mut plan, &layout, decl);
        return plan;
    }
    let mut report = HooksReport::new("install", false, &layout);
    match git(repo_root, &["config", "--local", "core.hooksPath", p]) {
        Ok(o) if o.status.success() => {
            report.item("core.hooksPath", &off.dir, "pointed");
            report.notice(NoticeSeverity::Info, shared_note(p));
        }
        Ok(o) => {
            report.refuse(format!(
                "git config core.hooksPath {p} failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ));
            return report;
        }
        Err(m) => {
            report.refuse(m);
            return report;
        }
    }
    if let Layout::Files {
        dir,
        unwritable,
        needs_exec,
        ..
    } = &layout
    {
        let target = Target {
            dir,
            unwritable: unwritable.as_deref(),
            needs_exec: *needs_exec,
        };
        install_files(&mut report, decl, &target, false);
        declared_problems(&mut report, &layout, decl);
    }
    report
}

fn shared_note(p: &str) -> String {
    format!("core.hooksPath is set to the relative value {p} in this clone's own configuration, which every worktree of the clone shares; each resolves it against its own working tree")
}

/// A declared hook the install cannot write is reported as `status` reports it,
/// so install never says `ok` while the declaration is false.
fn declared_problems(report: &mut HooksReport, layout: &Layout, decl: &HooksDecl) {
    for w in wiring(layout, decl).into_iter().flatten() {
        if decl.managed(&w.hook) == HookManaged::Declared && !w.state.is_ok() {
            let path = w.path.display().to_string();
            report.items.retain(|i| i.hook != w.hook || i.path != path);
            report.item(&w.hook, &w.path, w.state.status_word());
            report.problems.push(w.message);
            if report.exit == 0 {
                report.status = "drift";
                report.exit = 1;
            }
        }
    }
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
            install_files(&mut report, decl, &target, dry_run);
            declared_problems(&mut report, layout, decl);
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

/// Why urzua will not write `path` itself: it is a symlink, wherever it
/// points, or a hard link, which a rename would silently split from its other
/// names.
fn link_problem(hook: &str, path: &Path, on_not_run: HookOnNotRun) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    let line = shell_line(hook, on_not_run);
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(path)
            .map(|t| t.display().to_string())
            .unwrap_or_else(|_| "an unreadable target".to_string());
        return Some(format!(
            "{} is a symlink to {target}; urzua never writes through a symlink, so add this line to the hook it points at: {line}",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.is_file() && meta.nlink() > 1 {
            return Some(format!(
                "{} is a hard link ({} names); rewriting it would split it from its other names, so add this line to it: {line} -- or, if the repository maintains this hook itself, declare it (hooks.{hook}.managed: declared)",
                path.display(),
                meta.nlink()
            ));
        }
    }
    None
}

/// What to write for one hook, `Ok(None)` when nothing changes, `Err` the
/// refusal.
fn plan_hook(
    report: &mut HooksReport,
    hook: &'static str,
    path: &Path,
    decl: &HooksDecl,
    target: &Target,
) -> Result<Option<(String, &'static str)>, String> {
    if decl.managed(hook) == HookManaged::Declared {
        report.item(hook, path, "declared");
        report.notice(
            NoticeSeverity::Info,
            format!("hooks.{hook}.managed is declared: the repository maintains {} itself, so urzua writes nothing to it; `urzua hooks verify --run` proves it runs urzua", path.display()),
        );
        return Ok(None);
    }
    if let Some(why) = link_problem(hook, path, decl.on_not_run) {
        return Err(why);
    }
    let Some(content) = read_optional(path)? else {
        let new = insert_block(SHEBANG, true, decl.on_not_run, hook);
        return Ok(Some((new, "created")));
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
        let created = Marker::read(&content).created;
        let rest = remove_blocks(&content).map_err(|m| format!("{} {m}", path.display()))?;
        let new = insert_block(&rest, created, decl.on_not_run, hook);
        return Ok((new != content).then_some((new, "updated")));
    }
    if let Err(interp) = shell_hook(&content) {
        return Err(format!(
            "{} is a {interp} script, so urzua will not add a shell block to it; add this line to it: {}",
            path.display(),
            foreign_line(&interp, hook)
        ));
    }
    Ok(Some((
        insert_block(&content, false, decl.on_not_run, hook),
        "extended",
    )))
}

fn install_files(report: &mut HooksReport, decl: &HooksDecl, target: &Target, dry_run: bool) {
    let mut writes: Vec<Write> = Vec::new();
    for hook in HOOKS {
        let path = target.dir.join(hook);
        let before = report.items.len();
        match plan_hook(report, hook, &path, decl, target) {
            Err(why) => report.refuse(why),
            Ok(None) if report.items.len() == before => report.item(hook, &path, "unchanged"),
            Ok(None) => {}
            Ok(Some((content, state))) => match target.unwritable {
                Some(why) => report.refuse(format!(
                    "{why}; add this line to {} to run urzua from it: {}",
                    path.display(),
                    shell_line(hook, decl.on_not_run)
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
        if refused && !dry_run {
            report.item(w.hook, &w.path, "not-written");
            continue;
        }
        if dry_run {
            report.item(w.hook, &w.path, &would(w.state));
            continue;
        }
        if let Err(e) = write_atomic(&w.path, &w.content) {
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

/// Writes `content` to a new temporary file beside `path`, under an
/// unpredictable name created exclusively (so a symlink planted at that name
/// is never followed), and renames it over `path`, keeping `path`'s mode (a
/// new file is executable). The owner cannot be kept without privilege: the
/// file belongs to whoever runs the command. The rename is refused when
/// `path` is no longer a regular file with one name. The temporary file is
/// removed on every failure.
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let existing = std::fs::symlink_metadata(path).ok();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("hook");
    let tmp = dir.join(format!(".{name}.urzua-{}", random_hex()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    let written = (|| {
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = existing
                .as_ref()
                .map_or(0o755, |m| m.permissions().mode() & 0o7777);
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
        }
        let now = std::fs::symlink_metadata(path).ok();
        if now
            .as_ref()
            .is_some_and(|m| !m.file_type().is_file() || nlink(m) > 1)
        {
            return Err(std::io::Error::other(
                "it is no longer a regular file with one name; nothing was written",
            ));
        }
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

fn nlink(meta: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.nlink()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        1
    }
}

/// 32 hex characters from the system's random source, or, where there is
/// none, from the standard library's per-process random hasher keys.
pub(crate) fn random_hex() -> String {
    use std::io::Read as _;
    let mut bytes = [0u8; 16];
    let random = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes));
    if random.is_err() {
        use std::hash::{BuildHasher, Hasher};
        // Each RandomState carries keys the standard library seeds from the OS.
        let half = || {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u32(std::process::id());
            h.finish()
        };
        bytes = (u128::from(half()) << 64 | u128::from(half())).to_le_bytes();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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
        .flat_map(|h| ["--hook-type".to_string(), h.to_string()])
        .collect();
    if !missing.is_empty() {
        report.next(
            std::iter::once("pre-commit".to_string())
                .chain(std::iter::once("install".to_string()))
                .chain(missing)
                .collect(),
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
            words(["urzua", "hooks", "install", "--write-config"]),
            "appends the urzua stanza to .pre-commit-config.yaml",
        );
        return false;
    }
    if let Some(why) = link_problem("pre-commit", config, decl.on_not_run) {
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
    } else if let Err(e) = write_atomic(config, &new) {
        report.refuse(format!("could not write {}: {e}", config.display()));
        return false;
    } else {
        report.item("pre-commit-framework", config, "extended");
    }
    true
}

pub fn run_uninstall(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let (ws, layout) = match open(config) {
        Ok(p) => p,
        Err(m) => return not_run(m),
    };
    let mut report = HooksReport::new("uninstall", dry_run, &layout);
    let (files, unwritable): (Vec<(&str, PathBuf)>, Option<String>) = match &layout {
        Layout::Framework { config, .. } => (vec![("pre-commit-framework", config.clone())], None),
        Layout::Files {
            dir, unwritable, ..
        } => (
            HOOKS.iter().map(|h| (*h, dir.join(h))).collect(),
            unwritable.clone(),
        ),
    };
    let framework = matches!(layout, Layout::Framework { .. });
    let committed = ws.config.hooks.path.as_ref().filter(|p| match &layout {
        Layout::Files { dir, .. } => resolve(dir) == resolve(&ws.repo_root.join(p)),
        Layout::Framework { .. } => false,
    });
    if committed.is_some() {
        for (hook, path) in &files {
            report.item(hook, path, "left");
        }
    }
    for (hook, path) in files.into_iter().filter(|_| committed.is_none()) {
        uninstall_one(
            &mut report,
            hook,
            &path,
            unwritable.as_deref(),
            framework,
            dry_run,
        );
    }
    if report.exit == 0 {
        unpoint(&mut report, &ws.repo_root, &ws.config.hooks, dry_run);
    }
    if let Some(p) = committed {
        let unset = report.items.iter().any(|i| {
            i.hook == "core.hooksPath" && matches!(i.state.as_str(), "unset" | "would-unset")
        });
        let also = if unset {
            " and unsets this clone's core.hooksPath"
        } else {
            ""
        };
        report.notice(
            NoticeSeverity::Info,
            format!("{p} is the directory hooks.path names, where the repository commits its hooks; uninstall leaves its files as they are{also}"),
        );
    }
    emit(&report)
}

/// Unsets `core.hooksPath` only when it is the directory `hooks.path` names.
fn unpoint(report: &mut HooksReport, repo_root: &Path, decl: &HooksDecl, dry_run: bool) {
    let Some(configured) = &decl.path else {
        return;
    };
    match unpointed(repo_root, decl) {
        Err(m) => return report.refuse(m),
        Ok(Some(_)) => return,
        Ok(None) => {}
    }
    let dir = repo_root.join(configured);
    let (scope, origin) = hooks_path_origin(repo_root);
    if !set_in_own_config(repo_root) {
        report.item("core.hooksPath", &dir, "left");
        return report.notice(
            NoticeSeverity::Info,
            format!("core.hooksPath is set at scope {scope} in {origin}, not in this repository's own configuration file; urzua unsets only a value it could have set there"),
        );
    }
    if dry_run {
        return report.item("core.hooksPath", &dir, "would-unset");
    }
    match git(
        repo_root,
        &["config", "--local", "--unset", "core.hooksPath"],
    ) {
        Ok(o) if o.status.success() => report.item("core.hooksPath", &dir, "unset"),
        Ok(o) => report.refuse(format!(
            "git config --unset core.hooksPath failed: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(m) => report.refuse(m),
    }
}

/// Removes the blocks from one file, under the same guards as `install`: a
/// symlinked file or a directory urzua will not write is refused, naming the
/// lines to remove by hand.
fn uninstall_one(
    report: &mut HooksReport,
    hook: &str,
    path: &Path,
    unwritable: Option<&str>,
    framework: bool,
    dry_run: bool,
) {
    let by_hand = |why: &str| {
        format!(
            "{why}; remove the lines from `{BEGIN}` to `{END}` in {} by hand",
            path.display()
        )
    };
    if let Some(why) = unwritable {
        return report.refuse(by_hand(why));
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || nlink(&meta) > 1 {
            return report.refuse(by_hand(&format!(
                "{} is a symlink or a hard link; urzua never writes through one",
                path.display()
            )));
        }
    }
    let content = match read_optional(path) {
        Ok(Some(c)) => c,
        Ok(None) => return report.item(hook, path, "absent"),
        Err(m) => return report.refuse(m),
    };
    if !content.contains(BEGIN) && !content.contains(END) {
        return report.item(hook, path, "not-ours");
    }
    let created = Marker::read(&content).created;
    let rest = match remove_blocks(&content) {
        Ok(r) => r,
        Err(m) => return report.refuse(format!("{} {m}", path.display())),
    };
    let delete = !framework && created && rest == SHEBANG;
    let state = if delete { "removed" } else { "block-removed" };
    if dry_run {
        return report.item(hook, path, &would(state));
    }
    let done = if delete {
        std::fs::remove_file(path)
    } else {
        write_atomic(path, &rest)
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
                fallback_policy(),
            )
        }
    };
    let decl = &ws.config.hooks;
    let Some(args) = decl.args(&hook) else {
        return not_run(Failure::usage(format!(
            "'{hook}' is not a hook urzua runs; the hooks are {}",
            HOOKS.join(", ")
        )));
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
    let out = match cmd.args(&args).current_dir(&ws.repo_root).output() {
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
            if decl.print_success {
                eprintln!("{hook}: {shown}: no blocking findings");
            }
            ExitCode::SUCCESS
        }
        Some(1) => blocked(&hook, &shown, &report, fail_on_of(&args, ws.config.fail_on)),
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

/// The threshold the hook's own arguments set (`--fail-on`), else the
/// configuration's, so the findings printed are the ones that blocked.
fn fail_on_of(args: &[String], configured: FailOn) -> FailOn {
    let value = args.iter().enumerate().find_map(|(i, a)| {
        a.strip_prefix("--fail-on=")
            .map(str::to_string)
            .or_else(|| (a == "--fail-on").then(|| args.get(i + 1).cloned().unwrap_or_default()))
    });
    match value.as_deref() {
        Some("warning" | "warn") => FailOn::Warning,
        Some("error") => FailOn::Error,
        _ => configured,
    }
}

/// The policy the block passes for when the configuration cannot be read.
fn fallback_policy() -> HookOnNotRun {
    match std::env::var("URZUA_HOOKS_ON_NOT_RUN").as_deref() {
        Ok("block") => HookOnNotRun::Block,
        _ => HookOnNotRun::default(),
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
    fn the_block_goes_at_the_top_and_comes_out_byte_for_byte() {
        for (content, before) in [
            ("#!/bin/sh\n# c\nmake lint\nexit 0\n", "#!/bin/sh\n# c\n"),
            ("make lint\n", ""),
            (
                "#!/usr/bin/env sh\n. \"$(dirname -- \"$0\")/_/husky.sh\"\n\nnpx x\n",
                "#!/usr/bin/env sh\n. \"$(dirname -- \"$0\")/_/husky.sh\"\n",
            ),
            ("#!/bin/sh", "#!/bin/sh"),
            ("", ""),
            ("#!/bin/sh\r\necho a\r\n", "#!/bin/sh\r\n"),
        ] {
            let out = insert_block(content, false, HookOnNotRun::Warn, "pre-commit");
            assert!(out.starts_with(before), "{content:?}: {out:?}");
            assert!(
                out[before.len()..]
                    .trim_start_matches(['\r', '\n'])
                    .starts_with(BEGIN),
                "{content:?}"
            );
            assert_eq!(remove_blocks(&out).unwrap(), content, "{content:?}");
            assert_eq!(Marker::read(&out).hook, "pre-commit");
        }
    }

    #[test]
    fn the_first_command_skips_comments_and_the_husky_line_only() {
        assert_eq!(
            first_command("#!/bin/sh\n# x\n\nurzua check\n"),
            Some("urzua check")
        );
        assert_eq!(
            first_command("#!/bin/sh\nset -e\nurzua check\n"),
            Some("set -e")
        );
        assert_eq!(
            first_command("#!/usr/bin/env node\n// x\nrun()\n"),
            Some("run()")
        );
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
        assert!(invokes_urzua(
            &shell_line("pre-commit", HookOnNotRun::Warn),
            None,
            false
        ));
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
    fn env_flags_are_skipped_to_find_the_interpreter() {
        assert!(shell_hook("#!/usr/bin/env -S bash -eu\n").is_ok());
        assert!(shell_hook("#!/usr/bin/env -i PATH=/bin sh\n").is_ok());
        assert_eq!(
            shell_hook("#!/usr/bin/env -S node --x\n"),
            Err("node".to_string())
        );
    }

    #[test]
    fn the_hooks_own_fail_on_decides_what_is_printed() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            fail_on_of(&args(&["check", "--fail-on", "warning"]), FailOn::Error),
            FailOn::Warning
        );
        assert_eq!(
            fail_on_of(&args(&["check", "--fail-on=error"]), FailOn::Warning),
            FailOn::Error
        );
        assert_eq!(
            fail_on_of(&args(&["check"]), FailOn::Warning),
            FailOn::Warning
        );
    }

    fn version(v: &str) -> Vec<u64> {
        v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
    }

    #[test]
    fn first_version_names_the_release_that_ships_hooks_run() {
        let changelog = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../CHANGELOG.md"
        ))
        .expect("CHANGELOG.md");
        let mut sections: Vec<(String, String)> = Vec::new();
        for line in changelog.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                let v = rest.split_whitespace().next().unwrap_or("").to_string();
                sections.push((v, String::new()));
            } else if let Some(last) = sections.last_mut() {
                last.1.push_str(line);
                last.1.push('\n');
            }
        }
        let shipped = sections
            .iter()
            .rev()
            .find(|(_, body)| body.contains("urzua hooks install"));
        match shipped {
            Some((v, _)) => assert_eq!(FIRST_VERSION, v, "hooks run first shipped in {v}"),
            None => assert!(
                version(FIRST_VERSION) > version(env!("CARGO_PKG_VERSION")),
                "FIRST_VERSION {FIRST_VERSION} must name the coming release, newer than {}",
                env!("CARGO_PKG_VERSION")
            ),
        }
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
