//! `urzua check` (SPEC-2): the validator.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{
    CheckReport, CouldNotRun, Finding, ReportStatus, RuleExecution, ScopeInfo,
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
    let scoped = match scope_to_requested_paths(&repo_root, &discovered.paths, &paths) {
        Ok(p) => p,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

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

    let mut required_by_type = HashMap::new();
    let mut header_layout_by_type = HashMap::new();
    let mut known_fields_by_type = HashMap::new();
    let mut declared_fields_by_type: HashMap<String, std::collections::HashSet<String>> =
        HashMap::new();
    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        required_by_type.insert(name.clone(), cfg.required_fields.clone());
        if let Some(layout) = cfg.header_layout {
            header_layout_by_type.insert(name.clone(), layout);
        }
        // Every declared name, for rules that judge a value rather than the
        // field set. `known_fields_by_type` is absent for a type declaring only
        // `required_fields`, because `header.field-set-consistency` skips such a
        // type by design -- a rule reading that map would skip it too, silently.
        let declared: std::collections::HashSet<String> = cfg
            .required_fields
            .iter()
            .chain(cfg.known_fields.iter().flatten())
            .cloned()
            .collect();
        declared_fields_by_type.insert(name.clone(), declared.clone());
        if cfg.known_fields.is_some() {
            known_fields_by_type.insert(name.clone(), declared);
        }
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    // A `claim_paths` entry that does not resolve is a typo, not an empty
    // directory, and the rule reports `ran` with zero findings either way --
    // indistinguishable from a clean corpus. `OptionRequired` stops this rule
    // going inert when `claim_paths` is missing; a misspelled one must not get
    // through the same door (BUG-56). Checked here because it is a config
    // error, not a finding about a record.
    if let Some(setting) = config.rules.get(rules::RULE_CLAIM_STATUS_AGREEMENT) {
        if setting.level != urzua_core::config::RuleLevel::Off {
            for prefix in setting.claim_paths.iter().flatten() {
                // `is_dir()` folds an unreadable directory into "not a
                // directory", which is the right verdict here: either way the
                // rule cannot reach its input, and saying so beats reporting a
                // clean run over files it never opened.
                // `exists()` rather than `is_dir()`, and only a truly absent
                // path aborts: git does not track empty directories, so a repo
                // following the documented `claim_paths: [".changeset"]` pattern
                // would otherwise lose `check` entirely the moment a release
                // consumes the last fragment.
                // A root symlinked to an ancestor escaped the declared prefix
                // entirely (BUG-69). Rejecting every symlink also rejected a
                // link to a legitimate directory, with a message saying it was
                // not readable when it was (BUG-80) -- so the target is
                // resolved and required to stay inside the repository instead.
                let declared = repo_root.join(prefix);
                let resolved = declared.canonicalize().ok();
                // Both sides canonicalised: the repo root may itself reach
                // through a link (macOS `/tmp`), and comparing a resolved path
                // against an unresolved root reports every entry as outside.
                let root = repo_root
                    .canonicalize()
                    .unwrap_or_else(|_| repo_root.clone());
                let inside = resolved
                    .as_ref()
                    .is_some_and(|r| r.starts_with(&root) && r.is_dir());
                if !inside {
                    let why = match &resolved {
                        Some(r) if !r.is_dir() => "is not a directory",
                        Some(_) => "resolves outside the repository",
                        None => "does not resolve",
                    };
                    return emit(&CouldNotRun::from(format!(
                        "claim.status-agreement: claim_paths entry '{prefix}' {why}"
                    )));
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
    let rule_results: Vec<(RuleExecution, Vec<Finding>)> = vec![
        crate::gate::gated(&config, rules::RULE_HEADER_REQUIRED_FIELDS, || {
            rules::header_required_fields(&records, &required_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_POINTER_RESOLUTION, || {
            rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_POINTER_TARGET_STATUS, || {
            let not_in = config
                .rules
                .get(rules::RULE_POINTER_TARGET_STATUS)
                .and_then(|s| s.not_in.clone())
                .unwrap_or_default();
            rules::pointer_target_status(
                &records,
                &pointer_fields_by_type,
                &narrative_fields_by_type,
                &not_in,
            )
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_QUALITY, || {
            rules::field_quality(&records, &required_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_PENDING, || {
            rules::field_pending(&records, &required_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_CLAIM_STATUS_AGREEMENT, || {
            let setting = config.rules.get(rules::RULE_CLAIM_STATUS_AGREEMENT);
            let closed = setting
                .and_then(|s| s.closed_statuses.clone())
                .unwrap_or_default();
            rules::claim_status_agreement(&records, &claims, &closed)
        }),
        crate::gate::gated(&config, rules::RULE_FILENAME_TITLE_CONSISTENCY, || {
            rules::filename_title_consistency(&records, &full_text)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
            || rules::supersession_reciprocity(&records, &config),
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
            rules::header_layout_consistency(&records, &header_layout_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_FIELD_SET_CONSISTENCY, || {
            rules::header_field_set_consistency(&records, &known_fields_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_FIELD_UNTRIMMED_VALUE, || {
            rules::field_untrimmed_value(&records, &declared_fields_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_NARRATIVE_FIELD_STALE, || {
            let terminal = config
                .rules
                .get(rules::RULE_NARRATIVE_FIELD_STALE)
                .and_then(|s| s.terminal_statuses.clone())
                .unwrap_or_default();
            rules::narrative_field_stale(&records, &config, &terminal)
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
        notices: Vec::new(),
    };

    emit(&report)
}
