//! `urzua audit` (SPEC-11): cross-record reconciliation, supersession
//! reciprocity and dangling cross-references, reusing the same rule
//! functions `check` calls rather than a second implementation (ADR-0030).
//! Never writes -- a bulk cross-reference rewrite is a real data-loss risk
//! without a review step this command doesn't have.

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

    let (records, _full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let (record_index, identity_collisions) = rules::build_index_reporting_collisions(&records);

    let (exec1, findings1) = crate::gate::gated(&config, rules::RULE_POINTER_RESOLUTION, || {
        rules::pointer_resolution(&records, &config, &record_index)
    });
    let (exec2, findings2) = crate::gate::gated(
        &config,
        rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
        || rules::supersession_reciprocity(&records, &config, &record_index),
    );
    let (exec3, findings3) = crate::gate::gated(&config, rules::RULE_IDENTITY_COLLISION, || {
        rules::identity_collision(&records, identity_collisions)
    });
    let mut findings = findings1;
    findings.extend(findings2);
    findings.extend(findings3);

    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let executed = [exec1.clone(), exec2.clone(), exec3.clone()];
    let status = if records.is_empty() || !crate::gate::any_rule_looked(&executed) {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let rules_executed = vec![exec1, exec2, exec3];
    let report = CheckReport {
        status,
        files_examined: records.len(),
        records_read_by_any_rule: urzua_core::report::records_read_by_any_rule(&rules_executed),
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
