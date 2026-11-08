//! Shared, non-command-specific plumbing every command builds on: locating
//! the repo root, loading config, loading records, and scoping/annotating
//! what was discovered. None of this is a command in its own right.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use urzua_core::config::{Config, LocatorKind};
use urzua_core::record::Record;
use urzua_core::report::{Failure, Notice, NoticeSeverity, ScopeSource};
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

pub(crate) fn load_config(path: &PathBuf) -> Result<Config, Failure> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read config at {}: {e}", path.display()))?;
    urzua_core::config::parse(&content).map_err(|e| match e {
        // The structured form rides along so a could-not-run report can carry
        // `{found, supported, current, migration}` (`ADR-77`).
        urzua_core::config::ConfigError::SchemaVersionOutOfWindow(v) => Failure {
            message: v.to_string(),
            schema_version: Some(Box::new(v)),
        },
        other => Failure::from(other.to_string()),
    })
}

/// Whether a record type governs the file at `rel_path`: the file sits directly
/// in one of the type's directories, has a governed record filename and is not
/// excluded. The one predicate `load_records` and the post-command check share.
pub(crate) fn type_governs(
    type_config: &urzua_core::config::RecordTypeConfig,
    rel_path: &Path,
) -> bool {
    if !type_config.owns_dir(rel_path.parent()) {
        return false;
    }
    let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    urzua_core::record::is_governed_record_filename(file_name) && !type_config.excludes(file_name)
}

/// Whether any record type of `config` governs the file at `rel_path`.
pub(crate) fn is_governed(config: &Config, rel_path: &Path) -> bool {
    config
        .record_types
        .values()
        .any(|t| type_governs(t, rel_path))
}

/// Every directory some record type declares, sorted, once each.
fn record_dirs(config: &Config) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = config
        .record_types
        .values()
        .flat_map(|t| t.all_dirs().map(PathBuf::from))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// The files a run reads (`ADR-92`): the tracked and staged files, plus, when
/// the configuration sets `discovery.include_untracked`, the untracked,
/// non-ignored record files directly under a declared record directory. The one
/// call every command makes, so `check`, `file_texts` and `session` cannot see
/// different corpora.
pub(crate) fn discover_files(
    repo_root: &Path,
    config: &Config,
) -> Result<urzua_io::DiscoveredFiles, urzua_io::DiscoveryError> {
    if !config.discovery.include_untracked {
        return urzua_io::discover_tracked_files(repo_root, None);
    }
    let dirs = record_dirs(config);
    let governs = |p: &Path| is_governed(config, p);
    urzua_io::discover_tracked_files(
        repo_root,
        Some(&urzua_io::UntrackedInclusion {
            dirs: &dirs,
            governs: &governs,
        }),
    )
}

/// The untracked, non-ignored files under a declared record directory that a
/// record type governs: what the option would include (`ADR-92` clause 2),
/// listed so a run that did not include them can name them (clause 4).
pub(crate) fn untracked_record_files(
    repo_root: &Path,
    config: &Config,
) -> Result<Vec<PathBuf>, urzua_io::DiscoveryError> {
    let dirs = record_dirs(config);
    let governs = |p: &Path| is_governed(config, p);
    urzua_io::untracked_governed_files(
        repo_root,
        &urzua_io::UntrackedInclusion {
            dirs: &dirs,
            governs: &governs,
        },
    )
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
            if !type_governs(type_config, rel_path) {
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

    // The core opens no file, so the Done-means items of an opted-in type are
    // read here and handed to it on each record (`ADR-96`).
    urzua_core::rules::attach_done_items(&mut records, config, &full_text);
    Ok((records, full_text))
}

/// One warning naming the tracked records that are missing from the working
/// tree and not staged for deletion. They are judged as removed; the notice is
/// what keeps that from being silent.
pub(crate) fn missing_record_notices(missing: &std::collections::HashSet<PathBuf>) -> Vec<Notice> {
    if missing.is_empty() {
        return Vec::new();
    }
    let mut paths: Vec<String> = missing.iter().map(|p| p.display().to_string()).collect();
    paths.sort();
    let shown = paths.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    let more = paths.len().saturating_sub(5);
    vec![Notice {
        severity: NoticeSeverity::Warning,
        subject: crate::NOTICE_MISSING_RECORD.to_string(),
        message: format!(
            "{} tracked record(s) are missing from the working tree and not staged for deletion, so they are judged as removed: {shown}{}; `git rm` them to record the deletion, or `git checkout` them to restore",
            paths.len(),
            if more > 0 { format!(" and {more} more") } else { String::new() }
        ),
        detail: None,
    }]
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
        let mut record = Record::parse_with_model(
            rel_path.to_path_buf(),
            type_name.clone(),
            content,
            type_config.header_shape,
            type_prefix,
            &model,
        );
        urzua_core::rules::attach_done_items_to(&mut record, config, content);
        return Ok(record);
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
    /// Records that carry a `Realized-by` locator and have no git history (an
    /// untracked or staged-new file, or an unborn `HEAD`), so there is no date
    /// to judge their locator against (`ADR-92` clause 7). Not drift, and not
    /// "no drift": disclosed by [`no_history_notice`].
    pub(crate) no_history: HashSet<PathBuf>,
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
            detail: None,
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
            detail: None,
        });
    }
    out
}

/// Notice subject for the records drift could not judge for want of history
/// (`ADR-92` clause 7).
pub(crate) const NOTICE_UNTRACKED_NOT_DRIFT_JUDGED: &str = "discovery.untracked-not-drift-judged";

/// Notice subject for the untracked records a run did not examine (`ADR-92`
/// clause 4).
pub(crate) const NOTICE_UNTRACKED_SKIPPED: &str = "discovery.untracked-skipped";

/// The most untracked paths the skipped notice names; the count says the rest.
const SKIPPED_SHOWN: usize = 20;

/// The info notice for records with a locator and no history, which drift did
/// not judge (`ADR-92` clause 7). Present only when there are some, so it is
/// the count and not a standing line.
pub(crate) fn no_history_notice(
    no_history: &HashSet<PathBuf>,
) -> Option<urzua_core::report::Notice> {
    if no_history.is_empty() {
        return None;
    }
    let n = no_history.len();
    Some(urzua_core::report::Notice {
        severity: urzua_core::report::NoticeSeverity::Info,
        subject: NOTICE_UNTRACKED_NOT_DRIFT_JUDGED.to_string(),
        message: format!(
            "{n} record{} with a Realized-by locator {} no git history (untracked or not yet committed), so embodiment drift did not judge {}: a record is judged once it has a commit",
            if n == 1 { "" } else { "s" },
            if n == 1 { "has" } else { "have" },
            if n == 1 { "it" } else { "them" },
        ),
        detail: None,
    })
}

/// The info notice naming the untracked records a run skipped (`ADR-92`
/// clause 4): at most [`SKIPPED_SHOWN`] paths, and the count. `None` when there
/// are none.
pub(crate) fn skipped_untracked_notice(skipped: &[PathBuf]) -> Option<urzua_core::report::Notice> {
    if skipped.is_empty() {
        return None;
    }
    let n = skipped.len();
    let shown: Vec<String> = skipped
        .iter()
        .take(SKIPPED_SHOWN)
        .map(|p| p.display().to_string())
        .collect();
    let more = n.saturating_sub(SKIPPED_SHOWN);
    Some(urzua_core::report::Notice {
        severity: urzua_core::report::NoticeSeverity::Info,
        subject: NOTICE_UNTRACKED_SKIPPED.to_string(),
        message: format!(
            "{n} untracked record{} under a declared record directory {} not examined: {}{}; `git add` {} or set `discovery.include_untracked: true` to examine untracked records",
            if n == 1 { "" } else { "s" },
            if n == 1 { "was" } else { "were" },
            shown.join(", "),
            if more > 0 {
                format!(" (and {more} more)")
            } else {
                String::new()
            },
            if n == 1 { "it" } else { "them" },
        ),
        detail: None,
    })
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
    let mut no_history = HashSet::new();
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
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    let locator_opts: rules::LocatorExistsOpts =
        config.rule_opts(rules::RULE_EMBODIMENT_LOCATOR_EXISTS);
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

    // One blame per claim -- a record's evidence field line and each Done-means
    // item clause (`ADR-96` section 5) -- run up front on a few threads (the
    // per-record loop below stays sequential and cheap). A claim's locators
    // drift against the commit that last touched the claim's own line.
    let claims_of = |record: &Record| -> Vec<(usize, String)> {
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        let mut claims = Vec::new();
        if let (Some(value), Some(line)) = (
            record.header.get(locator_field),
            record
                .header
                .fields
                .iter()
                .find(|f| f.key.as_str() == locator_field)
                .map(|f| f.line),
        ) {
            claims.push((line, value.to_string()));
        }
        if let Some(items) = &record.items {
            for clause in items
                .items
                .iter()
                .filter_map(|i| i.evidence.as_ref())
                .filter(|c| c.valid)
            {
                claims.push((clause.line, clause.text.clone()));
            }
        }
        claims
    };
    let work: Vec<(PathBuf, usize)> = records
        .iter()
        .flat_map(|r| {
            claims_of(r)
                .into_iter()
                .map(move |(line, _)| (r.path.clone(), line))
        })
        .collect();
    let mut blames = blame_lines_parallel(repo_root, &work, proposed).into_iter();

    for record in records {
        let claims = claims_of(record);
        if claims.is_empty() {
            continue;
        }
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
        // Every claim's blame is consumed, in order, whether or not an earlier
        // claim already drifted the record.
        let references: Vec<_> = claims.iter().map(|_| blames.next()).collect();
        'claims: for ((_, realized_by_value), reference) in claims.iter().zip(references) {
            let reference_commit = match reference {
                Some(Ok(Some(c))) => c,
                Some(Err(e)) => {
                    note(&mut problems, e);
                    continue;
                }
                // No history for the line: nothing to judge, and said so
                // (`ADR-92` clause 7).
                Some(Ok(None)) => {
                    no_history.insert(record.path.clone());
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
                // BUG-176: a directory or glob locator the config accepts
                // The files such a locator covers. A kind the config does not
                // declare is not judged this way: `embodiment.locator-exists`
                // reports it and drift is read from the path's history as before.
                let covered: Option<(LocatorKind, Vec<String>)> = if parsed.scope.is_none() {
                    texts
                        .locator_files(&parsed.path)
                        .ok()
                        .and_then(|m| Some((m.kind?, m.files)))
                        .filter(|(kind, _)| {
                            *kind != LocatorKind::File && locator_opts.locator_kinds.contains(kind)
                        })
                } else {
                    None
                };
                if let Some((kind, files)) = &covered {
                    // Nothing matched, or a pin the config does not allow on this
                    // kind, is `embodiment.locator-exists`' finding.
                    if files.is_empty()
                        || (parsed.anchor.is_some() && !locator_opts.pinnable.contains(kind))
                    {
                        continue;
                    }
                }
                if let (Some(urzua_core::locator::Anchor::Pin(pinned)), true) =
                    (&parsed.anchor, parsed.scope.is_some() || covered.is_some())
                {
                    // Implements: ADR-74 (a scoped pin drifts on its span's hash)
                    // The file's last commit says nothing about the scope, so there
                    // is no file-based grace window: only a declared audit date can
                    // hold a scoped pin back. A directory or glob pin is held back
                    // the same way (`BUG-176`).
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
                            break 'claims;
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
                                urzua_io::last_commit_for_path(repo_root, &locator_path)
                                    .unwrap_or_else(|e| {
                                        note(&mut problems, e);
                                        None
                                    })
                            })
                            .as_deref()
                            .is_some_and(&mut held_back);
                    if changed && !fresh {
                        drifted.insert(record.path.clone());
                        break 'claims;
                    }
                    if fresh {
                        fresh_records.insert(record.path.clone());
                    }
                    continue;
                }
                let Some(locator_commit) = last_commit_for_path
                    .entry(locator_path.clone())
                    .or_insert_with(|| {
                        let commit = match &covered {
                            // The newest commit among the matched files.
                            Some((_, files)) => {
                                let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
                                urzua_io::last_commit_for_paths(repo_root, &files)
                            }
                            None => urzua_io::last_commit_for_path(repo_root, &locator_path),
                        };
                        commit.unwrap_or_else(|e| {
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
                        urzua_io::commit_strictly_before(
                            repo_root,
                            &reference_commit,
                            locator_commit,
                        )
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
                        break 'claims;
                    }
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
        no_history,
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

/// What `embodiment.claim-expired` is handed beyond the records (`ADR-97`):
/// the commit time of each record's `Realized-by` line, the seed of its
/// first-cycle offset, and the git problems met.
#[derive(Default)]
pub(crate) struct ClaimMeasures {
    pub line_times: HashMap<PathBuf, i64>,
    pub seeds: HashMap<PathBuf, u64>,
    pub problems: Vec<String>,
}

/// The first 8 bytes of the SHA-256 of a `Stable-Id`, as a big-endian number
/// (`ADR-97` section 2): the whole of a record's offset seed.
pub(crate) fn offset_seed(stable_id: &str) -> u64 {
    let hex = urzua_io::sha256_hex_of_str(stable_id.trim());
    u64::from_str_radix(&hex[..16], 16).unwrap_or(0)
}

/// Measures the records `embodiment.claim-expired` judges. A record with a
/// valid audit date is dated by it and costs no `git blame`, as the rule reads.
pub(crate) fn compute_claim_measures(
    repo_root: &Path,
    records: &[Record],
    config: &Config,
    now_unix: i64,
) -> ClaimMeasures {
    let opts: rules::ClaimExpiredOpts = config.rule_opts(rules::RULE_EMBODIMENT_CLAIM_EXPIRED);
    let mut out = ClaimMeasures::default();
    let mut work: Vec<(PathBuf, usize)> = Vec::new();
    for record in records
        .iter()
        .filter(|r| opts.periods.iter().any(|p| p.record_type == r.record_type))
    {
        if let Some(id) = record.header.get_reserved("Stable-Id") {
            out.seeds.insert(record.path.clone(), offset_seed(id));
        }
        let audited = opts
            .anchor_field
            .as_deref()
            .and_then(|f| record.header.get(f))
            .and_then(|v| rules::date_unix(v).ok())
            .is_some_and(|unix| unix.div_euclid(DAY_SECS) <= now_unix.div_euclid(DAY_SECS));
        if audited {
            continue;
        }
        let locator_field = rules::relation_field_name(
            config,
            &record.record_type,
            urzua_core::config::RelationRole::EmbodimentLocator,
        );
        if let Some(field) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == locator_field)
        {
            work.push((record.path.clone(), field.line));
        }
    }
    let commits = blame_lines_parallel(repo_root, &work, None);
    for ((path, _), commit) in work.into_iter().zip(commits) {
        match commit {
            // An all-zero hash is a line not committed yet: touched now.
            Ok(Some(commit)) if commit.chars().all(|c| c == '0') => {
                out.line_times.insert(path, now_unix);
            }
            Ok(Some(commit)) => {
                if let Some(time) = urzua_io::commit_unix_time(repo_root, &commit) {
                    out.line_times.insert(path, time);
                }
            }
            Ok(None) => {}
            Err(e) => {
                if !out.problems.contains(&e) {
                    out.problems.push(e);
                }
            }
        }
    }
    out
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
    read_files_where(repo_root, discovered, rule, |path| {
        prefixes
            .iter()
            .any(|entry| rules::path_is_published(path, entry))
    })
}

/// [`read_files_under`] for entries that are a prefix or a glob (`MILE-158`):
/// the boundary rule's paths.
pub(crate) fn read_files_matching(
    repo_root: &std::path::Path,
    discovered: &urzua_io::DiscoveredFiles,
    entries: &[String],
    rule: &str,
) -> Result<Vec<rules::PublishedFile>, String> {
    read_files_where(repo_root, discovered, rule, |path| {
        entries
            .iter()
            .any(|entry| rules::path_matches_entry(path, entry))
    })
}

fn read_files_where(
    repo_root: &std::path::Path,
    discovered: &urzua_io::DiscoveredFiles,
    rule: &str,
    wanted: impl Fn(&std::path::Path) -> bool,
) -> Result<Vec<rules::PublishedFile>, String> {
    let mut out = Vec::new();
    for path in &discovered.paths {
        if discovered.staged_deletions.contains(path) || !wanted(path) {
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

/// Reads the declared override log for `override.unreviewed` (`MILE-188`), in
/// the CLI because `urzua-core` is pure (`ADR-5`). Only read when the rule is
/// enabled. A missing file and one that is not text are handed over as such,
/// never as empty; a path resolving outside the repository, or any other I/O
/// failure, aborts, since a log the rule could not read is one it did not judge.
pub(crate) fn read_override_log(
    repo_root: &std::path::Path,
    config: &Config,
) -> Result<Option<rules::OverrideLog>, String> {
    let enabled = config
        .rules
        .get(rules::RULE_OVERRIDE_UNREVIEWED)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off);
    let (true, Some(path)) = (enabled, &config.override_log) else {
        return Ok(None);
    };
    let repo_canonical = repo_root
        .canonicalize()
        .map_err(|e| format!("override.unreviewed: could not resolve the repository root: {e}"))?;
    let content = match repo_root.join(path).canonicalize() {
        Ok(resolved) if !path_is_inside_repo(&repo_canonical, &resolved) => {
            return Err(format!(
                "override.unreviewed: {path} resolves outside the repository"
            ))
        }
        Ok(resolved) => match urzua_io::read_to_string(&resolved) {
            Ok(text) => rules::OverrideLogContent::Text(text),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                rules::OverrideLogContent::NotText
            }
            Err(_) if resolved.is_dir() => rules::OverrideLogContent::NotText,
            Err(e) => return Err(format!("override.unreviewed: could not read {path}: {e}")),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => rules::OverrideLogContent::Missing,
        Err(e) => {
            return Err(format!(
                "override.unreviewed: could not resolve {path}: {e}"
            ))
        }
    };
    Ok(Some(rules::OverrideLog {
        path: path.clone(),
        content,
    }))
}

/// What the fetched pull-request refs hold for the ids `added_ids` names
/// (`ADR-93`, `identity.open-pr-collision`): every ref under `refs/urzua/pr`, and,
/// for each file directly in a declared record directory on one whose id is in
/// `added_ids`, its path and `Stable-Id`. One `for-each-ref` and one `ls-tree` per
/// distinct tip, then a read only of the files that can collide. `check` never
/// fetches, so this reads what is already there.
pub(crate) fn read_pull_ref_claims(
    repo_root: &Path,
    config: &Config,
    added_ids: &HashSet<urzua_core::values::RecordId>,
) -> Result<rules::PullRefClaims, String> {
    let git = |args: &[&str]| -> Result<Vec<u8>, String> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(args)
            .output()
            .map_err(|e| format!("could not run git: {e}"))?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(format!(
                "git {} failed: {}",
                args[0],
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    };
    let listed = git(&[
        "for-each-ref",
        "--format=%(objectname) %(refname)",
        "refs/urzua/pr",
    ])?;
    let mut by_commit: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let mut refs = Vec::new();
    for line in String::from_utf8_lossy(&listed).lines() {
        if let Some((sha, name)) = line.split_once(' ') {
            by_commit
                .entry(sha.to_string())
                .or_default()
                .push(name.to_string());
            refs.push(name.to_string());
        }
    }
    refs.sort();
    let mut dirs: Vec<String> = config
        .record_types
        .values()
        .flat_map(|t| t.all_dirs().map(str::to_string))
        .collect();
    dirs.sort();
    dirs.dedup();
    let commits: Vec<String> = by_commit.keys().cloned().collect();
    let listing = urzua_io::files_in_dirs_at_commits(repo_root, &commits, &dirs)?;
    let mut holders = Vec::new();
    let mut ids: HashMap<PathBuf, Option<urzua_core::values::RecordId>> = HashMap::new();
    for (sha, names) in &by_commit {
        for path in listing.get(sha).into_iter().flatten() {
            let path = PathBuf::from(path);
            // The id comes from the filename under the type's own identity grammar,
            // as it does for a record on disk; the content is read only on a match.
            // Most paths repeat across hundreds of tips, and building the grammar is
            // not free, so each path is resolved once.
            let id = ids
                .entry(path.clone())
                .or_insert_with(|| {
                    if !is_governed(config, &path) {
                        return None;
                    }
                    let shell = parse_prospective_record(config, &path, "").ok()?;
                    rules::record_id(&shell).map(|id| urzua_core::values::RecordId::new(&id))
                })
                .clone();
            let Some(id) = id.filter(|id| added_ids.contains(id)) else {
                continue;
            };
            let Ok(content) = urzua_io::read_at_commit(repo_root, sha, &path) else {
                continue;
            };
            let stable_id = parse_prospective_record(config, &path, &content)
                .ok()
                .and_then(|r| r.header.get("Stable-Id").map(|v| v.trim().to_string()));
            for name in names {
                holders.push(rules::PullRefHolder {
                    id: id.clone(),
                    reference: name.clone(),
                    path: path.display().to_string(),
                    stable_id: stable_id.clone(),
                });
            }
        }
    }
    holders.sort_by(|a, b| (&a.reference, &a.path).cmp(&(&b.reference, &b.path)));
    Ok(rules::PullRefClaims { refs, holders })
}
