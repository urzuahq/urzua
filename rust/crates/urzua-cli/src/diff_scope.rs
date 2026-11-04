//! Which records a `check --base` run examines (`MILE-123`, SPEC-2).
//!
//! A diff-scoped run reports on the changed records *and* on every record a
//! change reaches. Reporting on the changed files alone hides the defect a
//! change introduces in an unchanged record: delete a target and its
//! referrer, untouched, is left pointing at nothing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use urzua_core::config::{Config, RelationRole};
use urzua_core::record::Record;
use urzua_core::report::DiffScope;
use urzua_core::rules;

/// The examined set and the account of how it was reached.
pub(crate) struct DiffExamination {
    /// Record paths to examine and report on.
    pub examined: HashSet<PathBuf>,
    pub scope: DiffScope,
}

fn display(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Vec<String> {
    let mut out: Vec<String> = paths
        .into_iter()
        .map(|p| p.as_ref().display().to_string())
        .collect();
    out.sort();
    out
}

/// The ids of records that were deleted: read from the merge base, since they
/// are no longer in the tree. A deleted path that no configured type governs
/// has no id to look up and is skipped.
fn deleted_record_ids(
    repo_root: &Path,
    config: &Config,
    changes: &urzua_io::GitChanges,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for path in &changes.deleted {
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !urzua_core::record::is_governed_record_filename(file_name) {
            continue;
        }
        for type_name in config.sorted_type_names() {
            let type_config = &config.record_types[type_name];
            if path.parent() != Some(Path::new(&type_config.dir)) {
                continue;
            }
            let content = urzua_io::read_at_commit(repo_root, &changes.merge_base, path)
                .map_err(|e| format!("could not read deleted record {}: {e}", path.display()))?;
            let record = Record::parse_with_shape_and_prefix(
                path.clone(),
                type_name.clone(),
                &content,
                type_config.header_shape,
                type_config
                    .prefix
                    .clone()
                    .unwrap_or_else(|| type_name.to_ascii_uppercase()),
            );
            if let Some(id) = rules::record_id(&record) {
                ids.push(urzua_core::values::RecordId::new(&id).as_str().to_string());
            }
        }
    }
    Ok(ids)
}

pub(crate) fn examine(
    repo_root: &Path,
    config: &Config,
    config_path: &Path,
    records: &[Record],
    changes: &urzua_io::GitChanges,
) -> Result<DiffExamination, String> {
    let changed_paths = display(&changes.changed);

    let config_rel = config_path
        .canonicalize()
        .ok()
        .and_then(|c| {
            repo_root
                .canonicalize()
                .ok()
                .and_then(|r| c.strip_prefix(r).ok().map(Path::to_path_buf))
        })
        .or_else(|| {
            config_path
                .strip_prefix(repo_root)
                .ok()
                .map(Path::to_path_buf)
        });
    if let Some(rel) = config_rel.filter(|rel| changes.changed.contains(rel)) {
        return Ok(DiffExamination {
            examined: records.iter().map(|r| r.path.clone()).collect(),
            scope: DiffScope {
                changed_paths,
                dependents: Vec::new(),
                whole_corpus_reason: Some(format!(
                    "the configuration file {} changed, so every record is examined",
                    rel.display()
                )),
            },
        });
    }

    let changed_records: HashSet<&Path> = records
        .iter()
        .filter(|r| changes.changed.contains(&r.path))
        .map(|r| r.path.as_path())
        .collect();

    // The ids a change can strand a reference to: every changed record's, and
    // every deleted record's, which only the merge base still holds.
    let mut ids: HashSet<String> = deleted_record_ids(repo_root, config, changes)?
        .into_iter()
        .collect();
    for record in records
        .iter()
        .filter(|r| changed_records.contains(r.path.as_path()))
    {
        if let Some(id) = rules::record_id(record) {
            ids.insert(urzua_core::values::RecordId::new(&id).as_str().to_string());
        }
    }

    let mut dependents: HashSet<PathBuf> = HashSet::new();
    for record in records {
        if changed_records.contains(record.path.as_path()) {
            continue;
        }
        let names_a_change = rules::header_references(record)
            .iter()
            .any(|r| ids.contains(r.as_str()));
        // An unchanged record sharing a changed record's id is half of a
        // collision the change created.
        let shares_an_id = rules::record_id(record)
            .is_some_and(|id| ids.contains(urzua_core::values::RecordId::new(&id).as_str()));
        let evidence_changed = {
            let field = rules::relation_field_name(
                config,
                &record.record_type,
                RelationRole::EmbodimentLocator,
            );
            record.header.get(field).is_some_and(|value| {
                rules::realized_by_locator_paths(value)
                    .iter()
                    .any(|locator| {
                        // An empty locator is a path prefix of everything.
                        if locator.is_empty() {
                            return false;
                        }
                        let locator = Path::new(locator);
                        changes.changed.iter().any(|c| c.starts_with(locator))
                    })
            })
        };
        if names_a_change || shares_an_id || evidence_changed {
            dependents.insert(record.path.clone());
        }
    }

    let examined: HashSet<PathBuf> = changed_records
        .iter()
        .map(|p| p.to_path_buf())
        .chain(dependents.iter().cloned())
        .collect();
    Ok(DiffExamination {
        examined,
        scope: DiffScope {
            changed_paths,
            dependents: display(&dependents),
            whole_corpus_reason: None,
        },
    })
}
