//! `urzua check` (SPEC-2): the validator.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{
    CheckReport, CouldNotRun, Finding, Notice, NoticeSeverity, ReportStatus, RuleExecution,
    ScopeInfo,
};
use urzua_core::rules;

use crate::discovery::{
    compute_drifted_records, find_repo_root, load_config, load_records, scope_to_requested_paths,
};
use crate::emit;

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
                    let resolved = path.canonicalize().map_err(|e| {
                        format!("claim_paths: could not resolve {}: {e}", path.display())
                    })?;
                    if !resolved.starts_with(&repo_canonical) {
                        return Err(format!(
                            "claim_paths: {} resolves outside the repository",
                            path.display()
                        ));
                    }
                    if prefix_canonical.starts_with(&resolved) {
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
                    let resolved = path.canonicalize().map_err(|e| {
                        format!("claim_paths: could not resolve {}: {e}", path.display())
                    })?;
                    if !resolved.starts_with(&repo_canonical) {
                        return Err(format!(
                            "claim_paths: {} resolves outside the repository",
                            path.display()
                        ));
                    }
                    paths.push(path);
                }
            }
        }
        // Sorted so a finding's order does not depend on the filesystem.
        paths.sort();
        for path in paths {
            let content = std::fs::read_to_string(&path)
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

pub fn run(config_path: Option<PathBuf>, paths: Vec<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&paths) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path =
        config_path.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let requested_scopes = match crate::discovery::relative_scopes(&repo_root, &paths) {
        Ok(s) => s,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let scoped = scope_to_requested_paths(&discovered.paths, &requested_scopes);

    // A path argument narrows what is *reported on*. It must not narrow the
    // corpus a pointer resolves against: a target outside the requested path
    // still exists, and judging it absent turns a clean corpus into a failing
    // one purely by how the check was invoked (BUG-60).
    let (records, full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let in_scope: std::collections::HashSet<&std::path::Path> =
        scoped.iter().map(|p| p.as_path()).collect();

    // An unresolvable `claim_paths` entry left the rule reporting `ran` with
    // zero findings, indistinguishable from a clean corpus (BUG-56). Being
    // *visible* is what that bug asked for; aborting was one way to achieve it
    // and took `check` down with a directory git cannot keep. A `Notice` is
    // visible and never moves the exit code (ADR-46), so absence is reported
    // and a path that is actively wrong still aborts.
    let mut claim_path_notices: Vec<Notice> = Vec::new();
    if let Some(setting) = config.rules.get(rules::RULE_CLAIM_STATUS_AGREEMENT) {
        if setting.level != urzua_core::config::RuleLevel::Off {
            // Both sides canonicalised: the repo root may itself reach through
            // a link (macOS `/tmp`), and comparing a resolved path against an
            // unresolved root reports every entry as outside. Computed once,
            // outside the loop: it does not depend on which prefix is being
            // checked.
            let root = repo_root
                .canonicalize()
                .unwrap_or_else(|_| repo_root.clone());
            for prefix in setting.claim_paths.iter().flatten() {
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
                        claim_path_notices.push(Notice {
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
                    Ok(r) if !r.starts_with(&root) => {
                        return emit(&CouldNotRun::from(format!(
                            "claim.status-agreement: claim_paths entry '{prefix}' resolves outside the repository"
                        )))
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    let claims = {
        let paths = config
            .rules
            .get(rules::RULE_CLAIM_STATUS_AGREEMENT)
            .filter(|s| s.level != urzua_core::config::RuleLevel::Off)
            .and_then(|s| s.claim_paths.clone())
            .unwrap_or_default();
        match read_claim_files(&repo_root, &paths) {
            Ok(c) => c,
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    };

    let drifted = compute_drifted_records(&repo_root, &records);

    // Each rule's (RuleExecution, Vec<Finding>) collected into one list --
    // rules_executed/findings both derive from it below, so there's no
    // second hand-written list that has to be kept in sync by hand.
    // Built once and shared: pointer.resolution, pointer.target-status,
    // narrative-field.stale, claim.status-agreement and
    // relation.supersession-reciprocity each need to resolve a reference
    // against every record, and independently rebuilding this from scratch
    // for each was the same O(corpus) traversal five times over one `check`
    // run.
    let record_index = rules::build_normalized_index(&records);

    let rule_results: Vec<(RuleExecution, Vec<Finding>)> = vec![
        crate::gate::gated(&config, rules::RULE_HEADER_REQUIRED_FIELDS, || {
            rules::header_required_fields(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_POINTER_RESOLUTION, || {
            rules::pointer_resolution(&records, &config, &record_index)
        }),
        crate::gate::gated(&config, rules::RULE_POINTER_TARGET_STATUS, || {
            let not_in = config
                .rules
                .get(rules::RULE_POINTER_TARGET_STATUS)
                .and_then(|s| s.not_in.clone())
                .unwrap_or_default();
            rules::pointer_target_status(&records, &config, &not_in, &record_index)
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_QUALITY, || {
            rules::field_quality(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_PENDING, || {
            rules::field_pending(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_CLAIM_STATUS_AGREEMENT, || {
            let setting = config.rules.get(rules::RULE_CLAIM_STATUS_AGREEMENT);
            let closed = setting
                .and_then(|s| s.closed_statuses.clone())
                .unwrap_or_default();
            rules::claim_status_agreement(&claims, &closed, &record_index)
        }),
        crate::gate::gated(&config, rules::RULE_FILENAME_TITLE_CONSISTENCY, || {
            rules::filename_title_consistency(&records, &full_text)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
            || rules::supersession_reciprocity(&records, &config, &record_index),
        ),
        crate::gate::gated(
            &config,
            rules::RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
            || rules::revision_log_change_class(&records, &full_text),
        ),
        crate::gate::gated(&config, rules::RULE_EMBODIMENT_CONSISTENCY, || {
            rules::embodiment_consistency(&records, &config, &drifted)
        }),
        crate::gate::gated(&config, rules::RULE_EMBODIMENT_LOCATOR_EXISTS, || {
            // Tracked *and* on disk: tracked alone passes a staged deletion,
            // since discovery unions `ls-files` with the cached diff, and on
            // disk alone passes a gitignored file that then fails in CI.
            let tracked: std::collections::HashSet<&std::path::Path> =
                discovered.paths.iter().map(|p| p.as_path()).collect();
            let present = |p: &str| {
                let path = std::path::Path::new(p);
                if !tracked.contains(path) {
                    return false;
                }
                // `is_file()` folds every error into `false`, so an unreadable
                // path would be reported as missing. This rule is declared
                // `error` in repositories that enable it, and a blocking
                // finding the tool cannot substantiate is worse than silence --
                // so only a definite absence counts.
                repo_root.join(path).try_exists().unwrap_or(true)
            };
            rules::embodiment_locator_exists(&records, &config, &present)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
            || rules::embodiment_locator_promotion_candidate(&records, &config),
        ),
        crate::gate::gated(&config, rules::RULE_HEADER_LAYOUT_CONSISTENCY, || {
            rules::header_layout_consistency(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_FIELD_SET_CONSISTENCY, || {
            rules::header_field_set_consistency(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_UNTRIMMED_VALUE, || {
            rules::field_untrimmed_value(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_FIELD_CASE_MISMATCH, || {
            rules::header_field_case_mismatch(&records, &config)
        }),
        crate::gate::gated(&config, rules::RULE_NARRATIVE_FIELD_STALE, || {
            let terminal = config
                .rules
                .get(rules::RULE_NARRATIVE_FIELD_STALE)
                .and_then(|s| s.terminal_statuses.clone())
                .unwrap_or_default();
            rules::narrative_field_stale(&records, &config, &terminal, &record_index)
        }),
        crate::gate::gated(&config, rules::RULE_TYPE_DIR_MATCHES_NOTHING, || {
            let mut matched: HashMap<String, usize> = HashMap::new();
            for r in &records {
                *matched.entry(r.record_type.clone()).or_insert(0) += 1;
            }
            let dir_exists = |d: &str| repo_root.join(d).is_dir();
            rules::type_dir_matches_nothing(&config, &config_path, &matched, &dir_exists)
        }),
        crate::gate::gated(&config, rules::RULE_IDENTITY_COLLISION, || {
            rules::identity_collision(&records)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
            || rules::type_record_outside_declared_dir(&config, &discovered.paths),
        ),
        crate::gate::gated(&config, rules::RULE_TYPE_NO_DECLARED_SPEC, || {
            rules::type_no_declared_spec(&config, &config_path)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_DEPRECATED_SHAPE, || {
            rules::header_deprecated_shape(&config, &config_path)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_POINTER_FIELD_CLEAN, || {
            rules::header_pointer_field_clean(&records, &config)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_CONFIG_POINTER_DECLARATION_MISSING,
            || rules::config_pointer_declaration_missing(&config, &config_path),
        ),
        crate::gate::gated(&config, rules::RULE_CONFIG_POINTER_FIELD_NOT_KNOWN, || {
            rules::config_pointer_field_not_known(&config, &config_path)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
            || rules::config_pointer_narrative_overlap(&config, &config_path),
        ),
    ];

    let mut rules_executed = Vec::with_capacity(rule_results.len());
    let mut findings = Vec::new();
    for (exec, rule_findings) in rule_results {
        rules_executed.push(exec);
        findings.extend(rule_findings);
    }

    // Last, and over the executions rather than the corpus: it judges whether
    // the configuration reached what it declared (MILE-106), which is only
    // answerable once every other rule has reported.
    let (scope_exec, scope_findings) =
        crate::gate::gated(&config, rules::RULE_CONFIG_SCOPE_MATCHES_NOTHING, || {
            rules::config_scope_matches_nothing(&rules_executed, &config_path)
        });
    rules_executed.push(scope_exec);
    findings.extend(scope_findings);

    // Scoped against the requested prefixes themselves, never against the
    // discovered set: that set holds the *tracked* files under the scope, and a
    // rule may report on a file git does not track -- a claim file is read
    // straight off disk -- whose finding then belonged to no scope at all and
    // was dropped, so `check .` passed a corpus `check` blocked (BUG-67).
    // A finding about the config survives every scope, being outside all of them.
    if !requested_scopes.is_empty() {
        findings.retain(|f| {
            f.file == config_path.as_path()
                || requested_scopes.iter().any(|s| f.file.starts_with(s))
        });
    }

    // A waiver is a record (ADR-0011), never a config-level ignore list.
    // Waived findings stay listed -- only excluded from blocking/status.
    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

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
    let status = if blocking {
        ReportStatus::FindingsPresent
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
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
        notices: claim_path_notices,
    };

    emit(&report)
}
