//! `urzua explain` (SPEC-13): "which decisions govern this file" -- every
//! record whose `Realized-by` names `path` as evidence (ADR-0024).

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, ExplainReport, ReportStatus};

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::emit;

pub fn run(config_path: Option<PathBuf>, path: String) -> ExitCode {
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
    let governing_records = urzua_core::graph::explain(&records, &path);

    emit(&ExplainReport {
        status: ReportStatus::Ok,
        path,
        governing_records,
        notices: Vec::new(),
    })
}
