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

pub fn run(config_path: Option<PathBuf>, paths: Vec<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&paths) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
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
        rules::header_required_fields(&records, &required_by_type),
        rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type),
        rules::field_quality(&records, &required_by_type),
        rules::filename_title_consistency(&records, &full_text),
        rules::supersession_reciprocity(&records),
        rules::revision_log_change_class(&records, &full_text),
        rules::embodiment_consistency(&records, &drifted),
        rules::embodiment_locator_promotion_candidate(&records),
        rules::header_layout_consistency(&records, &header_layout_by_type),
        rules::header_field_set_consistency(&records, &known_fields_by_type),
        rules::narrative_field_stale(&records, &config),
        rules::type_no_declared_spec(&config, &config_path),
        rules::header_deprecated_shape(&config, &config_path),
        rules::header_pointer_field_clean(&records, &config),
        rules::config_pointer_declaration_missing(&config, &config_path),
        rules::config_pointer_field_not_known(&config, &config_path),
        rules::config_pointer_narrative_overlap(&config, &config_path),
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
            source: format!("{:?}", discovered.source),
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
        notices: Vec::new(),
    };

    emit(&report)
}
