//! `urzua export --format json` (SPEC-1): every record of every configured
//! type as plain data, so tooling reads records through the same parser
//! `check` uses instead of a second one (`MILE-115`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::config::RelationRole;
use urzua_core::report::{
    ExportReport, ExportTypeCount, ExportedField, ExportedRecord, ExportedSectionEntry, Notice,
    NoticeSeverity, ReportStatus,
};

use urzua_core::config::Config;
use urzua_core::record::Record;

use crate::emit;
use crate::session::{emit_or_not_run, Session};
use urzua_core::report::Failure;

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(build(config_path, &RecordFilter::default()).map(|report| emit(&report)))
}

/// Which records `build` keeps. The default keeps every record with its text,
/// which is `urzua export --format json`; the MCP server narrows it
/// (`urzua_list_records`, `SPEC-26`).
#[derive(Default)]
pub(crate) struct RecordFilter {
    /// Keep only this record type.
    pub record_type: Option<String>,
    /// Keep only records whose status field holds one of these values, compared
    /// as written. The field is the type's declared `status` relation field
    /// (`ADR-61`), so no status name is known here.
    pub status: Vec<String>,
}

/// The export report, filtered. A filter that names a type the configuration
/// does not declare, or a status no kept record holds, says so in a notice.
pub(crate) fn build(
    config_path: Option<PathBuf>,
    filter: &RecordFilter,
) -> Result<ExportReport, Failure> {
    // `raw_text` publishes a file's contents; `Session::open` refuses a tracked
    // record that resolves outside the repository or its type's directory, so an
    // export cannot disclose an arbitrary file under a repository path.
    let Session {
        config,
        records,
        full_text,
        ..
    } = Session::open(config_path)?;

    let mut exported: Vec<ExportedRecord> = records
        .iter()
        .map(|record| exported_record(record, &config, &full_text))
        .collect();
    exported.sort_by(|a, b| (&a.record_type, &a.path).cmp(&(&b.record_type, &b.path)));
    let notices = apply_filter(&mut exported, &config, filter);

    let types = config
        .sorted_type_names()
        .into_iter()
        .map(|name| ExportTypeCount {
            record_type: name.clone(),
            dir: config.record_types[name].dir.clone(),
            records: exported.iter().filter(|r| &r.record_type == name).count(),
        })
        .collect();

    Ok(ExportReport {
        status: ReportStatus::Ok,
        record_count: exported.len(),
        types,
        records: exported,
        roadmap: (!config.roadmap.pairs.is_empty())
            .then(|| urzua_core::rules::compute_roadmap(&records, &config)),
        notices,
    })
}

/// One record as plain data: its header, sections and complete text.
fn exported_record(
    record: &Record,
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) -> ExportedRecord {
    let text = full_text
        .get(&record.path)
        .map(String::as_str)
        .unwrap_or_default();
    let read = urzua_core::rules::exported_sections(
        record,
        &config.record_types[&record.record_type],
        text,
    );
    let (sections, preamble) = match read {
        Some((sections, preamble)) => (
            Some(
                sections
                    .into_iter()
                    .map(|s| ExportedSectionEntry {
                        name: s.name,
                        heading: s.heading,
                        line: s.line,
                        end_line: s.end_line,
                        content: s.content,
                    })
                    .collect(),
            ),
            Some(preamble),
        ),
        None => (None, None),
    };
    ExportedRecord {
        sections,
        preamble,
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
        raw_text: text.to_string(),
    }
}

/// Narrows `exported` to what `filter` names and returns the notices for a
/// filter value that matched nothing (`SPEC-1`: no silent no-op).
fn apply_filter(
    exported: &mut Vec<ExportedRecord>,
    config: &Config,
    filter: &RecordFilter,
) -> Vec<Notice> {
    let mut notices = Vec::new();
    if let Some(wanted) = &filter.record_type {
        if !config.record_types.contains_key(wanted) {
            notices.push(Notice::new(
                NoticeSeverity::Warning,
                "list-records.type",
                format!("no record type `{wanted}` is declared"),
            ));
        }
        exported.retain(|r| &r.record_type == wanted);
    }
    if !filter.status.is_empty() {
        let status_of = |r: &ExportedRecord| -> Option<String> {
            let field = config.record_types[&r.record_type].relation_field(RelationRole::Status);
            r.header
                .iter()
                .find(|f| f.name == field)
                .map(|f| f.value.clone())
        };
        let held: Vec<Option<String>> = exported.iter().map(status_of).collect();
        for wanted in &filter.status {
            if !held.iter().any(|h| h.as_deref() == Some(wanted.as_str())) {
                notices.push(Notice::new(
                    NoticeSeverity::Info,
                    "list-records.status",
                    format!("no record of the selected type holds the status `{wanted}`"),
                ));
            }
        }
        let mut keep = held
            .iter()
            .map(|h| h.as_ref().is_some_and(|v| filter.status.contains(v)));
        exported.retain(|_| keep.next().unwrap_or(false));
    }
    notices
}
