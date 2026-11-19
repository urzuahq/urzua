//! `urzua board`: one HTML page showing every record, laid out only from what
//! the configuration declares and read from the export's view model. Written to
//! a file, or served at a local address with `--serve`. Stdout stays JSON: the
//! page is a file or a response, never stdout. `--prompt` prints, in a JSON
//! answer, the text a person gives a coding agent to build a view of their own.
//!
//! `--serve` is in `board_server`: a read-only, foreground, standard-library
//! server on 127.0.0.1.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::report::{
    DetailValue, ExportReport, Failure, FindingSeverity, Notice, NoticeSeverity, ReadStamp, Report,
    ReportStatus,
};

use super::board_declared::{declared, prompt};
use super::board_page::{progress_gap_sentence, progress_gaps, render, CheckLine, CheckNote};
use super::check::{self, CheckArgs};
use super::export::{self, ExportOptions, RecordFilter};
use crate::emit;
use crate::session::{emit_or_not_run, Workspace};
use std::sync::atomic::{AtomicU64, Ordering};

/// How long the last `GET /check` took, in seconds; 0 until one has run. The
/// served page says so beside its link.
pub(super) static LAST_CHECK_SECS: AtomicU64 = AtomicU64::new(0);

/// The file `urzua board` writes when `--out` is not given, under the
/// repository's derived-state directory.
const DEFAULT_FILE: &str = ".urzua/cache/board.html";

pub struct BoardArgs {
    pub out: Option<PathBuf>,
    pub serve: bool,
    pub port: u16,
    pub allowed_hosts: Vec<String>,
    pub prompt: bool,
}

/// What `urzua board` printed when it wrote a file.
#[derive(serde::Serialize)]
struct BoardReport {
    status: ReportStatus,
    file: PathBuf,
    bytes: usize,
    record_count: usize,
    read: ReadStamp,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

impl Report for BoardReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

/// The one line `urzua board --serve` prints when it is listening.
#[derive(serde::Serialize)]
pub(super) struct Listening {
    pub(super) status: ReportStatus,
    pub(super) listening: String,
    pub(super) address: String,
    pub(super) port: u16,
}

impl Report for Listening {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

/// What `urzua board --prompt` printed: the text a person gives their coding
/// agent, written from the configuration.
#[derive(serde::Serialize)]
struct PromptReport {
    status: ReportStatus,
    prompt: String,
}

impl Report for PromptReport {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

fn print_prompt(config_path: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let ws = Workspace::open(&[], config_path)?;
    Ok(emit(&PromptReport {
        status: ReportStatus::Ok,
        prompt: prompt(&declared(&ws.config)),
    }))
}

pub fn run(config_path: Option<PathBuf>, args: BoardArgs) -> ExitCode {
    emit_or_not_run(if args.prompt {
        print_prompt(config_path)
    } else if args.serve {
        super::board_server::serve(config_path, args.port, args.allowed_hosts)
    } else {
        write_file(config_path, args.out)
    })
}

pub(super) fn check_line(config_path: &Option<PathBuf>) -> Option<CheckLine> {
    let report = check::execute(config_path.clone(), CheckArgs::default()).ok()?;
    let count = |severity: FindingSeverity| {
        report
            .findings
            .iter()
            .filter(|f| f.waived.is_none() && f.severity == severity)
            .count()
    };
    Some(CheckLine {
        status: serde_json::to_value(report.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default(),
        errors: count(FindingSeverity::Error),
        warnings: count(FindingSeverity::Warning),
        infos: count(FindingSeverity::Info),
        waived: report
            .findings
            .iter()
            .filter(|f| f.waived.is_some())
            .count(),
    })
}

/// The page, and the export it was drawn from. The file carries the check
/// summary, computed when it is written. A served page does not wait for one:
/// `check` takes as long as it takes, so it links `GET /check`, and carries the
/// digest it was built at (taken before the records are read, so a change made
/// while it builds shows as a difference at the next poll).
pub(super) fn build_page(
    config_path: &Option<PathBuf>,
    served_at: Option<&str>,
) -> Result<(String, ExportReport), Failure> {
    let options = ExportOptions {
        section_text: false,
        ..ExportOptions::default()
    };
    let export = export::build_with(config_path.clone(), &RecordFilter::default(), &options)?;
    #[cfg(feature = "test-seams")]
    super::board_server::test_delay("URZUA_BOARD_TEST_BUILD_DELAY_MS");
    let line = served_at
        .is_none()
        .then(|| check_line(config_path))
        .flatten();
    let note = match (&line, served_at) {
        (_, Some(_)) => CheckNote::Linked(match LAST_CHECK_SECS.load(Ordering::Relaxed) {
            0 => None,
            secs => Some(secs),
        }),
        (Some(line), None) => CheckNote::Line(line),
        (None, None) => CheckNote::Unavailable,
    };
    let ws = Workspace::open(&[], config_path.clone())?;
    let page = render(&export, &note, served_at, &declared(&ws.config));
    Ok((page, export))
}

/// What `GET /stamp` answers: where the working tree is, and a digest that
/// changes when a record file or the configuration does.
#[derive(serde::Serialize)]
pub(super) struct Stamp {
    pub(super) commit: Option<String>,
    pub(super) branch: Option<String>,
    pub(super) digest: String,
}

fn file_lines(root: &Path, dir: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        out.push(file_line(
            &format!("{dir}/{}", entry.file_name().to_string_lossy()),
            &entry.path(),
        ));
    }
}

fn file_line(shown: &str, path: &Path) -> String {
    let meta = std::fs::metadata(path).ok().filter(|m| m.is_file());
    let size = meta.as_ref().map_or(0, std::fs::Metadata::len);
    let modified = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    format!("{shown} {size} {modified}")
}

/// The stamp: git's commit and branch, and a digest of the path, size and
/// modification time of every file in a record directory and of the
/// configuration. It opens no record, runs no rule and builds no export, so
/// it costs two `git` calls and a directory listing.
pub(super) fn stamp(config_path: &Option<PathBuf>) -> Result<Stamp, Failure> {
    let ws = Workspace::open(&[], config_path.clone())?;
    let (commit, branch) = export::git_identity(&ws.repo_root);
    let mut dirs: Vec<&str> = ws
        .config
        .record_types
        .values()
        .flat_map(|t| t.all_dirs())
        .collect();
    dirs.sort_unstable();
    dirs.dedup();
    let mut lines = Vec::new();
    for dir in dirs {
        file_lines(&ws.repo_root, dir, &mut lines);
    }
    lines.push(file_line("config", &ws.config_path));
    lines.sort_unstable();
    let material = format!(
        "{}\n{}\n{}",
        commit.as_deref().unwrap_or_default(),
        branch.as_deref().unwrap_or_default(),
        lines.join("\n")
    );
    Ok(Stamp {
        commit,
        branch,
        digest: urzua_io::sha256_hex_of_str(&material),
    })
}

fn write_file(config_path: Option<PathBuf>, out: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let target = match out {
        Some(path) => path,
        None => Workspace::open(&[], config_path.clone())?
            .repo_root
            .join(DEFAULT_FILE),
    };
    let (page, export) = build_page(&config_path, None)?;
    let mut notices = progress_notices(&export);
    notices.extend(export.notices);
    if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    std::fs::write(&target, &page)
        .map_err(|e| format!("could not write {}: {e}", target.display()))?;
    Ok(emit(&BoardReport {
        status: ReportStatus::Ok,
        file: target,
        bytes: page.len(),
        record_count: export.record_count,
        read: export.read,
        notices,
    }))
}

fn progress_notices(export: &ExportReport) -> Vec<Notice> {
    progress_gaps(export)
        .into_iter()
        .map(|(name, statuses)| {
            let mut notice = Notice::new(
                NoticeSeverity::Info,
                "board.progress",
                progress_gap_sentence(name, statuses),
            );
            notice.detail = Some(
                [
                    ("type".to_string(), DetailValue::Text(name.to_string())),
                    ("statuses".to_string(), DetailValue::List(statuses.to_vec())),
                ]
                .into(),
            );
            notice
        })
        .collect()
}
