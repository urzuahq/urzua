//! Edit-tool writes outside the session's repository (`ADR-89` amendment of
//! 2026-11-09, `harness.outside_writes`). The session's repository is the one
//! its gates belong to; a linked worktree of it, or any other directory, is a
//! place those gates would otherwise never see.
//!
//! - `allow` (and undeclared): nothing here; the repository that holds the file
//!   judges it, and a place with no gates judges nothing.
//! - `shared`: a file whose git toplevel shares the session repository's
//!   `git rev-parse --git-common-dir` (a linked worktree of the same repository)
//!   is also judged by its path relative to that toplevel against the session's
//!   own acceptance and protected-path gates. A deny wins. A separate clone
//!   shares no common directory and stays an accepted risk.
//! - `deny`: an agent's write outside the session's repository is refused while
//!   any guardrail is enabled, except under `allow_paths`.

use std::path::{Path, PathBuf};

use urzua_core::harness::OutsideWrites;

use super::{absolute, gate_denial, resolve_links, Actor, Request, Verdict};
use crate::session::Workspace;

/// The verdict that refuses an agent's write outside the session's repository,
/// or `None` when the write is inside it or this policy has nothing to say.
pub(super) fn denial(session_root: &Path, cwd: &Path, request: &Request) -> Option<Verdict> {
    if request.actor() != Actor::Agent {
        return None;
    }
    let ws = Workspace::open(&[session_root.to_path_buf()], None).ok()?;
    let harness = ws.config.harness.as_ref()?;
    let mode = harness.outside_writes();
    if mode == OutsideWrites::Allow || !harness.any_enabled() {
        return None;
    }
    let session = session_root.canonicalize().ok()?;
    for path in &request.paths {
        let abs = absolute(cwd, path);
        let target = resolve_links(&abs).unwrap_or_else(|| abs.clone());
        if target.starts_with(&session) {
            continue;
        }
        let verdict = match mode {
            OutsideWrites::Deny => deny_outside(&session, &target, &harness.allow_paths, path),
            _ => shared_denial(&session, &ws.config, &target, request),
        };
        if verdict.is_some() {
            return verdict;
        }
    }
    None
}

/// `outside_writes: deny`: refused unless the file is under an `allow_paths`
/// directory.
fn deny_outside(
    session: &Path,
    target: &Path,
    allow_paths: &[String],
    written: &str,
) -> Option<Verdict> {
    if allow_paths
        .iter()
        .filter_map(|p| expand(p))
        .any(|dir| target.starts_with(&dir))
    {
        return None;
    }
    Some(Verdict::new(
        "block",
        Some(written),
        format!(
            "urzua harness (outside-writes): refused. {written} is outside the session repository {}, and harness.outside_writes is deny, so an agent's edit through the edit tools may not write there. List its directory under harness.allow_paths in .urzua/config.yaml, or make the change outside the agent harness. The override of ADR-78 is not accepted for this verdict.",
            session.display()
        ),
    ))
}

/// One `allow_paths` entry as an absolute directory with links resolved where
/// it exists: `~` is the home directory.
fn expand(entry: &str) -> Option<PathBuf> {
    let dir = match entry.strip_prefix('~') {
        Some(rest) => {
            let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
            PathBuf::from(home).join(rest.trim_start_matches('/'))
        }
        None => PathBuf::from(entry),
    };
    Some(resolve_links(&dir).unwrap_or(dir))
}

/// `outside_writes: shared`: a file of a linked worktree of the session's
/// repository is judged by its path in that worktree against the session's gates.
fn shared_denial(
    session: &Path,
    config: &urzua_core::config::Config,
    target: &Path,
    request: &Request,
) -> Option<Verdict> {
    let (toplevel, common) = toplevel_and_common_dir(target)?;
    let (_, session_common) = toplevel_and_common_dir(session)?;
    if common != session_common {
        return None;
    }
    let rel = target.strip_prefix(&toplevel).ok()?;
    let mapped = session.join(rel).to_string_lossy().to_string();
    let content = request.content.as_deref()?;
    let mut verdict = gate_denial(session, config, &mapped, content, request)?;
    verdict.message = format!(
        "{} (the file is {} in a linked worktree of this repository at {}, judged by this repository's configuration)",
        verdict.message,
        rel.display(),
        toplevel.display()
    );
    Some(verdict)
}

/// The git toplevel of the directory holding `path` (the nearest existing one)
/// and the repository's common directory, both canonical; `None` outside git.
fn toplevel_and_common_dir(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let dir = path.ancestors().find(|d| d.is_dir())?;
    let out = urzua_io::git::GitRun::new()
        .args([
            "-c",
            "safe.directory=*",
            "rev-parse",
            "--show-toplevel",
            "--git-common-dir",
        ])
        .cwd(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut lines = text.lines();
    let top = Path::new(lines.next()?.trim()).canonicalize().ok()?;
    let common = Path::new(lines.next()?.trim());
    let common = if common.is_absolute() {
        common.to_path_buf()
    } else {
        dir.join(common)
    };
    Some((top, common.canonicalize().ok()?))
}
