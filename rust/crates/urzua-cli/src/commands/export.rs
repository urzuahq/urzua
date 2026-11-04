//! `urzua export --format json` (SPEC-1): every record of every configured
//! type as plain data, so tooling reads records through the same parser
//! `check` uses instead of a second one (`MILE-115`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::report::{
    ExportReport, ExportTypeCount, ExportedField, ExportedRecord, ReportStatus,
};

use urzua_core::config::Config;

use crate::emit;
use crate::session::{emit_or_not_run, Session};

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(run_inner(config_path))
}

fn run_inner(config_path: Option<PathBuf>) -> Result<ExitCode, String> {
    // `raw_text` publishes a file's contents, so a tracked record that is a
    // symlink resolving outside the repository must not be read: the export
    // would otherwise disclose an arbitrary file under a repository path.
    // Refuse the whole export rather than drop the record silently.
    let Session {
        config,
        records,
        full_text,
        ..
    } = Session::open_with(&[], config_path, |repo_root, config, discovered| {
        match first_record_outside_repo(repo_root, &discovered.paths, config) {
            Some(escaping) => Err(format!(
                "{} resolves outside the repository; export refuses to read it",
                escaping.display()
            )),
            None => Ok(()),
        }
    })?;

    let mut exported: Vec<ExportedRecord> = records
        .iter()
        .map(|record| ExportedRecord {
            record_type: record.record_type.clone(),
            path: record.path.clone(),
            id: urzua_core::rules::record_id(record),
            stable_id: record.header.get_reserved("Stable-Id").map(str::to_string),
            // A type declaring `header_shape: none` has no header by
            // design; that is readable, not a parse failure.
            header_readable: config.record_types[&record.record_type].has_no_header()
                || !record.header.is_unreadable(),
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

    Ok(emit(&ExportReport {
        status: ReportStatus::Ok,
        record_count: exported.len(),
        types,
        records: exported,
        notices: Vec::new(),
    }))
}

/// The first tracked file in a record directory whose real location is
/// outside `repo_root`, if any.
fn first_record_outside_repo(
    repo_root: &Path,
    paths: &[PathBuf],
    config: &Config,
) -> Option<PathBuf> {
    let root = repo_root.canonicalize().ok()?;
    paths
        .iter()
        .filter(|p| config.record_types.values().any(|t| t.owns_dir(p.parent())))
        .find(|p| match repo_root.join(p).canonicalize() {
            Ok(real) => !real.starts_with(&root),
            Err(_) => false,
        })
        .cloned()
}
