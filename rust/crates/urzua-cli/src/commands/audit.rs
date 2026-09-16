//! `urzua audit` (SPEC-11): cross-record reconciliation, supersession
//! reciprocity and dangling cross-references, reusing the same rule
//! functions `check` calls rather than a second implementation (ADR-0030).
//! Never writes -- a bulk cross-reference rewrite is a real data-loss risk
//! without a review step this command doesn't have.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CheckReport, CouldNotRun, ReportStatus, ScopeInfo};
use urzua_core::rules;

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::emit;

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
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

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);

    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    let (exec1, findings1) =
        rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type);
    let (exec2, findings2) = rules::supersession_reciprocity(&records);
    let mut findings = findings1;
    findings.extend(findings2);

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
        rules_executed: vec![exec1, exec2],
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
