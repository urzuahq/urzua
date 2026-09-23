//! Shared, non-command-specific plumbing every command builds on: locating
//! the repo root, loading config, loading records, and scoping/annotating
//! what was discovered. None of this is a command in its own right.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::report::ScopeSource;
use urzua_core::rules;

/// The contract value for how a run selected its records. The adapter lives
/// here because `urzua-core` (which owns `ScopeSource`) and `urzua-io` (which
/// owns `DiscoverySource`) cannot see each other -- `urzua-cli` is the only
/// crate that does. One function, not one per command, so the two can never
/// disagree about the same fact (ADR-0030).
pub(crate) fn scope_source(source: urzua_io::DiscoverySource) -> ScopeSource {
    match source {
        urzua_io::DiscoverySource::GitTracked => ScopeSource::TrackedSweep,
        urzua_io::DiscoverySource::Argv => ScopeSource::Argv,
    }
}

pub(crate) fn find_repo_root(paths: &[PathBuf]) -> Result<PathBuf, String> {
    let start = paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));
    let mut dir = start
        .canonicalize()
        .map_err(|e| format!("could not resolve {}: {e}", start.display()))?;
    if dir.is_file() {
        dir = dir
            .parent()
            .ok_or_else(|| "path has no parent directory".to_string())?
            .to_path_buf();
    }
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err("no .git directory found above the given path".to_string()),
        }
    }
}

/// One place that names the config file, so a format change is one edit
/// rather than fourteen (ADR-52).
pub(crate) fn default_config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".urzua/config.yaml")
}

pub(crate) fn load_config(path: &PathBuf) -> Result<Config, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read config at {}: {e}", path.display()))?;
    urzua_core::config::parse(&content).map_err(|e| e.to_string())
}

pub(crate) fn load_records(
    repo_root: &Path,
    discovered: &[PathBuf],
    staged_deletions: &std::collections::HashSet<PathBuf>,
    config: &Config,
) -> Result<(Vec<Record>, HashMap<PathBuf, String>), String> {
    let mut records = Vec::new();
    let mut full_text = HashMap::new();
    let mut unreadable: Vec<String> = Vec::new();
    for (type_name, type_config) in &config.record_types {
        let type_dir = PathBuf::from(&type_config.dir);
        for rel_path in discovered {
            // This directory, not this subtree (RFC-35). Prefix matching let
            // two types claim one record whenever their dirs nested, which no
            // schema rule resolved -- and four heuristics accreted in `init`
            // guessing what an adopter meant by the overlap. A directory is a
            // type; `docs/rfc` holds RFCs.
            if rel_path.parent() != Some(type_dir.as_path()) {
                continue;
            }
            let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !urzua_core::record::is_governed_record_filename(file_name) {
                continue;
            }
            let full_path = repo_root.join(rel_path);
            match urzua_io::read_to_string(&full_path) {
                Ok(content) => {
                    let type_prefix = type_config
                        .prefix
                        .clone()
                        .unwrap_or_else(|| type_name.to_ascii_uppercase());
                    records.push(Record::parse_with_shape_and_prefix(
                        rel_path.clone(),
                        type_name.clone(),
                        &content,
                        type_config.header_shape,
                        type_prefix,
                    ));
                    full_text.insert(rel_path.clone(), content);
                }
                // A record the tool cannot read is not a record that passes:
                // dropping it shrank the corpus with nothing in the report to
                // show a file had gone missing (BUG-76). A staged deletion is
                // the one legitimate absence, and only git can say which
                // absences those are -- treating every `NotFound` as one let an
                // unstaged `rm` shrink the corpus in silence (BUG-82).
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound
                        && staged_deletions.contains(rel_path) =>
                {
                    continue
                }
                Err(e) => unreadable.push(format!("{}: {e}", rel_path.display())),
            }
        }
    }
    if !unreadable.is_empty() {
        return Err(format!(
            "could not read {} tracked record(s): {}",
            unreadable.len(),
            unreadable.join("; ")
        ));
    }

    Ok((records, full_text))
}

/// Restrict discovered files to those under any of the requested paths,
/// relative to `repo_root`. Empty `requested` means no restriction -- the
/// existing "check everything" behavior when no path argument is given.
///
/// BUG-0001: `check`'s `paths` argument was previously used only to locate
/// the repo root, never to filter which files were actually examined --
/// `check docs/adr/` and `check docs/` returned identical results. Caught
/// by hand while exercising a genuinely narrower path for the first time;
/// no existing test used anything but `check docs/`, so nothing exercised
/// the narrower case at all.
/// The requested paths as repository-relative prefixes. Separated from the file
/// filtering because a finding is scoped by the path it names, which may be a
/// file no type owns and git does not track (BUG-67).
pub(crate) fn relative_scopes(
    repo_root: &Path,
    requested: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for p in requested {
        let canonical = p
            .canonicalize()
            .map_err(|e| format!("could not resolve {}: {e}", p.display()))?;
        let relative = canonical
            .strip_prefix(repo_root)
            .map_err(|_| format!("{} is outside the repository", p.display()))?
            .to_path_buf();
        out.push(relative);
    }
    Ok(out)
}

/// A directly-named file outside `tracked` (`BUG-24`): an explicit argv path
/// is "used as given" (SPEC-2), but only when it names one file precisely --
/// a directory argument still respects the tracked-only sweep
/// (`an_untracked_scratch_file_is_never_examined`, `ADR-6`). Additive to the
/// tracked sweep, never a replacement of it (`BUG-60`).
pub(crate) fn resolve_argv_overrides(
    repo_root: &Path,
    requested_scopes: &[PathBuf],
    tracked: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let tracked: std::collections::HashSet<&PathBuf> = tracked.iter().collect();
    requested_scopes
        .iter()
        .filter(|scope| !tracked.contains(scope))
        .filter_map(|scope| {
            let full = repo_root.join(scope);
            // `symlink_metadata`, not `metadata`: harmless either way in
            // practice, since `scope` already came through `relative_scopes`'
            // own `canonicalize()`, which resolves every symlink before this
            // function ever runs (`BUG-127`, investigated and found
            // unreachable through the only real call site).
            match std::fs::symlink_metadata(&full) {
                Ok(meta) if meta.is_file() => Some(Ok(scope.clone())),
                Ok(_) => None,
                Err(e) => Some(Err(format!("could not stat {}: {e}", full.display()))),
            }
        })
        .collect()
}

/// Takes the caller's already-resolved scopes rather than a raw path list
/// and re-deriving them: this and `check.rs`'s own `requested_scopes` used to
/// call `relative_scopes` independently on the same `paths`, each doing one
/// `canonicalize()` per argument -- the same filesystem work twice for one
/// invocation.
pub(crate) fn scope_to_requested_paths(
    discovered: &[PathBuf],
    relative_scopes: &[PathBuf],
) -> Vec<PathBuf> {
    if relative_scopes.is_empty() {
        return discovered.to_vec();
    }

    discovered
        .iter()
        .filter(|d| relative_scopes.iter().any(|s| d.starts_with(s)))
        .cloned()
        .collect()
}

/// Which records have at least one `Realized-by` locator that changed, per
/// git history, since the `Realized-by` line was last touched (ADR-0032).
/// The one piece of I/O `embodiment_consistency` needs but can't do itself
/// -- computed here and handed in as plain data, same shape as `full_text`.
pub(crate) fn compute_drifted_records(
    repo_root: &Path,
    records: &[Record],
    config: &Config,
) -> HashSet<PathBuf> {
    let mut drifted = HashSet::new();

    for record in records {
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        let Some(realized_by_value) = record.header.get(locator_field) else {
            continue;
        };
        let Some(field) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == locator_field)
        else {
            continue;
        };
        let Ok(Some(reference_commit)) =
            urzua_io::commit_for_line(repo_root, &record.path, field.line)
        else {
            continue;
        };

        for locator in rules::realized_by_locator_paths(realized_by_value) {
            let locator_path = PathBuf::from(&locator);
            let Ok(Some(locator_commit)) = urzua_io::last_commit_for_path(repo_root, &locator_path)
            else {
                continue;
            };
            if urzua_io::commit_strictly_before(repo_root, &reference_commit, &locator_commit) {
                drifted.insert(record.path.clone());
                break;
            }
        }
    }

    drifted
}
