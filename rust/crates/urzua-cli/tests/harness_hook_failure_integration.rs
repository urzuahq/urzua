//! How the installed hook command fails: a panic in the binary, a binary that
//! is missing or too old, an answer it does not know, an engine that does not
//! answer in time. Each ends in one JSON document, and an edit nothing judged
//! is refused unless the configuration can be read with certainty to let it
//! through. Real git repositories, the installed command run through `sh`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n";

fn harness(guardrails: &str) -> String {
    format!("{TYPES}harness:\n  adapters: [claude-code]\n  guardrails:\n{guardrails}    editor-hook:\n      enabled: true\n{AUTHORITY}")
}

const DENYING: &str = "    protected-paths:\n      enabled: true\n";
const ALLOWING: &str = "    protected-paths:\n      enabled: true\n      on_unjudged: allow\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-hookfail-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("notes.txt"), "x\n").unwrap();
    dir
}

/// The edit hook command an install writes, taken from a scratch install so
/// every test runs the command the agent runs.
fn edit_command() -> String {
    let dir = repo("command", &harness(DENYING));
    let out = common::urzua()
        .args(["harness", "install"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let s: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap())
            .unwrap();
    s["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string()
}

fn payload(dir: &Path) -> String {
    serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join("notes.txt").to_string_lossy(), "content": "y\n"}}).to_string()
}

/// A `PATH` of symlinks to `tools`, plus `urzua` as given: this build, a stand-in
/// script, or none.
struct Path_ {
    dir: common::TempDir,
}

fn path_with(urzua: Option<&str>) -> Path_ {
    let dir = common::TempDir::new("urzua-hookfail-path");
    for tool in ["sh", "git", "awk", "dirname", "cat", "sleep"] {
        let found = std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .map(|d| d.join(tool))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{tool} must be installed"));
        std::os::unix::fs::symlink(found, dir.join(tool)).unwrap();
    }
    match urzua {
        Some("build") => {
            std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_urzua"), dir.join("urzua")).unwrap()
        }
        Some(script) => {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(dir.join("urzua"), format!("#!/bin/sh\n{script}\n")).unwrap();
            std::fs::set_permissions(dir.join("urzua"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        None => {}
    }
    Path_ { dir }
}

/// The edit command run in `dir` with `path`; stdout, which must be exactly one
/// JSON document or nothing.
fn run(cmd: &str, dir: &Path, path: &Path_, envs: &[(&str, &str)]) -> Option<serde_json::Value> {
    let mut c = Command::new("/bin/sh");
    c.args(["-c", cmd])
        .current_dir(dir)
        .env_clear()
        .env("PATH", path.dir.path())
        .env("CLAUDE_PROJECT_DIR", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        c.env(k, v);
    }
    let mut child = c.spawn().unwrap();
    let _ = child
        .stdin
        .take()
        .unwrap()
        .write_all(payload(dir).as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    if text.trim().is_empty() {
        return None;
    }
    let mut docs = serde_json::Deserializer::from_str(&text).into_iter::<serde_json::Value>();
    let first = docs
        .next()
        .unwrap()
        .unwrap_or_else(|e| panic!("{e}: {text}"));
    assert!(docs.next().is_none(), "more than one JSON document: {text}");
    Some(first)
}

fn denied(v: &Option<serde_json::Value>) -> bool {
    v.as_ref()
        .is_some_and(|v| v["hookSpecificOutput"]["permissionDecision"] == "deny")
}

#[test]
fn a_panic_in_the_hook_ends_in_one_json_document_that_denies() {
    let cmd = edit_command();
    // An allowing configuration: the refusal is the panic's, not the fallback's.
    let dir = repo("panic", &harness(ALLOWING));
    let path = path_with(Some("build"));
    for at in ["translate", "outer"] {
        let v = run(&cmd, &dir, &path, &[("URZUA_HOOK_PANIC_FOR_TESTS", at)]);
        assert!(denied(&v), "{at}: {v:?}");
        assert!(v.unwrap().to_string().contains("the hook failed"), "{at}");
    }
}

#[test]
fn a_binary_that_writes_half_an_answer_and_fails_leaves_only_the_fallback() {
    let cmd = edit_command();
    let dir = repo("half", &harness(DENYING));
    let path = path_with(Some("printf '{\"systemMessage\": \"partial\"}\\n'; exit 3"));
    let v = run(&cmd, &dir, &path, &[]);
    assert!(denied(&v), "{v:?}");
    assert!(!v.unwrap().to_string().contains("partial"));
}

/// Configurations the binary reads as refusing an unjudged edit (or cannot be
/// sure do not) that the fallback must refuse too.
const NOT_CERTAIN: &[(&str, &str)] = &[
    ("block", "    protected-paths:\n      enabled: true\n"),
    (
        "True",
        "    protected-paths:\n      enabled: True\n      on_unjudged: allow\n",
    ),
    ("TRUE", "    protected-paths:\n      enabled: TRUE\n"),
    (
        "quoted-key",
        "    \"protected-paths\":\n      enabled: true\n      on_unjudged: allow\n",
    ),
    (
        "single-quoted-key",
        "    'protected-paths': {enabled: true, on_unjudged: allow}\n",
    ),
    (
        "anchor-alias",
        "    x: &g {enabled: true}\n    protected-paths: *g\n",
    ),
    ("tag", "    protected-paths: !!map {enabled: true}\n"),
    (
        "flow-nested",
        "    nested: {protected-paths: {enabled: true}}\n",
    ),
    (
        "flow-on-later-lines",
        "    protected-paths: {\n      enabled: true}\n",
    ),
    (
        "comment-says-allow",
        "    protected-paths:\n      enabled: true # on_unjudged: allow\n",
    ),
    (
        "xallow",
        "    protected-paths: {enabled: true, on_unjudged: xallow}\n",
    ),
    (
        "value-on-next-line",
        "    protected-paths:\n      enabled:\n        true\n",
    ),
];

#[test]
fn without_a_working_binary_every_shape_the_awk_cannot_read_with_certainty_is_refused() {
    let cmd = edit_command();
    for (binary, label) in [
        (None, "absent"),
        (
            Some("echo 'error: unrecognized subcommand hook' >&2; exit 2"),
            "too old",
        ),
    ] {
        let path = path_with(binary);
        for (name, guardrails) in NOT_CERTAIN {
            let config = format!("{TYPES}harness:\n  adapters: [claude-code]\n  guardrails:\n{guardrails}{AUTHORITY}");
            let dir = repo(&format!("shape-{name}"), &config);
            let v = run(&cmd, &dir, &path, &[]);
            assert!(denied(&v), "{label} / {name}: {v:?}");
            assert!(
                v.unwrap().to_string().contains("installed or upgraded"),
                "{label} / {name}"
            );
        }
    }
}

#[test]
fn without_a_working_binary_a_configuration_read_with_certainty_as_open_lets_the_edit_through() {
    let cmd = edit_command();
    let path = path_with(None);
    for (name, guardrails) in [
        ("block", ALLOWING),
        ("inline", "    protected-paths: {enabled: true, on_unjudged: allow}  # open\n    acceptance: {enabled: false}\n"),
        ("off", "    protected-paths:\n      enabled: false\n"),
    ] {
        let dir = repo(&format!("open-{name}"), &harness(guardrails));
        let v = run(&cmd, &dir, &path, &[]);
        assert!(!denied(&v), "{name}: {v:?}");
        assert!(v.unwrap()["systemMessage"].as_str().unwrap().contains("could NOT run"), "{name}");
    }
}

#[test]
fn without_a_working_binary_a_parents_or_an_unreadable_configuration_is_refused() {
    let cmd = edit_command();
    let path = path_with(None);
    // The configuration is in a parent of the session's directory.
    let dir = repo("parent", &harness(DENYING));
    let sub: PathBuf = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    assert!(
        denied(&run(&cmd, &sub, &path, &[])),
        "a parent's configuration governs"
    );
    // A configuration that cannot be read.
    if common::permission_bits_are_enforced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = repo("unreadable", &harness(ALLOWING));
        let cfg = dir.join(".urzua/config.yaml");
        std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o000)).unwrap();
        let v = run(&cmd, &dir, &path, &[]);
        std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(denied(&v), "{v:?}");
    }
}

#[test]
fn an_answer_with_an_unknown_verdict_word_is_an_unjudged_edit_not_a_silent_allow() {
    let cmd = edit_command();
    let path = path_with(Some("build"));
    let engine = serde_json::json!([
        "sh",
        "-c",
        "cat >/dev/null; echo '{\"verdict\":\"maybe\",\"message\":\"\"}'"
    ])
    .to_string();
    let dir = repo("unknown-deny", &harness(DENYING));
    let v = run(
        &cmd,
        &dir,
        &path,
        &[("URZUA_HOOK_ENGINE_FOR_TESTS", &engine)],
    );
    assert!(denied(&v), "{v:?}");
    let dir = repo("unknown-allow", &harness(ALLOWING));
    let v = run(
        &cmd,
        &dir,
        &path,
        &[("URZUA_HOOK_ENGINE_FOR_TESTS", &engine)],
    );
    assert!(!denied(&v), "{v:?}");
    assert!(v.expect("not silent").to_string().contains("'maybe'"));
}

#[test]
fn an_engine_that_does_not_answer_before_the_deadline_ends_in_a_refusal() {
    let cmd = edit_command();
    let path = path_with(Some("build"));
    let engine = serde_json::json!(["sleep", "30"]).to_string();
    // Even where the configuration lets an unjudged edit through.
    let dir = repo("deadline", &harness(ALLOWING));
    let start = std::time::Instant::now();
    let v = run(
        &cmd,
        &dir,
        &path,
        &[
            ("URZUA_HOOK_ENGINE_FOR_TESTS", &engine),
            ("URZUA_HOOK_DEADLINE_FOR_TESTS", "1"),
        ],
    );
    assert!(
        start.elapsed() < std::time::Duration::from_secs(15),
        "{:?}",
        start.elapsed()
    );
    assert!(denied(&v), "{v:?}");
    assert!(v.unwrap().to_string().contains("deadline"));
}

const GATE: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {paths: [\".claude\"], by: maintainers}\n";

/// Configurations whose engine refuses an agent's write to `.claude`, each in
/// a shape a text scan misreads.
fn refusing_shapes() -> Vec<(&'static str, String)> {
    let h = |g: &str| format!("{TYPES}harness:\n  guardrails:\n{g}{GATE}");
    vec![
        ("True", h("    protected-paths: {enabled: True}\n")),
        ("TRUE", h("    protected-paths: {enabled: TRUE}\n")),
        ("quoted-key", h("    \"protected-paths\": {enabled: true}\n")),
        ("one-line-harness", format!("{TYPES}harness: {{guardrails: {{protected-paths: {{enabled: true}}}}}}\n{GATE}")),
        ("anchor-alias", format!("{TYPES}x: &g {{enabled: true}}\nharness:\n  guardrails:\n    protected-paths: *g\n{GATE}")),
        ("json", r#"{"schema_version": 2, "record_types": {"adr": {"dir": "docs/adr", "required_fields": ["Status"]}}, "harness": {"guardrails": {"protected-paths": {"enabled": true}}}, "authority": {"deciders": {"maintainers": {"logins": ["Alice"]}}, "gates": [{"paths": [".claude"], "by": "maintainers"}]}}"#.to_string()),
        ("trailing-comment", h("    protected-paths:\n      enabled: true # on_unjudged: allow false\n")),
        ("comment-line-in-block", h("    protected-paths:\n      # on_unjudged: allow\n      enabled: true\n")),
        ("quoted-allow-with-hash", h("    protected-paths:\n      enabled: true\n      on_unjudged: 'allow #x'\n")),
    ]
}

fn write_settings(dir: &Path) -> String {
    serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join(".claude/settings.json").to_string_lossy(), "content": "{}"}}).to_string()
}

const CRASHING: &str = r#"["sh", "-c", "kill -SEGV $$"]"#;

#[test]
fn with_the_engine_crashing_every_shape_the_engine_refuses_is_refused_by_the_hook() {
    for (name, config) in refusing_shapes() {
        let dir = repo(&format!("crash-{name}"), &config);
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        // The engine itself refuses the write under this configuration.
        let out = common::run_hook(&dir, "edit", &write_settings(&dir), None, Some(&dir), &[]);
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("\"deny\""),
            "{name}: the engine refuses: {out:?}"
        );
        let out = common::run_hook(
            &dir,
            "edit",
            &write_settings(&dir),
            Some(&["sh", "-c", "kill -SEGV $$"]),
            Some(&dir),
            &[],
        );
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.contains("\"permissionDecision\": \"deny\""),
            "{name}: {text}"
        );
        let _ = CRASHING;
    }
}

/// A repository that does not use urzua: no configuration in it or any parent.
fn ungoverned() -> common::TempDir {
    let dir = common::TempDir::new("urzua-hookfail-ungoverned");
    git(&dir, &["init", "-q"]);
    dir
}

#[test]
fn in_a_repository_with_no_configuration_the_fallback_allows_a_write_and_an_mcp_call_silently() {
    let cmd = edit_command();
    let dir = ungoverned();
    for (binary, label) in [
        (None, "absent"),
        (
            Some("echo 'unrecognized subcommand' >&2; exit 2"),
            "too old",
        ),
    ] {
        let path = path_with(binary);
        assert_eq!(
            run(&cmd, &dir, &path, &[]),
            None,
            "{label}: a Write says nothing"
        );
        let mcp = serde_json::json!({"tool_name": "mcp__fs__write_file", "cwd": dir.to_string_lossy(), "tool_input": {"path": dir.join("a.txt").to_string_lossy(), "content": "x"}}).to_string();
        let mut c = Command::new("/bin/sh");
        c.args(["-c", &cmd])
            .current_dir(&dir)
            .env_clear()
            .env("PATH", path.dir.path())
            .env("CLAUDE_PROJECT_DIR", dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let _ = child.stdin.take().unwrap().write_all(mcp.as_bytes());
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            (out.status.code(), out.stdout.is_empty()),
            (Some(0), true),
            "{label}: {out:?}"
        );
    }
}

#[test]
fn in_a_repository_with_no_configuration_the_binary_says_nothing_even_when_the_engine_crashes() {
    let dir = ungoverned();
    let write = serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join("a.txt").to_string_lossy(), "content": "x"}}).to_string();
    let bash = serde_json::json!({"tool_name": "Bash", "cwd": dir.to_string_lossy(), "tool_input": {"command": "true"}}).to_string();
    let start = serde_json::json!({"cwd": dir.to_string_lossy(), "session_id": "s"}).to_string();
    for (event, payload) in [
        ("edit", &write),
        ("post-command", &bash),
        ("session-start", &start),
    ] {
        let out = common::run_hook(&dir, event, payload, None, Some(&dir), &[]);
        assert!(
            out.stdout.is_empty(),
            "{event}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
    let out = common::run_hook(
        &dir,
        "edit",
        &write,
        Some(&["sh", "-c", "kill -SEGV $$"]),
        Some(&dir),
        &[],
    );
    assert!(
        out.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}
