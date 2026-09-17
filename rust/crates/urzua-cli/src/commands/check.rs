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

/// A rule a repository has not declared does not run at all -- it is not run
/// and then filtered, because an opt-in rule that still costs its own
/// execution is opt-in in name only. The declared `level` replaces whatever
/// severity the rule body chose, which is the whole point of MILE-80: the
/// severity is the repository's call, not `rules.rs`'s.
fn gated(
    config: &urzua_core::config::Config,
    id: &str,
    run: impl FnOnce() -> (RuleExecution, Vec<Finding>),
) -> (RuleExecution, Vec<Finding>) {
    use urzua_core::config::RuleLevel;
    use urzua_core::report::{FindingSeverity, RuleStatus};

    let level = config.rules.get(id).map(|s| s.level);
    match level {
        None | Some(RuleLevel::Off) => (
            RuleExecution {
                rule: id.to_string(),
                records_examined: 0,
                status: RuleStatus::NotEnabled,
            },
            Vec::new(),
        ),
        Some(level) => {
            let severity = match level {
                RuleLevel::Error => FindingSeverity::Error,
                RuleLevel::Warn => FindingSeverity::Warning,
                RuleLevel::Off => unreachable!("handled above"),
            };
            let (exec, mut findings) = run();
            for f in &mut findings {
                f.severity = severity;
            }
            (exec, findings)
        }
    }
}

/// Reads the files a repository pointed `claim.status-agreement` at. Kept in
/// the CLI because `urzua-core` is pure (ADR-5) -- the rule is given file
/// contents, never a path to open.
fn read_claim_files(repo_root: &std::path::Path, prefixes: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for prefix in prefixes {
        let dir = repo_root.join(prefix);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
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

    let scoped = match scope_to_requested_paths(&repo_root, &discovered.paths, &paths) {
        Ok(p) => p,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let (records, full_text) = load_records(&repo_root, &scoped, &config);

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

    let drifted = compute_drifted_records(&repo_root, &records);

    // Each rule's (RuleExecution, Vec<Finding>) collected into one list --
    // rules_executed/findings both derive from it below, so there's no
    // second hand-written list that has to be kept in sync by hand.
    let rule_results: Vec<(RuleExecution, Vec<Finding>)> = vec![
        gated(&config, rules::RULE_HEADER_REQUIRED_FIELDS, || {
            rules::header_required_fields(&records, &required_by_type)
        }),
        gated(&config, rules::RULE_POINTER_RESOLUTION, || {
            rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type)
        }),
        gated(&config, rules::RULE_POINTER_TARGET_STATUS, || {
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
        gated(&config, rules::RULE_FIELD_QUALITY, || {
            rules::field_quality(&records, &required_by_type)
        }),
        gated(&config, rules::RULE_FIELD_PENDING, || {
            rules::field_pending(&records, &required_by_type)
        }),
        gated(&config, rules::RULE_CLAIM_STATUS_AGREEMENT, || {
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
        gated(&config, rules::RULE_FILENAME_TITLE_CONSISTENCY, || {
            rules::filename_title_consistency(&records, &full_text)
        }),
        gated(
            &config,
            rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
            || rules::supersession_reciprocity(&records),
        ),
        gated(
            &config,
            rules::RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
            || rules::revision_log_change_class(&records, &full_text),
        ),
        gated(&config, rules::RULE_EMBODIMENT_CONSISTENCY, || {
            rules::embodiment_consistency(&records, &drifted)
        }),
        gated(
            &config,
            rules::RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
            || rules::embodiment_locator_promotion_candidate(&records),
        ),
        gated(&config, rules::RULE_HEADER_LAYOUT_CONSISTENCY, || {
            rules::header_layout_consistency(&records, &header_layout_by_type)
        }),
        gated(&config, rules::RULE_HEADER_FIELD_SET_CONSISTENCY, || {
            rules::header_field_set_consistency(&records, &known_fields_by_type)
        }),
        gated(&config, rules::RULE_NARRATIVE_FIELD_STALE, || {
            rules::narrative_field_stale(&records, &config)
        }),
        gated(&config, rules::RULE_TYPE_NO_DECLARED_SPEC, || {
            rules::type_no_declared_spec(&config, &config_path)
        }),
        gated(&config, rules::RULE_HEADER_DEPRECATED_SHAPE, || {
            rules::header_deprecated_shape(&config, &config_path)
        }),
        gated(&config, rules::RULE_HEADER_POINTER_FIELD_CLEAN, || {
            rules::header_pointer_field_clean(&records, &config)
        }),
        gated(
            &config,
            rules::RULE_CONFIG_POINTER_DECLARATION_MISSING,
            || rules::config_pointer_declaration_missing(&config, &config_path),
        ),
        gated(&config, rules::RULE_CONFIG_POINTER_FIELD_NOT_KNOWN, || {
            rules::config_pointer_field_not_known(&config, &config_path)
        }),
        gated(
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

    // A waiver is a record (ADR-0011), never a config-level ignore list.
    // Waived findings stay listed -- only excluded from blocking/status.
    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let status = if records.is_empty() {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let report = CheckReport {
        status,
        files_examined: records.len(),
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
