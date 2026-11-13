//! A minimal Model Context Protocol server over stdio (`SPEC-26`, `ADR-99`).
//!
//! This crate is the protocol layer and nothing else: newline-delimited
//! JSON-RPC 2.0 framing and the five methods a tools-only server answers
//! (`initialize`, `notifications/initialized`, `ping`, `tools/list`,
//! `tools/call`). It reads from a `BufRead`, writes to a `Write`, and has no
//! filesystem, process or network code: the report-producing calls are supplied
//! by the caller through [`Host`], and every tool is advertised read-only.
//!
//! Implements: SPEC-26

#![forbid(unsafe_code)]

use std::io::{BufRead, Write};

use serde_json::{json, Map, Value};

/// The one protocol revision this server speaks (`SPEC-26`).
pub const PROTOCOL_VERSION: &str = "2025-06-18";

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;

/// The JSON type of one tool argument.
#[derive(Clone, Copy)]
pub enum ParamKind {
    Text,
    TextList,
}

/// One declared tool argument. The advertised input schema and the check
/// applied to a call are both generated from this, so they cannot disagree.
pub struct Param {
    pub name: &'static str,
    pub kind: ParamKind,
    pub required: bool,
    pub description: &'static str,
}

/// One tool: its name, what it does, and the arguments it takes.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub params: &'static [Param],
}

/// What a tool produced: the report as JSON, and whether it is a could-not-run.
pub struct ToolResult {
    pub report: Value,
    pub is_error: bool,
}

/// The caller's side: the tools it offers and how to run one.
pub trait Host {
    fn tools(&self) -> &'static [ToolSpec];
    /// Run a tool already known to `tools()` whose arguments passed the
    /// declared check. `Err` is an invalid-params message.
    fn call(&mut self, name: &str, args: &Map<String, Value>) -> Result<ToolResult, String>;
}

/// Serve until `input` closes. Every line written to `output` is one JSON-RPC
/// message; nothing else is ever written to it.
pub fn serve<H: Host>(
    host: &mut H,
    server_version: &str,
    input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_line(host, server_version, &line) {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The response to one frame, or `None` for a notification or a client reply.
pub fn handle_line<H: Host>(host: &mut H, server_version: &str, line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(error(
                Value::Null,
                PARSE_ERROR,
                &format!("parse error: {e}"),
            ))
        }
    };
    let Some(object) = message.as_object() else {
        return Some(error(
            Value::Null,
            INVALID_REQUEST,
            "a message must be a JSON object (batches are not supported)",
        ));
    };
    let id = object.get("id").filter(|v| v.is_string() || v.is_number());
    let method = object.get("method").and_then(Value::as_str);
    let valid = object.get("jsonrpc").and_then(Value::as_str) == Some("2.0");
    match (valid, method) {
        // A reply to a request the server never sent: nothing to answer.
        (true, None) if object.contains_key("result") || object.contains_key("error") => None,
        (true, Some(method)) => {
            let result = dispatch(host, server_version, method, object.get("params"));
            // No `id` is a notification, which is never answered.
            let id = object.get("id")?.clone();
            Some(match result {
                Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
                Err((code, message)) => error(id, code, &message),
            })
        }
        _ => Some(error(
            id.cloned().unwrap_or(Value::Null),
            INVALID_REQUEST,
            "not a JSON-RPC 2.0 request",
        )),
    }
}

type RpcError = (i64, String);

fn dispatch<H: Host>(
    host: &mut H,
    server_version: &str,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "urzua", "version": server_version},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": host.tools().iter().map(describe).collect::<Vec<_>>()})),
        "tools/call" => call_tool(host, params),
        // Notifications (`notifications/initialized`, `notifications/cancelled`)
        // reach here too and are dropped by the caller; a request for anything
        // else is not found.
        m if m.starts_with("notifications/") => Ok(Value::Null),
        _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
    }
}

fn describe(tool: &ToolSpec) -> Value {
    let properties: Map<String, Value> = tool
        .params
        .iter()
        .map(|p| {
            let schema = match p.kind {
                ParamKind::Text => json!({"type": "string", "description": p.description}),
                ParamKind::TextList => json!({
                    "type": "array",
                    "items": {"type": "string"},
                    "description": p.description,
                }),
            };
            (p.name.to_string(), schema)
        })
        .collect();
    let required: Vec<&str> = tool
        .params
        .iter()
        .filter(|p| p.required)
        .map(|p| p.name)
        .collect();
    json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
        "annotations": {"readOnlyHint": true},
    })
}

fn call_tool<H: Host>(host: &mut H, params: Option<&Value>) -> Result<Value, RpcError> {
    let invalid = |message: String| (INVALID_PARAMS, message);
    let params = params
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("tools/call needs a params object".to_string()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("tools/call needs a tool `name`".to_string()))?;
    let spec = host
        .tools()
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| invalid(format!("unknown tool: {name}")))?;
    let empty = Map::new();
    let args = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(map)) => map,
        Some(_) => return Err(invalid("`arguments` must be an object".to_string())),
    };
    check_arguments(spec, args).map_err(invalid)?;
    let result = host.call(name, args).map_err(invalid)?;
    let text = result.report.to_string();
    Ok(json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": result.report,
        "isError": result.is_error,
    }))
}

/// An argument the tool does not declare, a missing required one, or one of
/// the wrong type is refused, never ignored.
fn check_arguments(spec: &ToolSpec, args: &Map<String, Value>) -> Result<(), String> {
    for key in args.keys() {
        if !spec.params.iter().any(|p| p.name == key) {
            return Err(format!("{}: unknown argument `{key}`", spec.name));
        }
    }
    for param in spec.params {
        match (args.get(param.name), param.kind) {
            (None, _) if param.required => {
                return Err(format!("{}: missing argument `{}`", spec.name, param.name))
            }
            (None, _) | (Some(Value::String(_)), ParamKind::Text) => {}
            (Some(Value::Array(items)), ParamKind::TextList)
                if items.iter().all(Value::is_string) => {}
            (Some(_), _) => {
                return Err(format!(
                    "{}: `{}` has the wrong type",
                    spec.name, param.name
                ))
            }
        }
    }
    Ok(())
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    static TOOLS: &[ToolSpec] = &[ToolSpec {
        name: "fake_list",
        description: "a read tool",
        params: &[
            Param {
                name: "kind",
                kind: ParamKind::Text,
                required: false,
                description: "k",
            },
            Param {
                name: "ids",
                kind: ParamKind::TextList,
                required: false,
                description: "i",
            },
            Param {
                name: "must",
                kind: ParamKind::Text,
                required: true,
                description: "m",
            },
        ],
    }];

    impl Host for Fake {
        fn tools(&self) -> &'static [ToolSpec] {
            TOOLS
        }
        fn call(&mut self, _: &str, args: &Map<String, Value>) -> Result<ToolResult, String> {
            Ok(ToolResult {
                report: json!({"status": "ok", "got": args.len()}),
                is_error: false,
            })
        }
    }

    fn ask(line: &str) -> Option<Value> {
        handle_line(&mut Fake, "9.9.9", line)
    }

    fn err_code(line: &str) -> i64 {
        ask(line).unwrap()["error"]["code"].as_i64().unwrap()
    }

    #[test]
    fn initialize_pins_the_protocol_revision_and_advertises_tools_only() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}"#).unwrap();
        assert_eq!(r["id"], 1);
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(r["result"]["capabilities"], json!({"tools": {}}));
        assert_eq!(r["result"]["serverInfo"]["version"], "9.9.9");
    }

    #[test]
    fn a_notification_is_never_answered() {
        assert!(ask(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        assert!(ask(r#"{"jsonrpc":"2.0","method":"no/such"}"#).is_none());
        assert!(ask(r#"{"jsonrpc":"2.0","id":3,"result":{}}"#).is_none());
    }

    #[test]
    fn ping_and_list_answer_and_every_tool_is_read_only() {
        assert_eq!(
            ask(r#"{"jsonrpc":"2.0","id":"a","method":"ping"}"#).unwrap()["result"],
            json!({})
        );
        let r = ask(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        let tool = &r["result"]["tools"][0];
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert_eq!(tool["inputSchema"]["required"], json!(["must"]));
    }

    #[test]
    fn errors_carry_the_standard_codes() {
        assert_eq!(err_code("{not json"), PARSE_ERROR);
        assert_eq!(err_code("[]"), INVALID_REQUEST);
        assert_eq!(err_code(r#"{"id":1,"method":"ping"}"#), INVALID_REQUEST);
        assert_eq!(
            err_code(r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#),
            METHOD_NOT_FOUND
        );
    }

    #[test]
    fn a_call_is_refused_for_an_unknown_tool_or_a_bad_argument() {
        let call = |p: &str| {
            err_code(&format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{p}}}"#
            ))
        };
        assert_eq!(call(r#"{"name":"fake_write"}"#), INVALID_PARAMS);
        assert_eq!(call(r#"{"name":"fake_list"}"#), INVALID_PARAMS);
        assert_eq!(
            call(r#"{"name":"fake_list","arguments":{"must":"x","extra":1}}"#),
            INVALID_PARAMS
        );
        assert_eq!(
            call(r#"{"name":"fake_list","arguments":{"must":1}}"#),
            INVALID_PARAMS
        );
        assert_eq!(
            call(r#"{"name":"fake_list","arguments":{"must":"x","ids":[1]}}"#),
            INVALID_PARAMS
        );
        assert_eq!(
            call(r#"{"name":"fake_list","arguments":[]}"#),
            INVALID_PARAMS
        );
    }

    #[test]
    fn a_result_is_the_report_as_structured_content_and_identical_text() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fake_list","arguments":{"must":"x"}}}"#).unwrap();
        let result = &r["result"];
        assert_eq!(result["isError"], false);
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, result["structuredContent"]);
        assert_eq!(result["content"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn serve_writes_one_frame_per_line_and_nothing_else() {
        let input = "\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\nnot json\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n";
        let mut out = Vec::new();
        serve(&mut Fake, "1", input.as_bytes(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let v: Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["jsonrpc"], "2.0");
        }
    }
}
