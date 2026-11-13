//! `urzua mcp` (SPEC-26, ADR-99): a read-only Model Context Protocol server on
//! stdio. Each tool builds the report the matching command builds, through the
//! same function, and hands it to the protocol layer (`urzua-mcp`) as JSON. This
//! module and that crate contain no code that writes (`mcp_integration` scans
//! both for write APIs).

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Map, Value};
use urzua_core::report::{CouldNotRun, Failure};
use urzua_mcp::{Host, Param, ParamKind, ToolResult, ToolSpec};

use crate::commands::check::{self, CheckArgs};
use crate::commands::export::{self, RecordFilter};
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
];

struct Server {
    config: Option<PathBuf>,
}

pub fn run(config: Option<PathBuf>, root: Option<PathBuf>) -> ExitCode {
    match start(config, root) {
        Ok(mut server) => {
            let stdin = std::io::stdin().lock();
            let stdout = std::io::stdout().lock();
            match urzua_mcp::serve(&mut server, env!("CARGO_PKG_VERSION"), stdin, stdout) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => could_not_start(&format!("the stdio transport failed: {e}")),
            }
        }
        Err(message) => could_not_start(&message),
    }
}

/// Resolves the root, then proves the configuration loads, so a server that
/// cannot read the corpus fails at start rather than on its first call.
fn start(config: Option<PathBuf>, root: Option<PathBuf>) -> Result<Server, String> {
    // A relative `--config` means relative to where the server was launched.
    let config = config
        .map(std::path::absolute)
        .transpose()
        .map_err(|e| e.to_string())?;
    if let Some(root) = root {
        std::env::set_current_dir(&root)
            .map_err(|e| format!("could not use {} as the root: {e}", root.display()))?;
    }
    Workspace::open(&[], config.clone()).map_err(|e| e.to_string())?;
    Ok(Server { config })
}

/// Stdout carries protocol frames only, so a start failure is JSON on stderr.
fn could_not_start(message: &str) -> ExitCode {
    let report = CouldNotRun::from(message.to_string());
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        serde_json::to_string(&report).unwrap_or_default()
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

/// A report, or the could-not-run shape with `isError` (`ADR-46`).
fn outcome<R: serde::Serialize>(built: Result<R, Failure>) -> ToolResult {
    match built {
        Ok(report) => ToolResult {
            report: serde_json::to_value(report).unwrap_or(Value::Null),
            is_error: false,
        },
        Err(failure) => ToolResult {
            report: serde_json::to_value(CouldNotRun::from(failure)).unwrap_or(Value::Null),
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
