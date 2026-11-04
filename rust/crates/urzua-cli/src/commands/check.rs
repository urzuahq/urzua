//! `urzua check` (SPEC-2): the validator.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{
    CheckReport, CouldNotRun, Notice, NoticeSeverity, ReportStatus, ScopeInfo,
};
use urzua_core::rules;

use crate::discovery::{compute_drifted_records, scope_to_requested_paths};
use crate::emit;
use crate::session::Session;

/// Whether an already-resolved (canonicalized) path stays inside the
/// repository -- the one containment test every `claim_paths` check needs,
/// so a future change to the policy (e.g. `BUG-117`'s reflexive-`starts_with`
/// case) has one place to land. Both `resolve_inside_repo`, below, and the
/// pre-flight `claim_paths` validation in `run` call this rather than each
/// inlining `starts_with`.
fn path_is_inside_repo(repo_canonical: &std::path::Path, resolved: &std::path::Path) -> bool {
    resolved.starts_with(repo_canonical)
}

/// Canonicalizes `candidate` and confirms it stays inside `repo_canonical` --
/// the one check `claim_paths`' directory and file branches both need, so a
/// future change to the symlink/containment policy has one place to land
/// rather than two that can drift apart.
fn resolve_inside_repo(
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
fn read_claim_files(
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

pub fn run(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    base: Option<String>,
    fail_on: Option<urzua_core::config::FailOn>,
) -> ExitCode {
    // Narrowing happens between discovery and loading, so the corpus a
    // pointer resolves against stays whole (BUG-60, MILE-123): the requested
    // scopes and the changed set only narrow what is *reported on*.
    let mut requested_scopes = Vec::new();
    let mut changed = None;
    let Session {
        repo_root,
        config_path,
        config,
        discovered,
        records,
        full_text,
    } = match Session::open_with(&paths, config_path, |repo_root, _config, discovered| {
        requested_scopes = crate::discovery::relative_scopes(repo_root, &paths)?;

        // Argv paths override discovery (SPEC-2, BUG-24) -- see resolve_argv_overrides.
        if !requested_scopes.is_empty() {
            let extra = crate::discovery::resolve_argv_overrides(
                repo_root,
                &requested_scopes,
                &discovered.paths,
            )?;
            if !extra.is_empty() && base.is_some() {
                // The diff lists tracked paths only, so a named file outside the
                // tracked set could never be examined: refusing is the alternative
                // to dropping it in silence (`BUG-24`'s failure shape).
                return Err(
                    "--base cannot be combined with a named file git does not track; \
                 drop --base or track the file"
                        .to_string(),
                );
            }
            if !extra.is_empty() {
                discovered.paths.extend(extra);
                discovered.paths.sort();
                discovered.paths.dedup();
                discovered.source = urzua_io::DiscoverySource::Argv;
            }
        }

        // Computed against the full tracked set, then used only to narrow what is
        // reported on: the corpus a pointer resolves against stays whole (`MILE-123`).
        if let Some(base) = &base {
            changed =
                Some(urzua_io::changed_paths_since(repo_root, base).map_err(|e| e.to_string())?);
            discovered.source = urzua_io::DiscoverySource::GitDiff;
        }
        Ok(())
    }) {
        Ok(session) => session,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut scoped = scope_to_requested_paths(&discovered.paths, &requested_scopes);

    // A diff run examines the changed records and every record a change
    // reaches, not the changed files alone (`MILE-123`): the corpus above is
    // whole, so a dependent can be found and judged.
    let mut diff_scope = None;
    // How many records the diff reached before a path argument narrowed it.
    let mut reached_before_narrowing = 0usize;
    if let Some(changes) = &changed {
        let examination = match crate::diff_scope::examine(
            &repo_root,
            &config,
            &config_path,
            &records,
            changes,
        ) {
            Ok(e) => e,
            Err(e) => return emit(&CouldNotRun::from(e)),
        };
        reached_before_narrowing = examination.examined.len();
        scoped.retain(|p| examination.examined.contains(p));
        let mut scope = examination.scope;
        let kept: std::collections::HashSet<&std::path::Path> =
            scoped.iter().map(|p| p.as_path()).collect();
        scope
            .dependents
            .retain(|d| kept.contains(std::path::Path::new(d)));
        diff_scope = Some(scope);
    }
    let in_scope: std::collections::HashSet<&std::path::Path> =
        scoped.iter().map(|p| p.as_path()).collect();

    // An unresolvable `claim_paths` entry left the rule reporting `ran` with
    // zero findings, indistinguishable from a clean corpus (BUG-56). Being
    // *visible* is what that bug asked for; aborting was one way to achieve it
    // and took `check` down with a directory git cannot keep. A `Notice` is
    // visible and never moves the exit code (ADR-46), so absence is reported
    // and a path that is actively wrong still aborts.
    let mut notices: Vec<Notice> = Vec::new();
    // `claim.status-agreement`'s options, only when the rule is enabled -- the one
    // read, shared by the pre-flight `claim_paths` validation and the read step
    // below, so the two cannot disagree about whether the rule is running.
    let claim_opts =
        config.enabled_rule_opts::<rules::ClaimOpts>(rules::RULE_CLAIM_STATUS_AGREEMENT);
    if let Some(setting) = &claim_opts {
        // Both sides canonicalised: the repo root may itself reach through
        // a link (macOS `/tmp`), and comparing a resolved path against an
        // unresolved root reports every entry as outside. Computed once,
        // outside the loop: it does not depend on which prefix is being
        // checked.
        let root = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.clone());
        for prefix in &setting.claim_paths {
            // A root symlinked to an ancestor escaped the declared prefix
            // entirely (BUG-69). Rejecting every symlink also rejected a
            // link to a legitimate directory, with a message saying it was
            // not readable when it was (BUG-80) -- so the target is
            // resolved and required to stay inside the repository instead.
            let declared = repo_root.join(prefix);
            let resolved = declared.canonicalize();
            match &resolved {
                // Absent is the state this guard was written to tolerate and
                // did not: git tracks no empty directory, so `.changeset`
                // ceases to exist the moment a release consumes the last
                // fragment, and aborting took `check` down with it. The rule
                // has no input, which is reported rather than fatal.
                //
                // Only `NotFound`: an ancestor with its execute bit removed
                // returns `PermissionDenied` here, and `Path::exists()`
                // reports it as `false` too (it swallows every error, not
                // only absence) -- collapsing that into the same Notice
                // would report `check` clean over claim files it never had
                // permission to read, reintroducing `BUG-56` under a
                // permissions error instead of a typo.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    notices.push(Notice {
                        severity: NoticeSeverity::Warning,
                        subject: rules::RULE_CLAIM_STATUS_AGREEMENT.to_string(),
                        message: format!(
                            "claim.status-agreement: claim_paths entry '{prefix}' does not exist -- the rule has no input"
                        ),
                    })
                }
                // Any other I/O failure -- permission denied is the one
                // observed in practice -- is not absence and must not read
                // as it.
                Err(e) => {
                    return emit(&CouldNotRun::from(format!(
                        "claim.status-agreement: claim_paths entry '{prefix}' could not be read: {e}"
                    )))
                }
                // Actively wrong, rather than merely empty: a file where a
                // directory was declared, or a link out of the repository
                // (BUG-69). Neither is a state waiting to be filled in.
                Ok(r) if !r.is_dir() => {
                    return emit(&CouldNotRun::from(format!(
                        "claim.status-agreement: claim_paths entry '{prefix}' is not a directory"
                    )))
                }
                Ok(r) if !path_is_inside_repo(&root, r) => {
                    return emit(&CouldNotRun::from(format!(
                        "claim.status-agreement: claim_paths entry '{prefix}' resolves outside the repository"
                    )))
                }
                Ok(_) => {}
            }
        }
    }

    let claims = {
        let paths = claim_opts.map_or_else(Vec::new, |o| o.claim_paths);
        match read_claim_files(&repo_root, &paths) {
            Ok(c) => c,
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    };

    let published_paths: Vec<String> = config
        .enabled_rule_opts::<rules::PublishedPathOpts>(
            rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        )
        .map_or_else(Vec::new, |o| o.published_paths);
    let published_files = match crate::discovery::read_files_under(
        &repo_root,
        &discovered,
        &published_paths,
        rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
    ) {
        Ok(files) => files,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let index_specs: Vec<urzua_core::config::IndexSpec> = config
        .enabled_rule_opts::<rules::IndexOpts>(rules::RULE_INDEX_FRESHNESS)
        .map_or_else(Vec::new, |o| o.indexes);
    let index_artifacts = match read_index_artifacts(&repo_root, &index_specs) {
        Ok(artifacts) => artifacts,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let back_pointer_sources =
        match crate::discovery::read_back_pointer_sources(&repo_root, &discovered, &config) {
            Ok(sources) => sources,
            Err(e) => return emit(&CouldNotRun::from(e)),
        };

    let decision_gate_sources =
        match crate::discovery::read_decision_gate_sources(&repo_root, &discovered, &config) {
            Ok(sources) => sources,
            Err(e) => return emit(&CouldNotRun::from(e)),
        };

    // `compute_drifted_records` spawns several `git` subprocesses per
    // `Realized-by`-carrying record; skip that work entirely when its only
    // consumer, `embodiment.consistency`, won't read the result (`BUG-115`),
    // matching the `claim_paths` gate just above.
    let mut drift = if config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off)
    {
        compute_drifted_records(&repo_root, &records, &config)
    } else {
        Default::default()
    };

    let claim_ages = if config
        .rules
        .get(rules::RULE_EMBODIMENT_CONSISTENCY)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off)
    {
        let (ages, problems) = crate::discovery::compute_claim_ages(
            &repo_root,
            &records,
            &config,
            urzua_io::now_unix(),
        );
        for problem in problems {
            if !drift.git_problems.contains(&problem) {
                drift.git_problems.push(problem);
            }
        }
        ages
    } else {
        std::collections::HashMap::new()
    };
    notices.extend(crate::discovery::drift_notices(
        drift.shallow,
        &drift.git_problems,
    ));

    // Each rule's (RuleExecution, Vec<Finding>) collected into one list --
    // rules_executed/findings both derive from it below, so there's no
    // second hand-written list that has to be kept in sync by hand.
    // Built once and shared: pointer.resolution, pointer.target-status,
    // narrative-field.stale, claim.status-agreement and
    // relation.supersession-reciprocity each need to resolve a reference
    // against every record, and independently rebuilding this from scratch
    // for each was the same O(corpus) traversal five times over one `check`
    // run. identity.collision needs the collisions this same build already
    // computes, so it's built once here rather than a second time inside
    // that rule (`BUG-116`).
    let (record_index, identity_collisions) = rules::build_index_reporting_collisions(&records);
    notices.extend(rules::identity_collision_notices(&identity_collisions));
    // A record satisfying a required section through a declared alias is
    // disclosed (`MILE-165`), only when `section.required` is on.
    if config
        .rules
        .get(rules::RULE_SECTION_REQUIRED)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off)
    {
        notices.extend(rules::section_alias_notices(&records, &config, &full_text));
    }

    // Records a source comment cites (`MILE-128`): code-tier evidence for
    // `embodiment.consistency`, read from files the CLI already loaded.
    let back_pointed = back_pointer_sources
        .as_ref()
        .map(|s| {
            rules::back_pointer_evidence(
                &s.files,
                &s.back_pointer_paths,
                &s.test_paths,
                &config,
                &record_index,
            )
        })
        .unwrap_or_default();

    // The diff rules judge the changed set itself, so they get it as data and
    // `None` on a run with no `--base`.
    let changed_paths: Option<Vec<PathBuf>> = changed
        .as_ref()
        .map(|c| c.changed.iter().cloned().collect());

    // The committed snapshot of issue and pull request numbers (`MILE-143`),
    // read only when the rule that uses it is enabled. A file read, never a
    // request: `check` stays offline and deterministic.
    let refs_snapshot = load_refs_snapshot(&config, &repo_root);
    if let (Some(refs), Ok(snapshot)) = (issue_refs_declaration(&config), &refs_snapshot) {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: rules::RULE_CITATION_ISSUE_EXISTS.to_string(),
            message: format!(
                "citations were checked against {}, generated {} for {} ({} issues, {} pull requests); run `urzua refs refresh` to update it",
                refs.snapshot,
                snapshot.generated_at,
                snapshot.repository,
                snapshot.issues.len(),
                snapshot.pulls.len(),
            ),
        });
    }

    let may_exist = |p: &std::path::Path| repo_root.join(p).try_exists().unwrap_or(true);
    let dir_exists = |d: &str| repo_root.join(d).is_dir();
    let live_paths: Vec<PathBuf> = discovered
        .paths
        .iter()
        .filter(|p| !discovered.staged_deletions.contains(*p))
        .cloned()
        .collect();
    let base_ctx = rules::RuleCtx {
        config: &config,
        config_path: &config_path,
        records: &records,
        full_text: &full_text,
        index: &record_index,
        collisions: &identity_collisions,
        claims: &claims,
        published_files: &published_files,
        published_paths: &published_paths,
        index_artifacts: &index_artifacts,
        back_pointer_sources: back_pointer_sources.as_ref(),
        decision_gate_sources: decision_gate_sources.as_ref(),
        back_pointed: &back_pointed,
        drifted: &drift.drifted,
        fresh: &drift.fresh,
        claim_ages: &claim_ages,
        changed_paths: changed_paths.as_deref(),
        refs_snapshot: &refs_snapshot,
        tracked_paths: &discovered.paths,
        live_paths: &live_paths,
        may_exist: &may_exist,
        dir_exists: &dir_exists,
        run: rules::RunState::empty(),
    };

    let mut rules_executed = Vec::new();
    let mut findings = Vec::new();
    for (exec, rule_findings) in rules::run_phase(rules::Phase::Main, &base_ctx) {
        rules_executed.push(exec);
        findings.extend(rule_findings);
    }

    // Over the executions rather than the corpus: it judges whether the
    // configuration reached what it declared (MILE-106), which is only
    // answerable once every other rule has reported.
    let after_rules = rules::RuleCtx {
        run: rules::RunState {
            executions: &rules_executed,
            ..rules::RunState::empty()
        },
        ..base_ctx
    };
    let after_rules_results = rules::run_phase(rules::Phase::AfterRules, &after_rules);
    for (exec, rule_findings) in after_rules_results {
        rules_executed.push(exec);
        findings.extend(rule_findings);
    }

    // A scoped run keeps a finding by the anchor its rule declares
    // (`rules::Anchor`): a finding about the config, or on a file no record-type
    // `dir` covers, belongs to every scope (`BUG-67`, `BUG-86`, `BUG-121`).
    if !requested_scopes.is_empty() {
        let narrowing = rules::Narrowing::Paths(&requested_scopes);
        findings.retain(|f| rules::finding_in_scope(&config, f, &narrowing));
    }

    // A diff run reports only on what changed, under the same anchors.
    if let (Some(_), Some(changes)) = (&diff_scope, &changed) {
        let examined = |p: &std::path::Path| in_scope.contains(p);
        let narrowing = rules::Narrowing::Diff {
            examined: &examined,
            changed: &changes.changed,
        };
        findings.retain(|f| rules::finding_in_scope(&config, f, &narrowing));
    }

    // Every other finding names a repository-relative file; the config rules
    // were handed the path the config was loaded from, which is absolute by
    // default, so the same finding differed by checkout (BUG-185). Relativized
    // here, after the filters above compare against the loaded path, and
    // before waivers match on it. A `--config` outside the repository has no
    // relative form and stays as given.
    for f in &mut findings {
        if f.file == config_path.as_path() {
            if let Ok(rel) = f.file.strip_prefix(&repo_root) {
                f.file = rel.to_path_buf();
            }
        }
    }

    // A waiver is a record (ADR-0011), never a config-level ignore list.
    // Waived findings stay listed -- only excluded from blocking/status.
    let waivers = urzua_core::waiver::load_waivers(&records);
    let today = urzua_io::today();

    // Judged against the findings as they stand before any waiver is applied,
    // and after every scope filter: a waiver is stale only against a run that
    // could have produced its finding (MILE-168).
    let whole_corpus = requested_scopes.is_empty() && diff_scope.is_none();
    let reached = |p: &std::path::Path| whole_corpus || in_scope.contains(p);
    let after_scoping = rules::RuleCtx {
        run: rules::RunState {
            executions: &rules_executed,
            findings: &findings,
            waivers: &waivers,
            whole_corpus,
            reached: &reached,
            today: &today,
        },
        ..base_ctx
    };
    let after_scoping_results = rules::run_phase(rules::Phase::AfterScoping, &after_scoping);
    for (exec, rule_findings) in after_scoping_results {
        rules_executed.push(exec);
        findings.extend(rule_findings);
    }

    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &today);

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let examined: Vec<&urzua_core::record::Record> = records
        .iter()
        .filter(|r| in_scope.contains(r.path.as_path()))
        .collect();

    // `blocking` is decided over findings that survive the scope filter, and a
    // config finding survives every scope. Deciding `not-run` from the record
    // count alone therefore produced `status: not-run` beside `blocking: true`
    // -- a run that established nothing and found a blocking error at once. A
    // blocking finding is something established, so it settles the question
    // before the record count is consulted.
    // A diff in which no governed record changed, and nothing reaches one, is
    // a run that was asked about nothing and found nothing: `ok`, not
    // `not-run` (a run that could not look). The notice and the empty
    // `changed_paths` keep it distinguishable from a whole-corpus pass.
    let empty_diff = diff_scope
        .as_ref()
        .is_some_and(|d| examined.is_empty() && d.whole_corpus_reason.is_none());
    if empty_diff && reached_before_narrowing > 0 {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: "scope".to_string(),
            message: format!(
                "the path argument excluded all {reached_before_narrowing} record(s) the change since {} reached (changed or dependent); nothing was examined",
                base.as_deref().unwrap_or_default(),
            ),
        });
    } else if empty_diff {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: "scope".to_string(),
            message: format!(
                "no governed record changed since {} ({} changed path(s), none a record or the configuration); nothing was examined",
                base.as_deref().unwrap_or_default(),
                diff_scope.as_ref().map_or(0, |d| d.changed_paths.len()),
            ),
        });
    }
    let status = if blocking {
        ReportStatus::FindingsPresent
    } else if empty_diff && crate::gate::any_rule_looked(&rules_executed) {
        if active_findings().count() == 0 {
            ReportStatus::Ok
        } else {
            ReportStatus::FindingsPresent
        }
    } else if examined.is_empty() || !crate::gate::any_rule_looked(&rules_executed) {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let report = CheckReport {
        status,
        files_examined: examined.len(),
        // Counted over the same set `files_examined` counts, or the report
        // carries two numbers on different denominators -- which is `BUG-40`,
        // in the field added this release to end it. Rules run over the whole
        // corpus because a reference resolves against records outside the
        // scope (BUG-67); what is *reported on* is the scope.
        records_read_by_any_rule: urzua_core::report::records_read_by_any_rule_within(
            &rules_executed,
            &in_scope,
        ),
        rules_executed,
        scope: ScopeInfo {
            source: crate::discovery::scope_source(discovered.source),
            base: base.clone(),
            record_types: config.record_types.keys().cloned().collect(),
            diff: diff_scope,
        },
        blocking,
        findings,
        notices,
        // The flag wins over the configuration; either only moves the exit code.
        fail_on: fail_on.unwrap_or(config.fail_on),
    };

    emit(&report)
}

/// `citation.issue-exists`'s `issue_refs`, when the rule is enabled.
fn issue_refs_declaration(
    config: &urzua_core::config::Config,
) -> Option<&urzua_core::config::IssueRefs> {
    config
        .rules
        .get(rules::RULE_CITATION_ISSUE_EXISTS)
        .filter(|s| s.level != urzua_core::config::RuleLevel::Off)
        .and_then(|s| s.issue_refs.as_ref())
}

/// The snapshot `citation.issue-exists` judges against, or why it cannot be
/// used. When the rule is not enabled nothing is read.
fn load_refs_snapshot(
    config: &urzua_core::config::Config,
    repo_root: &std::path::Path,
) -> Result<urzua_core::refs_snapshot::RefsSnapshot, String> {
    let Some(refs) = issue_refs_declaration(config) else {
        return Err("not enabled".to_string());
    };
    let path = repo_root.join(&refs.snapshot);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("{} does not exist", refs.snapshot)
        } else {
            format!("{} could not be read: {e}", refs.snapshot)
        }
    })?;
    let parsed: urzua_core::refs_snapshot::RefsSnapshot = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not a valid snapshot: {e}", refs.snapshot))?;
    urzua_core::refs_snapshot::validate_snapshot(parsed, &refs.repository)
        .map_err(|why| format!("{} {why}", refs.snapshot))
}
