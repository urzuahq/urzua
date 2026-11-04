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
    compute_drifted_records_at(repo_root, records, config, urzua_io::now_unix())
}

/// The records whose locators drifted. `drifted` holds those with drift
/// outside `drift_grace_days`; `fresh` holds those whose only drift is inside
/// the window, which the rules treat as not drifted yet still never
/// contradict (`MILE-129`). The two sets are disjoint.
#[derive(Default)]
pub(crate) struct DriftedRecords {
    pub(crate) drifted: HashSet<PathBuf>,
    pub(crate) fresh: HashSet<PathBuf>,
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
) -> DriftedRecords {
    let mut drifted = HashSet::new();
    let mut fresh_records = HashSet::new();
    let grace_secs = config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.drift_grace_days)
        .map(|d| i64::from(d) * DAY_SECS);
    let mut commit_times: HashMap<String, Option<i64>> = HashMap::new();
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

        for (locator, pin) in rules::realized_by_pinned_locators(realized_by_value) {
            let locator_path = PathBuf::from(&locator);
            if let Some(pinned) = pin {
                // A pin replaces the git-history heuristic for this locator.
                let current = urzua_io::sha256_hex_of_file(&repo_root.join(&locator_path));
                let changed = current.is_some_and(|c| c != pinned);
                let fresh = changed
                    && last_commit_for_path
                        .entry(locator_path.clone())
                        .or_insert_with(|| {
                            urzua_io::last_commit_for_path(repo_root, &locator_path).unwrap_or(None)
                        })
                        .as_deref()
                        .is_some_and(&mut within_grace);
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
                    urzua_io::last_commit_for_path(repo_root, &locator_path).unwrap_or(None)
                })
            else {
                continue;
            };
            let drifted_pair = *strictly_before
                .entry((reference_commit.clone(), locator_commit.clone()))
                .or_insert_with(|| {
                    urzua_io::commit_strictly_before(repo_root, &reference_commit, locator_commit)
                });
            if drifted_pair {
                if within_grace(locator_commit) {
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
) -> HashMap<PathBuf, u64> {
    let mut ages = HashMap::new();
    if config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.claim_max_age_days)
        .is_none()
    {
        return ages;
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
        let Ok(Some(commit)) = urzua_io::commit_for_line(repo_root, &record.path, field.line)
        else {
            continue;
        };
        let Some(time) = urzua_io::commit_unix_time(repo_root, &commit) else {
            continue;
        };
        ages.insert(
            record.path.clone(),
            u64::try_from((now_unix - time) / DAY_SECS).unwrap_or(0),
        );
    }
    ages
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

/// The source files `embodiment.back-pointer-resolution` scans, with the
/// declared `back_pointer_paths` (`MILE-128`).
pub(crate) struct BackPointerSources {
    pub(crate) files: Vec<rules::PublishedFile>,
    pub(crate) back_pointer_paths: Vec<String>,
}

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
    let files = read_files_under(repo_root, discovered, &source_paths, rule)?;
    Ok(Some(BackPointerSources {
        files,
        back_pointer_paths: source_paths,
    }))
}
