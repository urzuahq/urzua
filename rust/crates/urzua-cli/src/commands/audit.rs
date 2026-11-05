//! `urzua audit` (SPEC-11): cross-record reconciliation, supersession
//! reciprocity and dangling cross-references, reusing the same rule
//! functions `check` calls rather than a second implementation (ADR-0030).
//! Never writes -- a bulk cross-reference rewrite is a real data-loss risk
//! without a review step this command doesn't have.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CheckReport, ReportStatus, ScopeInfo};
use urzua_core::rules;

use crate::emit;
use crate::session::{emit_or_not_run, Session};
use urzua_core::report::Failure;

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(run_inner(config_path))
}

fn run_inner(config_path: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let Session {
        config,
        discovered,
        records,
        ..
    } = Session::open(config_path)?;

    let (record_index, identity_collisions) = rules::build_index_reporting_collisions(&records);
    let notices = rules::identity_collision_notices(&identity_collisions);

    let (exec1, findings1) = crate::gate::gated(&config, rules::RULE_POINTER_RESOLUTION, || {
        rules::pointer_resolution(&records, &config, &record_index)
    });
    let (exec2, findings2) = crate::gate::gated(
        &config,
        rules::RULE_RELATION_SUPERSESSION_RECIPROCITY,
        || rules::supersession_reciprocity(&records, &config, &record_index),
    );
    let (exec3, findings3) = crate::gate::gated(&config, rules::RULE_IDENTITY_COLLISION, || {
        rules::identity_collision(&records, &identity_collisions)
    });
    let (exec4, findings4) =
        crate::gate::gated(&config, rules::RULE_FILENAME_IDENTITY_PATTERN, || {
            rules::filename_identity_pattern(&records, &config)
        });
    let mut findings = findings1;
    findings.extend(findings2);
    findings.extend(findings3);
    findings.extend(findings4);

    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let executed = [exec1.clone(), exec2.clone(), exec3.clone(), exec4.clone()];
    let status = if records.is_empty() || !crate::gate::any_rule_looked(&executed) {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let rules_executed = vec![exec1, exec2, exec3, exec4];
    let report = CheckReport {
        status,
        files_examined: records.len(),
        records_read_by_any_rule: urzua_core::report::records_read_by_any_rule(&rules_executed),
        rules_executed,
        scope: ScopeInfo {
            source: crate::discovery::scope_source(discovered.source),
            base: None,
            diff: None,
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
        notices,
        fail_on: config.fail_on,
    };

    Ok(emit(&report))
}
