//! Where the hook looks when the engine gives no verdict. It must decide from
//! the inputs the engine reads: every edited path placed as the engine places
//! it (relative to the request's directory, links resolved) and every
//! configuration that governs it, wherever the session started. Each row runs
//! the real engine first and records its decision, then runs the hook with the
//! engine crashing and asserts the hook is at least as strict.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

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

#[test]
fn an_unreadable_edit_payload_is_silent_where_nothing_governs_and_refused_where_a_config_is_closed()
{
    let w = world();
    let out = hook(&w.ung, &Project::Dir(&w.ung), "garbage{", None);
    assert!(out.is_empty(), "{out}");
    let out = hook(&w.gov, &Project::Dir(&w.gov), "garbage{", None);
    assert_eq!(rank(&out), 2, "{out}");
}

#[test]
fn an_unreadable_edit_payload_under_an_open_config_goes_through_with_a_notice() {
    let root = common::TempDir::new("urzua-placement-open-unreadable");
    let dir = root.join("o");
    repo(&dir, Some(&format!("{TYPES}harness:\n  guardrails:\n    protected-paths: {{enabled: true, on_unjudged: allow}}\n{GATE}")));
    let out = hook(&dir, &Project::Dir(&dir), "garbage{", None);
    assert_eq!(rank(&out), 1, "{out}");
    assert!(out.contains("NOT judged"), "{out}");
}

#[test]
fn the_session_environment_cannot_point_git_at_another_repository() {
    let w = world();
    let git_dir = w.ung.join(".git");
    let envs: Vec<(&str, &std::ffi::OsStr)> = vec![
        ("CLAUDE_PROJECT_DIR", std::ffi::OsStr::new("")),
        ("GIT_DIR", git_dir.as_os_str()),
        ("GIT_WORK_TREE", w.ung.as_os_str()),
    ];
    let out = common::run_hook(&w.gov, "edit", "garbage{", None, None, &envs);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let out = String::from_utf8(out.stdout).unwrap();
    assert_eq!(rank(&out), 2, "{out}");
}
