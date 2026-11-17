//! `urzua harness hook claude-code <event>` does what the Python translator it
//! replaced did (`MILE-339`): every event shape the translator's tests fed is fed
//! to both, each in its own copy of the same repository, and their outputs are
//! compared byte for byte (the repository's path written as `<DIR>`). The
//! script is kept here, under `tests/fixtures/harness/`, for this comparison
//! only; nothing installs it. Needs `git`, `python3` and `sh`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";
const SCRIPT: &str = ".claude/hooks/urzua-claude-code.py";
const NOTEBOOK: &str = r#"{"cells": [{"id": "c1", "cell_type": "code", "source": "print(1)", "metadata": {}, "outputs": [], "execution_count": null}], "metadata": {}, "nbformat": 4, "nbformat_minor": 5}"#;

fn config(extra: &str) -> String {
    format!("{TYPES}harness:\n  guardrails:\n    acceptance: {{enabled: true{extra}}}\n    protected-paths: {{enabled: true{extra}}}\n    post-command: {{enabled: true}}\n    editor-hook: {{enabled: true}}\n    session-start: {{enabled: true}}\n{AUTHORITY}")
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A repository with the configuration, a record, a notebook, a note and the
/// script at the place it was installed (so its project directory is the
/// repository), committed.
fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-parity-{name}"));
    for d in ["docs/adr", ".urzua", ".claude/hooks"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    std::fs::write(dir.join("notes.txt"), "one\ntwo\ntwo\n").unwrap();
    std::fs::write(dir.join("n.ipynb"), NOTEBOOK).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/harness/urzua-claude-code.py"),
        dir.join(SCRIPT),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn feed(c: &mut Command, payload: &str) -> String {
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// What the script printed for `payload`, with `engine` after `--`.
fn script(dir: &Path, event: &str, payload: &str, engine: &[&str]) -> String {
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(dir.join(SCRIPT))
        .args([event, "--"])
        .args(engine)
        .current_dir(dir)
        .env("HOME", dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    feed(&mut c, payload)
}

/// What the binary printed for `payload`, with `engine` standing in when it is
/// not the binary's own.
fn binary(dir: &Path, event: &str, payload: &str, engine: &[&str]) -> String {
    let out = common::run_hook(
        dir,
        event,
        payload,
        common::engine_or_own(engine),
        Some(dir),
        &[("HOME", dir.as_os_str())],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// One case: a payload made for a repository, the event, the engine.
struct Case {
    name: &'static str,
    event: &'static str,
    engine: &'static [&'static str],
    payload: fn(&Path) -> String,
    /// Run in the repository first, the same in both copies.
    setup: Option<&'static str>,
}

fn tool(dir: &Path, tool: &str, input: serde_json::Value) -> String {
    serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": tool, "cwd": dir.to_string_lossy(), "tool_input": input, "session_id": "s", "tool_use_id": "t"}).to_string()
}

fn abs(dir: &Path, rel: &str) -> String {
    dir.join(rel).to_string_lossy().to_string()
}

const OWN: &[&str] = &["urzua", "harness", "run", "edit"];
const FAILS: &[&str] = &["false"];
const MISSING: &[&str] = &["urzua-no-such-binary"];

fn edit_cases() -> Vec<Case> {
    let c = |name, payload: fn(&Path) -> String| Case {
        name,
        event: "edit",
        engine: OWN,
        payload,
        setup: None,
    };
    vec![
        c("write-accepted", |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, PATH_1), "content": ACCEPTED}))),
        c("write-proposed", |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, PATH_1), "content": PROPOSED.replace("prose", "more prose")}))),
        c("write-protected", |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, ".claude/settings.json"), "content": "{}"}))),
        c("write-relative", |_| tool(Path::new("."), "Write", serde_json::json!({"file_path": PATH_1, "content": ACCEPTED}))),
        c("write-unicode", |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, "notes.txt"), "content": "é😀\u{1}"}))),
        c("edit-ok", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, "notes.txt"), "old_string": "one", "new_string": "uno"}))),
        c("edit-absent", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, "notes.txt"), "old_string": "absent", "new_string": "x"}))),
        c("edit-absent-protected", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, ".claude/settings.json"), "old_string": "absent", "new_string": "x"}))),
        c("edit-not-unique", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, "notes.txt"), "old_string": "two", "new_string": "x"}))),
        c("edit-replace-all", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, "notes.txt"), "old_string": "two", "new_string": "x", "replace_all": true}))),
        c("edit-missing-file", |d| tool(d, "Edit", serde_json::json!({"file_path": abs(d, "gone.txt"), "old_string": "a", "new_string": "b"}))),
        c("multiedit", |d| tool(d, "MultiEdit", serde_json::json!({"file_path": abs(d, PATH_1), "edits": [{"old_string": "Proposed", "new_string": "Accepted"}, {"old_string": "prose", "new_string": "p"}]}))),
        c("notebook-replace", |d| tool(d, "NotebookEdit", serde_json::json!({"notebook_path": abs(d, "n.ipynb"), "cell_id": "c1", "new_source": "print(2)", "edit_mode": "replace"}))),
        c("notebook-insert", |d| tool(d, "NotebookEdit", serde_json::json!({"notebook_path": abs(d, "n.ipynb"), "new_source": "x", "edit_mode": "insert", "cell_type": "markdown"}))),
        c("notebook-missing-cell", |d| tool(d, "NotebookEdit", serde_json::json!({"notebook_path": abs(d, "n.ipynb"), "cell_id": "zz", "new_source": "x"}))),
        c("read-tool", |d| tool(d, "Read", serde_json::json!({"file_path": abs(d, PATH_1)}))),
        c("no-path", |d| tool(d, "Write", serde_json::json!({"content": "x"}))),
        c("bad-cwd", |d| serde_json::json!({"tool_name": "Write", "cwd": "/nonexistent-urzua-cwd", "tool_input": {"file_path": abs(d, ".claude/settings.json"), "content": "{}"}}).to_string()),
        c("unreadable", |_| "garbage{".to_string()),
        c("not-an-object", |_| "[1, 2]".to_string()),
    ]
}

fn mcp_cases() -> Vec<Case> {
    let c = |name, payload: fn(&Path) -> String| Case {
        name,
        event: "edit",
        engine: OWN,
        payload,
        setup: None,
    };
    vec![
        c("mcp-write-protected", |d| tool(d, "mcp__fs__write_file", serde_json::json!({"path": abs(d, ".claude/settings.json"), "content": "{}"}))),
        c("mcp-write-accepted", |d| tool(d, "mcp__fs__write_file", serde_json::json!({"path": abs(d, PATH_1), "content": ACCEPTED}))),
        c("mcp-write-ordinary", |d| tool(d, "mcp__fs__write_file", serde_json::json!({"path": abs(d, "notes.txt"), "content": "x"}))),
        c("mcp-read", |d| tool(d, "mcp__fs__read_file", serde_json::json!({"path": abs(d, ".claude/settings.json")}))),
        c("mcp-read-output", |d| tool(d, "mcp__x__fetch", serde_json::json!({"url": "https://example.com/a", "output_path": abs(d, ".claude/settings.json")}))),
        c("mcp-no-file", |_| tool(Path::new("."), "mcp__github__create_issue", serde_json::json!({"title": "t", "body": "b"}))),
        c("mcp-edit", |d| tool(d, "mcp__fs__edit_file", serde_json::json!({"path": abs(d, PATH_1), "edits": [{"oldText": "a", "newText": "b"}]}))),
        c("mcp-edit-ordinary", |d| tool(d, "mcp__fs__edit_file", serde_json::json!({"path": abs(d, "notes.txt"), "edits": [{"oldText": "a", "newText": "b"}]}))),
        c("mcp-move", |d| tool(d, "mcp__fs__move_file", serde_json::json!({"source": abs(d, "notes.txt"), "destination": abs(d, ".claude/x.json")}))),
        c("mcp-many-files", |d| tool(d, "mcp__x__write_files", serde_json::json!({"files": [{"path": abs(d, "a.txt"), "content": "x"}, {"path": abs(d, PATH_1), "content": PROPOSED}, {"path": abs(d, "b.txt"), "content": "y"}]}))),
        c("mcp-many-with-protected", |d| tool(d, "mcp__x__write_files", serde_json::json!({"files": [{"path": abs(d, "a.txt"), "content": "x"}, {"path": abs(d, ".claude/s.json"), "content": "y"}]}))),
        c("mcp-file-uri", |d| tool(d, "mcp__x__write_note", serde_json::json!({"uri": format!("FILE://{}", abs(d, ".claude/settings.json")), "content": "x"}))),
        c("mcp-url", |d| tool(d, "mcp__x__write_note", serde_json::json!({"uri": format!("https://example.com{}", abs(d, ".claude/settings.json")), "content": "x"}))),
        c("mcp-home", |_| tool(Path::new("."), "mcp__x__write_file", serde_json::json!({"path": "~/.claude/settings.json", "content": "{}"}))),
        c("mcp-keyed-path", |d| tool(d, "mcp__x__put", serde_json::json!({"files": {abs(d, ".claude/k.json"): "{}"}}))),
        c("mcp-loose", |d| tool(d, "mcp__x__run", serde_json::json!({"args": ["--to", abs(d, ".claude/settings.json")]}))),
        c("mcp-two-contents", |d| tool(d, "mcp__x__write_file", serde_json::json!({"path": abs(d, "notes.txt"), "content": "a", "text": "b"}))),
        c("mcp-nul-and-spaces", |d| tool(d, "mcp__x__write_file", serde_json::json!({"path": format!("  {}\u{0}x ", abs(d, ".claude/settings.json")), "content": "{}"}))),
        c("mcp-deep", |d| {
            let mut v = serde_json::json!({"path": abs(d, "notes.txt"), "content": "x"});
            for _ in 0..30 {
                v = serde_json::json!({"wrap": v});
            }
            tool(d, "mcp__fs__write_file", v)
        }),
        c("mcp-too-many-strings", |_| {
            let loose: Vec<String> = (0..600).map(|n| format!("s{n}.txt")).collect();
            tool(Path::new("."), "mcp__x__run", serde_json::json!({"args": loose}))
        }),
        c("mcp-too-many-records", |d| {
            let files: Vec<serde_json::Value> = (2..12)
                .map(|n| serde_json::json!({"path": abs(d, &format!("docs/adr/ADR-{n}-x.md")), "content": PROPOSED}))
                .collect();
            tool(d, "mcp__x__write_files", serde_json::json!({"files": files}))
        }),
    ]
}

fn post(d: &Path, tool: &str) -> String {
    serde_json::json!({"hook_event_name": "PostToolUse", "tool_name": tool, "cwd": d.to_string_lossy(), "tool_input": {"command": "true"}, "tool_response": {"stdout": ""}}).to_string()
}

fn other_cases() -> Vec<Case> {
    vec![
        Case { name: "post-clean", event: "post-command", engine: &["urzua", "harness", "run", "post-command"], payload: |d| post(d, "Bash"), setup: None },
        Case { name: "post-other-tool", event: "post-command", engine: &["urzua", "harness", "run", "post-command"], payload: |d| post(d, "Read"), setup: Some("echo x >> docs/adr/ADR-1-a.md") },
        Case { name: "post-protected", event: "post-command", engine: &["urzua", "harness", "run", "post-command"], payload: |d| post(d, "Bash"), setup: Some("echo '# x' >> .urzua/config.yaml") },
        Case { name: "post-record", event: "post-command", engine: &["urzua", "harness", "run", "post-command"], payload: |d| post(d, "Bash"), setup: Some("sed -i 's/Proposed/Accepted/' docs/adr/ADR-1-a.md") },
        Case { name: "post-fails", event: "post-command", engine: FAILS, payload: |d| post(d, "Bash"), setup: None },
        Case { name: "post-unreadable", event: "post-command", engine: &["urzua", "harness", "run", "post-command"], payload: |_| "{".to_string(), setup: None },
        Case { name: "session", event: "session-start", engine: &["urzua", "harness", "run", "session-start"], payload: |d| serde_json::json!({"hook_event_name": "SessionStart", "cwd": d.to_string_lossy(), "session_id": "s"}).to_string(), setup: None },
        Case { name: "session-fails", event: "session-start", engine: FAILS, payload: |d| serde_json::json!({"hook_event_name": "SessionStart", "cwd": d.to_string_lossy()}).to_string(), setup: None },
        Case { name: "edit-engine-fails-protected", event: "edit", engine: FAILS, payload: |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, ".claude/settings.json"), "content": "{}"})), setup: None },
        Case { name: "edit-engine-missing-ordinary", event: "edit", engine: MISSING, payload: |d| tool(d, "Write", serde_json::json!({"file_path": abs(d, "notes.txt"), "content": "x"})), setup: None },
        Case { name: "mcp-engine-fails", event: "edit", engine: FAILS, payload: |d| tool(d, "mcp__fs__write_file", serde_json::json!({"path": abs(d, ".claude/settings.json"), "content": "{}"})), setup: None },
    ]
}

fn sh(dir: &Path, script: &str) {
    let status = Command::new("sh")
        .args(["-c", script])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "{script}");
}

fn compare(config_name: &str, config: &str, cases: Vec<Case>) {
    let mut differ = Vec::new();
    let (total, mut spoke, mut denied) = (cases.len(), 0, 0);
    for case in cases {
        let (a, b) = (
            fixture(&format!("{config_name}-{}-py", case.name), config),
            fixture(&format!("{config_name}-{}-rs", case.name), config),
        );
        if let Some(s) = case.setup {
            sh(&a, s);
            sh(&b, s);
        }
        let engine: Vec<&str> = if case.engine == OWN {
            vec!["urzua", "harness", "run", case.event]
        } else {
            case.engine.to_vec()
        };
        let from_script = script(&a, case.event, &(case.payload)(&a), &engine);
        let from_binary = binary(&b, case.event, &(case.payload)(&b), &engine);
        let norm = |text: &str, dir: &Path| text.replace(&dir.to_string_lossy().to_string(), "<DIR>");
        let (s, r) = (norm(&from_script, &a), norm(&from_binary, &b));
        spoke += usize::from(!s.is_empty());
        denied += usize::from(s.contains("\"permissionDecision\": \"deny\""));
        if s != r {
            differ.push(format!("{config_name}/{}:\n  script: {s}\n  binary: {r}", case.name));
        }
    }
    assert!(differ.is_empty(), "{} case(s) differ:\n{}", differ.len(), differ.join("\n"));
    // The comparison is not of silence: most cases say something, some refuse.
    assert!(spoke * 2 >= total && (denied > 0 || config_name == "allow"), "{config_name}: {spoke} of {total} spoke, {denied} denied");
}

#[test]
fn every_edit_tool_shape_gives_the_scripts_output_byte_for_byte() {
    compare("deny", &config(""), edit_cases());
    compare("allow", &config(", on_unjudged: allow"), edit_cases());
}

#[test]
fn every_mcp_shape_gives_the_scripts_output_byte_for_byte() {
    compare("deny", &config(""), mcp_cases());
    compare("allow", &config(", on_unjudged: allow"), mcp_cases());
}

#[test]
fn post_command_session_start_and_a_failed_engine_give_the_scripts_output_byte_for_byte() {
    compare("deny", &config(""), other_cases());
    compare("allow", &config(", on_unjudged: allow"), other_cases());
}

#[test]
fn the_script_is_kept_only_as_a_test_fixture() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(manifest.join("tests/fixtures/harness/urzua-claude-code.py").is_file());
    let bundle: PathBuf = manifest.join("bundle");
    let mut stack = vec![bundle];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                assert!(!["py", "sh"].contains(&ext), "{} is executable code in the bundle", p.display());
            }
        }
    }
}
