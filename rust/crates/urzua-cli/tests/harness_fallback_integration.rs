//! The installed hook command when `urzua` cannot run (absent from PATH, or a
//! release without `harness hook`). It decides from the session's directory
//! alone and reads no path from the payload: no configuration found is silent;
//! a configuration read with certainty as open lets the call through with a
//! notice; any other refuses every call the edit hook is registered for. Each
//! case runs under dash, bash and busybox sh, each with mawk, gawk and busybox
//! awk, where installed.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n";
const GATE: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {paths: [\".claude\"], by: maintainers}\n";

fn which(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// The three kinds of session: no configuration, an open one, a closed one.
fn session(kind: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fallback-{kind}"));
    git(&dir, &["init", "-q"]);
    let config = match kind {
        "open" => Some(format!("{TYPES}harness:\n  adapters: [claude-code]\n  guardrails:\n    protected-paths: {{enabled: true, on_unjudged: allow}}\n    editor-hook: {{enabled: false}}\n{GATE}")),
        "closed" => Some(format!("{TYPES}harness:\n  adapters: [claude-code]\n  guardrails:\n    protected-paths: {{enabled: true}}\n{GATE}")),
        _ => None,
    };
    if let Some(c) = config {
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(dir.join(".urzua/config.yaml"), c).unwrap();
    }
    dir
}

/// The installed commands by event, from a scratch install.
fn commands() -> std::collections::BTreeMap<String, String> {
    let dir = session("closed");
    let out = common::urzua()
        .args(["harness", "install"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let s: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap())
            .unwrap();
    let mut by = std::collections::BTreeMap::new();
    for (event, groups) in s["hooks"].as_object().unwrap() {
        let g = &groups[0];
        by.insert(
            event.clone(),
            g["hooks"][0]["command"].as_str().unwrap().to_string(),
        );
        if let Some(m) = g["matcher"].as_str() {
            by.insert(format!("{event}-matcher"), m.to_string());
        }
    }
    by
}

struct Combo {
    shell: Vec<PathBuf>,
    awk: PathBuf,
}

fn combos() -> Vec<Combo> {
    let mut shells: Vec<Vec<PathBuf>> = vec![vec![PathBuf::from("/bin/sh")]];
    shells.extend(which("bash").map(|b| vec![b]));
    shells.extend(which("busybox").map(|b| vec![b, PathBuf::from("sh")]));
    let mut awks: Vec<PathBuf> = ["mawk", "gawk"].iter().filter_map(|a| which(a)).collect();
    awks.extend(which("busybox"));
    let mut out = Vec::new();
    for s in &shells {
        for a in &awks {
            out.push(Combo {
                shell: s.clone(),
                awk: a.clone(),
            });
        }
    }
    out
}

/// `cmd` run as the host runs it, with no `urzua` on PATH (or `old` as a
/// release too old for `harness hook`); stdout, and the time it took.
fn run(
    cmd: &str,
    combo: &Combo,
    project: &str,
    cwd: &Path,
    payload: &str,
    old: bool,
) -> (String, std::time::Duration) {
    let tools = common::TempDir::new("urzua-fallback-path");
    for tool in ["dirname", "cat"] {
        std::os::unix::fs::symlink(which(tool).unwrap(), tools.join(tool)).unwrap();
    }
    std::os::unix::fs::symlink(&combo.awk, tools.join("awk")).unwrap();
    if old {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(
            tools.join("urzua"),
            "#!/bin/sh\necho 'unrecognized subcommand' >&2\nexit 2\n",
        )
        .unwrap();
        std::fs::set_permissions(tools.join("urzua"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let mut c = Command::new(&combo.shell[0]);
    c.args(&combo.shell[1..])
        .args(["-c", cmd])
        .current_dir(cwd)
        .env_clear()
        .env("PATH", tools.path())
        .env("CLAUDE_PROJECT_DIR", project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let start = std::time::Instant::now();
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    // A hook that never ends is one the host lets through: none may.
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(20) {
            let _ = child.kill();
            panic!("{:?} {:?} did not terminate", combo.shell, combo.awk);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    (String::from_utf8(out.stdout).unwrap(), start.elapsed())
}

fn denied(out: &str) -> bool {
    out.replace(": ", ":")
        .contains("\"permissionDecision\":\"deny\"")
}

fn calls(dir: &Path) -> Vec<(&'static str, &'static str, String)> {
    let d = dir.to_string_lossy().to_string();
    let p = |tool: &str, input: serde_json::Value| {
        serde_json::json!({"tool_name": tool, "cwd": d, "tool_input": input}).to_string()
    };
    vec![
        (
            "Write",
            "PreToolUse",
            p(
                "Write",
                serde_json::json!({"file_path": format!("{d}/a.txt"), "content": "x"}),
            ),
        ),
        (
            "Edit",
            "PreToolUse",
            p(
                "Edit",
                serde_json::json!({"file_path": format!("{d}/a.txt"), "old_string": "x", "new_string": "y"}),
            ),
        ),
        (
            "MultiEdit",
            "PreToolUse",
            p(
                "MultiEdit",
                serde_json::json!({"file_path": format!("{d}/a.txt"), "edits": []}),
            ),
        ),
        (
            "NotebookEdit",
            "PreToolUse",
            p(
                "NotebookEdit",
                serde_json::json!({"notebook_path": format!("{d}/n.ipynb"), "new_source": "x"}),
            ),
        ),
        (
            "an MCP call",
            "PreToolUse",
            p(
                "mcp__fs__write_file",
                serde_json::json!({"path": format!("{d}/a.txt"), "content": "x"}),
            ),
        ),
        (
            "Bash",
            "PostToolUse",
            p("Bash", serde_json::json!({"command": "true"})),
        ),
    ]
}

#[test]
fn without_urzua_the_hook_decides_from_the_session_alone_in_every_shell_and_awk() {
    let cmds = commands();
    // Read is no tool the hooks are registered for: the host never runs them for it.
    let matcher = regex_free_match(&cmds["PreToolUse-matcher"], "Read");
    assert!(!matcher, "{}", cmds["PreToolUse-matcher"]);
    for kind in ["none", "open", "closed"] {
        let dir = session(kind);
        for combo in combos() {
            for old in [false, true] {
                for (name, event, payload) in calls(&dir) {
                    let (out, _) = run(
                        &cmds[event],
                        &combo,
                        &dir.to_string_lossy(),
                        &dir,
                        &payload,
                        old,
                    );
                    let what = format!(
                        "{kind} {name} {:?} {:?} old={old}: {out}",
                        combo.shell, combo.awk
                    );
                    match (kind, event) {
                        ("none", _) => assert!(out.is_empty(), "{what}"),
                        ("open", _) => {
                            assert!(!denied(&out) && out.contains("could NOT run"), "{what}")
                        }
                        ("closed", "PreToolUse") => {
                            assert!(
                                denied(&out) && out.contains("every edit is refused"),
                                "{what}"
                            )
                        }
                        ("closed", _) => {
                            assert!(!denied(&out) && out.contains("could NOT run"), "{what}")
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

/// Whether the host's matcher, a `|`-separated list of tool names and `mcp__.*`,
/// covers `tool`.
fn regex_free_match(matcher: &str, tool: &str) -> bool {
    matcher
        .split('|')
        .any(|m| m == tool || (m.ends_with(".*") && tool.starts_with(&m[..m.len() - 2])))
}

#[test]
fn without_urzua_an_ordinary_mcp_call_where_nothing_is_governed_is_silent() {
    let cmds = commands();
    let dir = session("none");
    let d = dir.to_string_lossy().to_string();
    for input in [
        serde_json::json!({"path": "."}),
        serde_json::json!({"path": "./src"}),
        serde_json::json!({"url": "https://example.com/x", "path": "file:///etc/x"}),
    ] {
        let payload =
            serde_json::json!({"tool_name": "mcp__fs__list", "cwd": d, "tool_input": input})
                .to_string();
        let write = serde_json::json!({"tool_name": "Write", "cwd": d, "tool_input": {"file_path": "./src/main.rs", "content": "x"}}).to_string();
        for combo in combos() {
            for p in [&payload, &write] {
                let (out, _) = run(&cmds["PreToolUse"], &combo, &d, &dir, p, false);
                assert!(out.is_empty(), "{:?} {:?}: {out}", combo.shell, combo.awk);
            }
        }
    }
}

#[test]
fn without_urzua_the_walk_up_from_the_session_ends_on_double_slash_paths() {
    let cmds = commands();
    let dir = session("none");
    let payload = serde_json::json!({"tool_name": "Write", "cwd": "/", "tool_input": {"file_path": "/x", "content": "x"}}).to_string();
    // busybox `dirname //` is `//`; the walk must still end.
    for project in ["//", "//nonexistent//x", "///a//b"] {
        for combo in combos() {
            let (out, took) = run(&cmds["PreToolUse"], &combo, project, &dir, &payload, false);
            assert!(out.is_empty(), "{project} {:?}: {out}", combo.shell);
            assert!(
                took < std::time::Duration::from_secs(10),
                "{project} {:?}: {took:?}",
                combo.shell
            );
        }
    }
}
