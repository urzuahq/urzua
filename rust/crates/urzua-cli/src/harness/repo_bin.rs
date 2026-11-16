//! The binary a repository carries at `.urzua/bin/urzua` (`ADR-89` amendment of
//! 2026-11-16): when it may be run by the hooks, and what is said when it may
//! not. The translator and the record helper run it only when its content hash
//! is the one `.urzua/harness.lock` pins, it is not tracked by git, and it is
//! executable; this module is the engine's side of the same rules, for
//! `status`, `doctor` and the session-start check.

use std::path::Path;

use super::lock::Lock;

pub const BIN_PATH: &str = ".urzua/bin/urzua";
pub const BIN_DIR: &str = ".urzua/bin";
/// The `.gitignore` line that keeps the binary out of commits.
pub const IGNORE_LINE: &str = ".urzua/bin/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinState {
    Absent,
    NotExecutable,
    Tracked,
    /// The lock pins no hash for it (no lock, or a lock written without it).
    Unpinned,
    /// Its content is not the one the lock pins.
    Mismatch,
    Verified,
}

impl BinState {
    /// The `status` item state for a binary that is present.
    pub fn item_state(self) -> &'static str {
        match self {
            BinState::Verified => "ok",
            BinState::Mismatch => "modified",
            BinState::Unpinned => "unpinned",
            BinState::Tracked => "tracked",
            BinState::NotExecutable => "not-executable",
            BinState::Absent => "missing",
        }
    }
}

/// A regular file with an execute bit (Unix); any regular file elsewhere.
pub fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn content_hash(root: &Path) -> Option<String> {
    urzua_io::sha256_hex_of_file(&root.join(BIN_PATH)).map(|h| format!("sha256:{h}"))
}

/// Whether git tracks the binary (it must never be a committed file).
pub fn is_tracked(root: &Path) -> bool {
    urzua_io::git::GitRun::new()
        .dash_c(root)
        .args(["ls-files", "--error-unmatch", "--", BIN_PATH])
        .discard_stderr()
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn state(root: &Path, lock: Option<&Lock>) -> BinState {
    let path = root.join(BIN_PATH);
    if !path.is_file() {
        return BinState::Absent;
    }
    if !is_executable(&path) {
        return BinState::NotExecutable;
    }
    if is_tracked(root) {
        return BinState::Tracked;
    }
    let Some(pinned) = lock.and_then(|l| l.binary.as_deref()) else {
        return BinState::Unpinned;
    };
    match content_hash(root) {
        Some(h) if h == pinned => BinState::Verified,
        _ => BinState::Mismatch,
    }
}

/// Why the hooks do not run the repository's binary, for the person; `None`
/// when it is absent or usable.
pub fn refusal(state: BinState) -> Option<String> {
    let why = match state {
        BinState::Absent | BinState::Verified => return None,
        BinState::NotExecutable => "it has no execute bit".to_string(),
        BinState::Tracked => "git tracks it (a committed binary is code someone else chose, so it is never run)".to_string(),
        BinState::Unpinned => "`.urzua/harness.lock` pins no hash for it (a person runs `urzua harness install` or `update` after installing it)".to_string(),
        BinState::Mismatch => "its content is not the one `.urzua/harness.lock` pins (it was replaced after the install, or the lock changed)".to_string(),
    };
    Some(format!(
        "{BIN_PATH} is NOT used by the hooks: {why}. They run URZUA_BIN or the urzua on PATH instead, and a session with neither judges nothing."
    ))
}

/// Adds `.urzua/bin/` to `.gitignore` when no line already ignores the
/// directory. Returns the text to write, or `None` when nothing is to change.
pub fn gitignore_with_entry(text: &str) -> Option<String> {
    let ignored = text.lines().map(str::trim).any(|l| {
        matches!(
            l,
            ".urzua/bin" | ".urzua/bin/" | "/.urzua/bin" | "/.urzua/bin/" | ".urzua/bin/*"
        )
    });
    if ignored {
        return None;
    }
    let mut out = text.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        "\n# The binary the harness hooks may run (pinned by hash in .urzua/harness.lock).\n{IGNORE_LINE}\n"
    ));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ignore_entry_is_added_once_and_a_covering_line_is_left_alone() {
        let once = gitignore_with_entry("target/\n").unwrap();
        assert!(once.contains("\n.urzua/bin/\n"));
        assert_eq!(gitignore_with_entry(&once), None);
        assert_eq!(gitignore_with_entry("a\n/.urzua/bin\n"), None);
        assert!(gitignore_with_entry("").unwrap().ends_with(".urzua/bin/\n"));
    }
}
