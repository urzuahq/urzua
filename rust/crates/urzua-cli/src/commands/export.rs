//! `urzua export --format json` (SPEC-1): every record of every configured
//! type as plain data, so tooling reads records through the same parser
//! `check` uses instead of a second one (`MILE-115`).

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{
    CouldNotRun, ExportReport, ExportTypeCount, ExportedField, ExportedRecord, ReportStatus,
};

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

    let (records, full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut exported: Vec<ExportedRecord> = records
        .iter()
        .map(|record| ExportedRecord {
            record_type: record.record_type.clone(),
            path: record.path.clone(),
            id: urzua_core::rules::record_id(record),
            stable_id: record.header.get_reserved("Stable-Id").map(str::to_string),
            header_readable: !record.header.is_unreadable(),
            header: record
                .header
                .fields
                .iter()
                .map(|f| ExportedField {
                    name: f.key.to_string(),
                    value: f.value.clone(),
                    line: f.line,
                })
                .collect(),
            raw_text: full_text.get(&record.path).cloned().unwrap_or_default(),
        })
        .collect();
    exported.sort_by(|a, b| (&a.record_type, &a.path).cmp(&(&b.record_type, &b.path)));

    let types = config
        .sorted_type_names()
        .into_iter()
        .map(|name| ExportTypeCount {
            record_type: name.clone(),
            dir: config.record_types[name].dir.clone(),
            records: exported.iter().filter(|r| &r.record_type == name).count(),
        })
        .collect();

    emit(&ExportReport {
        status: ReportStatus::Ok,
        record_count: exported.len(),
        types,
        records: exported,
        notices: Vec::new(),
    })
}
