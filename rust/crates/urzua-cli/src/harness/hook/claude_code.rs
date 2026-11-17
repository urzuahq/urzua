//! The Claude Code adapter (`ADR-89` section 6): Claude Code's hook payload in,
//! urzua's neutral request to `urzua harness run`, the neutral verdict back out
//! as Claude Code's hook output. It holds no rule knowledge.
//!
//! The project directory is taken from the payload's `cwd`, never from
//! `CLAUDE_PROJECT_DIR`, for the directory the engine runs in: an unset variable
//! must not become an opaque blocking exit. The variable only names the
//! session's own project (`project_dir`), which the engine judges first.
//!
//! When it cannot build a request or the engine cannot be run, the call proceeds
//! after a stated message, except where the configuration turns on the
//! acceptance or protected-paths guardrail and leaves `on_unjudged: deny`: an
//! edit that was not judged is refused. The engine decides that when it can be
//! run; when it cannot, the configuration is read as text.
//!
//! The editor hook covers Write, Edit, MultiEdit and NotebookEdit, and any MCP
//! tool (`mcp__*`), whose input is scanned by `mcp` for every path-like field.

mod command;
mod mcp;
mod text;

pub use command::hook_command;

use std::path::{Path, PathBuf};

use super::super::json::{self, J};
use super::super::run::unjudged::{unjudged_policy, Unjudged};
use super::hook_json::{dumps, dumps_indent1, obj, s};
use super::{engine, text as vtext, truthy};

pub const NAME: &str = "claude-code";
pub const EVENTS: &[&str] = &["edit", "post-command", "session-start"];
/// The variable Claude Code sets to the session's project for hook commands.
pub const PROJECT_ENV: &str = "CLAUDE_PROJECT_DIR";

const EDIT_TOOLS: [&str; 4] = ["Write", "Edit", "MultiEdit", "NotebookEdit"];
/// The verdicts that let an edit through with nothing to say: judged and
/// allowed, or a file no record type governs.
const QUIET: [&str; 2] = ["allow", "not-governed"];
const ALLOWED_NOTES: [&str; 3] = ["allow-unjudged", "allow-override", "allow-with-warnings"];
/// Why an edit nobody judged is refused when the configuration was read without
/// the engine.
pub(super) const CLOSED: &str = "The configuration does not show with certainty that an unjudged edit may go through (acceptance or protected-paths is at on_unjudged: deny, or the configuration could not be read with certainty), so an agent's edit that was not judged is refused. A person edits outside the agent harness, or sets on_unjudged: allow in .urzua/config.yaml.";
/// How an error about a tool input of the wrong shape begins.
const MALFORMED: &str = "the tool input is malformed: ";
const POST_NOT_JUDGED: &str =
    "urzua harness (post-command): NOT judged; run 'urzua check' before committing.";

/// What one judgement of an edit came to.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Block,
    Stated,
    Note,
    Allow,
}

type Result1 = (Kind, String);

/// One run of the translator: the event, the project, and what it prints.
struct Run {
    event: String,
    project: Option<String>,
    out: String,
}

impl Run {
    fn argv(&self) -> String {
        super::engine_display(&self.event)
    }

    fn print(&mut self, v: &J) {
        self.out.push_str(&dumps(v));
        self.out.push('\n');
    }

    /// The engine could not give a verdict: say so, loudly, and let the call proceed.
    fn stated(&mut self, message: &str, event: &str) {
        eprintln!("{message}");
        if event == "post-command" {
            return self.relay_post_command(message, POST_NOT_JUDGED);
        }
        let hook = if event == "session-start" {
            "SessionStart"
        } else {
            "PreToolUse"
        };
        self.print(&obj(vec![
            ("systemMessage", s(message)),
            (
                "hookSpecificOutput",
                obj(vec![
                    ("hookEventName", s(hook)),
                    ("additionalContext", s(message)),
                ]),
            ),
        ]));
    }

    /// A post-command report goes to one audience per message: `person` as the
    /// message shown to the person, `agent` as the model's context.
    fn relay_post_command(&mut self, person: &str, agent: &str) {
        let mut members = Vec::new();
        if !person.is_empty() {
            members.push(("systemMessage", s(person)));
        }
        if !agent.is_empty() && agent != person {
            members.push((
                "hookSpecificOutput",
                obj(vec![
                    ("hookEventName", s("PostToolUse")),
                    ("additionalContext", s(agent)),
                ]),
            ));
        }
        if !members.is_empty() {
            self.print(&obj(members));
        }
    }

    fn deny(&mut self, message: &str) {
        self.print(&obj(vec![(
            "hookSpecificOutput",
            obj(vec![
                ("hookEventName", s("PreToolUse")),
                ("permissionDecision", s("deny")),
                ("permissionDecisionReason", s(message)),
            ]),
        )]));
    }

    fn note(&mut self, message: &str) {
        self.print(&obj(vec![
            ("systemMessage", s(message)),
            (
                "hookSpecificOutput",
                obj(vec![
                    ("hookEventName", s("PreToolUse")),
                    ("additionalContext", s(message)),
                ]),
            ),
        ]));
    }

    fn request(&self, event: &str, tool: Option<&str>, cwd: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "event": event, "tool": tool, "paths": [], "content": null,
            "actor": "agent", "cwd": cwd, "project_dir": self.project,
        })
    }

    /// The decision without an engine verdict, from the inputs the engine would
    /// have read: the edited paths placed as it places them and every
    /// configuration that governs them.
    fn policy(&self, cwd: Option<&str>, paths: &[String]) -> Unjudged {
        let here = std::env::current_dir().unwrap_or_default();
        let base = cwd.map(PathBuf::from).unwrap_or(here);
        unjudged_policy(&base, self.project.as_deref(), paths)
    }

    /// An edit the translator could not judge: the engine says whether the
    /// configuration refuses it; the engine being unrunnable falls back to the
    /// config text. A call that is not `loud` says nothing unless refused.
    fn unjudged_result(
        &mut self,
        message: &str,
        cwd: Option<&str>,
        paths: Vec<String>,
        engine_failed: bool,
        loud: bool,
    ) -> Result1 {
        if !engine_failed {
            let mut request = self.request("edit", None, cwd);
            request["paths"] = serde_json::json!(paths);
            request["adapter_error"] = serde_json::json!(text::reason_of(message));
            let known = |v: &serde_json::Value| {
                let k = vtext(v, "verdict");
                k == "block" || QUIET.contains(&k) || ALLOWED_NOTES.contains(&k)
            };
            if let Some(verdict) = engine(&self.event, &request, cwd).ok().filter(known) {
                // The engine allowed it with nothing to say (not governed, or a
                // guardrail the configuration turned off).
                if QUIET.contains(&vtext(&verdict, "verdict")) {
                    return (Kind::Allow, String::new());
                }
                if vtext(&verdict, "verdict") == "block" {
                    let m = verdict
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or(message);
                    return (Kind::Block, m.to_string());
                }
                if !loud {
                    return (Kind::Allow, String::new());
                }
                let m = vtext(&verdict, "message");
                return (
                    Kind::Stated,
                    if m.is_empty() { message } else { m }.to_string(),
                );
            }
        }
        match self.policy(cwd, &paths) {
            Unjudged::NotGoverned => return (Kind::Allow, String::new()),
            Unjudged::Closed if loud => return (Kind::Block, format!("{message} {CLOSED}")),
            _ => {}
        }
        if loud {
            (Kind::Stated, message.to_string())
        } else {
            (Kind::Allow, String::new())
        }
    }

    /// One answer for the judgements: any block refuses the call, the other
    /// messages are shown.
    fn decide(&mut self, results: &[Result1]) {
        let pick = |k: Kind| -> Vec<&str> {
            results
                .iter()
                .filter(|(kind, m)| *kind == k && (k == Kind::Block || !m.is_empty()))
                .map(|(_, m)| m.as_str())
                .collect()
        };
        let blocks = pick(Kind::Block);
        if !blocks.is_empty() {
            return self.deny(&blocks.join("\n"));
        }
        let stated = pick(Kind::Stated);
        let notes = pick(Kind::Note);
        if !stated.is_empty() {
            let all: Vec<&str> = stated.into_iter().chain(notes).collect();
            return self.stated(&all.join("\n"), "edit");
        }
        if !notes.is_empty() {
            self.note(&notes.join("\n"));
        }
    }

    fn unjudged_edit(
        &mut self,
        message: &str,
        cwd: Option<&str>,
        paths: Vec<String>,
        failed: bool,
    ) {
        let r = self.unjudged_result(message, cwd, paths, failed, true);
        self.decide(&[r]);
    }

    fn engine_unrunnable(&mut self, cwd: Option<&str>, path: &str) -> Result1 {
        let message = format!("urzua harness: urzua did NOT judge this call (the engine command `{}` could not be run or answered with something that is not a verdict). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`.", self.argv());
        self.unjudged_result(message.as_str(), cwd, vec![path.to_string()], true, true)
    }
}

/// Whether `d` is a directory the engine can be run in.
fn usable_dir(d: &str) -> bool {
    if d.is_empty() || !Path::new(d).is_dir() {
        return false;
    }
    // Whether the directory can be entered is known only by entering it; the
    // process is the hook's own and goes back at once.
    let here = std::env::current_dir();
    let ok = std::env::set_current_dir(d).is_ok();
    if let Ok(h) = here {
        let _ = std::env::set_current_dir(h);
    }
    ok
}

pub(super) fn to_value(j: &J) -> serde_json::Value {
    serde_json::from_str(&j.compact()).unwrap_or(serde_json::Value::Null)
}

/// A JSON value as a message shows it: a string bare, `None`, `True` and `False` for null and booleans.
fn shown(j: Option<&J>) -> String {
    match j {
        None | Some(J::Null) => "None".into(),
        Some(J::Bool(true)) => "True".into(),
        Some(J::Bool(false)) => "False".into(),
        Some(J::Str(t)) => t.clone(),
        Some(J::Num(n)) => n.clone(),
        Some(other) => other.compact(),
    }
}

/// The truthiness the adapter gives a payload value: empty and zero are false.
fn j_truthy(j: Option<&J>) -> bool {
    match j {
        None | Some(J::Null) | Some(J::Bool(false)) => false,
        Some(J::Num(n)) => n.parse::<f64>().ok().is_none_or(|f| f != 0.0),
        Some(J::Str(t)) => !t.is_empty(),
        Some(J::Arr(a)) => !a.is_empty(),
        Some(J::Obj(o)) => !o.is_empty(),
        Some(J::Bool(true)) => true,
    }
}

fn read_text(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("cannot read {path}: {}", io_error_text(&e, path)))?;
    String::from_utf8(bytes).map_err(|e| format!("cannot read {path}: {e}"))
}

/// An I/O error as messages show it: `[Errno 2] No such file or directory: 'path'`.
fn io_error_text(e: &std::io::Error, path: &str) -> String {
    let text = e.to_string();
    let base = text.split(" (os error").next().unwrap_or(&text).to_string();
    match e.raw_os_error() {
        Some(n) => format!("[Errno {n}] {base}: '{path}'"),
        None => base,
    }
}

/// The notebook as it will be once NotebookEdit has run.
fn notebook_proposed(inp: &J, path: &str) -> Result<String, String> {
    let text = read_text(path).map_err(|e| e.replacen("cannot read", "cannot read notebook", 1))?;
    let mut nb = json::parse(&text).map_err(|e| format!("cannot read notebook {path}: {e}"))?;
    let Some(J::Arr(cells)) = nb.get_mut("cells") else {
        return Err(format!("{path} is not a notebook with cells"));
    };
    let mode = if j_truthy(inp.get("edit_mode")) {
        shown(inp.get("edit_mode"))
    } else {
        "replace".to_string()
    };
    let cell_id = inp.get("cell_id").filter(|c| !matches!(c, J::Null));
    let source = inp.get("new_source").cloned().unwrap_or(s(""));
    let mut index = None;
    if let Some(id) = cell_id {
        index = cells
            .iter()
            .position(|c| matches!(c, J::Obj(_)) && c.get("id") == Some(id));
        let shown = shown(Some(id));
        if index.is_none() && !shown.is_empty() && shown.chars().all(|c| c.is_ascii_digit()) {
            index = shown.parse::<usize>().ok().filter(|n| *n < cells.len());
        }
        if index.is_none() {
            return Err(format!("the cell {shown} is not in the notebook"));
        }
    }
    match mode.as_str() {
        "insert" => {
            let kind = if j_truthy(inp.get("cell_type")) {
                inp.get("cell_type").cloned().unwrap_or(s("code"))
            } else {
                s("code")
            };
            let mut cell = obj(vec![
                ("cell_type", kind.clone()),
                ("metadata", J::Obj(Vec::new())),
                ("source", source),
            ]);
            if kind == s("code") {
                cell.set("outputs", J::Arr(Vec::new()));
                cell.set("execution_count", J::Null);
            }
            cells.insert(index.map_or(0, |i| i + 1), cell);
        }
        "replace" | "delete" => {
            let i = index.ok_or("the edit names no cell")?;
            if mode == "delete" {
                cells.remove(i);
            } else {
                cells[i].set("source", source);
                if j_truthy(inp.get("cell_type")) {
                    cells[i].set(
                        "cell_type",
                        inp.get("cell_type").cloned().unwrap_or(J::Null),
                    );
                }
            }
        }
        other => return Err(format!("an unknown edit_mode '{other}'")),
    }
    Ok(dumps_indent1(&nb) + "\n")
}

/// The file's text once the edit tool has run.
fn proposed(tool: &str, inp: &J, path: &str) -> Result<J, String> {
    if tool == "NotebookEdit" {
        return notebook_proposed(inp, path).map(J::Str);
    }
    if tool == "Write" {
        return Ok(inp.get("content").cloned().unwrap_or(s("")));
    }
    let single = [inp.clone()];
    let edits: &[J] = if tool == "MultiEdit" {
        match inp.get("edits") {
            Some(J::Arr(list)) => list,
            _ => return Err(format!("{MALFORMED}the edits are not a list")),
        }
    } else {
        &single
    };
    let mut text = read_text(path)?;
    for e in edits {
        let field = |k: &str| match e.get(k) {
            None => Ok(String::new()),
            Some(J::Str(t)) => Ok(t.clone()),
            Some(_) => Err(format!("{MALFORMED}{k} is not a string")),
        };
        let (old, new) = (field("old_string")?, field("new_string")?);
        let all = j_truthy(e.get("replace_all"));
        let n = if old.is_empty() {
            0
        } else {
            text.matches(old.as_str()).count()
        };
        if n == 0 {
            return Err("an old_string is not in the file".into());
        }
        if n > 1 && !all {
            return Err("an old_string is not unique in the file".into());
        }
        text = if all {
            text.replace(old.as_str(), &new)
        } else {
            text.replacen(old.as_str(), &new, 1)
        };
    }
    Ok(J::Str(text))
}

pub fn translate(event: &str, payload: &[u8], project: Option<&Path>) -> String {
    let mut run = Run {
        event: event.to_string(),
        project: project.map(|p| p.to_string_lossy().to_string()),
        out: String::new(),
    };
    main(&mut run, payload);
    run.out
}

/// The output when the hook could not finish (a panic, the deadline): an edit
/// is refused, whatever the configuration, because nothing judged it and the
/// configuration was not read; the other events only say so.
pub fn fail_closed(event: &str, why: &str) -> String {
    let mut run = Run {
        event: event.to_string(),
        project: None,
        out: String::new(),
    };
    let message =
        format!("urzua harness: {why}, so this call was NOT judged. Run: urzua harness status");
    if event == "edit" {
        run.deny(&format!(
            "{message}. The edit is refused because nothing judged it."
        ));
    } else {
        run.stated(&message, event);
    }
    run.out
}

fn main(run: &mut Run, payload: &[u8]) {
    super::test_panic("translate");
    let event = run.event.clone();
    let parsed = std::str::from_utf8(payload)
        .map_err(|e| e.to_string())
        .and_then(json::parse);
    let payload = match parsed {
        Ok(p @ J::Obj(_)) => p,
        _ => {
            let message =
                "urzua harness: the hook payload was unreadable, so this call was NOT judged";
            if event == "edit" {
                let project = run.project.clone();
                return run.unjudged_edit(message, project.as_deref(), Vec::new(), false);
            }
            return run.stated(message, &event);
        }
    };
    // A working directory that is gone, is not a directory or cannot be entered
    // is replaced by the session's project.
    let cwd = match payload.get("cwd") {
        Some(J::Str(d)) if usable_dir(d) => Some(d.clone()),
        _ => run.project.clone(),
    };
    let cwd = cwd.as_deref();
    match event.as_str() {
        "session-start" => {
            let request = run.request("session-start", None, cwd);
            judge_simple(run, request, cwd)
        }
        "post-command" => {
            // Any Bash command: the engine reads git, never the command.
            if payload.get("tool_name").and_then(J::as_str) != Some("Bash") {
                return;
            }
            let request = run.request("post-command", Some("Bash"), cwd);
            judge_simple(run, request, cwd)
        }
        _ => edit(run, &payload, cwd),
    }
}

fn edit(run: &mut Run, payload: &J, cwd: Option<&str>) {
    let Some(tool) = payload.get("tool_name").and_then(J::as_str) else {
        return;
    };
    let empty = J::Obj(Vec::new());
    let inp = match payload.get("tool_input") {
        Some(v) if j_truthy(Some(v)) => v,
        _ => &empty,
    };
    if !matches!(inp, J::Obj(_)) {
        return;
    }
    if tool.starts_with("mcp__") {
        let caps = mcp::Caps::configured(run.project.as_deref().or(cwd));
        if let Some(target) = mcp::write(tool, inp, &caps) {
            mcp::judge(run, tool, target, cwd, &caps);
        }
        return;
    }
    let key = if tool == "NotebookEdit" {
        "notebook_path"
    } else {
        "file_path"
    };
    let raw = inp.get(key);
    if !EDIT_TOOLS.contains(&tool) || !j_truthy(raw) {
        return;
    }
    let path = shown(raw);
    let content = match raw {
        Some(J::Str(_)) => proposed(tool, inp, &path),
        _ => Err(format!("{MALFORMED}{key} is not a string")),
    };
    let content = match content {
        Ok(c) => c,
        Err(e) => {
            let message = format!("urzua harness: urzua did NOT judge this edit ({tool} of {path}: {e}). It is allowed unless the configuration refuses an edit that was not judged; run 'urzua check' before committing.");
            // A tool input of the wrong shape says nothing reliable about the
            // file, so the configuration alone decides, as for an unreadable
            // payload: refused wherever a guardrail refuses an unjudged edit.
            if e.starts_with(MALFORMED) {
                return run.unjudged_edit(&message, cwd, Vec::new(), true);
            }
            return run.unjudged_edit(&message, cwd, vec![path], false);
        }
    };
    let mut request = run.request("edit", Some(tool), cwd);
    request["paths"] = serde_json::json!([path]);
    request["content"] = to_value(&content);
    let verdict = match engine(&run.event, &request, cwd) {
        Ok(v) => v,
        Err(_) => {
            let message = format!("urzua harness: urzua did NOT judge this call (the engine command `{}` could not be run or answered with something that is not a verdict). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`.", run.argv());
            return run.unjudged_edit(&message, cwd, vec![path], true);
        }
    };
    let (kind, message) = (vtext(&verdict, "verdict"), vtext(&verdict, "message"));
    if kind == "block" {
        run.deny(message);
    } else if ALLOWED_NOTES.contains(&kind) {
        run.note(message);
    } else if !QUIET.contains(&kind) {
        let message = unknown_verdict(kind);
        run.unjudged_edit(&message, cwd, vec![path], true);
    }
}

/// An answer whose verdict word this build does not know is not an allow: it
/// is an edit nobody judged.
pub(super) fn unknown_verdict(kind: &str) -> String {
    format!("urzua harness: urzua did NOT judge this edit (the engine answered with the verdict '{kind}', which this build does not know). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`.")
}

/// session-start and post-command: one request, one report, never a decision.
fn judge_simple(run: &mut Run, request: serde_json::Value, cwd: Option<&str>) {
    let event = run.event.clone();
    let verdict = match engine(&event, &request, cwd) {
        Ok(v) => v,
        Err(_) => {
            if run.policy(cwd, &[]) == Unjudged::NotGoverned {
                return;
            }
            let message = format!("urzua harness: urzua did NOT judge this call (the engine command `{}` could not be run or answered with something that is not a verdict). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`.", run.argv());
            return run.stated(&message, &event);
        }
    };
    let (kind, message) = (vtext(&verdict, "verdict"), vtext(&verdict, "message"));
    if event == "session-start" {
        if kind != "allow" && !message.is_empty() {
            run.print(&obj(vec![
                ("systemMessage", s(message)),
                (
                    "hookSpecificOutput",
                    obj(vec![
                        ("hookEventName", s("SessionStart")),
                        ("additionalContext", s(message)),
                    ]),
                ),
            ]));
        }
        return;
    }
    if kind == "allow" {
        return;
    }
    if verdict.get("agent").is_some() || verdict.get("person").is_some() {
        let person = if truthy(verdict.get("person")) {
            vtext(&verdict, "person")
        } else {
            ""
        };
        let agent = if truthy(verdict.get("agent")) {
            vtext(&verdict, "agent")
        } else {
            ""
        };
        return run.relay_post_command(person, agent);
    }
    run.relay_post_command("", message);
}
