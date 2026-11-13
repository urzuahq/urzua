//! Which records a `check --base` run examines (`MILE-123`, SPEC-2).
//!
//! A diff-scoped run reports on the changed records *and* on every record a
//! change reaches. Reporting on the changed files alone hides the defect a
//! change introduces in an unchanged record: delete a target and its
//! referrer, untouched, is left pointing at nothing.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use urzua_core::config::{Config, RecordTypeConfig, RelationRole};
use urzua_core::record::Record;
use urzua_core::report::{DiffScope, Notice, NoticeSeverity};
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
    // `Realized-by` plus the Done-means item clauses (`ADR-96`).
    rules::evidence_text(record, field)
        .map(|value| rules::realized_by_locator_paths(&value))
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

/// A record's pairing identity ([`rules::rename_identity`]).
type Identity = (String, String);

/// Why a deleted record's base copy is being read, which says what an unreadable
/// copy costs and so which rule the warning names.
#[derive(Clone, Copy)]
enum UnreadableCopy {
    /// To pair a renamed record with the status it had: a read failure leaves
    /// the new record unpaired (`status.transition-allowed`).
    RenamePairing,
    /// To judge the deletion itself (`diff.decider-required`).
    Deletion,
}

impl UnreadableCopy {
    fn notice(self, path: &Path, e: impl std::fmt::Display) -> Notice {
        let (subject, message) = match self {
            Self::RenamePairing => (
                rules::RULE_STATUS_TRANSITION_ALLOWED,
                format!(
                    "{}: could not read the merge base's copy of a deleted record, so a record renamed from it cannot be paired and its Status move is not judged: {e}",
                    path.display()
                ),
            ),
            Self::Deletion => (
                rules::RULE_DIFF_DECIDER_REQUIRED,
                format!(
                    "{}: could not read the merge base's copy of a deleted record, so its deletion is not judged: {e}",
                    path.display()
                ),
            ),
        };
        Notice {
            severity: NoticeSeverity::Warning,
            subject: subject.to_string(),
            message,
            detail: None,
        }
    }
}

/// A deleted record's path, pairing identity and `Status` (`None` where the
/// base copy has no readable one).
type DeletedCopy = (PathBuf, Option<Identity>, Option<String>);

/// The statuses of the deleted base copies, by pairing identity.
type DeletedByIdentity = HashMap<Identity, Vec<Option<String>>>;

/// The base copies of the records the diff deleted, by `Stable-Id`, for one
/// record type (`BUG-191`). The diff reports a rename as a deletion plus an
/// addition, and the `Stable-Id` is the identity that survives it: a record
/// added at a new path whose `Stable-Id` a deleted record carried, and whose
/// record id (type and number) is the same, is that record renamed: the map is
/// keyed by [`rules::rename_identity`], so a new record that merely copies a
/// deleted one's `Stable-Id` under another id is not a rename. The `Status` is
/// `None` where the base copy has no readable one. A deleted path the base
/// cannot be read at is skipped with a warning notice, as the record it names
/// is then just new.
fn deleted_by_stable_id(
    repo_root: &Path,
    config: &Config,
    type_name: &str,
    status_field: &str,
    changes: &urzua_io::GitChanges,
    notices: &mut Vec<Notice>,
    warned: &mut HashSet<PathBuf>,
) -> DeletedByIdentity {
    let mut out = DeletedByIdentity::new();
    for (_, identity, status) in deleted_copies(
        repo_root,
        config,
        None,
        type_name,
        status_field,
        changes,
        UnreadableCopy::RenamePairing,
        notices,
        warned,
    ) {
        if let Some(identity) = identity {
            out.entry(identity).or_default().push(status);
        }
    }
    out
}

/// Every record of one type the diff deleted, as the base held it: its path,
/// its pairing identity (if any) and its status, in path order. A deleted
/// file is a record of the type when the type's declaration (`dir`,
/// `exclude_files`) says so in the head's `config` or, when `base_types` is
/// given, in the base revision's: a pull request cannot take a record out of
/// the type's reach by editing the declaration in the same change. The base's
/// declaration is tried first, and parses the base copy. A base copy that
/// cannot be read is left out and noted once per path: `warned` holds the paths
/// already noted, so the two readers of the same deletion (`committed_statuses`
/// and `deleted_gated_records`) do not both warn.
#[allow(clippy::too_many_arguments)]
fn deleted_copies(
    repo_root: &Path,
    config: &Config,
    base_types: Option<&BTreeMap<String, RecordTypeConfig>>,
    type_name: &str,
    status_field: &str,
    changes: &urzua_io::GitChanges,
    reason: UnreadableCopy,
    notices: &mut Vec<Notice>,
    warned: &mut HashSet<PathBuf>,
) -> Vec<DeletedCopy> {
    let mut out = Vec::new();
    let declared: Vec<&RecordTypeConfig> = base_types
        .and_then(|t| t.get(type_name))
        .into_iter()
        .chain(config.record_types.get(type_name))
        .collect();
    if declared.is_empty() {
        return out;
    }
    let mut deleted: Vec<&PathBuf> = changes.deleted.iter().collect();
    deleted.sort();
    for path in deleted {
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !urzua_core::record::is_governed_record_filename(file_name) {
            continue;
        }
        let Some(type_config) = declared
            .iter()
            .find(|t| t.owns_dir(path.parent()) && !t.excludes(file_name))
        else {
            continue;
        };
        let content = match urzua_io::read_at_commit(repo_root, &changes.merge_base, path) {
            Ok(content) => content,
            Err(e) => {
                if warned.insert(path.clone()) {
                    notices.push(reason.notice(path, e));
                }
                continue;
            }
        };
        let prefix = type_config
            .prefix
            .clone()
            .unwrap_or_else(|| type_name.to_ascii_uppercase());
        let old = match crate::discovery::resolved_model(type_name, type_config) {
            Ok(model) => Record::parse_with_model(
                path.clone(),
                type_name.to_string(),
                &content,
                type_config.header_shape,
                prefix,
                &model,
            ),
            Err(_) => Record::parse_with_shape_and_prefix(
                path.clone(),
                type_name.to_string(),
                &content,
                type_config.header_shape,
                prefix,
            ),
        };
        let status =
            urzua_core::authority::gate_status(&old.header, status_field).map(str::to_string);
        out.push((path.clone(), rules::rename_identity(&old), status));
    }
    out
}

/// The records the diff deletes that are not a rename, for the record types in
/// `gated` (the types an authority status gate names), with the `Status` each
/// held at the base (`ADR-89` amendment of 2026-11-10, deletions). A deleted
/// record whose `Stable-Id` and record id both still exist on a head record of
/// its type is that record renamed or moved, not removed, and is left out; one
/// with no `Stable-Id` cannot pair and is deleted. `base_types` is the base
/// revision's record types, given under `authority.decider_table: base` or
/// `both`: a file is a record of a gated type when either revision's
/// declaration owns it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deleted_gated_records(
    repo_root: &Path,
    config: &Config,
    base_types: Option<&BTreeMap<String, RecordTypeConfig>>,
    records: &[Record],
    changes: &urzua_io::GitChanges,
    gated: &std::collections::BTreeSet<&str>,
    notices: &mut Vec<Notice>,
    warned: &mut HashSet<PathBuf>,
) -> Vec<rules::DeletedRecord> {
    let mut out = Vec::new();
    if changes.deleted.is_empty() {
        return out;
    }
    for type_name in gated {
        let field = rules::relation_field_name(config, type_name, RelationRole::Status);
        let heads: HashSet<Identity> = records
            .iter()
            .filter(|r| r.record_type == *type_name)
            .filter_map(rules::rename_identity)
            .collect();
        for (path, identity, status) in deleted_copies(
            repo_root,
            config,
            base_types,
            type_name,
            field,
            changes,
            UnreadableCopy::Deletion,
            notices,
            warned,
        ) {
            let renamed = identity.is_some_and(|i| heads.contains(&i));
            if !renamed {
                out.push(rules::DeletedRecord {
                    path,
                    record_type: type_name.to_string(),
                    status,
                });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Each changed record's `Status` at the merge base, for the types that declare
/// `transitions` (`ADR-72`) and the types in `also` (the types an `authority`
/// status gate names, `MILE-200`, which `diff.decider-required` reads). A path the base does not have is looked up by
/// `Stable-Id` among the records the diff deleted, because the diff reports a
/// rename as a deletion plus an addition (`BUG-191`): a renamed record is
/// judged against the status it had before. The pairing needs the same record
/// id too (a slug rename keeps its number): a record that copies a deleted
/// one's `Stable-Id` under another id, or has no id, is new, so a head cannot
/// pick which base record it is judged against. One with no match, or no
/// `Stable-Id`, reads as new; one matching several deleted records is
/// unreadable, since which of them it was cannot be told. A base copy that
/// exists but cannot be read (not UTF-8, a missing object) is unreadable too,
/// with a notice carrying the error: only a path the base lacks reads as new,
/// since treating a read error as new would skip the transition check.
pub(crate) fn committed_statuses(
    repo_root: &Path,
    config: &Config,
    records: &[Record],
    changes: &urzua_io::GitChanges,
    also: &std::collections::BTreeSet<&str>,
    notices: &mut Vec<Notice>,
    warned: &mut HashSet<PathBuf>,
) -> HashMap<PathBuf, CommittedStatus> {
    let mut out = HashMap::new();
    // The deleted records' Stable-Ids, read once per type and only when a
    // changed record has no copy at its own path.
    let mut renamed_from: HashMap<String, DeletedByIdentity> = HashMap::new();
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
            && !also.contains(record.record_type.as_str())
        {
            continue;
        }
        let committed = match urzua_io::read_at_commit_if_present(
            repo_root,
            &changes.merge_base,
            &record.path,
        ) {
            Err(e) => {
                notices.push(Notice {
                    severity: NoticeSeverity::Warning,
                    subject: rules::RULE_STATUS_TRANSITION_ALLOWED.to_string(),
                    message: format!(
                        "{}: could not read the merge base's copy, so its Status move is not judged: {e}",
                        record.path.display()
                    ),
                    detail: None,
                });
                CommittedStatus::Unreadable
            }
            Ok(None) => {
                let field =
                    rules::relation_field_name(config, &record.record_type, RelationRole::Status);
                let deleted = renamed_from
                    .entry(record.record_type.clone())
                    .or_insert_with(|| {
                        deleted_by_stable_id(
                            repo_root,
                            config,
                            &record.record_type,
                            field,
                            changes,
                            notices,
                            warned,
                        )
                    });
                let matches: &[Option<String>] = rules::rename_identity(record)
                    .and_then(|identity| deleted.get(&identity))
                    .map_or(&[], Vec::as_slice);
                match matches {
                    [] => CommittedStatus::New,
                    [Some(status)] => CommittedStatus::Status(status.clone()),
                    _ => CommittedStatus::Unreadable,
                }
            }
            Ok(Some(content)) => {
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
                match urzua_core::authority::gate_status(&old.header, field) {
                    Some(v) => CommittedStatus::Status(v.to_string()),
                    None => CommittedStatus::Unreadable,
                }
            }
        };
        out.insert(record.path.clone(), committed);
    }
    out
}

/// The config file's path as the diff names it, when the diff changed it.
///
/// Both spellings are tried: a config that is a tracked symlink changes at the
/// link's own path when retargeted, while its canonical path is the unchanged
/// target; editing the target changes the canonical path instead.
fn changed_config_path(
    repo_root: &Path,
    config_path: &Path,
    changes: &urzua_io::GitChanges,
) -> Option<PathBuf> {
    let mut config_rels: Vec<PathBuf> = Vec::new();
    if let Ok(rel) = config_path.strip_prefix(repo_root) {
        config_rels.push(rel.to_path_buf());
    }
    if let (Ok(c), Ok(r)) = (config_path.canonicalize(), repo_root.canonicalize()) {
        if let Ok(rel) = c.strip_prefix(r) {
            config_rels.push(rel.to_path_buf());
        }
    }
    config_rels
        .into_iter()
        .find(|rel| changes.changed.contains(rel))
}

/// Whether any cited locator covers a changed path.
fn cites_changed_path(locators: &[String], changes: &urzua_io::GitChanges) -> bool {
    locators.iter().any(|locator| {
        // An empty locator is a path prefix of everything.
        if locator.is_empty() {
            return false;
        }
        if rules::is_path_glob(locator) {
            // A glob locator covers files, not a path prefix: match it
            // with the matcher locator resolution uses. A glob that does
            // not compile cannot be matched, and locator resolution
            // reports it on the record.
            return rules::path_glob_regex(locator).is_ok_and(|re| {
                changes
                    .changed
                    .iter()
                    .any(|c| re.is_match(&c.to_string_lossy()))
            });
        }
        let path = Path::new(locator);
        changes.changed.iter().any(|c| c.starts_with(path))
    })
}

pub(crate) fn examine(
    repo_root: &Path,
    config: &Config,
    config_path: &Path,
    records: &[Record],
    changes: &urzua_io::GitChanges,
) -> Result<DiffExamination, String> {
    let changed_paths = display(&changes.changed);

    if let Some(rel) = changed_config_path(repo_root, config_path, changes) {
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
        let evidence_changed = cites_changed_path(&locators, changes);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A deleted record whose base copy cannot be read is left out and noted,
    /// as `committed_statuses` notes a changed record's.
    #[test]
    fn an_unreadable_deleted_base_copy_is_skipped_with_a_warning() {
        let repo = std::env::temp_dir().join(format!("urzua-diff-scope-{}", std::process::id()));
        std::fs::create_dir_all(&repo).unwrap();
        let git = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap();
        assert!(git.success());
        let config = urzua_core::config::parse(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n",
        )
        .unwrap();
        let path = PathBuf::from("docs/adr/ADR-1-a.md");
        let changes = urzua_io::GitChanges {
            merge_base: "0000000000000000000000000000000000000000".to_string(),
            changed: [path.clone()].into(),
            deleted: [path.clone()].into(),
            added: Default::default(),
        };
        let mut notices = Vec::new();
        let copies = deleted_copies(
            &repo,
            &config,
            None,
            "adr",
            "Status",
            &changes,
            UnreadableCopy::Deletion,
            &mut notices,
            &mut HashSet::new(),
        );
        assert!(copies.is_empty());
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(matches!(notices[0].severity, NoticeSeverity::Warning));
        assert!(notices[0].message.contains("ADR-1-a.md"), "{notices:?}");
        assert_eq!(notices[0].subject, rules::RULE_DIFF_DECIDER_REQUIRED);
        let _ = std::fs::remove_dir_all(&repo);
    }

    /// The same unreadable deleted copy, reached by the rename pairing and by the
    /// deletion judging, is warned once, under the rule of whichever reached it
    /// first.
    #[test]
    fn an_unreadable_deleted_copy_is_warned_once_per_path() {
        let repo = std::env::temp_dir().join(format!("urzua-diff-once-{}", std::process::id()));
        std::fs::create_dir_all(&repo).unwrap();
        assert!(std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());
        let config = urzua_core::config::parse(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n",
        )
        .unwrap();
        let path = PathBuf::from("docs/adr/ADR-1-a.md");
        let changes = urzua_io::GitChanges {
            merge_base: "0000000000000000000000000000000000000000".to_string(),
            changed: [path.clone()].into(),
            deleted: [path.clone()].into(),
            added: Default::default(),
        };
        let gated: std::collections::BTreeSet<&str> = ["adr"].into();
        let (mut notices, mut warned) = (Vec::new(), HashSet::new());
        let _ = deleted_by_stable_id(
            &repo,
            &config,
            "adr",
            "Status",
            &changes,
            &mut notices,
            &mut warned,
        );
        let _ = deleted_gated_records(
            &repo,
            &config,
            None,
            &[],
            &changes,
            &gated,
            &mut notices,
            &mut warned,
        );
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert_eq!(notices[0].subject, rules::RULE_STATUS_TRANSITION_ALLOWED);
        assert!(
            notices[0].message.contains("renamed from it"),
            "{notices:?}"
        );
        // The deletion reached first names the decider rule.
        let (mut notices, mut warned) = (Vec::new(), HashSet::new());
        let _ = deleted_gated_records(
            &repo,
            &config,
            None,
            &[],
            &changes,
            &gated,
            &mut notices,
            &mut warned,
        );
        let _ = deleted_by_stable_id(
            &repo,
            &config,
            "adr",
            "Status",
            &changes,
            &mut notices,
            &mut warned,
        );
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert_eq!(notices[0].subject, rules::RULE_DIFF_DECIDER_REQUIRED);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
