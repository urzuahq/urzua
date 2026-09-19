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
fn read_claim_files(repo_root: &std::path::Path, prefixes: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for prefix in prefixes {
        let dir = repo_root.join(prefix);
        // A path that does not resolve is a typo, not an empty directory. The
        // rule reported `ran` with zero findings either way, which is
        // indistinguishable from a clean corpus -- and `OptionRequired` exists
        // to stop this rule going inert when `claim_paths` is *missing*, so a
        // misspelled one must not get through the same door (BUG-56).
        // `claim_paths` is a path *prefix*, so the claims may sit any depth
        // below it. Reading one level deep made a nested layout report a clean
        // run over an empty claim list -- through the same door BUG-56's guard
        // was built to close (BUG-63).
        let mut paths = Vec::new();
        let mut pending = vec![dir.clone()];
        while let Some(current) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for path in entries.flatten().map(|e| e.path()) {
                // `symlink_metadata`, so a link is never descended into: a link
                // to an ancestor gives an unbounded walk that leaves the
                // declared prefix and reports findings against paths that do
                // not exist (BUG-69).
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.is_dir() {
                    pending.push(path);
                } else if meta.is_file() && path.extension().is_some_and(|e| e == "md") {
                    paths.push(path);
                }
            }
        }
        // Sorted so a finding's order does not depend on the filesystem.
        paths.sort();
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let shown = path
                    .strip_prefix(repo_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.push((shown, content));
            }
        }
    }
    out
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
    let (records, full_text) = load_records(&repo_root, &discovered.paths, &config);
    let in_scope: std::collections::HashSet<&std::path::Path> =
        scoped.iter().map(|p| p.as_path()).collect();

    let mut required_by_type = HashMap::new();
    let mut header_layout_by_type = HashMap::new();
    let mut known_fields_by_type = HashMap::new();
    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        required_by_type.insert(name.clone(), cfg.required_fields.clone());
        if let Some(layout) = cfg.header_layout {
            header_layout_by_type.insert(name.clone(), layout);
        }
        if let Some(known) = &cfg.known_fields {
            let allowed: std::collections::HashSet<String> = cfg
                .required_fields
                .iter()
                .chain(known.iter())
                .map(|f| f.to_ascii_lowercase())
                .collect();
            known_fields_by_type.insert(name.clone(), allowed);
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
                let declared = repo_root.join(prefix);
                if !declared.exists() || !declared.is_dir() {
                    return emit(&CouldNotRun::from(format!(
                        "claim.status-agreement: claim_paths entry '{prefix}' is not a readable directory"
                    )));
                }
            }
        }
    }

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
            let paths = setting
                .and_then(|s| s.claim_paths.clone())
                .unwrap_or_default();
            let closed = setting
                .and_then(|s| s.closed_statuses.clone())
                .unwrap_or_default();
            let claims = read_claim_files(&repo_root, &paths);
            rules::claim_status_agreement(&records, &claims, &closed)
        }),
        crate::gate::gated(&config, rules::RULE_FILENAME_TITLE_CONSISTENCY, || {
            rules::filename_title_consistency(&records, &full_text)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
            || rules::supersession_reciprocity(&records),
        ),
        crate::gate::gated(
            &config,
            rules::RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
            || rules::revision_log_change_class(&records, &full_text),
        ),
        crate::gate::gated(&config, rules::RULE_EMBODIMENT_CONSISTENCY, || {
            rules::embodiment_consistency(&records, &drifted)
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
            rules::embodiment_locator_exists(&records, &present)
        }),
        crate::gate::gated(
            &config,
            rules::RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
            || rules::embodiment_locator_promotion_candidate(&records),
        ),
        crate::gate::gated(&config, rules::RULE_HEADER_LAYOUT_CONSISTENCY, || {
            rules::header_layout_consistency(&records, &header_layout_by_type)
        }),
        crate::gate::gated(&config, rules::RULE_HEADER_FIELD_SET_CONSISTENCY, || {
            rules::header_field_set_consistency(&records, &known_fields_by_type)
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

    let status = if examined.is_empty() {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let report = CheckReport {
        status,
        files_examined: examined.len(),
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
