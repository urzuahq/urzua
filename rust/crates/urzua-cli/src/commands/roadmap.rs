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
        repo_root,
        config,
        discovered,
        records,
        ..
    } = Session::open(config_path)?;
    let mut roadmap = urzua_core::rules::compute_roadmap(&records, &config);
    // A type that declares `delivery.items` reports its item counts (`ADR-96`).
    if records.iter().any(|r| r.items.is_some()) {
        crate::file_texts::with_locator_env(&repo_root, &config, &discovered.paths, |env| {
            urzua_core::rules::add_item_counts(&mut roadmap, &records, &config, env);
        });
    }
    Ok(emit(&RoadmapReport {
        status: ReportStatus::Ok,
        roadmap,
        notices: Vec::new(),
    }))
}
