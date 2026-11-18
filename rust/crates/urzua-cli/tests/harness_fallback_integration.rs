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
    let (out, _, took) = run_env(cmd, combo, Some(project), cwd, payload, old, &[]);
    (out, took)
}

/// [`run`] with `CLAUDE_PROJECT_DIR` optional and extra variables; also stderr.
fn run_env(
    cmd: &str,
    combo: &Combo,
    project: Option<&str>,
    cwd: &Path,
    payload: &str,
    old: bool,
    envs: &[(&str, &str)],
) -> (String, String, std::time::Duration) {
    let tools = common::TempDir::new("urzua-fallback-path");
    for tool in ["dirname", "cat", "tr", "wc"] {
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
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = project {
        c.env("CLAUDE_PROJECT_DIR", p);
    }
    for (k, v) in envs {
        c.env(k, v);
    }
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
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        start.elapsed(),
    )
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

fn write_payload(dir: &Path) -> String {
    serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join(".claude/settings.json").to_string_lossy(), "content": "{}"}}).to_string()
}

/// A repository with `config` written as bytes (`None`: `.urzua/config.yaml` is
/// a directory).
fn repo_with(name: &str, config: Option<&[u8]>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fallback-shape-{name}"));
    git(&dir, &["init", "-q"]);
    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    match config {
        Some(c) => std::fs::write(dir.join(".urzua/config.yaml"), c).unwrap(),
        None => std::fs::create_dir_all(dir.join(".urzua/config.yaml")).unwrap(),
    }
    dir
}

fn ordinary_payload(dir: &Path) -> String {
    serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join("notes.txt").to_string_lossy(), "content": "x"}}).to_string()
}

/// What a shape must give: the binary's own unjudged decision, and the
/// fallback's.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Want {
    /// Both let the call through, the fallback with its notice.
    Open,
    /// Both refuse.
    Closed,
    /// The binary lets it through and the fallback refuses: a documented limit
    /// of reading YAML with awk, never the other way round.
    Stricter,
}

const OPEN_TAIL: &str = "harness:\n  guardrails:\n    editor-hook: {enabled: true}\n";
const CLOSED_TAIL: &str = "harness:\n  guardrails:\n    protected-paths: {enabled: true}\n";

fn shapes() -> Vec<(&'static str, Option<Vec<u8>>, Want)> {
    use Want::*;
    let g = |body: &str| format!("{TYPES}harness:\n  guardrails:\n{body}{GATE}").into_bytes();
    let o = |before: &str| format!("{before}{OPEN_TAIL}").into_bytes();
    let b = |s: &[&[u8]]| s.concat();
    let utf16: Vec<u8> = String::from_utf8(g("    protected-paths: {enabled: false}\n"))
        .unwrap()
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    let mut nul = g("    protected-paths: {enabled: false}\n");
    nul.insert(10, 0);
    let long_line = o(&format!(
        "x: [{}]\n",
        (0..22_000)
            .map(|n| format!("'p{n} (a: b) ; c'"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let deep_block: String = (0..150)
        .map(|n| format!("{}k{n}:\n", "  ".repeat(n)))
        .collect::<String>()
        + &"  ".repeat(150)
        + "x: 1\n";
    let placeholders: Vec<String> = (0..40)
        .map(|n| {
            format!("'Describe step {n} (e.g.: `run it`) \u{2014} then; check {{x}} [y], z #{n}'")
        })
        .collect();
    vec![
        // Refusals the earlier reviews established.
        ("an escaped key", Some(g("    \"protected\\x2dpaths\":\n      enabled: true\n")), Closed),
        ("a quoted key", Some(g("    \"protected-paths\":\n      enabled: true\n")), Closed),
        ("broken YAML", Some(format!("{TYPES}schema_version: [\nharness:\n  guardrails:\n    protected-paths: {{enabled: false}}\n{GATE}").into_bytes()), Closed),
        ("UTF-16", Some(utf16), Closed),
        ("a NUL byte", Some(nul), Closed),
        ("a tab in indentation", Some(format!("{TYPES}harness:\n\tguardrails:\n\t\tprotected-paths: {{enabled: false}}\n{GATE}").into_bytes()), Closed),
        ("a repeated key", Some(g("    protected-paths:\n      enabled: true\n      on_unjudged: deny\n      on_unjudged: allow\n")), Closed),
        ("a child deeper than its siblings", Some(g("    protected-paths:\n      enabled: false\n        on_unjudged: allow\n")), Closed),
        ("the configuration a directory", None, Closed),
        ("a second document", Some(format!("{TYPES}{OPEN_TAIL}---\n{CLOSED_TAIL}{GATE}").into_bytes()), Closed),
        ("a quoted scalar left open", Some(o("x: 'abc\n")), Closed),
        ("a quoted harness key", Some(format!("{TYPES}\"harness\":\n  guardrails:\n    protected-paths: {{enabled: true}}\n{GATE}").into_bytes()), Closed),
        ("the whole document a flow map", Some(b"{schema_version: 2, harness: {guardrails: {protected-paths: {enabled: true}}}}\n".to_vec()), Closed),
        ("a list item among the guardrails", Some(g("    - protected-paths: {enabled: true}\n")), Closed),
        ("an anchor inside harness", Some(g("    editor-hook: &e {enabled: true}\n")), Stricter),
        ("a complex harness key", Some(format!("{TYPES}? harness\n: guardrails: {{protected-paths: {{enabled: true}}}}\n{GATE}").into_bytes()), Closed),
        ("harness a list", Some(format!("{TYPES}harness:\n- guardrails\n{GATE}").into_bytes()), Closed),
        // A key with a blank before its colon is the same key.
        ("a space before the colon, closed", Some(format!("harness :\n  guardrails :\n    protected-paths : {{enabled: true}}\n").into_bytes()), Closed),
        ("a tab before the colon, closed", Some(b"harness:\n  guardrails:\n    protected-paths\t: {enabled: true}\n".to_vec()), Closed),
        ("a space before the colon, block form, closed", Some(b"harness:\n  guardrails:\n    protected-paths :\n      enabled: true\n".to_vec()), Closed),
        ("a space before the colon, open", Some(b"harness :\n  guardrails :\n    editor-hook : {enabled: true}\n".to_vec()), Open),
        // The root indentation is the first content line's.
        ("a whole document indented, closed", Some(b"  harness:\n    guardrails:\n      protected-paths: {enabled: true}\n".to_vec()), Closed),
        ("an indented document after ---, closed", Some(b"---\n  harness:\n    guardrails:\n      protected-paths: {enabled: true}\n".to_vec()), Closed),
        ("a whole document indented, open", Some(b"  harness:\n    guardrails:\n      editor-hook: {enabled: true}\n".to_vec()), Open),
        // Line breaks awk does not see.
        ("a lone carriage return", Some(b"a: 1\rharness:\r  guardrails:\r    protected-paths: {enabled: true}\r".to_vec()), Closed),
        ("a next-line character", Some(b(&[b"a: 1\xc2\x85", CLOSED_TAIL.as_bytes()])), Closed),
        ("a line separator", Some(b(&[b"a: 1\xe2\x80\xa8", CLOSED_TAIL.as_bytes()])), Closed),
        ("a paragraph separator", Some(b(&[b"a: 1\xe2\x80\xa9", CLOSED_TAIL.as_bytes()])), Closed),
        ("a byte order mark on a later line", Some(b(&[b"a: 1\n\xef\xbb\xbf", OPEN_TAIL.as_bytes()])), Closed),
        ("a doubled byte order mark", Some(b(&[b"\xef\xbb\xbf\xef\xbb\xbf", OPEN_TAIL.as_bytes()])), Stricter),
        // A tab in leading whitespace on a line the reader would skip.
        ("a tab before a comment", Some(o("a: 1\n\t# c\n")), Closed),
        ("a tab-only line", Some(o("a: 1\n\t\n")), Closed),
        ("a tab in block scalar indentation", Some(o("a: |\n  x\n\t y\n")), Closed),
        // A line under a one-line guardrail entry.
        ("a deeper line under a one-line guardrail", Some(b"harness:\n  guardrails:\n    editor-hook: {enabled: true}\n        protected-paths: {enabled: true}\n".to_vec()), Closed),
        // Files the engine cannot load, with guardrails that read as open.
        ("a byte that is not UTF-8", Some(b(&[b"a: \xff\n", OPEN_TAIL.as_bytes()])), Closed),
        ("a form feed", Some(o("a: x\x0cy\n")), Closed),
        ("a vertical tab", Some(o("a: x\x0by\n")), Closed),
        ("a delete character", Some(o("a: x\x7fy\n")), Closed),
        ("a start-of-heading character", Some(o("a: x\x01y\n")), Closed),
        ("a duplicate top-level key", Some(o("a: 1\na: 2\n")), Closed),
        ("a mapping value inside a plain value", Some(o("a: b: c\n")), Closed),
        ("a list item as a plain value", Some(o("a: - b\n")), Closed),
        ("a value running into the next key", Some(b"a: 1harness:\n  guardrails:\n    editor-hook: {enabled: true}\n".to_vec()), Closed),
        ("flow nested 200 deep", Some(o(&format!("a: {}{}\n", "[".repeat(200), "]".repeat(200)))), Closed),
        ("blocks nested 150 deep", Some(o(&deep_block)), Closed),
        ("guardrails indented past a sibling", Some(b"harness:\n  adapters: claude-code\n    guardrails:\n      editor-hook: {enabled: true}\n".to_vec()), Closed),
        ("a directive with no document start", Some(o("%YAML 1.2\n")), Closed),
        // The binary's own reading does not resolve merge keys, so it lets
        // this through; the fallback refuses any merge key at the root.
        ("a merge key at the root carrying harness", Some(b"base: &b\n  harness:\n    guardrails:\n      protected-paths: {enabled: true}\n<<: *b\n".to_vec()), Stricter),
        ("a 400 KB line", Some(long_line), Stricter),
        // Ordinary YAML that must stay open.
        ("acceptance asking in session, open", Some(g("    acceptance: {enabled: true, on_unjudged: allow, in_session: ask}\n")), Open),
        ("acceptance asking in session, block, open", Some(g("    acceptance:\n      enabled: true\n      on_unjudged: allow\n      in_session: refuse\n")), Open),
        ("acceptance asking in session, closed", Some(g("    acceptance: {enabled: true, in_session: ask}\n")), Closed),
        ("an in-session value it does not know", Some(g("    acceptance: {enabled: true, on_unjudged: allow, in_session: maybe}\n")), Stricter),
        ("in_session under another guardrail", Some(g("    protected-paths: {enabled: true, on_unjudged: allow, in_session: ask}\n")), Stricter),
        ("editor-hook with pending_pair, one line", Some(g("    editor-hook: {enabled: true, on_unjudged: allow, pending_pair: allow}\n")), Open),
        ("editor-hook with pending_pair, block", Some(g("    editor-hook:\n      enabled: true\n      on_unjudged: allow\n      pending_pair: allow\n")), Open),
        ("a double-quoted allow", Some(g("    protected-paths:\n      enabled: true\n      on_unjudged: \"allow\"\n")), Open),
        ("a single-quoted allow", Some(g("    protected-paths: {enabled: true, on_unjudged: 'allow'}\n")), Open),
        ("a guardrail's name in a value", Some(format!("description: Records the acceptance of a decision\n{TYPES}harness:\n  guardrails:\n    session-start: {{enabled: true}}\n{GATE}").into_bytes()), Open),
        ("a flow map with a single-quoted value in a list item", Some(o("x:\n  - {a: 'b', c: d}\n")), Open),
        ("a flow map with one single-quoted value in a list item", Some(o("x:\n  - {name: 'a'}\n")), Open),
        ("a section list item with a quoted name", Some(o("x:\n  - {name: 'TL;DR', shape: y-statement}\n")), Open),
        ("a flow map in a list item", Some(o("x:\n  - {a: b, c: d}\n")), Open),
        ("a flow map with a single-quoted value", Some(o("x: {a: 'b'}\n")), Open),
        ("a block list item with a quoted value", Some(o("x:\n  - name: 'TL;DR'\n")), Open),
        ("flow maps with double-quoted and unquoted values, nested in lists", Some(o("a:\n  b:\n    - c:\n        - {d: \"e\", f: g}\n        - {h: 'i', j: \"k\"}\n    - [l, 'm', \"n\"]\n")), Open),
        ("quoted strings holding punctuation", Some(o("a: 'x;y#z: {w} [v], u'\nb: \"x;y#z: {w} [v], u\"\nc: 'it''s'\nd: [ '}', \"]\", '{[' ]\n")), Open),
        ("a multi-line flow list", Some(o("a: [one,\n    'two: 2',\n    \"three, 3\"]\nb: {c: d,\n    e: 'f'}\n")), Open),
        ("a long inline list of quoted placeholders", Some(o(&format!("placeholders: [{}]\n", placeholders.join(", ")))), Open),
        ("a literal block scalar with guardrail-looking lines", Some(o("notes: |\n  harness:\n    guardrails:\n      protected-paths: {enabled: true}\n  it's [ { '\n")), Open),
        ("a folded block scalar with guardrail-looking lines", Some(o("notes: >-\n  acceptance: {enabled: true}\n  editor-hook:\n    on_unjudged: deny\n")), Open),
        ("comments holding quotes and brackets", Some(o("# it's a \"comment\" [with {brackets}\na: b # don't [ {\n")), Open),
        ("a plain scalar with an apostrophe", Some(o("a: it's plain\nb: x [y\n")), Open),
        ("a closing bracket aligned with its key", Some(o("placeholders: [\n  'a',\n]\n")), Open),
        ("a flow continued left of its key", Some(o("a:\n  b: [1,\n 2]\n")), Open),
        ("a multi-line quoted scalar", Some(o("description: \"one\n  two\"\n")), Open),
        ("an anchor and alias outside harness", Some(o("a: &x [1]\nb: *x\nc:\n  <<: {d: 1}\n")), Open),
        ("harness null", Some(b"harness: ~\n".to_vec()), Open),
        ("a document end", Some(o("a: 1\n").into_iter().chain(b"...\n".iter().copied()).collect()), Open),
        ("a YAML directive before ---", Some(b(&[b"%YAML 1.2\n---\n", OPEN_TAIL.as_bytes()])), Open),
        ("a quoted top-level key", Some(o("\"x\": 1\n'y': 2\n")), Open),
        ("a doubled quote before a colon", Some(o("a: 'x'' b: c'\n")), Open),
        ("an escaped double quote before a colon", Some(o("a: \"x\\\" b: c\"\n")), Open),
        ("a multi-line flow inside harness", Some(b"harness:\n  adapters: [claude-code,\n    codex]\n  guardrails:\n    editor-hook: {enabled: true}\n".to_vec()), Open),
    ]
}

fn seam_rank(dir: &Path) -> u8 {
    let out = common::run_hook(
        dir,
        "edit",
        &ordinary_payload(dir),
        Some(&["false"]),
        Some(dir),
        &[],
    );
    fallback_rank(&String::from_utf8(out.stdout).unwrap())
}

#[test]
fn without_urzua_every_shape_is_read_as_the_binary_reads_it_or_more_strictly() {
    let cmds = commands();
    let mut wrong = Vec::new();
    for (name, config, want) in shapes() {
        let dir = repo_with(&name.replace([' ', ',', '\''], "-"), config.as_deref());
        let binary = seam_rank(&dir);
        let (eb, ef) = match want {
            Want::Open => (1, 1),
            Want::Closed => (2, 2),
            Want::Stricter => (1, 2),
        };
        if binary != eb {
            wrong.push(format!(
                "{name}: the binary gives {binary}, the shape says {want:?}"
            ));
        }
        for combo in combos() {
            let payload = ordinary_payload(&dir);
            let (out, took) = run(
                &cmds["PreToolUse"],
                &combo,
                &dir.to_string_lossy(),
                &dir,
                &payload,
                false,
            );
            let fallback = fallback_rank(&out);
            assert!(fallback >= binary, "{name} {:?} {:?}: the fallback ({fallback}) is more permissive than the binary ({binary})", combo.shell, combo.awk);
            if fallback != ef {
                wrong.push(format!(
                    "{name} {:?} {:?}: fallback {fallback}, want {want:?}",
                    combo.shell, combo.awk
                ));
            }
            assert!(
                took < std::time::Duration::from_secs(5),
                "{name} {:?} {:?}: took {took:?}",
                combo.shell,
                combo.awk
            );
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn without_urzua_a_session_through_a_symlink_into_a_governed_subdirectory_is_refused() {
    let cmds = commands();
    let root = common::TempDir::new("urzua-fallback-link");
    let gov = root.join("gov");
    std::fs::create_dir_all(gov.join("sub")).unwrap();
    git(&gov, &["init", "-q"]);
    std::fs::create_dir_all(gov.join(".urzua")).unwrap();
    std::fs::write(
        gov.join(".urzua/config.yaml"),
        format!("{TYPES}harness:\n  guardrails:\n    protected-paths: {{enabled: true}}\n{GATE}"),
    )
    .unwrap();
    let link = root.join("link-gov-sub");
    std::os::unix::fs::symlink(gov.join("sub"), &link).unwrap();
    let l = link.to_string_lossy().to_string();
    for combo in combos() {
        // The project variable names the link.
        let (out, _, _) = run_env(
            &cmds["PreToolUse"],
            &combo,
            Some(&l),
            &link,
            &write_payload(&link),
            false,
            &[],
        );
        assert!(denied(&out), "project {:?}: {out}", combo.shell);
        // No project variable; the working directory is the link, logically.
        let (out, _, _) = run_env(
            &cmds["PreToolUse"],
            &combo,
            None,
            &link,
            &write_payload(&link),
            false,
            &[("PWD", &l)],
        );
        assert!(denied(&out), "pwd {:?}: {out}", combo.shell);
    }
}

#[test]
fn without_urzua_either_the_project_or_the_working_directory_finding_a_closed_config_refuses() {
    let cmds = commands();
    let none = session("none");
    let closed = session("closed");
    for combo in combos() {
        let (out, _, _) = run_env(
            &cmds["PreToolUse"],
            &combo,
            Some(&none.to_string_lossy()),
            &closed,
            &write_payload(&closed),
            false,
            &[],
        );
        assert!(
            denied(&out),
            "working directory closed {:?}: {out}",
            combo.shell
        );
        let (out, _, _) = run_env(
            &cmds["PreToolUse"],
            &combo,
            Some(&closed.to_string_lossy()),
            &none,
            &write_payload(&none),
            false,
            &[],
        );
        assert!(denied(&out), "project closed {:?}: {out}", combo.shell);
    }
}

#[test]
fn without_urzua_silence_is_stdout_and_stderr_and_post_command_says_one_short_line() {
    let cmds = commands();
    let none = session("none");
    let open = session("open");
    for combo in combos() {
        for event in ["PreToolUse", "PostToolUse", "SessionStart"] {
            let (out, err, _) = run_env(
                &cmds[event],
                &combo,
                Some(&none.to_string_lossy()),
                &none,
                &write_payload(&none),
                false,
                &[],
            );
            assert!(
                out.is_empty() && err.is_empty(),
                "{event} {:?}: out {out} err {err}",
                combo.shell
            );
        }
        let bash = serde_json::json!({"tool_name": "Bash", "cwd": open.to_string_lossy(), "tool_input": {"command": "true"}}).to_string();
        let (out, _, _) = run_env(
            &cmds["PostToolUse"],
            &combo,
            Some(&open.to_string_lossy()),
            &open,
            &bash,
            false,
            &[],
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("hookSpecificOutput").is_none() && out.lines().count() == 1,
            "{out}"
        );
    }
}

/// Every configuration people actually write that this repository holds: its
/// own, the acceptance corpora and fixtures, what `urzua init` writes for each
/// profile and in adopt mode, and the YAML examples in the reference docs that
/// load. On each, the fallback must give the binary's own unjudged answer.
fn real_configs() -> Vec<(String, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut out = vec![(
        ".urzua/config.yaml".to_string(),
        std::fs::read(root.join(".urzua/config.yaml")).unwrap(),
    )];
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")];
    let mut found = Vec::new();
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "yaml" || x == "yml") {
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                let config = text
                    .lines()
                    .any(|l| l.starts_with("schema_version:") || l.starts_with("record_types:"));
                if config {
                    found.push(p);
                }
            }
        }
    }
    found.sort();
    for p in found {
        let name = p
            .strip_prefix(&root)
            .unwrap_or(&p)
            .to_string_lossy()
            .to_string();
        out.push((name, std::fs::read(&p).unwrap()));
    }
    let help = common::urzua().args(["init", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&help.stdout).to_string();
    let profiles: Vec<String> = help
        .split("built-in profiles:")
        .nth(1)
        .and_then(|r| r.lines().next())
        .unwrap_or("")
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    assert!(
        !profiles.is_empty(),
        "the profiles `urzua init` offers: {help}"
    );
    let mut runs: Vec<(String, Vec<String>)> = profiles
        .iter()
        .map(|p| {
            (
                format!("init --types {p}"),
                vec!["--types".to_string(), p.clone()],
            )
        })
        .collect();
    runs.push((
        format!("init --types {}", profiles.join(",")),
        vec!["--types".to_string(), profiles.join(",")],
    ));
    runs.push(("init (adopt)".to_string(), Vec::new()));
    for (name, args) in runs {
        let dir = common::TempDir::new("urzua-fallback-init");
        git(&dir, &["init", "-q"]);
        if args.is_empty() {
            std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
            std::fs::write(
                dir.join("docs/adr/ADR-1-use-postgres.md"),
                "---\nStatus: Accepted\nDate: 2026-01-01\n---\n# ADR-1: Use postgres\n",
            )
            .unwrap();
            git(&dir, &["add", "-A"]);
        }
        let res = common::urzua()
            .arg("init")
            .args(&args)
            .current_dir(&dir)
            .output()
            .unwrap();
        let config = std::fs::read(dir.join(".urzua/config.yaml"))
            .unwrap_or_else(|_| panic!("{name} wrote a configuration: {res:?}"));
        out.push((name, config));
    }
    let mut docs: Vec<PathBuf> = std::fs::read_dir(root.join("docs/reference"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    docs.push(root.join("README.md"));
    docs.sort();
    for doc in docs {
        let text = std::fs::read_to_string(&doc).unwrap();
        let mut block: Option<String> = None;
        let mut indent = 0;
        let mut n = 0;
        for line in text.lines() {
            let t = line.trim_start();
            match &mut block {
                None if t == "```yaml" || t == "```yml" => {
                    indent = line.len() - t.len();
                    block = Some(String::new());
                }
                Some(b) if t.starts_with("```") => {
                    n += 1;
                    let parsed = yaml_serde::from_str::<yaml_serde::Value>(b);
                    if parsed.is_ok_and(|v| v.is_mapping()) {
                        let name = format!(
                            "{} yaml block {n}",
                            doc.file_name().unwrap().to_string_lossy()
                        );
                        out.push((name, b.clone().into_bytes()));
                    }
                    block = None;
                }
                Some(b) => {
                    b.push_str(line.get(indent..).unwrap_or(t));
                    b.push('\n');
                }
                None => {}
            }
        }
    }
    out
}

/// The fallback's answer for one configuration: 2 refuses, 1 lets the call
/// through with its notice, 0 is silent.
fn fallback_rank(out: &str) -> u8 {
    if denied(out) {
        2
    } else {
        u8::from(!out.trim().is_empty())
    }
}

/// Real configurations on which the fallback may refuse where the binary
/// would not, each with why. Empty: on valid configurations people write,
/// "not certain" is a bug.
const LEGITIMATELY_UNCERTAIN: &[(&str, &str)] = &[];

#[test]
fn without_urzua_every_real_configuration_is_read_as_the_binary_reads_it() {
    let cmds = commands();
    let configs = real_configs();
    assert!(
        configs.len() > 30,
        "the corpus found the configurations: {}",
        configs.len()
    );
    let mut disagreements = Vec::new();
    for (name, config) in &configs {
        let dir = repo_with("real", Some(config));
        let payload = ordinary_payload(&dir);
        // The binary's own decision where the engine gives no verdict.
        let out = common::run_hook(&dir, "edit", &payload, Some(&["false"]), Some(&dir), &[]);
        let binary = fallback_rank(&String::from_utf8(out.stdout).unwrap());
        for combo in combos() {
            let (out, _) = run(
                &cmds["PreToolUse"],
                &combo,
                &dir.to_string_lossy(),
                &dir,
                &payload,
                false,
            );
            let fallback = fallback_rank(&out);
            assert!(
                fallback >= binary,
                "{name} {:?} {:?}: the fallback ({fallback}) is more permissive than the binary ({binary})",
                combo.shell, combo.awk
            );
            let excused = LEGITIMATELY_UNCERTAIN.iter().any(|(n, _)| n == name);
            if fallback != binary && !excused {
                disagreements.push(format!(
                    "{name} {:?} {:?}: binary {binary}, fallback {fallback}",
                    combo.shell, combo.awk
                ));
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} configurations read: {}",
        configs.len(),
        disagreements.join("\n")
    );
    eprintln!("{} real configurations read alike", configs.len());
}
