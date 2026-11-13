//! `urzua graph` (SPEC-13): the full record-relationship graph, as data
//! (ADR-0024).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{GraphReport, ReportStatus};
use urzua_core::rules;

use crate::emit;
use crate::session::{emit_or_not_run, Session};
use urzua_core::report::Failure;

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(build(config_path).map(|report| emit(&report)))
}

/// The report `urzua graph` prints, also built by the MCP server.
pub(crate) fn build(config_path: Option<PathBuf>) -> Result<GraphReport, Failure> {
    let Session {
        config, records, ..
    } = Session::open(config_path)?;

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

    let (edges, collisions) = urzua_core::graph::graph(
        &records,
        &config,
        &pointer_fields_by_type,
        &narrative_fields_by_type,
    );

    let mut notices = rules::identity_collision_notices(&collisions);
    let (population, left_out) = urzua_core::graph::read_population(&records, &config, "graph");
    notices.extend(left_out);

    Ok(GraphReport {
        status: ReportStatus::Ok,
        edges,
        population,
        notices,
    })
}
