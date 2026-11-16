//! `harness.limits` (ADR-101 amendment of 2026-11-15, SPEC-3): undeclared, every
//! limit is the built-in value; a raised value takes effect in the engine, the
//! installed hook entry and the translator; a value below a floor and an unknown
//! key stop the load. Real git repositories, the real binary and translator.
//! Needs `git`, `python3` and `sh`.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    statuses:\n      values: [Proposed, Accepted]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted], by: maintainers}\n    - {paths: [\".claude\"], by: maintainers}\n";
const TRANSLATOR: &str = "bundle/claude-code/files/.claude/hooks/urzua-claude-code.py";

fn config(limits: &str) -> String {
    format!(
        "{TYPES}harness:\n  adapters: [claude-code]\n{limits}  guardrails:\n    editor-hook: {{enabled: true}}\n    session-start: {{enabled: true}}\n    acceptance: {{enabled: true}}\n    protected-paths: {{enabled: true}}\n{AUTHORITY}"
    )
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-limits-{name}"));
    for d in ["docs/adr", ".urzua", ".claude"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value, Output) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code().unwrap(), v, out)
}

fn timeouts(dir: &Path) -> Vec<u64> {
    let s: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap())
            .unwrap();
    let mut out = Vec::new();
    for groups in s["hooks"].as_object().unwrap().values() {
        for group in groups.as_array().unwrap() {
            for hook in group["hooks"].as_array().unwrap() {
                if let Some(t) = hook["timeout"].as_u64() {
                    out.push(t);
                }
            }
        }
    }
    out
}

#[test]
fn a_config_declaring_no_limit_behaves_as_before() {
    let dir = fixture("none", &config(""));
    let (code, v, _) = urzua(&dir, &["harness", "limits"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["limits"],
        serde_json::json!({
            "hook_timeout_seconds": 120,
            "paths_per_call": 512,
            "records_per_call": 8,
            "loose_strings_per_call": 512,
            "input_depth": 24,
            "path_length": 4096,
            "string_length": 65536,
            "symlink_hops": 40,
            "json_depth": 128,
            "mcp_line_bytes": 1048576,
            "gate_walk_files": 50000,
        })
    );
    let (code, v, _) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(timeouts(&dir), [120]);
    let text = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    assert!(text.contains("\"timeout\": 120\n"), "{text}");
    assert!(!text.contains("limit:hook_timeout"), "{text}");
}

#[test]
fn a_raised_hook_timeout_is_installed_and_a_changed_one_is_drift_until_update() {
    let dir = fixture(
        "timeout",
        &config("  limits: {hook_timeout_seconds: 300}\n"),
    );
    let (code, v, _) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(timeouts(&dir), [300]);
    let (code, v, _) = urzua(&dir, &["harness", "status"]);
    assert_eq!(code, 0, "{v}");

    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config("  limits: {hook_timeout_seconds: 200}\n"),
    )
    .unwrap();
    let (code, v, _) = urzua(&dir, &["harness", "status"]);
    assert_eq!(code, 1, "{v}");
    let edit = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["key"].as_str().is_some_and(|k| k.ends_with("|edit")))
        .unwrap();
    assert_eq!(edit["state"], "out-of-date", "{v}");
    let (_, v, _) = urzua(&dir, &["doctor"]);
    assert!(
        v.to_string().contains("is out-of-date"),
        "doctor names the drift: {v}"
    );
    let start = {
        let mut child = common::urzua()
            .args(["harness", "run", "session-start"])
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let request = serde_json::json!({"event": "session-start", "cwd": dir.to_string_lossy()});
        let _ = child
            .stdin
            .take()
            .unwrap()
            .write_all(request.to_string().as_bytes());
        child.wait_with_output().unwrap()
    };
    let start = String::from_utf8_lossy(&start.stdout);
    assert!(
        start.contains("is out of date") && start.contains("hook_timeout_seconds"),
        "the session-start check names the drift: {start}"
    );
    let (code, v, _) = urzua(&dir, &["harness", "update"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(timeouts(&dir), [200]);
    let (code, v, _) = urzua(&dir, &["harness", "status"]);
    assert_eq!(code, 0, "{v}");
}

#[test]
fn a_limit_below_its_floor_or_an_unknown_limit_stops_the_load() {
    for (limits, needles) in [
        (
            "  limits: {paths_per_call: 100}\n",
            vec!["harness.limits.paths_per_call", "security floor of 512"],
        ),
        (
            "  limits: {mcp_line_bytes: 1024}\n",
            vec!["harness.limits.mcp_line_bytes", "security floor of 1048576"],
        ),
        (
            "  limits: {hook_timeout_seconds: 1}\n",
            vec!["harness.limits.hook_timeout_seconds", "from 10 to 600"],
        ),
        (
            "  limits: {path_lenght: 5000}\n",
            vec!["'path_lenght'", "did you mean 'path_length'?"],
        ),
    ] {
        let dir = fixture("floor", &config(limits));
        for args in [&["check"][..], &["harness", "limits"]] {
            let (code, _, out) = urzua(&dir, args);
            let text = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            assert_eq!(code, 2, "{limits} {args:?}: {text}");
            for n in &needles {
                assert!(text.contains(n), "{limits} {args:?}: {text}");
            }
        }
    }
}

/// The caps the translator holds after `load_limits`, with `bin` as the engine.
fn translator_caps(dir: &Path, bin: &Path) -> serde_json::Value {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSLATOR);
    let code = "import importlib.util,json,sys\nspec=importlib.util.spec_from_file_location('t',sys.argv[1])\nm=importlib.util.module_from_spec(spec)\nspec.loader.exec_module(m)\nm.load_limits(['urzua'],sys.argv[2])\nprint(json.dumps({k:getattr(m,k) for k in m.MCP_LIMIT_KEYS}))";
    let out = Command::new("python3")
        .args(["-I", "-B", "-c", code])
        .arg(&script)
        .arg(dir)
        .env("URZUA_BIN", bin)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

const FLOORS: &str = r#"{"MCP_MAX_DEPTH":24,"MCP_MAX_PATH":4096,"MCP_MAX_STRING":65536,"MCP_MAX_LOOSE":512,"MCP_MAX_RECORDS":8,"MCP_MAX_ITEMS":512}"#;

#[test]
fn the_translator_takes_each_raised_limit_from_urzua_and_never_goes_below_a_floor() {
    let raised = "  limits: {input_depth: 30, path_length: 5000, string_length: 70000, loose_strings_per_call: 600, records_per_call: 9, paths_per_call: 700}\n";
    let dir = fixture("translator", &config(raised));
    let bin = Path::new(env!("CARGO_BIN_EXE_urzua"));
    assert_eq!(
        translator_caps(&dir, bin),
        serde_json::json!({"MCP_MAX_DEPTH": 30, "MCP_MAX_PATH": 5000, "MCP_MAX_STRING": 70000, "MCP_MAX_LOOSE": 600, "MCP_MAX_RECORDS": 9, "MCP_MAX_ITEMS": 700})
    );
    let floors: serde_json::Value = serde_json::from_str(FLOORS).unwrap();
    let plain = fixture("translator-plain", &config(""));
    assert_eq!(translator_caps(&plain, bin), floors);

    // An engine that cannot answer, answers garbage or answers lower than a
    // floor leaves every floor.
    let fakes = common::TempDir::new("urzua-limits-fakes");
    for (name, body) in [
        ("garbage", "echo not json"),
        ("lower", "echo '{\"limits\": {\"input_depth\": 1, \"paths_per_call\": 0, \"records_per_call\": true}}'"),
        ("fails", "exit 2"),
    ] {
        let fake = fakes.join(name);
        std::fs::write(&fake, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(translator_caps(&dir, &fake), floors, "{name}");
    }
    assert_eq!(translator_caps(&dir, &fakes.join("absent")), floors);
}

fn translate(dir: &Path, tool: &str, input: serde_json::Value) -> serde_json::Value {
    let payload = serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": tool, "cwd": dir.to_string_lossy(), "tool_input": input}).to_string();
    let mut child = Command::new("python3")
        .arg("-I")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSLATOR))
        .args(["edit", "--", "urzua", "harness", "run", "edit"])
        .current_dir(dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
}

#[test]
fn a_raised_cap_judges_more_of_a_call_and_beyond_it_is_still_not_judged() {
    let mut deep = serde_json::json!({"path": "ok.txt", "content": "x"});
    for _ in 0..27 {
        deep = serde_json::json!({"wrap": deep});
    }
    let files: Vec<serde_json::Value> = (0..520)
        .map(|n| serde_json::json!({"path": format!("f{n}.txt"), "content": "x"}))
        .collect();
    let calls = [
        ("mcp__fs__write_file", deep),
        (
            "mcp__fs__write_files",
            serde_json::json!({ "files": files }),
        ),
    ];
    let unjudged = |v: &serde_json::Value| v.to_string().contains("did NOT judge");

    let plain = fixture("cap-plain", &config(""));
    for (tool, input) in &calls {
        let v = translate(&plain, tool, input.clone());
        assert!(unjudged(&v), "{tool} beyond the floor is not judged: {v}");
    }
    let raised = fixture(
        "cap-raised",
        &config("  limits: {input_depth: 40, paths_per_call: 600}\n"),
    );
    for (tool, input) in &calls {
        let v = translate(&raised, tool, input.clone());
        assert!(!unjudged(&v), "{tool} within the raised cap is judged: {v}");
    }
    let mut deeper = serde_json::json!({"path": "ok.txt", "content": "x"});
    for _ in 0..45 {
        deeper = serde_json::json!({"wrap": deeper});
    }
    let v = translate(&raised, "mcp__fs__write_file", deeper);
    assert!(
        unjudged(&v),
        "beyond the raised cap is still not judged: {v}"
    );
}
