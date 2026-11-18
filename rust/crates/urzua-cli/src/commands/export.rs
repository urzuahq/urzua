//! `urzua export --format json` (SPEC-1): every record of every configured
//! type as plain data, so tooling reads records through the same parser
//! `check` uses instead of a second one (`MILE-115`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::RelationRole;
use urzua_core::report::{
    ChecklistItem, ExportReport, ExportTypeCount, ExportedField, ExportedRecord,
    ExportedSectionEntry, Notice, NoticeSeverity, ReadStamp, ReportStatus,
};

use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::view_model::{self, RecordView, ViewInput};

use crate::emit;
use crate::session::{emit_or_not_run, Session};
use urzua_core::report::Failure;

pub fn run(config_path: Option<PathBuf>, options: &ExportOptions) -> ExitCode {
    emit_or_not_run(
        build_with(config_path, &RecordFilter::default(), options).map(|report| emit(&report)),
    )
}

/// What an export carries beyond the filter.
pub(crate) struct ExportOptions {
    /// Each record's `raw_text`. Off drops the key, which is most of a large
    /// export's size.
    pub raw_text: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions { raw_text: true }
    }
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
    build_with(config_path, filter, &ExportOptions::default())
}

/// [`build`] with the options of the command line.
pub(crate) fn build_with(
    config_path: Option<PathBuf>,
    filter: &RecordFilter,
    options: &ExportOptions,
) -> Result<ExportReport, Failure> {
    // `raw_text` publishes a file's contents; `Session::open` refuses a tracked
    // record that resolves outside the repository or its type's directory, so an
    // export cannot disclose an arbitrary file under a repository path.
    let session = Session::open(config_path)?;
    let Session {
        repo_root,
        config,
        records,
        full_text,
        ..
    } = &session;

    let (edges, _) = crate::commands::graph::edges_of(records, config);
    let model = view_model::compute(&ViewInput {
        records,
        config,
        full_text,
        edges: &edges,
    });
    let checklists = checklists(&session);
    let mut exported: Vec<ExportedRecord> = records
        .iter()
        .zip(model.records)
        .map(|(record, view)| {
            let checklist = checklists.get(&record.path);
            exported_record(record, config, full_text, view, checklist, options)
        })
        .collect();
    exported.sort_by(|a, b| {
        a.record_type
            .cmp(&b.record_type)
            .then_with(|| {
                let id = |r: &ExportedRecord| r.id.clone().unwrap_or_default();
                urzua_core::rules::natural_cmp(&id(a), &id(b))
            })
            .then_with(|| a.path.cmp(&b.path))
    });
    let notices = apply_filter(&mut exported, config, filter);

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
        type_roles: model.roles,
        read: read_stamp(repo_root, config),
        records: exported,
        roadmap: (!config.roadmap.pairs.is_empty())
            .then(|| urzua_core::rules::compute_roadmap(records, config)),
        notices,
    })
}

/// A declared checklist section's name and its items, as the delivery rule
/// judges them, for each record whose type declares one.
type Checklists = HashMap<PathBuf, (String, Vec<ChecklistItem>)>;

fn checklists(session: &Session) -> Checklists {
    let Session {
        repo_root,
        config,
        discovered,
        records,
        ..
    } = session;
    let declared = urzua_core::rules::statuses_by_type(config);
    let mut out = Checklists::new();
    crate::file_texts::with_locator_env(repo_root, config, &discovered.paths, records, |env| {
        for record in records {
            let Some(items) = &record.items else { continue };
            let Some(delivery) = declared
                .get(&record.record_type)
                .and_then(|d| d.delivery.as_ref())
            else {
                continue;
            };
            let Some(decl) = &delivery.items else { continue };
            let judged = urzua_core::rules::judge_items(record, items, config, delivery, env);
            let listed = judged
                .items
                .iter()
                .zip(&items.items)
                .map(|(standing, item)| ChecklistItem {
                    n: standing.n,
                    line: standing.line,
                    text: standing.text.clone(),
                    resolved: standing.resolved(),
                    evidence: item.evidence.as_ref().map(|c| c.text.clone()),
                    waived: standing.is_waived(),
                })
                .collect();
            out.insert(record.path.clone(), (decl.section.clone(), listed));
        }
    });
    out
}

/// The commit and branch the working tree is on, whether any file under a
/// record directory differs from the commit, and when this was read. A failure
/// to ask git is an absent value, never a guess.
fn read_stamp(repo_root: &Path, config: &Config) -> ReadStamp {
    let git = |args: &[&str]| {
        urzua_io::git::stdout_in(repo_root, args)
            .ok()
            .map(|out| String::from_utf8_lossy(&out).trim().to_string())
            .filter(|text| !text.is_empty())
    };
    let mut status = vec!["status", "--porcelain", "--untracked-files=all", "--"];
    let dirs: Vec<&str> = config
        .record_types
        .values()
        .flat_map(|t| t.all_dirs())
        .collect();
    status.extend(&dirs);
    ReadStamp {
        commit: git(&["rev-parse", "HEAD"]),
        branch: git(&["symbolic-ref", "--short", "-q", "HEAD"]),
        uncommitted_record_changes: !dirs.is_empty() && git(&status).is_some(),
        time: urzua_io::read_instant(),
    }
}

/// One record as plain data: its header, sections and complete text, and what
/// a view of it needs.
fn exported_record(
    record: &Record,
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    view: RecordView,
    checklist: Option<&(String, Vec<ChecklistItem>)>,
    options: &ExportOptions,
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
                    .map(|s| {
                        let (ticked, unticked) = view_model::checkbox_counts(&s.content);
                        let items = checklist
                            .filter(|(section, _)| *section == s.name || *section == s.heading)
                            .map(|(_, items)| items.clone());
                        ExportedSectionEntry {
                            name: s.name,
                            heading: s.heading,
                            line: s.line,
                            end_line: s.end_line,
                            content: s.content,
                            ticked,
                            unticked,
                            items,
                        }
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
        raw_text: options.raw_text.then(|| text.to_string()),
        view,
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
