//! Which records a `check --base` run examines (`MILE-123`, SPEC-2).
//!
//! A diff-scoped run reports on the changed records *and* on every record a
//! change reaches. Reporting on the changed files alone hides the defect a
//! change introduces in an unchanged record: delete a target and its
//! referrer, untouched, is left pointing at nothing.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use urzua_core::config::{Config, RelationRole};
use urzua_core::record::Record;
use urzua_core::report::DiffScope;
use urzua_core::rules::{self, CommittedStatus};

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

/// The locators a record cites under its type's embodiment-locator field.
fn cited_locators(config: &Config, record: &Record) -> Vec<String> {
    let field =
        rules::relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
    record
        .header
        .get(field)
        .map(rules::realized_by_locator_paths)
        .unwrap_or_default()
}

/// The ids and cited locators of records that were deleted: read from the
/// merge base, since they are no longer in the tree. A deleted path that no
/// configured type governs has nothing to look up and is skipped.
fn deleted_record_facts(
    repo_root: &Path,
    config: &Config,
    changes: &urzua_io::GitChanges,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut ids = Vec::new();
    let mut locators = Vec::new();
    for path in &changes.deleted {
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !urzua_core::record::is_governed_record_filename(file_name) {
            continue;
        }
        for type_name in config.sorted_type_names() {
            let type_config = &config.record_types[type_name];
            if !type_config.owns_dir(path.parent()) || type_config.excludes(file_name) {
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
            locators.extend(cited_locators(config, &record));
        }
    }
    Ok((ids, locators))
}

/// The `Scope` of every waiver a change touched, as it was and as it is: a
/// changed or deleted waiver can un-waive a finding on a record that did not
/// change, so those records are dependents. `"*"` stands for every record.
fn touched_waiver_scopes(
    repo_root: &Path,
    config: &Config,
    records: &[Record],
    changes: &urzua_io::GitChanges,
) -> Result<Vec<String>, String> {
    let suppression_type = urzua_core::waiver::SuppressionType::from_config(config);
    let type_name = suppression_type.name.as_str();
    let mut waivers = urzua_core::waiver::load_suppressions(
        &records
            .iter()
            .filter(|r| changes.changed.contains(&r.path))
            .cloned()
            .collect::<Vec<_>>(),
        &suppression_type,
    );
    if let Some(type_config) = config.record_types.get(type_name) {
        for path in &changes.changed {
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !urzua_core::record::is_governed_record_filename(file_name)
                || !type_config.owns_dir(path.parent())
                || type_config.excludes(file_name)
            {
                continue;
            }
            // An added waiver has no old version: the read fails and there is
            // nothing to un-waive.
            let Ok(content) = urzua_io::read_at_commit(repo_root, &changes.merge_base, path) else {
                continue;
            };
            let record = Record::parse_with_shape_and_prefix(
                path.clone(),
                type_name.to_string(),
                &content,
                type_config.header_shape,
                type_config
                    .prefix
                    .clone()
                    .unwrap_or_else(|| type_name.to_ascii_uppercase()),
            );
            waivers.extend(urzua_core::waiver::load_suppressions(
                &[record],
                &suppression_type,
            ));
        }
    }
    Ok(waivers.into_iter().map(|w| w.scope).collect())
}

/// Each changed record's `Status` at the merge base, for the types that declare
/// `transitions` (`ADR-72`). A path the base does not have reads as new; the
/// diff reports a rename as a deletion plus an addition, so a renamed record
/// reads as new too.
pub(crate) fn committed_statuses(
    repo_root: &Path,
    config: &Config,
    records: &[Record],
    changes: &urzua_io::GitChanges,
) -> HashMap<PathBuf, CommittedStatus> {
    let mut out = HashMap::new();
    for record in records {
        if !changes.changed.contains(&record.path) {
            continue;
        }
        let Some(type_config) = config.record_types.get(&record.record_type) else {
            continue;
        };
        if !type_config
            .status_decl()
            .is_some_and(|d| d.has_transitions())
        {
            continue;
        }
        let committed = match urzua_io::read_at_commit(repo_root, &changes.merge_base, &record.path)
        {
            Err(_) => CommittedStatus::New,
            Ok(content) => {
                let old = Record::parse_with_shape_and_prefix(
                    record.path.clone(),
                    record.record_type.clone(),
                    &content,
                    type_config.header_shape,
                    type_config
                        .prefix
                        .clone()
                        .unwrap_or_else(|| record.record_type.to_ascii_uppercase()),
                );
                let field =
                    rules::relation_field_name(config, &record.record_type, RelationRole::Status);
                match old.header.get(field).map(str::trim) {
                    Some(v) if !v.is_empty() => CommittedStatus::Status(v.to_string()),
                    _ => CommittedStatus::Unreadable,
                }
            }
        };
        out.insert(record.path.clone(), committed);
    }
    out
}

pub(crate) fn examine(
    repo_root: &Path,
    config: &Config,
    config_path: &Path,
    records: &[Record],
    changes: &urzua_io::GitChanges,
) -> Result<DiffExamination, String> {
    let changed_paths = display(&changes.changed);

    // Both spellings: a config that is a tracked symlink changes at the link's
    // own path when retargeted, while its canonical path is the unchanged
    // target; editing the target changes the canonical path instead.
    let mut config_rels: Vec<PathBuf> = Vec::new();
    if let Ok(rel) = config_path.strip_prefix(repo_root) {
        config_rels.push(rel.to_path_buf());
    }
    if let (Ok(c), Ok(r)) = (config_path.canonicalize(), repo_root.canonicalize()) {
        if let Ok(rel) = c.strip_prefix(r) {
            config_rels.push(rel.to_path_buf());
        }
    }
    if let Some(rel) = config_rels
        .into_iter()
        .find(|rel| changes.changed.contains(rel))
    {
        return Ok(DiffExamination {
            examined: records.iter().map(|r| r.path.clone()).collect(),
            scope: DiffScope {
                changed_paths,
                dependents: Vec::new(),
                whole_corpus_reason: Some(format!(
                    "the configuration file {} changed, so every record is in scope before path arguments narrow it",
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
    let (deleted_ids, deleted_locators) = deleted_record_facts(repo_root, config, changes)?;
    let mut ids: HashSet<String> = deleted_ids.into_iter().collect();
    // The locators a changed or deleted record cites: an unchanged record
    // citing one too is half of a shared-locator finding the change created.
    let mut changed_locators: HashSet<String> = deleted_locators.into_iter().collect();
    for record in records
        .iter()
        .filter(|r| changed_records.contains(r.path.as_path()))
    {
        if let Some(id) = rules::record_id(record) {
            ids.insert(urzua_core::values::RecordId::new(&id).as_str().to_string());
        }
        changed_locators.extend(cited_locators(config, record));
    }

    let waiver_scopes: HashSet<String> =
        touched_waiver_scopes(repo_root, config, records, changes)?
            .into_iter()
            .collect();

    let mut dependents: HashSet<PathBuf> = HashSet::new();
    for record in records {
        if changed_records.contains(record.path.as_path()) {
            continue;
        }
        let waiver_touched = waiver_scopes.contains("*")
            || waiver_scopes.contains(record.path.to_string_lossy().as_ref());
        let names_a_change = rules::header_references(record)
            .iter()
            .any(|r| ids.contains(r.as_str()));
        // An unchanged record sharing a changed record's id is half of a
        // collision the change created.
        let shares_an_id = rules::record_id(record)
            .is_some_and(|id| ids.contains(urzua_core::values::RecordId::new(&id).as_str()));
        let locators = cited_locators(config, record);
        let evidence_changed = locators.iter().any(|locator| {
            // An empty locator is a path prefix of everything.
            if locator.is_empty() {
                return false;
            }
            let path = Path::new(locator);
            changes.changed.iter().any(|c| c.starts_with(path))
        });
        let shares_a_locator = locators
            .iter()
            .any(|l| !l.is_empty() && changed_locators.contains(l));
        if waiver_touched || names_a_change || shares_an_id || evidence_changed || shares_a_locator
        {
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
