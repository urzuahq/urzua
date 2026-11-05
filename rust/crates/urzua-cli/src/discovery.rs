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
        urzua_io::DiscoverySource::GitDiff => ScopeSource::GitDiff,
    }
}

/// There is no fallback that walks the directory (`BUG-170`, SPEC-2): discovery
/// is the tracked set, so a command that reads a repository needs one.
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
            None => {
                return Err(
                    "urzua needs a git repository: no .git directory found above the given path"
                        .to_string(),
                )
            }
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
    // Sorted, not `config.record_types`' own HashMap order: a cross-type ID
    // collision's first-seen-wins index slot (`build_index_reporting_collisions`)
    // would otherwise depend on the per-process hash-map seed, so which of
    // two colliding records other rules resolve against could flip between
    // runs of the same unchanged corpus.
    let type_names = config.sorted_type_names();
    for type_name in type_names {
        let type_config = &config.record_types[type_name];
        let model = urzua_core::document_model::declared_model(type_name, type_config)
            .map_err(|e| e.to_string())?
            .map(|m| {
                m.resolve()
                    .map_err(|e| format!("record type '{type_name}': {e}"))
            })
            .transpose()?
            .unwrap_or_default();
        for rel_path in discovered {
            // This directory, not this subtree (RFC-35). Prefix matching let
            // two types claim one record whenever their dirs nested, which no
            // schema rule resolved -- and four heuristics accreted in `init`
            // guessing what an adopter meant by the overlap. A directory is a
            // type; `docs/rfc` holds RFCs.
            if !type_config.owns_dir(rel_path.parent()) {
                continue;
            }
            let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !urzua_core::record::is_governed_record_filename(file_name)
                || type_config.excludes(file_name)
            {
                continue;
            }
            let full_path = repo_root.join(rel_path);
            match urzua_io::read_to_string(&full_path) {
                Ok(content) => {
                    let type_prefix = type_config
                        .prefix
                        .clone()
                        .unwrap_or_else(|| type_name.to_ascii_uppercase());
                    records.push(Record::parse_with_model(
                        rel_path.clone(),
                        type_name.clone(),
                        &content,
                        type_config.header_shape,
                        type_prefix,
                        &model,
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

/// Record files that exist on disk directly in a record type's directories but
/// that git does not track, by identifier (`BUG-13`). Discovery reads only
/// tracked and staged files (`ADR-6`), so a reference to a record drafted but
/// not yet `git add`ed resolves to nothing; this is the on-disk fact that lets
/// a rule say so, supplied here because `urzua-core` touches no filesystem.
/// Read with the loader's own identity rules. A file that cannot be read yields
/// no entry: the hint is lost and the reference reads as missing, as before.
pub(crate) fn untracked_records(
    repo_root: &Path,
    config: &Config,
    tracked: &[PathBuf],
) -> urzua_core::rules::UntrackedRecords {
    let tracked: std::collections::HashSet<&Path> = tracked.iter().map(PathBuf::as_path).collect();
    let mut on_disk: Vec<PathBuf> = Vec::new();
    for type_config in config.record_types.values() {
        for dir in type_config.all_dirs() {
            let Ok(entries) = std::fs::read_dir(repo_root.join(dir)) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    continue;
                }
                let rel = Path::new(dir).join(entry.file_name());
                if !tracked.contains(rel.as_path()) {
                    on_disk.push(rel);
                }
            }
        }
    }
    on_disk.sort();
    on_disk.dedup();
    let Ok((records, _)) = load_records(
        repo_root,
        &on_disk,
        &std::collections::HashSet::new(),
        config,
    ) else {
        return Default::default();
    };
    records
        .iter()
        .filter_map(|record| {
            urzua_core::rules::record_id(record)
                .map(|id| (urzua_core::values::RecordId::new(&id), record.path.clone()))
        })
        .collect()
}

/// Parses `content` as the record at `rel_path` would be loaded, though the
/// file need not exist: the type is the one whose directory holds the path,
/// under the same filename rules `load_records` applies. For the proposed
/// content `fix --stdin-path` judges (`MILE-159`).
pub(crate) fn parse_prospective_record(
    config: &Config,
    rel_path: &Path,
    content: &str,
) -> Result<Record, String> {
    let file_name = rel_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for type_name in config.sorted_type_names() {
        let type_config = &config.record_types[type_name];
        if !type_config.owns_dir(rel_path.parent())
            || !urzua_core::record::is_governed_record_filename(file_name)
            || type_config.excludes(file_name)
        {
            continue;
        }
        let model = urzua_core::document_model::declared_model(type_name, type_config)
            .map_err(|e| e.to_string())?
            .map(|m| {
                m.resolve()
                    .map_err(|e| format!("record type '{type_name}': {e}"))
            })
            .transpose()?
            .unwrap_or_default();
        let type_prefix = type_config
            .prefix
            .clone()
            .unwrap_or_else(|| type_name.to_ascii_uppercase());
        return Ok(Record::parse_with_model(
            rel_path.to_path_buf(),
            type_name.clone(),
            content,
            type_config.header_shape,
            type_prefix,
            &model,
        ));
    }
    Err(format!(
        "no record type governs {} (a record type governs the .md files directly in its `dir`, other than excluded and underscore-prefixed names)",
        rel_path.display()
    ))
}

/// `p` (absolute, or relative to the working directory) as a repo-relative
/// path, without requiring that it exist.
pub(crate) fn relative_to_root(
    repo_root: &std::path::Path,
    p: &std::path::Path,
) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .and_then(|c| c.canonicalize())
        .unwrap_or_default();
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let mut normal = PathBuf::new();
    for c in joined.components() {
        match c {
            std::path::Component::ParentDir => {
                normal.pop();
            }
            std::path::Component::CurDir => {}
            other => normal.push(other),
        }
    }
    normal
        .strip_prefix(repo_root)
        .map(PathBuf::from)
        .map_err(|_| format!("{} is outside the repository", p.display()))
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
) -> DriftedRecords {
    compute_drifted_records_at(repo_root, records, config, urzua_io::now_unix(), None)
}

/// Drift for proposed content not yet on disk: `proposed` is blamed in place
/// of the working-tree file, so an unchanged `Realized-by` line keeps its
/// commit and an edited one counts as touched now (`MILE-159`).
pub(crate) fn compute_drifted_records_for_content(
    repo_root: &Path,
    records: &[Record],
    config: &Config,
    proposed: &str,
) -> DriftedRecords {
    compute_drifted_records_at(
        repo_root,
        records,
        config,
        urzua_io::now_unix(),
        Some(proposed),
    )
}

/// The records whose locators drifted. `drifted` holds those with drift
/// outside `drift_grace_days`; `fresh` holds those whose only drift is inside
/// the window, which the rules treat as not drifted yet still never
/// contradict (`MILE-129`). The two sets are disjoint.
#[derive(Default)]
pub(crate) struct DriftedRecords {
    pub(crate) drifted: HashSet<PathBuf>,
    pub(crate) fresh: HashSet<PathBuf>,
    /// Git failures met while computing drift, deduplicated. A failure is a
    /// record git could not judge, never "no drift" (`MILE-159`).
    pub(crate) git_problems: Vec<String>,
    /// The repository is a shallow clone, detected once per run: the history
    /// drift reads is truncated, so a zero here proves nothing (`MILE-159`).
    pub(crate) shallow: bool,
}

/// Notice subject for git trouble in the drift computation (`MILE-159`).
pub(crate) const NOTICE_DRIFT_GIT: &str = "embodiment-drift-git";

/// The notices a drift computation owes the reader: one for a shallow clone,
/// one summarizing git failures (the first few named, the rest counted).
pub(crate) fn drift_notices(shallow: bool, problems: &[String]) -> Vec<urzua_core::report::Notice> {
    use urzua_core::report::{Notice, NoticeSeverity};
    let mut out = Vec::new();
    if shallow {
        out.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_DRIFT_GIT.to_string(),
            message: "the repository is a shallow clone, so the git history that embodiment drift reads is truncated: a result of no drift cannot be trusted (fetch full history, e.g. `git fetch --unshallow`)".to_string(),
        });
    }
    if !problems.is_empty() {
        const SHOWN: usize = 3;
        let shown = problems
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let more = problems.len().saturating_sub(SHOWN);
        out.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_DRIFT_GIT.to_string(),
            message: format!(
                "{} git call(s) failed while computing embodiment drift, so the affected record(s) were not judged for drift: {shown}{}",
                problems.len(),
                if more > 0 {
                    format!(" (and {more} more)")
                } else {
                    String::new()
                }
            ),
        });
    }
    out
}

/// `git blame` for many (path, line) pairs at once, on a few threads: blame
/// is the dominant cost of a drift run and the calls are independent
/// (`MILE-159`). Results come back in input order.
fn blame_lines_parallel(
    repo_root: &Path,
    work: &[(PathBuf, usize)],
    proposed: Option<&str>,
) -> Vec<Result<Option<String>, String>> {
    if work.len() < 4 {
        return work
            .iter()
            .map(|(p, l)| urzua_io::commit_for_line_in(repo_root, p, *l, proposed))
            .collect();
    }
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(2, 8);
    let chunk = work.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = work
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(p, l)| urzua_io::commit_for_line_in(repo_root, p, *l, proposed))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

/// The seconds in one day, for the `MILE-129` day-denominated options.
const DAY_SECS: i64 = 86_400;

/// [`compute_drifted_records`] with the clock supplied. Two opt-in refinements
/// (`MILE-129`), both off unless declared: a locator carrying a `@sha256:` pin
/// drifts when its current content hash differs (and only then, whatever git
/// history says), and `drift_grace_days` holds back a locator whose last
/// commit is newer than the window.
pub(crate) fn compute_drifted_records_at(
    repo_root: &Path,
    records: &[Record],
    config: &Config,
    now_unix: i64,
    proposed: Option<&str>,
) -> DriftedRecords {
    let mut drifted = HashSet::new();
    let mut fresh_records = HashSet::new();
    let mut problems: Vec<String> = Vec::new();
    let note = |problems: &mut Vec<String>, e: String| {
        if !problems.contains(&e) {
            problems.push(e);
        }
    };
    let shallow = match urzua_io::is_shallow_repository(repo_root) {
        Ok(b) => b,
        Err(e) => {
            note(&mut problems, e);
            false
        }
    };
    let grace_secs = config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.drift_grace_days)
        .map(|d| i64::from(d) * DAY_SECS);
    let audit_field = config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.drift_audit_field.as_deref())
        .map(str::trim);
    let mut commit_times: HashMap<String, Option<i64>> = HashMap::new();
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf());
    // Implements: ADR-67 (the grace window)
    // Inside the window, so not yet drift. An undatable commit is never held
    // back: the window only ever suppresses what it can prove is fresh.
    let mut within_grace = |commit: &str| -> bool {
        let Some(grace) = grace_secs else {
            return false;
        };
        let time = *commit_times
            .entry(commit.to_string())
            .or_insert_with(|| urzua_io::commit_unix_time(repo_root, commit));
        time.is_some_and(|t| now_unix - t < grace)
    };

    // A locator cited by several records (a shared implementation module, a
    // shared test file) would otherwise re-run the same `git log`/`merge-base`
    // subprocess once per citing record -- both memoized here across the
    // whole corpus (`BUG-135`).
    let mut last_commit_for_path: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut strictly_before: HashMap<(String, String), bool> = HashMap::new();

    // One blame per record that carries a locator field, run up front on a
    // few threads (the per-record loop below stays sequential and cheap).
    let locator_line = |record: &Record| -> Option<usize> {
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        record.header.get(locator_field)?;
        record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == locator_field)
            .map(|f| f.line)
    };
    let work: Vec<(PathBuf, usize)> = records
        .iter()
        .filter_map(|r| locator_line(r).map(|l| (r.path.clone(), l)))
        .collect();
    let mut blames = blame_lines_parallel(repo_root, &work, proposed).into_iter();

    for record in records {
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        let Some(realized_by_value) = record.header.get(locator_field) else {
            continue;
        };
        // Implements: ADR-67 (the audit anchor)
        // A declared, well-formed last-audit date replaces the cited file's
        // last change as where the window counts from; a malformed one is
        // reported by the rule and falls back to the file (`MILE-166`).
        let audit_time = audit_field
            .and_then(|f| record.header.get(f))
            .and_then(|v| rules::date_unix(v).ok());
        let mut held_back = |commit: &str| match (audit_time, grace_secs) {
            (Some(t), Some(grace)) => now_unix - t < grace,
            _ => within_grace(commit),
        };
        if locator_line(record).is_none() {
            continue;
        }
        let reference_commit = match blames.next() {
            Some(Ok(Some(c))) => c,
            Some(Err(e)) => {
                note(&mut problems, e);
                continue;
            }
            _ => continue,
        };
        // An all-zero hash is a line the proposed content changed or added: it
        // is touched now, so no locator can have moved since (`MILE-159`).
        if reference_commit.bytes().all(|b| b == b'0') {
            continue;
        }

        for entry in urzua_core::locator::realized_by_locators(realized_by_value) {
            // A locator that does not parse is `embodiment.locator-exists`' finding.
            let Ok(parsed) = entry.parsed else { continue };
            // A commit anchor is reserved for MILE-84 and is reported by
            // `embodiment.locator-exists` as unsupported; it is not judged
            // for drift on the file's history either.
            if matches!(parsed.anchor, Some(urzua_core::locator::Anchor::Commit(_))) {
                continue;
            }
            let locator = parsed.path.clone();
            let locator_path = PathBuf::from(&locator);
            if let (Some(urzua_core::locator::Anchor::Pin(pinned)), Some(_)) =
                (&parsed.anchor, &parsed.scope)
            {
                // Implements: ADR-74 (a scoped pin drifts on its span's hash)
                // The file's last commit says nothing about the scope, so there
                // is no file-based grace window: only a declared audit date can
                // hold a scoped pin back.
                let urzua_core::fix::PinRead::Current { hash, .. } =
                    crate::pins::read_pin(repo_root, config, &texts, &parsed)
                else {
                    // A scope that does not resolve is reported by the rule and
                    // not also judged for drift.
                    continue;
                };
                if hash != *pinned {
                    let audited_fresh = matches!((audit_time, grace_secs), (Some(t), Some(grace)) if now_unix - t < grace);
                    if audited_fresh {
                        fresh_records.insert(record.path.clone());
                    } else {
                        drifted.insert(record.path.clone());
                        break;
                    }
                }
                continue;
            }
            if let Some(urzua_core::locator::Anchor::Pin(pinned)) = &parsed.anchor {
                // Implements: ADR-67 (a pin drifts on its content hash)
                // A pin replaces the git-history heuristic for this locator.
                let current = urzua_io::sha256_hex_of_file(&repo_root.join(&locator_path));
                let changed = current.is_some_and(|c| c != *pinned);
                let fresh = changed
                    && last_commit_for_path
                        .entry(locator_path.clone())
                        .or_insert_with(|| {
                            urzua_io::last_commit_for_path(repo_root, &locator_path).unwrap_or_else(
                                |e| {
                                    note(&mut problems, e);
                                    None
                                },
                            )
                        })
                        .as_deref()
                        .is_some_and(&mut held_back);
                if changed && !fresh {
                    drifted.insert(record.path.clone());
                    break;
                }
                if fresh {
                    fresh_records.insert(record.path.clone());
                }
                continue;
            }
            let Some(locator_commit) = last_commit_for_path
                .entry(locator_path.clone())
                .or_insert_with(|| {
                    urzua_io::last_commit_for_path(repo_root, &locator_path).unwrap_or_else(|e| {
                        note(&mut problems, e);
                        None
                    })
                })
            else {
                continue;
            };
            let drifted_pair = *strictly_before
                .entry((reference_commit.clone(), locator_commit.clone()))
                .or_insert_with(|| {
                    urzua_io::commit_strictly_before(repo_root, &reference_commit, locator_commit)
                        .unwrap_or_else(|e| {
                            note(&mut problems, e);
                            false
                        })
                });
            if drifted_pair {
                if held_back(locator_commit) {
                    fresh_records.insert(record.path.clone());
                } else {
                    drifted.insert(record.path.clone());
                    break;
                }
            }
        }
    }

    // Confirmed drift outranks fresh drift on the same record.
    fresh_records.retain(|path| !drifted.contains(path));
    DriftedRecords {
        drifted,
        fresh: fresh_records,
        git_problems: problems,
        shallow,
    }
}

/// Whole days since each record's `Realized-by` line was last touched
/// (`MILE-129`), for `claim_max_age_days`. Empty unless that option is set,
/// so a repository that never opted in pays for no extra `git blame`.
pub(crate) fn compute_claim_ages(
    repo_root: &Path,
    records: &[Record],
    config: &Config,
    now_unix: i64,
) -> (HashMap<PathBuf, u64>, Vec<String>) {
    let mut ages = HashMap::new();
    let mut problems: Vec<String> = Vec::new();
    if config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.claim_max_age_days)
        .is_none()
    {
        return (ages, problems);
    }
    for record in records {
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        let Some(field) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == locator_field)
        else {
            continue;
        };
        let commit = match urzua_io::commit_for_line(repo_root, &record.path, field.line) {
            Ok(Some(c)) => c,
            Ok(None) => continue,
            Err(e) => {
                if !problems.contains(&e) {
                    problems.push(e);
                }
                continue;
            }
        };
        let Some(time) = urzua_io::commit_unix_time(repo_root, &commit) else {
            continue;
        };
        ages.insert(
            record.path.clone(),
            u64::try_from((now_unix - time) / DAY_SECS).unwrap_or(0),
        );
    }
    (ages, problems)
}

/// Reads every tracked file under one of the declared path prefixes, for the
/// rules that scan paths (`boundary.published-path-reference`,
/// `embodiment.back-pointer-resolution`); `rule` names the caller in an I/O
/// error. Kept in the CLI because `urzua-core` is pure (ADR-5).
///
/// Nothing is skipped silently. A file that is not valid UTF-8, a symlink or
/// anything else that is not a regular file is handed over with no content, so
/// the rule counts it as unreadable instead of clean. A tracked file that is
/// gone from disk (a staged deletion, or one deleted and not yet staged) is
/// not published and is left out. Any other I/O failure aborts the run: a file
/// the rule could not read is a file it did not check.
pub(crate) fn read_files_under(
    repo_root: &std::path::Path,
    discovered: &urzua_io::DiscoveredFiles,
    prefixes: &[String],
    rule: &str,
) -> Result<Vec<rules::PublishedFile>, String> {
    let mut out = Vec::new();
    for path in &discovered.paths {
        if discovered.staged_deletions.contains(path)
            || !prefixes
                .iter()
                .any(|entry| rules::path_is_published(path, entry))
        {
            continue;
        }
        let full = repo_root.join(path);
        let meta = match std::fs::symlink_metadata(&full) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{rule}: could not stat {}: {e}", full.display())),
        };
        let content = if meta.is_file() {
            match urzua_io::read_to_string(&full) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => None,
                Err(e) => return Err(format!("{rule}: could not read {}: {e}", full.display())),
            }
        } else {
            None
        };
        out.push(rules::PublishedFile {
            path: path.clone(),
            content,
        });
    }
    Ok(out)
}

pub(crate) use rules::BackPointerSources;

/// Reads the back-pointer sources, or `None` when the rule is absent or off:
/// evidence is collected only while the rule is enabled, so a repository that
/// never opted in pays for no scan and sees no change in `Embodiment`.
pub(crate) fn read_back_pointer_sources(
    repo_root: &Path,
    discovered: &urzua_io::DiscoveredFiles,
    config: &Config,
) -> Result<Option<BackPointerSources>, String> {
    read_sources_for_rule(
        repo_root,
        discovered,
        config,
        rules::RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
    )
}

/// The same read for `embodiment.back-pointer-decision-accepted`, which
/// declares its own `back_pointer_paths` (`MILE-133`).
pub(crate) fn read_decision_gate_sources(
    repo_root: &Path,
    discovered: &urzua_io::DiscoveredFiles,
    config: &Config,
) -> Result<Option<BackPointerSources>, String> {
    read_sources_for_rule(
        repo_root,
        discovered,
        config,
        rules::RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
    )
}

fn read_sources_for_rule(
    repo_root: &Path,
    discovered: &urzua_io::DiscoveredFiles,
    config: &Config,
    rule: &str,
) -> Result<Option<BackPointerSources>, String> {
    let Some(source_paths) = config
        .rules
        .get(rule)
        .filter(|s| s.level != urzua_core::config::RuleLevel::Off)
        .and_then(|s| s.back_pointer_paths.clone())
    else {
        return Ok(None);
    };
    let test_paths = config
        .rules
        .get(rule)
        .and_then(|s| s.test_paths.clone())
        .unwrap_or_default();
    let files = read_files_under(repo_root, discovered, &source_paths, rule)?;
    Ok(Some(BackPointerSources {
        files,
        back_pointer_paths: source_paths,
        test_paths,
    }))
}

// ---- Claim and index readers (the filesystem side of claim.status-agreement and index.freshness) ----

/// Whether an already-resolved (canonicalized) path stays inside the
/// repository -- the one containment test every `claim_paths` check needs,
/// so a future change to the policy (e.g. `BUG-117`'s reflexive-`starts_with`
/// case) has one place to land. Both `resolve_inside_repo`, below, and the
/// pre-flight `claim_paths` validation in `run` call this rather than each
/// inlining `starts_with`.
pub(crate) fn path_is_inside_repo(
    repo_canonical: &std::path::Path,
    resolved: &std::path::Path,
) -> bool {
    resolved.starts_with(repo_canonical)
}

/// Canonicalizes `candidate` and confirms it stays inside `repo_canonical` --
/// the one check `claim_paths`' directory and file branches both need, so a
/// future change to the symlink/containment policy has one place to land
/// rather than two that can drift apart.
pub(crate) fn resolve_inside_repo(
    repo_canonical: &std::path::Path,
    candidate: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let resolved = candidate.canonicalize().map_err(|e| {
        format!(
            "claim_paths: could not resolve {}: {e}",
            candidate.display()
        )
    })?;
    if !path_is_inside_repo(repo_canonical, &resolved) {
        return Err(format!(
            "claim_paths: {} resolves outside the repository",
            candidate.display()
        ));
    }
    Ok(resolved)
}

/// Reads the files a repository pointed `claim.status-agreement` at. Kept in
/// the CLI because `urzua-core` is pure (ADR-5) -- the rule is given file
/// contents, never a path to open.
pub(crate) fn read_claim_files(
    repo_root: &std::path::Path,
    prefixes: &[String],
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let repo_canonical = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    for prefix in prefixes {
        let dir = repo_root.join(prefix);
        // An absent prefix is not an unreadable one. Git keeps no empty
        // directory, so a declared `.changeset` ceases to exist the moment a
        // release consumes the last fragment, and treating that as an I/O
        // failure took the whole command down. The caller reports the absence
        // as a `Notice`; there is simply nothing here to read.
        //
        // `try_exists`, not `exists`: the latter swallows every error into
        // `false`, including a permission failure on an ancestor, which would
        // silently read this claim path as absent rather than unreadable --
        // the caller's guard makes the same distinction for the same reason.
        match dir.try_exists() {
            Ok(false) => continue,
            Ok(true) => {}
            Err(e) => {
                return Err(format!(
                    "claim_paths: could not check {}: {e}",
                    dir.display()
                ))
            }
        }
        // `claim_paths` is a path *prefix*, so the claims may sit any depth
        // below it: reading one level deep made a nested layout report a clean
        // run over an empty claim list (BUG-63).
        //
        // Nothing here is skipped on error. A claim the rule could not read is
        // a claim it did not check, and continuing past it reports agreement
        // over an incomplete corpus -- the failure this whole rule exists to
        // prevent, arrived at from the inside.
        let mut paths = Vec::new();
        // A link may point anywhere inside the repository -- declaring
        // `claims/x -> shared` is the author saying claims also live there,
        // the same thing the declared root already does (BUG-85). What it may
        // not do is point at an *ancestor* of the prefix: that sweeps the whole
        // tree, so files never declared as claims produced blocking findings
        // under paths that do not exist (BUG-88).
        let prefix_canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        let mut visited: std::collections::HashSet<std::path::PathBuf> =
            std::collections::HashSet::new();
        visited.insert(prefix_canonical.clone());
        let mut pending = vec![dir.clone()];
        while let Some(current) = pending.pop() {
            let entries = std::fs::read_dir(&current)
                .map_err(|e| format!("claim_paths: could not read {}: {e}", current.display()))?;
            for entry in entries {
                let path = entry
                    .map_err(|e| {
                        format!(
                            "claim_paths: could not read an entry of {}: {e}",
                            current.display()
                        )
                    })?
                    .path();
                // `symlink_metadata`: a link is never descended into. A link to
                // an ancestor gives an unbounded walk that leaves the declared
                // prefix entirely -- measured at 904 files read, from other
                // repositories on disk (BUG-69).
                // A link is never *descended* into -- a link to an ancestor
                // gives an unbounded walk outside the declared prefix (BUG-69)
                // -- but a linked file is still read. `symlink_metadata`'s
                // `is_file()` is false for a link whatever it points at, so
                // testing it alone dropped a symlinked claim in silence, in a
                // walk that aborts on every other failure (BUG-75).
                // `symlink_metadata` first, so a dangling link is a named
                // error rather than a silent skip.
                std::fs::symlink_metadata(&path)
                    .map_err(|e| format!("claim_paths: could not stat {}: {e}", path.display()))?;
                let target = std::fs::metadata(&path).map_err(|e| {
                    format!("claim_paths: could not resolve {}: {e}", path.display())
                })?;
                if target.is_dir() {
                    // Followed only where it stays inside the repository, which
                    // is the same test the declared root gets: rejecting every
                    // link made the two paths disagree about one symlink
                    // (BUG-85). `visited` is what actually stops the unbounded
                    // walk a link to an ancestor produces (BUG-69).
                    let resolved = resolve_inside_repo(&repo_canonical, &path)?;
                    // `starts_with` is reflexive (`BUG-117`): a link resolving
                    // to exactly the prefix root, not a genuine ancestor,
                    // causes no unbounded walk -- `visited` (seeded with the
                    // prefix root before this loop starts) already dedupes it
                    // silently below.
                    if resolved != prefix_canonical && prefix_canonical.starts_with(&resolved) {
                        return Err(format!(
                            "claim_paths: {} resolves to an ancestor of the declared prefix, which would read the whole tree",
                            path.display()
                        ));
                    }
                    if visited.insert(resolved) {
                        pending.push(path);
                    }
                } else if target.is_file() && path.extension().is_some_and(|e| e == "md") {
                    // A linked file is read, so its *target* must also stay
                    // inside the repository: a symlink named `.md` otherwise
                    // feeds an arbitrary file on the machine to the claim
                    // scanner, and its contents steer the findings.
                    resolve_inside_repo(&repo_canonical, &path)?;
                    paths.push(path);
                }
            }
        }
        // Sorted so a finding's order does not depend on the filesystem.
        paths.sort();
        for path in paths {
            let content = urzua_io::read_to_string(&path)
                .map_err(|e| format!("claim_paths: could not read {}: {e}", path.display()))?;
            let shown = path
                .strip_prefix(repo_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            out.push((shown, content));
        }
    }
    Ok(out)
}

/// Reads the index artifacts `index.freshness` compares (`MILE-139`), in the
/// CLI because `urzua-core` is pure (`ADR-5`). A missing file and one that is
/// not text are handed over as such, never as empty. A path that resolves
/// outside the repository, or any other I/O failure, aborts: an artifact the
/// rule could not read is one it did not compare.
pub(crate) fn read_index_artifacts(
    repo_root: &std::path::Path,
    specs: &[urzua_core::config::IndexSpec],
) -> Result<Vec<rules::IndexArtifact>, String> {
    let repo_canonical = repo_root
        .canonicalize()
        .map_err(|e| format!("index.freshness: could not resolve the repository root: {e}"))?;
    let mut out = Vec::new();
    for spec in specs {
        let full = repo_root.join(&spec.path);
        let content = match full.canonicalize() {
            Ok(resolved) if !path_is_inside_repo(&repo_canonical, &resolved) => {
                return Err(format!(
                    "index.freshness: {} resolves outside the repository",
                    spec.path
                ))
            }
            Ok(resolved) => match urzua_io::read_to_string(&resolved) {
                Ok(text) => rules::IndexContent::Text(text),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    rules::IndexContent::NotText
                }
                Err(_) if resolved.is_dir() => rules::IndexContent::NotText,
                Err(e) => {
                    return Err(format!(
                        "index.freshness: could not read {}: {e}",
                        spec.path
                    ))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => rules::IndexContent::Missing,
            Err(e) => {
                return Err(format!(
                    "index.freshness: could not resolve {}: {e}",
                    spec.path
                ))
            }
        };
        out.push(rules::IndexArtifact {
            spec: spec.clone(),
            content,
        });
    }
    Ok(out)
}
