//! `urzua roadmap` (`ADR-82`, `MILE-182`): waves, the ready set, the critical
//! path, what each record blocks and a grouping, computed from the records and
//! printed as JSON. Read-only; nothing is stored.
//!
//! Implements: MILE-182

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{Failure, ReportStatus, RoadmapReport};

use crate::emit;
use crate::session::{emit_or_not_run, Session};

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(run_inner(config_path))
}

fn run_inner(config_path: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let Session {
        config, records, ..
    } = Session::open(config_path)?;
    Ok(emit(&RoadmapReport {
        status: ReportStatus::Ok,
        roadmap: urzua_core::rules::compute_roadmap(&records, &config),
        notices: Vec::new(),
    }))
}
