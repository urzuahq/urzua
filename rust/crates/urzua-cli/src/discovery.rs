//! Shared, non-command-specific plumbing every command builds on: locating
//! the repo root, loading config, loading records, and scoping/annotating
//! what was discovered. None of this is a command in its own right.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::rules;

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

pub(crate) fn load_config(path: &PathBuf) -> Result<Config, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read config at {}: {e}", path.display()))?;
    urzua_core::config::parse(&content).map_err(|e| e.to_string())
}

pub(crate) fn load_records(
    repo_root: &Path,
    discovered: &[PathBuf],
    config: &Config,
) -> (Vec<Record>, HashMap<PathBuf, String>) {
    let mut records = Vec::new();
    let mut full_text = HashMap::new();
    for (type_name, type_config) in &config.record_types {
        let type_dir = PathBuf::from(&type_config.dir);
        for rel_path in discovered {
            if !rel_path.starts_with(&type_dir) {
                continue;
            }
            let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if file_name.starts_with('_') || !file_name.ends_with(".md") {
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
                Err(_) => continue,
            }
        }
    }
    (records, full_text)
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
pub(crate) fn scope_to_requested_paths(
    repo_root: &Path,
    discovered: &[PathBuf],
    requested: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    if requested.is_empty() {
        return Ok(discovered.to_vec());
    }

    let mut relative_scopes = Vec::new();
    for p in requested {
        let canonical = p
            .canonicalize()
            .map_err(|e| format!("could not resolve {}: {e}", p.display()))?;
        let relative = canonical
            .strip_prefix(repo_root)
            .map_err(|_| format!("{} is outside the repository", p.display()))?
            .to_path_buf();
        relative_scopes.push(relative);
    }

    Ok(discovered
        .iter()
        .filter(|d| relative_scopes.iter().any(|s| d.starts_with(s)))
        .cloned()
        .collect())
}

/// Which records have at least one `Realized-by` locator that changed, per
/// git history, since the `Realized-by` line was last touched (ADR-0032).
/// The one piece of I/O `embodiment_consistency` needs but can't do itself
/// -- computed here and handed in as plain data, same shape as `full_text`.
pub(crate) fn compute_drifted_records(repo_root: &Path, records: &[Record]) -> HashSet<PathBuf> {
    let mut drifted = HashSet::new();

    for record in records {
        let Some(realized_by_value) = record.header.get("Realized-by") else {
            continue;
        };
        let Some(field) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case("Realized-by"))
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
