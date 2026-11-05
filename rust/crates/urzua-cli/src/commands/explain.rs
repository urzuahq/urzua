//! `urzua explain` (SPEC-13): "which decisions govern this file" -- every
//! record whose `Realized-by` names `path` as evidence (ADR-0024).

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{ExplainReport, ReportStatus};

use crate::emit;
use crate::session::{emit_or_not_run, Session};

pub fn run(config_path: Option<PathBuf>, path: String) -> ExitCode {
    emit_or_not_run(run_inner(config_path, path))
}

fn run_inner(config_path: Option<PathBuf>, path: String) -> Result<ExitCode, String> {
    let Session {
        config, records, ..
    } = Session::open(config_path)?;
    let governing_records = urzua_core::graph::explain(&records, &path, &config);
    let (population, notice) = urzua_core::graph::read_population(&records, &config, "explain");

    Ok(emit(&ExplainReport {
        status: ReportStatus::Ok,
        path,
        governing_records,
        population,
        notices: notice.into_iter().collect(),
    }))
}
