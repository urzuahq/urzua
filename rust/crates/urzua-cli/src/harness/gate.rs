//! The acceptance and protected-path guardrails of the editor hook (`ADR-89`
//! section 4, `MILE-200`).
//!
//! For an agent, a Write, Edit or MultiEdit that moves a record of a gated type
//! to a gated status, or that writes a gated path, is denied outright. The
//! override of `ADR-78` is a name in an environment variable the agent controls,
//! so it is not accepted for this verdict. A person edits outside the agent
//! harness and is not asked.
//!
//! This is the whole claim: the edit tools are refused. A shell command that
//! writes the same file is not seen here: the post-command check reports a
//! gated status move and a changed protected path afterwards, as a report only,
//! including a change the same command committed. In CI the evidence is
//! authorship, not who decided (`diff.decider-required`).
//!
//! Paths are matched case-insensitively with Unicode case folding, always, on
//! every filesystem: on a case-insensitive one (the macOS default) `.Hooks/x` is
//! the same file as `.hooks/x`, and a gate that read the two as different would be bypassed by
//! changing a letter's case. The cost on a case-sensitive filesystem is that a
//! differently-cased sibling of a protected path is protected too, which is the
//! safe direction. Names are not Unicode-normalised: a decomposed and a
//! composed spelling of one letter stay different paths.
//!
//! A write through a hardlink is judged as the file it shares an inode with
//! (Unix; see [`hardlink_aliases`]).

use std::path::{Path, PathBuf};

use urzua_core::authority::Gate;
use urzua_core::config::{Config, RelationRole};
use urzua_core::rules;

/// Why an agent's edit is refused, and by which guardrail.
pub(super) struct Denial {
    pub guardrail: &'static str,
    pub reason: String,
}

fn status_of(config: &Config, rel: &Path, text: &str) -> Option<(String, String)> {
    let record = crate::discovery::parse_prospective_record(config, rel, text).ok()?;
    let field = rules::relation_field_name(config, &record.record_type, RelationRole::Status);
    let status = record.header.get(field).map(str::trim)?.to_string();
    (!status.is_empty()).then_some((record.record_type.clone(), status))
}

/// The status the record holds before this edit: the copy on disk, else the
/// copy at `HEAD`, else none (a new record).
fn status_before(repo_root: &Path, config: &Config, rel: &Path) -> Option<Option<String>> {
    let text = match std::fs::read_to_string(repo_root.join(rel)) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match urzua_io::read_at_commit_if_present(repo_root, "HEAD", rel) {
                Ok(t) => t,
                Err(_) => return None,
            }
        }
        Err(_) => return None,
    };
    match text {
        None => Some(None),
        Some(text) => Some(status_of(config, rel, &text).map(|(_, s)| s)),
    }
}

/// The path gate that covers `rel`, compared with Unicode case folding.
pub(super) fn protected_gate<'a>(
    authority: &'a urzua_core::authority::AuthorityDecl,
    rel: &Path,
) -> Option<&'a Gate> {
    let folded = fold_path(&rel.to_string_lossy());
    authority
        .gates
        .iter()
        .filter(|g| g.is_path_gate())
        .find(|g| {
            g.paths
                .iter()
                .any(|e| rules::path_matches_entry(Path::new(&folded), &fold_path(e)))
        })
}

/// Unicode case folding, for comparing a path with a gate entry: each
/// character upper-cased then lower-cased, which folds `\u{df}` to `ss`, a
/// final sigma to sigma and the Kelvin sign to `k` as well as the ASCII letters.
pub(super) fn fold_path(text: &str) -> String {
    text.chars()
        .flat_map(char::to_uppercase)
        .flat_map(char::to_lowercase)
        .collect()
}

/// `rel` with a leading record directory spelled as the configuration spells
/// it, when `rel` spells it in another case (Unicode case folding):
/// `Docs/adr/x.md` is `docs/adr/x.md` on a case-insensitive filesystem. A path
/// under no declared directory is returned as it is.
pub(super) fn fold_to_declared_dir(config: &Config, rel: &Path) -> PathBuf {
    let text = rel.to_string_lossy();
    for t in config.record_types.values() {
        for dir in std::iter::once(&t.dir).chain(t.extra_dirs.iter()) {
            let dir = dir.trim_end_matches('/');
            if dir.is_empty() {
                continue;
            }
            let want = fold_path(dir);
            // The prefix may be a different number of bytes than `dir` once
            // folded, so try each place a component could end.
            for (n, _) in text.match_indices('/') {
                if n > 0 && fold_path(&text[..n]) == want {
                    return PathBuf::from(format!("{dir}{}", &text[n..]));
                }
            }
        }
    }
    rel.to_path_buf()
}

/// The files under the path-gate `entries` (relative to `root`), sorted, and
/// whether the walk stopped at its cap. A plain entry names a file or a
/// directory; a glob entry is walked from the directory before its first glob
/// character. `.git` is never entered and a symlinked directory is not followed.
pub(super) fn walk_entries(root: &Path, entries: &[String]) -> (Vec<String>, bool) {
    const CAP: usize = 50_000;
    let mut found = std::collections::BTreeSet::new();
    let mut visited = 0usize;
    let mut truncated = false;
    for entry in entries {
        let trimmed = entry.trim().trim_end_matches('/');
        // An empty or `.` entry covers nothing (see `path_is_published`).
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }
        let glob = rules::is_path_glob(trimmed);
        let start = if glob {
            let mut prefix = PathBuf::new();
            for part in trimmed.split('/') {
                if rules::is_path_glob(part) {
                    break;
                }
                prefix.push(part);
            }
            prefix
        } else {
            PathBuf::from(trimmed)
        };
        let mut stack = vec![start];
        while let Some(rel) = stack.pop() {
            if visited >= CAP {
                truncated = true;
                break;
            }
            visited += 1;
            let abs = root.join(&rel);
            let Ok(meta) = std::fs::symlink_metadata(&abs) else {
                continue;
            };
            if meta.is_dir() {
                if rel.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                if let Ok(read) = std::fs::read_dir(&abs) {
                    for child in read.flatten() {
                        stack.push(rel.join(child.file_name()));
                    }
                }
            } else if !glob || rules::path_matches_entry(&rel, trimmed) {
                found.insert(rel.to_string_lossy().to_string());
            }
        }
    }
    (found.into_iter().collect(), truncated)
}

/// The files and directories the harness maintains itself, which an agent's
/// edit tools must not write while the `protected-paths` guardrail is on: the
/// post-command baselines under `.urzua/cache` (a baseline written ahead of a
/// change disarms the report), the lock, the declared override log (its
/// `Reviewed-by` column is a person's review, and a row is appended only by the
/// override path of the editor hook, which writes it from the engine, not
/// through an edit tool), and the git hooks and configuration (a
/// `core.hooksPath` turns the commit layer off). They are not in the authority
/// table, so CI's `diff.decider-required` does not ask a decider for the
/// override rows an ordinary override appends.
pub(super) fn engine_owned(override_log: Option<&str>) -> Vec<String> {
    let mut owned: Vec<String> = [
        ".urzua/cache",
        super::lock::LOCK_PATH,
        ".git/hooks",
        ".git/config",
    ]
    .iter()
    .map(|p| p.to_string())
    .collect();
    if let Some(log) = override_log.map(|l| l.trim().trim_end_matches('/')) {
        if !log.is_empty() {
            owned.push(log.to_string());
        }
    }
    owned
}

/// The engine-owned entry that covers `rel`, compared with Unicode case folding.
fn owned_entry(config: &Config, rel: &Path) -> Option<String> {
    let folded = fold_path(&rel.to_string_lossy());
    engine_owned(config.override_log.as_deref())
        .into_iter()
        .find(|e| rules::path_matches_entry(Path::new(&folded), &fold_path(e)))
}

/// The path-gate entries of the table (every protected path).
pub(super) fn protected_entries(config: &Config) -> Vec<String> {
    let Some(authority) = config.authority.as_ref() else {
        return Vec::new();
    };
    authority
        .gates
        .iter()
        .filter(|g| g.is_path_gate())
        .flat_map(|g| g.paths.iter().cloned())
        .collect()
}

/// The record directories of the types a status gate names.
pub(super) fn gated_record_dirs(config: &Config) -> Vec<String> {
    let Some(authority) = config.authority.as_ref() else {
        return Vec::new();
    };
    authority
        .gated_types()
        .into_iter()
        .filter_map(|t| config.record_types.get(t))
        .flat_map(|t| t.all_dirs().map(|d| d.trim_end_matches('/').to_string()))
        .collect()
}

/// The names (relative to `root`) of the protected files and gated records
/// that are the same file as `path` through a hard link: a write to the alias
/// writes them. Empty when `path` has no other link, does not exist, or off
/// Unix, where the limit is documented instead. Looked up only for a file that
/// has more than one link, so an ordinary edit pays nothing.
#[cfg(unix)]
pub(super) fn hardlink_aliases(
    root: &Path,
    path: &Path,
    entries: &[String],
    record_dirs: &[String],
) -> Vec<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return Vec::new(),
        }
    };
    let Ok(meta) = std::fs::metadata(&abs) else {
        return Vec::new();
    };
    if !meta.is_file() || meta.nlink() < 2 {
        return Vec::new();
    }
    let (dev, ino) = (meta.dev(), meta.ino());
    // Whole directories for a record type: its files are all gated candidates.
    let mut roots: Vec<String> = entries.to_vec();
    roots.extend(record_dirs.iter().cloned());
    walk_entries(root, &roots)
        .0
        .into_iter()
        .filter(|rel| {
            std::fs::metadata(root.join(rel)).is_ok_and(|m| m.dev() == dev && m.ino() == ino)
        })
        .map(PathBuf::from)
        .collect()
}

/// Windows has no inode to compare through `std` without more than this
/// module should carry: a write through a hard link is judged by its own name.
#[cfg(not(unix))]
pub(super) fn hardlink_aliases(
    _root: &Path,
    _path: &Path,
    _entries: &[String],
    _record_dirs: &[String],
) -> Vec<PathBuf> {
    Vec::new()
}

fn gate_summary(gate: &Gate) -> String {
    format!("gate: {}, by '{}'", gate.describe(), gate.by)
}

/// Judges one edit against the loaded `authority` table. `None` means the gates
/// have nothing to say; the ordinary editor-hook judgement still follows.
pub(super) fn judge(
    repo_root: &Path,
    config: &Config,
    rel: &Path,
    content: &str,
) -> Option<Denial> {
    let (harness, authority) = (config.harness.as_ref()?, config.authority.as_ref()?);
    if harness.enabled("protected-paths") {
        if let Some(gate) = protected_gate(authority, rel) {
            return Some(Denial {
                guardrail: "protected-paths",
                reason: format!(
                    "{} is a protected path ({}); only that decider group may change it, and an agent's edit through the edit tools is refused",
                    rel.display(),
                    gate_summary(gate)
                ),
            });
        }
    }
    if harness.enabled("protected-paths") {
        if let Some(entry) = owned_entry(config, rel) {
            return Some(Denial {
                guardrail: "protected-paths",
                reason: format!(
                    "{} is kept by urzua itself ({entry}: the post-command baselines, the harness lock, the override log, the git hooks and configuration); an agent's edit through the edit tools is refused, because a change there would disarm a report or a review. urzua writes it; a person edits it outside the agent harness",
                    rel.display()
                ),
            });
        }
    }
    if harness.enabled("acceptance") && authority.has_status_gate() {
        let folded = fold_to_declared_dir(config, rel);
        let rel = folded.as_path();
        let (record_type, status) = status_of(config, rel, content)?;
        let gate = authority.status_gates(&record_type, &status).next()?;
        let field = rules::relation_field_name(config, &record_type, RelationRole::Status);
        // An edit that leaves the status where it was is not a move; any other
        // edit to a gated status, a record filed straight at it included, is.
        let from = match status_before(repo_root, config, rel) {
            Some(Some(before)) if before == status => return None,
            Some(Some(before)) => format!("moves {} from {field} '{before}' to '{status}'", rel.display()),
            Some(None) => format!("files {} already at {field} '{status}'", rel.display()),
            None => format!(
                "sets {} to {field} '{status}' (the copy it replaces could not be read, so it is treated as a move)",
                rel.display()
            ),
        };
        return Some(Denial {
            guardrail: "acceptance",
            reason: format!(
                "this edit {from}, a move the authority table reserves for a decider ({}); an agent cannot make it through the edit tools",
                gate_summary(gate)
            ),
        });
    }
    None
}

/// One status gate as the post-command baseline remembers it, so a gate the
/// command removed or narrowed still judges the move it was meant to stop.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct StatusGateEntry {
    pub record_type: String,
    pub to: Vec<String>,
    pub by: String,
}

/// The status gates in force: none unless the `acceptance` guardrail is on.
pub(super) fn status_gate_entries(config: &Config) -> Vec<StatusGateEntry> {
    let (Some(harness), Some(authority)) = (config.harness.as_ref(), config.authority.as_ref())
    else {
        return Vec::new();
    };
    if !harness.enabled("acceptance") {
        return Vec::new();
    }
    authority
        .gates
        .iter()
        .filter_map(|g| {
            Some(StatusGateEntry {
                record_type: g.record_type.clone()?,
                to: g.to.clone(),
                by: g.by.clone(),
            })
        })
        .collect()
}

/// The `(type, status)` pairs `old` gated that `now` does not: a gate removed,
/// narrowed, or lost to the guardrail being switched off. One line each.
pub(super) fn lost_status_gates(old: &[StatusGateEntry], now: &[StatusGateEntry]) -> Vec<String> {
    old.iter()
        .filter_map(|o| {
            let lost: Vec<&str> =
                o.to.iter()
                    .map(String::as_str)
                    .filter(|s| {
                        !now.iter()
                            .any(|n| n.record_type == o.record_type && n.to.iter().any(|t| t == s))
                    })
                    .collect();
            (!lost.is_empty()).then(|| format!("type {} to {}", o.record_type, lost.join(", ")))
        })
        .collect()
}

/// After a command has already run: the report line for a record whose status
/// on disk is a gated one that `base` did not hold (a new record, or one at
/// another status), else `None`. `base` is `HEAD`, or the commit the last
/// post-command check saw when the command committed. It judges the table as
/// [`judge`] does, and also the gates the baseline held (`baseline`): a gate
/// the command removed or narrowed, or the guardrail it switched off, still
/// reports the move it was meant to stop. It only describes; the post-command
/// check reports it and never blocks.
pub(super) fn gated_move_report(
    repo_root: &Path,
    config: &Config,
    rel: &Path,
    base: &str,
    baseline: &[StatusGateEntry],
) -> Option<String> {
    let folded = fold_to_declared_dir(config, rel);
    let rel = folded.as_path();
    let content = std::fs::read_to_string(repo_root.join(rel)).ok()?;
    let (record_type, status) = status_of(config, rel, &content)?;
    let current = status_gate_entries(config)
        .into_iter()
        .find(|g| g.record_type == record_type && g.to.contains(&status))
        .map(|g| {
            format!(
                "gate: type {} to {}, by '{}'",
                g.record_type,
                g.to.join(", "),
                g.by
            )
        });
    let summary = current.or_else(|| {
        baseline
            .iter()
            .find(|g| g.record_type == record_type && g.to.contains(&status))
            .map(|g| {
                format!(
                    "gate: type {} to {}, by '{}', no longer in the configuration",
                    g.record_type,
                    g.to.join(", "),
                    g.by
                )
            })
    })?;
    let field = rules::relation_field_name(config, &record_type, RelationRole::Status);
    let before = match urzua_io::read_at_commit_if_present(repo_root, base, rel) {
        Ok(Some(text)) => match status_of(config, rel, &text) {
            Some((_, before)) if before == status => return None,
            Some((_, before)) => format!("moved {} from {field} '{before}' to '{status}'", rel.display()),
            None => format!("set {} to {field} '{status}'", rel.display()),
        },
        Ok(None) => format!("filed {} already at {field} '{status}'", rel.display()),
        Err(_) => format!(
            "set {} to {field} '{status}' (its copy at {base} could not be read, so it is treated as a move)",
            rel.display()
        ),
    };
    Some(format!(
        "  {} [acceptance] error: the command {before}, a move the authority table reserves for a decider ({summary}). It was not prevented; CI's diff.decider-required checks who made it.",
        rel.display(),
    ))
}

/// After a command has already run: the report line for a path under a
/// protected-paths gate that the command changed (written, created, deleted or
/// committed), else `None`. A report only, like [`gated_move_report`]; the
/// command was not prevented.
pub(super) fn protected_change_report(
    config: &Config,
    rel: &Path,
    deleted: bool,
) -> Option<String> {
    let (harness, authority) = (config.harness.as_ref()?, config.authority.as_ref()?);
    if !harness.enabled("protected-paths") {
        return None;
    }
    let gate = protected_gate(authority, rel)?;
    Some(format!(
        "  {} [protected-paths] error: the command {} a protected path ({}). It was not prevented; CI's diff.decider-required checks who made it.",
        rel.display(),
        if deleted { "deleted" } else { "changed" },
        gate_summary(gate)
    ))
}

/// The report line for a path the previous baseline protected through one of
/// `baseline` (entries of the configuration as it was), for a gate the command
/// removed. `None` when no baseline entry covers the path.
pub(super) fn baseline_protected_report(
    rel: &Path,
    deleted: bool,
    baseline: &[String],
) -> Option<String> {
    let folded = fold_path(&rel.to_string_lossy());
    let entry = baseline
        .iter()
        .find(|e| rules::path_matches_entry(Path::new(&folded), &fold_path(e)))?;
    Some(format!(
        "  {} [protected-paths] error: the command {} a path the previous configuration protected (gate path '{}', no longer in the configuration). It was not prevented; CI's diff.decider-required checks who made it.",
        rel.display(),
        if deleted { "deleted" } else { "changed" },
        entry
    ))
}

/// What the raw configuration says about the gates when it does not load, read
/// leniently so a broken file does not silently turn a denying gate into an
/// allowing one (`ADR-89` section 3).
pub(super) struct RawGates {
    /// The protected-paths guardrail, when enabled and set to deny.
    pub protected: bool,
    /// The acceptance guardrail, when enabled and set to deny.
    pub acceptance: bool,
    pub paths: Vec<String>,
    /// The directories (the directory and every extra directory) of the types
    /// a status gate names.
    pub gated_dirs: Vec<String>,
    /// The files the harness maintains itself (`engine_owned`).
    pub owned: Vec<String>,
}

pub(super) fn raw_gates(config_text: &str) -> Option<RawGates> {
    let value: yaml_serde::Value = yaml_serde::from_str(config_text).ok()?;
    let guardrail = |name: &str| -> bool {
        let Some(g) = value
            .get("harness")
            .and_then(|h| h.get("guardrails"))
            .and_then(|g| g.get(name))
        else {
            return false;
        };
        let enabled = g.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
        let allow = g.get("on_unjudged").and_then(|o| o.as_str()) == Some("allow");
        enabled && !allow
    };
    let gates = value
        .get("authority")
        .and_then(|a| a.get("gates"))
        .and_then(|g| g.as_sequence())
        .cloned()
        .unwrap_or_default();
    let mut raw = RawGates {
        protected: guardrail("protected-paths"),
        acceptance: guardrail("acceptance"),
        paths: Vec::new(),
        gated_dirs: Vec::new(),
        owned: engine_owned(value.get("override_log").and_then(|l| l.as_str())),
    };
    for gate in &gates {
        if let Some(paths) = gate.get("paths").and_then(|p| p.as_sequence()) {
            raw.paths
                .extend(paths.iter().filter_map(|p| p.as_str().map(str::to_string)));
        }
        if let Some(t) = gate.get("type").and_then(|t| t.as_str()) {
            let declared = value.get("record_types").and_then(|r| r.get(t));
            let dirs = declared
                .and_then(|r| r.get("dir"))
                .and_then(|d| d.as_str())
                .into_iter()
                .chain(
                    declared
                        .and_then(|r| r.get("extra_dirs"))
                        .and_then(|d| d.as_sequence())
                        .into_iter()
                        .flatten()
                        .filter_map(|d| d.as_str()),
                );
            for dir in dirs {
                raw.gated_dirs.push(dir.trim_end_matches('/').to_string());
            }
        }
    }
    Some(raw)
}

impl RawGates {
    /// Whether the edit is one the unreadable configuration cannot be trusted
    /// to allow: a gated path, the configuration itself, or a record in a gated
    /// type's directory.
    pub fn covers(&self, rel: &Path, config_rel: Option<&Path>) -> Option<&'static str> {
        let folded = fold_path(&rel.to_string_lossy());
        let folded = Path::new(&folded);
        if self.protected
            && (config_rel
                .is_some_and(|c| fold_path(&c.to_string_lossy()) == folded.to_string_lossy())
                || self
                    .paths
                    .iter()
                    .any(|e| rules::path_matches_entry(folded, &fold_path(e)))
                || self
                    .owned
                    .iter()
                    .any(|e| rules::path_matches_entry(folded, &fold_path(e))))
        {
            return Some("protected-paths");
        }
        if self.acceptance {
            // Anywhere under a gated type's directory, not only a direct child.
            let under = |d: &String| rules::path_matches_entry(folded, &fold_path(d));
            if self.gated_dirs.iter().any(under) {
                return Some("acceptance");
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &str = "override_log: .urzua/log.md\nrecord_types:\n  adr:\n    dir: docs/adr\n    extra_dirs: [docs/old/adr]\nharness:\n  guardrails:\n    acceptance: {enabled: true}\n    protected-paths: {enabled: true}\nauthority:\n  gates:\n    - {type: adr, to: [Accepted], by: m}\n    - {paths: [.hooks], by: m}\n";

    #[test]
    fn an_unreadable_configuration_still_covers_extra_dirs_subdirectories_and_what_the_harness_keeps(
    ) {
        let raw = raw_gates(RAW).expect("the raw configuration reads");
        for rel in [
            "docs/adr/ADR-1-a.md",
            "docs/adr/nested/ADR-1-a.md",
            "docs/old/adr/ADR-2-b.md",
            "docs/old/adr/deeper/ADR-2-b.md",
        ] {
            assert_eq!(
                raw.covers(Path::new(rel), None),
                Some("acceptance"),
                "{rel}"
            );
        }
        for rel in [
            ".urzua/cache/post-command.json",
            ".urzua/harness.lock",
            ".urzua/log.md",
            ".git/hooks/pre-commit",
            ".git/config",
            ".hooks/settings.json",
        ] {
            assert_eq!(
                raw.covers(Path::new(rel), None),
                Some("protected-paths"),
                "{rel}"
            );
        }
        assert_eq!(raw.covers(Path::new("docs/other/x.md"), None), None);
        assert_eq!(raw.covers(Path::new("docs/adrx/x.md"), None), None);
    }
}
