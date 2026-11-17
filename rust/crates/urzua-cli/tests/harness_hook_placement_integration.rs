//! Where the hook looks when the engine gives no verdict. It must decide from
//! the inputs the engine reads: every edited path placed as the engine places
//! it (relative to the request's directory, links resolved) and every
//! configuration that governs it, wherever the session started. Each row runs
//! the real engine first and records its decision, then runs the hook with the
//! engine crashing and asserts the hook is at least as strict.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted]\n";
const GATE: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {paths: [\".claude\"], by: maintainers}\n";
const CRASH: &[&str] = &["sh", "-c", "kill -SEGV $$"];

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn repo(at: &Path, config: Option<&str>) {
    std::fs::create_dir_all(at.join(".claude")).unwrap();
    git(at, &["init", "-q"]);
    git(at, &["config", "user.email", "t@example.com"]);
    git(at, &["config", "user.name", "t"]);
    std::fs::write(at.join(".claude/settings.json"), "{}\n").unwrap();
    if let Some(c) = config {
        std::fs::create_dir_all(at.join(".urzua")).unwrap();
        std::fs::write(at.join(".urzua/config.yaml"), c).unwrap();
    }
    git(at, &["add", "-A"]);
    git(at, &["commit", "-q", "-m", "fixture"]);
}

/// `gov` refuses an unjudged edit of `.claude`; `open` protects `.claude` but
/// lets an unjudged edit through; `ung` uses no urzua. `ung/leaf` links to
/// gov's settings file, `ung/link` to `gov`; `wt` is a linked worktree of gov.
struct World {
    _root: common::TempDir,
    gov: PathBuf,
    open: PathBuf,
    ung: PathBuf,
    wt: PathBuf,
}

fn world() -> World {
    let root = common::TempDir::new("urzua-placement");
    let (gov, open, ung) = (root.join("gov"), root.join("open"), root.join("ung"));
    repo(
        &gov,
        Some(&format!(
            "{TYPES}harness:\n  guardrails:\n    protected-paths: {{enabled: true}}\n{GATE}"
        )),
    );
    repo(&open, Some(&format!("{TYPES}harness:\n  guardrails:\n    protected-paths: {{enabled: true, on_unjudged: allow}}\n{GATE}")));
    repo(&ung, None);
    std::os::unix::fs::symlink(gov.join(".claude/settings.json"), ung.join("leaf")).unwrap();
    std::os::unix::fs::symlink(&gov, ung.join("link")).unwrap();
    let wt = root.join("wt");
    git(&gov, &["worktree", "add", "-q", wt.to_str().unwrap()]);
    World {
        gov,
        open,
        ung,
        wt,
        _root: root,
    }
}

fn payload(cwd: &Path, file: &str) -> String {
    serde_json::json!({"tool_name": "Write", "cwd": cwd.to_string_lossy(), "tool_input": {"file_path": file, "content": "{}"}}).to_string()
}

/// How the session names its project.
enum Project<'a> {
    Dir(&'a Path),
    Unset,
    Missing,
}

fn hook(cwd: &Path, project: &Project, payload: &str, engine: Option<&[&str]>) -> String {
    let empty = std::ffi::OsString::new();
    let mut envs: Vec<(&str, &std::ffi::OsStr)> = Vec::new();
    let dir = match project {
        Project::Dir(d) => Some(*d),
        Project::Missing => None,
        Project::Unset => {
            envs.push(("CLAUDE_PROJECT_DIR", empty.as_os_str()));
            None
        }
    };
    let out = common::run_hook(cwd, "edit", payload, engine, dir, &envs);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn rank(out: &str) -> u8 {
    if out
        .replace(": ", ":")
        .contains("\"permissionDecision\":\"deny\"")
    {
        2
    } else {
        u8::from(!out.trim().is_empty())
    }
}

#[test]
fn where_the_engine_gives_no_verdict_the_hook_decides_from_the_engines_inputs() {
    let w = world();
    let gov_settings = w
        .gov
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    let rows: Vec<(&str, &Path, Project, String)> = vec![
        (
            "session in an ungoverned directory, absolute path",
            &w.ung,
            Project::Dir(&w.ung),
            gov_settings.clone(),
        ),
        (
            "session in an ungoverned directory, relative path",
            &w.ung,
            Project::Dir(&w.ung),
            "../gov/.claude/settings.json".into(),
        ),
        (
            "project variable unset",
            &w.ung,
            Project::Unset,
            gov_settings.clone(),
        ),
        (
            "project variable naming no directory",
            &w.ung,
            Project::Missing,
            gov_settings.clone(),
        ),
        (
            "worktree session editing the main checkout",
            &w.wt,
            Project::Dir(&w.wt),
            gov_settings.clone(),
        ),
        (
            "a symlink to a governed file",
            &w.ung,
            Project::Dir(&w.ung),
            w.ung.join("leaf").to_string_lossy().to_string(),
        ),
        (
            "a path through a symlinked directory and ..",
            &w.ung,
            Project::Dir(&w.ung),
            format!(
                "{}/link/../gov/.claude/settings.json",
                w.ung.to_string_lossy()
            ),
        ),
        (
            "an open session editing a closed repository",
            &w.open,
            Project::Dir(&w.open),
            gov_settings.clone(),
        ),
    ];
    let mcp = |cwd: &Path, tool: &str, input: serde_json::Value| {
        serde_json::json!({"tool_name": tool, "cwd": cwd.to_string_lossy(), "tool_input": input})
            .to_string()
    };
    let mut cases: Vec<(&str, &Path, Project, String)> = rows
        .into_iter()
        .map(|(n, c, pr, f)| (n, c, pr, payload(c, &f)))
        .collect();
    cases.extend([
        (
            "an MCP string under a key that names no path",
            &*w.ung,
            Project::Dir(&w.ung),
            mcp(
                &w.ung,
                "mcp__x__do",
                serde_json::json!({"thing": gov_settings}),
            ),
        ),
        (
            "the same string nested",
            &*w.ung,
            Project::Dir(&w.ung),
            mcp(
                &w.ung,
                "mcp__x__do",
                serde_json::json!({"o": {"thing": gov_settings}}),
            ),
        ),
        (
            "the same string relative",
            &*w.ung,
            Project::Dir(&w.ung),
            mcp(
                &w.ung,
                "mcp__x__do",
                serde_json::json!({"thing": "../gov/.claude/settings.json"}),
            ),
        ),
        (
            "a write tool's string under an unknown key",
            &*w.ung,
            Project::Dir(&w.ung),
            mcp(
                &w.ung,
                "mcp__fs__write_file",
                serde_json::json!({"args": {"p": gov_settings}}),
            ),
        ),
        (
            "an MCP string from a session inside the repository",
            &*w.gov,
            Project::Dir(&w.gov),
            mcp(
                &w.gov,
                "mcp__x__do",
                serde_json::json!({"thing": gov_settings}),
            ),
        ),
    ]);
    for (name, cwd, project, p) in &cases {
        let real = hook(cwd, project, p, None);
        assert_eq!(rank(&real), 2, "{name}: the engine refuses: {real}");
        let crashed = hook(cwd, project, p, Some(CRASH));
        assert!(
            rank(&crashed) >= rank(&real),
            "{name}: engine {real} hook {crashed}"
        );
    }
}

#[test]
fn an_editor_hook_set_to_deny_refuses_a_record_edit_when_the_engine_gives_no_verdict() {
    let root = common::TempDir::new("urzua-placement-editor");
    let dir = root.join("r");
    repo(&dir, Some(&format!("{TYPES}harness:\n  guardrails:\n    editor-hook: {{enabled: true, on_unjudged: deny}}\n")));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    let record = dir.join("docs/adr/ADR-1-a.md");
    let p = payload(&dir, &record.to_string_lossy());
    let out = hook(&dir, &Project::Dir(&dir), &p, Some(CRASH));
    assert_eq!(rank(&out), 2, "{out}");
}

#[test]
fn in_an_ungoverned_repository_failing_edits_and_reports_say_nothing() {
    let root = common::TempDir::new("urzua-placement-quiet");
    let dir = root.join("u");
    repo(&dir, None);
    std::fs::write(dir.join("notes.txt"), "one\n").unwrap();
    let d = dir.to_string_lossy().to_string();
    let edit = |input: serde_json::Value, tool: &str| {
        serde_json::json!({"tool_name": tool, "cwd": d, "tool_input": input}).to_string()
    };
    let cases = [
        (
            "edit of a missing file",
            edit(
                serde_json::json!({"file_path": format!("{d}/gone.txt"), "old_string": "a", "new_string": "b"}),
                "Edit",
            ),
        ),
        (
            "old_string not in the file",
            edit(
                serde_json::json!({"file_path": format!("{d}/notes.txt"), "old_string": "absent", "new_string": "b"}),
                "Edit",
            ),
        ),
        (
            "NotebookEdit of a file that is not a notebook",
            edit(
                serde_json::json!({"notebook_path": format!("{d}/notes.txt"), "cell_id": "c1", "new_source": "x"}),
                "NotebookEdit",
            ),
        ),
    ];
    for engine in [None, Some(CRASH)] {
        for (name, p) in &cases {
            let out = hook(&dir, &Project::Dir(&dir), p, engine);
            assert!(out.is_empty(), "{name} ({engine:?}): {out}");
        }
        for (event, p) in [
            ("post-command", serde_json::json!({"tool_name": "Bash", "cwd": d, "tool_input": {"command": "true"}}).to_string()),
            ("session-start", serde_json::json!({"cwd": d}).to_string()),
        ] {
            let out = common::run_hook(&dir, event, &p, engine, Some(&dir), &[]);
            assert!(out.stdout.is_empty(), "{event} ({engine:?}): {}", String::from_utf8_lossy(&out.stdout));
        }
    }
}

/// The installed edit command, from a scratch install.
fn edit_command() -> String {
    let root = common::TempDir::new("urzua-placement-cmd");
    let dir = root.join("c");
    repo(&dir, Some(&format!("{TYPES}harness:\n  adapters: [claude-code]\n  guardrails:\n    protected-paths: {{enabled: true}}\n{GATE}")));
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

/// The command run with no `urzua` on PATH, as from a session in `cwd`.
fn without_urzua(cmd: &str, cwd: &Path, payload: &str) -> String {
    let tools = common::TempDir::new("urzua-placement-path");
    for tool in ["sh", "git", "awk", "dirname", "cat", "grep", "sed", "pwd"] {
        if let Some(found) = std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .map(|d| d.join(tool))
            .find(|p| p.is_file())
        {
            std::os::unix::fs::symlink(found, tools.join(tool)).unwrap();
        }
    }
    let mut c = Command::new("/bin/sh");
    c.args(["-c", cmd])
        .current_dir(cwd)
        .env_clear()
        .env("PATH", tools.path())
        .env("CLAUDE_PROJECT_DIR", cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn without_urzua_an_edit_of_a_file_in_another_governed_repository_is_refused() {
    let cmd = edit_command();
    let w = world();
    let gov_settings = w
        .gov
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    for (name, cwd, file) in [
        ("from an ungoverned directory", &w.ung, gov_settings.clone()),
        ("from an open repository", &w.open, gov_settings.clone()),
        (
            "through a symlinked directory",
            &w.ung,
            format!("{}/link/.claude/settings.json", w.ung.to_string_lossy()),
        ),
        (
            "through a symlinked file",
            &w.ung,
            w.ung.join("leaf").to_string_lossy().to_string(),
        ),
    ] {
        let out = without_urzua(&cmd, cwd, &payload(cwd, &file));
        assert_eq!(rank(&out), 2, "{name}: {out}");
    }
    // A file in the ungoverned directory itself is allowed without a word.
    let out = without_urzua(
        &cmd,
        &w.ung,
        &payload(&w.ung, &w.ung.join("a.txt").to_string_lossy()),
    );
    assert!(out.is_empty(), "{out}");
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// The command run with no `urzua` on PATH, by `shell` (argv) and with `awk`
/// as the only awk.
fn without_urzua_using(
    cmd: &str,
    cwd: &Path,
    payload: &str,
    shell: &[PathBuf],
    awk: &Path,
) -> String {
    without_urzua_home(cmd, cwd, payload, shell, awk, None)
}

fn without_urzua_home(
    cmd: &str,
    cwd: &Path,
    payload: &str,
    shell: &[PathBuf],
    awk: &Path,
    home: Option<&Path>,
) -> String {
    let tools = common::TempDir::new("urzua-placement-shells");
    for tool in ["git", "dirname", "cat", "grep", "sed", "pwd", "tr", "wc"] {
        if let Some(found) = which(tool) {
            std::os::unix::fs::symlink(found, tools.join(tool)).unwrap();
        }
    }
    std::os::unix::fs::symlink(awk, tools.join("awk")).unwrap();
    let mut c = Command::new(&shell[0]);
    c.args(&shell[1..])
        .args(["-c", cmd])
        .current_dir(cwd)
        .env_clear()
        .env("PATH", tools.path())
        .env("CLAUDE_PROJECT_DIR", cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(h) = home {
        c.env("HOME", h);
    }
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn shells_and_awks() -> (Vec<Vec<PathBuf>>, Vec<PathBuf>) {
    let mut shells: Vec<Vec<PathBuf>> = vec![vec![PathBuf::from("/bin/sh")]];
    shells.extend(which("bash").map(|b| vec![b]));
    shells.extend(which("busybox").map(|b| vec![b, PathBuf::from("sh")]));
    let mut awks: Vec<PathBuf> = ["mawk", "gawk"].iter().filter_map(|a| which(a)).collect();
    awks.extend(which("busybox"));
    (shells, awks)
}

#[test]
fn without_urzua_a_value_that_is_not_a_plain_path_is_placed_as_the_engine_reads_it_or_refused() {
    let cmd = edit_command();
    let w = world();
    let root = w.ung.parent().unwrap().to_path_buf();
    let g = w
        .gov
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    let ung = w.ung.to_string_lossy().to_string();
    let mcp = |input: serde_json::Value| {
        serde_json::json!({"tool_name": "mcp__fs__write_file", "cwd": ung, "tool_input": input})
            .to_string()
    };
    let (head, tail) = g.split_at(g.len() / 2);
    let rows: Vec<(&str, String)> = vec![
        ("a file:// URL", mcp(serde_json::json!({"path": format!("file://{g}"), "content": "x"}))),
        ("a file: URL", mcp(serde_json::json!({"path": format!("file:{g}"), "content": "x"}))),
        ("an upper-case FILE:// URL", mcp(serde_json::json!({"path": format!("FILE://{g}"), "content": "x"}))),
        ("a leading space", mcp(serde_json::json!({"path": format!(" {g}"), "content": "x"}))),
        ("a ~ path", mcp(serde_json::json!({"path": "~/gov/.claude/settings.json", "content": "x"}))),
        ("a $HOME path", mcp(serde_json::json!({"path": "$HOME/gov/.claude/settings.json", "content": "x"}))),
        ("a ${HOME} path", mcp(serde_json::json!({"path": "${HOME}/gov/.claude/settings.json", "content": "x"}))),
        ("a list", mcp(serde_json::json!({"path": [g], "content": "x"}))),
        ("an object", mcp(serde_json::json!({"path": {"p": g}, "content": "x"}))),
        ("a trailing slash on a symlink to a file", payload(&w.ung, &format!("{ung}/leaf/"))),
        ("a raw line break inside the value", format!("{{\"tool_name\": \"mcp__fs__write_file\", \"cwd\": \"{ung}\", \"tool_input\": {{\"path\": \"{head}\n{tail}\", \"content\": \"x\"}}}}")),
    ];
    let home = [("HOME", root.as_os_str())];
    for (name, p) in &rows {
        let out = common::run_hook(&w.ung, "edit", p, None, Some(&w.ung), &home);
        let engine = String::from_utf8(out.stdout).unwrap();
        // A raw line break makes the payload JSON no host writes: the engine
        // reads two partial paths, neither of them protected, and lets the
        // call through. The fallback cannot read even that much, so it refuses.
        if *name == "a raw line break inside the value" {
            assert!(rank(&engine) < 2, "{name}: {engine}");
            continue;
        }
        assert_eq!(rank(&engine), 2, "{name}: the engine refuses: {engine}");
    }
    let (shells, awks) = shells_and_awks();
    for shell in &shells {
        for awk in &awks {
            for (name, p) in &rows {
                let out = without_urzua_home(&cmd, &w.ung, p, shell, awk, Some(&root));
                assert_eq!(rank(&out), 2, "{shell:?} {awk:?} {name}: {out}");
            }
            // The ordinary case stays quiet: an odd value naming a file where
            // nothing is governed, and a plain write.
            let quiet = mcp(serde_json::json!({"path": [format!("{ung}/a.txt")], "content": "x"}));
            assert_eq!(
                without_urzua_home(&cmd, &w.ung, &quiet, shell, awk, Some(&root)),
                "",
                "{shell:?} {awk:?}"
            );
            let plain = payload(&w.ung, &format!("{ung}/a.txt"));
            assert_eq!(
                without_urzua_home(&cmd, &w.ung, &plain, shell, awk, Some(&root)),
                "",
                "{shell:?} {awk:?}"
            );
        }
    }
}

#[test]
fn without_urzua_a_dot_dot_component_or_a_dangling_directory_link_is_refused_in_every_shell_and_awk(
) {
    let cmd = edit_command();
    let w = world();
    std::os::unix::fs::symlink(w.gov.join(".claude/newsub"), w.ung.join("dang")).unwrap();
    let ung = w.ung.to_string_lossy().to_string();
    let cases = [
        (
            "a .. after a missing directory",
            format!("{ung}/nonexist/../../gov/.claude/settings.json"),
        ),
        (
            "the same, relative",
            "nonexist/../../gov/.claude/settings.json".to_string(),
        ),
        ("a dangling directory link", format!("{ung}/dang/x.json")),
    ];
    let mut shells: Vec<Vec<PathBuf>> = vec![vec![PathBuf::from("/bin/sh")]];
    shells.extend(which("bash").map(|b| vec![b]));
    shells.extend(which("busybox").map(|b| vec![b, PathBuf::from("sh")]));
    let mut awks: Vec<PathBuf> = ["mawk", "gawk"].iter().filter_map(|a| which(a)).collect();
    awks.extend(which("busybox"));
    assert!(!awks.is_empty());
    for shell in &shells {
        for awk in &awks {
            for (name, file) in &cases {
                let out = without_urzua_using(&cmd, &w.ung, &payload(&w.ung, file), shell, awk);
                assert_eq!(rank(&out), 2, "{shell:?} {awk:?} {name}: {out}");
            }
        }
    }
}
