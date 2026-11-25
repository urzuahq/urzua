//! `urzua mcp` (SPEC-26, ADR-99): a read-only Model Context Protocol server on
//! stdio. Each tool builds the report the matching command builds, through the
//! same function, and hands it to the protocol layer (`urzua-mcp`) as JSON. This
//! module and that crate contain no code that writes (`mcp_integration` scans
//! both for write APIs).
//!
//! Implements: ADR-99

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Map, Value};
use urzua_core::report::FailureKind;
use urzua_core::report::{CouldNotRun, Failure};
use urzua_mcp::{Host, Param, ParamKind, ToolResult, ToolSpec};

use crate::commands::check::{self, CheckArgs};
use crate::commands::export::{self, RecordFilter};
use crate::commands::{explain, graph};
use crate::session::Workspace;

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "urzua_check",
        description: "Run urzua check over the corpus and return its report: findings, scope and the rules executed. Read-only; never applies a fix.",
        params: &[
            Param {
                name: "paths",
                kind: ParamKind::TextList,
                required: false,
                description: "Repository-relative record files or directories to report on; default the whole corpus.",
            },
            Param {
                name: "base",
                kind: ParamKind::Text,
                required: false,
                description: "Report only on records changed since the merge base of this git ref and HEAD.",
            },
            Param {
                name: "rules",
                kind: ParamKind::TextList,
                required: false,
                description: "Run only these declared rule ids.",
            },
        ],
    },
    ToolSpec {
        name: "urzua_list_records",
        description: "List the corpus's records (id, type, path, header fields) without their text, optionally only one type and only some statuses. Rejected and superseded decisions are type adr with status Rejected and Superseded.",
        params: &[
            Param {
                name: "type",
                kind: ParamKind::Text,
                required: false,
                description: "Only records of this configured record type.",
            },
            Param {
                name: "status",
                kind: ParamKind::TextList,
                required: false,
                description: "Only records whose status field holds one of these values, compared as written.",
            },
        ],
    },
    ToolSpec {
        name: "urzua_show_record",
        description: "Read one record by its id (for example ADR-23): its header fields, its sections, its complete text and the relation edges that touch it, supersession included. An unknown id is an error result.",
        params: &[Param {
            name: "id",
            kind: ParamKind::Text,
            required: true,
            description: "The record's display id, as urzua_list_records shows it.",
        }],
    },
    ToolSpec {
        name: "urzua_get_findings_for_path",
        description: "Which records govern a path: every record whose evidence names it, with the path's scope and coverage. This is urzua explain for a path.",
        params: &[Param {
            name: "path",
            kind: ParamKind::Text,
            required: true,
            description: "A repository-relative file, directory or glob, or path#scope.",
        }],
    },
    ToolSpec {
        name: "urzua_get_graph",
        description: "The full record-relationship graph: every Implements, Derives-from and Supersedes edge, with its kind and whether its target exists.",
        params: &[],
    },
];

struct Server {
    config: Option<PathBuf>,
    max_line_bytes: usize,
}

pub fn run(config: Option<PathBuf>, root: Option<PathBuf>) -> ExitCode {
    match start(config, root) {
        Ok(mut server) => {
            let stdin = std::io::stdin().lock();
            let stdout = std::io::stdout().lock();
            let max = server.max_line_bytes;
            match urzua_mcp::serve_with_limit(
                &mut server,
                env!("CARGO_PKG_VERSION"),
                stdin,
                stdout,
                max,
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => could_not_start(
                    FailureKind::Environment,
                    &format!("the stdio transport failed: {e}"),
                ),
            }
        }
        Err(failure) => could_not_start(failure.kind, &failure.message),
    }
}

/// Resolves the root, then proves the configuration loads, so a server that
/// cannot read the corpus fails at start rather than on its first call.
fn start(config: Option<PathBuf>, root: Option<PathBuf>) -> Result<Server, Failure> {
    // A relative `--config` means relative to where the server was launched.
    let config = config
        .map(std::path::absolute)
        .transpose()
        .map_err(|e| Failure::environment(e.to_string()))?;
    if let Some(root) = root {
        std::env::set_current_dir(&root).map_err(|e| {
            Failure::environment(format!("could not use {} as the root: {e}", root.display()))
        })?;
    }
    let ws = Workspace::open(&[], config.clone())?;
    let max_line_bytes = ws
        .config
        .harness
        .as_ref()
        .map_or(urzua_mcp::MAX_LINE_BYTES, |h| {
            h.limits
                .get_usize(urzua_core::harness::limits::MCP_LINE_BYTES)
        });
    Ok(Server {
        config,
        max_line_bytes,
    })
}

/// Stdout carries protocol frames only, so a start failure is JSON on stderr.
pub(crate) fn could_not_start(kind: FailureKind, message: &str) -> ExitCode {
    let report = CouldNotRun::new(kind, message.to_string());
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        crate::report_value(&report)
            .map(|v| v.to_string())
            .unwrap_or_default()
    );
    ExitCode::from(2)
}

impl Host for Server {
    fn tools(&self) -> &'static [ToolSpec] {
        TOOLS
    }

    fn call(&mut self, name: &str, args: &Map<String, Value>) -> Result<ToolResult, String> {
        match name {
            "urzua_check" => self.check(args),
            "urzua_list_records" => Ok(self.list_records(args)),
            "urzua_show_record" => Ok(self.show_record(args)),
            "urzua_get_findings_for_path" => self.findings_for_path(args),
            "urzua_get_graph" => Ok(outcome(graph::build(self.config.clone()))),
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

impl Server {
    fn check(&self, args: &Map<String, Value>) -> Result<ToolResult, String> {
        let paths = strings(args, "paths");
        if let Some(bad) = paths.iter().find(|p| !is_inside(Path::new(p))) {
            return Err(format!("`paths` must stay inside the repository: {bad}"));
        }
        let check_args = CheckArgs {
            paths: paths.iter().map(PathBuf::from).collect(),
            base: args.get("base").and_then(Value::as_str).map(str::to_string),
            rules: strings(args, "rules"),
            ..CheckArgs::default()
        };
        Ok(outcome(check::execute(self.config.clone(), check_args)))
    }

    fn list_records(&self, args: &Map<String, Value>) -> ToolResult {
        let filter = RecordFilter {
            record_type: args.get("type").and_then(Value::as_str).map(str::to_string),
            status: strings(args, "status"),
        };
        let mut result = outcome(export::build(self.config.clone(), &filter));
        // A listing names records. The roadmap spans the whole corpus whatever
        // the filter, so it is `urzua roadmap`'s, not a listing's.
        if let Some(report) = result.report.as_object_mut() {
            report.remove("roadmap");
        }
        if let Some(records) = result
            .report
            .get_mut("records")
            .and_then(Value::as_array_mut)
        {
            for record in records {
                if let Some(record) = record.as_object_mut() {
                    // A listing names records; reading one is `urzua_show_record`.
                    for text in ["raw_text", "sections", "preamble"] {
                        record.remove(text);
                    }
                }
            }
        }
        result
    }
}

impl Server {
    fn findings_for_path(&self, args: &Map<String, Value>) -> Result<ToolResult, String> {
        let path = text(args, "path");
        let file = path.split('#').next().unwrap_or_default();
        if !is_inside(Path::new(file)) {
            return Err(format!("`path` must stay inside the repository: {path}"));
        }
        Ok(outcome(explain::build(self.config.clone(), path)))
    }

    /// One record and the edges that touch it. An id no record carries is a
    /// could-not-run, never an empty record.
    fn show_record(&self, args: &Map<String, Value>) -> ToolResult {
        let id = text(args, "id");
        let built = export::build(self.config.clone(), &RecordFilter::default())
            .and_then(|export| Ok((export, graph::build(self.config.clone())?)));
        let (export, graph) = match built {
            Ok(both) => both,
            Err(failure) => return outcome::<CouldNotRun>(Err(failure)),
        };
        let Some(record) = export
            .records
            .iter()
            .find(|r| r.id.as_deref() == Some(id.as_str()))
        else {
            let message = format!("no record has the id `{id}`; urzua_list_records lists the ids");
            return outcome::<CouldNotRun>(Err(Failure::of(FailureKind::Usage, message)));
        };
        let edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.from == id || e.to == id)
            .collect();
        let mut notices = export.notices.clone();
        notices.extend(graph.notices.iter().cloned());
        ToolResult {
            report: serde_json::json!({
                "status": "ok",
                "record": record,
                "edges": edges,
                "notices": notices,
                "next_action": null,
                "next_action_argv": null,
                "next_action_reason": null,
                "next_action_inputs": [],
            }),
            is_error: false,
        }
    }
}

fn text(args: &Map<String, Value>, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A report, or the could-not-run shape with `isError` (`ADR-46`).
fn outcome<R: urzua_core::report::Report>(built: Result<R, Failure>) -> ToolResult {
    let (value, is_error) = match built {
        Ok(report) => {
            let value = crate::report_value(&report);
            // A report that says it could not run established nothing: an error
            // result, as the command line's exit 2 is a failure, not a finding.
            let not_run = value
                .as_ref()
                .is_ok_and(|v| v.get("status").and_then(Value::as_str) == Some("not-run"));
            (value, not_run)
        }
        Err(failure) => (
            crate::report_value(&CouldNotRun::from_failure(failure)),
            true,
        ),
    };
    match value {
        Ok(report) => ToolResult { report, is_error },
        // A report that is not a JSON object cannot carry the two next-action
        // keys: a could-not-run, as for `emit`.
        Err(message) => ToolResult {
            report: serde_json::json!({
                "status": "not-run",
                "error": message,
                "error_kinds": ["internal"],
                "next_action": null,
                "next_action_argv": null,
                "next_action_reason": null,
                "next_action_inputs": [],
            }),
            is_error: true,
        },
    }
}

fn strings(args: &Map<String, Value>, key: &str) -> Vec<String> {
    match args.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// A relative path with no `..`: the tools read the repository, not the disk.
fn is_inside(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
