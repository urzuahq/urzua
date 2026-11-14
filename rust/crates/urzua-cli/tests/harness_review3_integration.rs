//! The third review of the harness (ADR-89): the settings wrapper outside the
//! project root, a dangling symlink, a nested repository, MCP path discovery,
//! the session-start comparison with the lock, the files the harness keeps for
//! itself, a hidden change to a record, and an over-deep settings file. Real git
//! repositories, the real binary, the real translator and the real settings
//! commands. Needs `git`, `python3`, `awk` and `sh`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const TYPES: &str = "schema_version: 2\nrules:\n  header.required-fields: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    extra_dirs: [\"docs/old\"]\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".github/workflows\", \".urzua/config.yaml\"], by: maintainers}\n";
const LOG: &str = "override_log: \".urzua/override-log.md\"\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

fn harness(extra: &str) -> String {
    format!("harness:\n  guardrails:\n    acceptance: {{enabled: true{extra}}}\n    protected-paths: {{enabled: true{extra}}}\n    post-command: {{enabled: true}}\n    editor-hook: {{enabled: true}}\n    session-start: {{enabled: true}}\n")
}

fn config(extra: &str) -> String {
    format!("{TYPES}{LOG}{}{AUTHORITY}", harness(extra))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-review3-{name}"));
    populate(&dir, config);
    dir
}

fn populate(dir: &Path, config: &str) {
    for d in ["docs/adr", ".urzua", ".claude", ".github/workflows"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    git(dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    std::fs::write(dir.join(".github/workflows/ci.yml"), "name: ci\n").unwrap();
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_with_stdin(c: &mut Command, stdin: &str) -> Output {
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    // A command that exits without reading its input (the shell fallback) closes
    // the pipe first; that is not a failure of the test.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn urzua() -> Command {
    common::urzua()
}

/// Installs the claude-code adapter into the fixture (the translator, the
/// settings commands, the lock).
fn install(dir: &Path) {
    let out = urzua()
        .args(["harness", "install", "--adapter", "claude-code"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

fn run_edit_from(
    process_cwd: &Path,
    request_cwd: &Path,
    path: &str,
    content: &str,
) -> (i32, serde_json::Value) {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path], "content": content,
        "actor": "agent", "cwd": request_cwd.to_string_lossy(),
    });
    let mut c = urzua();
    c.args(["harness", "run", "edit"]).current_dir(process_cwd);
    let out = run_with_stdin(&mut c, &request.to_string());
    (out.status.code().unwrap(), json(&out))
}

fn run_edit(dir: &Path, path: &Path, content: &str) -> (i32, serde_json::Value) {
    run_edit_from(dir, dir, &path.to_string_lossy(), content)
}

fn run_event(dir: &Path, event: &str) -> serde_json::Value {
    let request = serde_json::json!({
        "event": event, "tool": "Bash", "paths": [], "actor": "agent",
        "cwd": dir.to_string_lossy(),
    });
    let mut c = urzua();
    c.args(["harness", "run", event]).current_dir(dir);
    let out = run_with_stdin(&mut c, &request.to_string());
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    json(&out)
}

fn bash(dir: &Path, script: &str) {
    let status = Command::new("sh")
        .args(["-c", script])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "{script}");
}

fn which(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} must be installed"))
}

/// A directory holding only symlinks to the named programs, as a `PATH`.
fn path_with(names: &[&str]) -> common::TempDir {
    let dir = common::TempDir::new("urzua-review3-path");
    for name in names {
        let target = if *name == "urzua" {
            PathBuf::from(env!("CARGO_BIN_EXE_urzua"))
        } else {
            which(name)
        };
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
    }
    dir
}

fn msg(v: &serde_json::Value) -> String {
    v["message"].as_str().unwrap_or_default().to_string()
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

// ------------------------------------------------- B1. the settings wrapper

fn hook_command(dir: &Path, handler: &str) -> String {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap())
            .unwrap();
    v["hooks"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|groups| groups.as_array().unwrap())
        .flat_map(|g| g["hooks"].as_array().unwrap())
        .map(|h| h["command"].as_str().unwrap().to_string())
        .find(|c| c.contains(&format!("urzua harness run {handler}")))
        .unwrap_or_else(|| panic!("no {handler} hook command"))
}

fn write_payload(cwd: &Path, file: &Path, content: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse", "tool_name": "Write", "cwd": cwd.to_string_lossy(),
        "tool_input": {"file_path": file.to_string_lossy(), "content": content},
    })
    .to_string()
}

/// Runs the installed edit command with `cwd` as the hook's directory.
fn run_command(
    dir: &Path,
    cwd: &Path,
    project_dir: Option<&Path>,
    path: Option<&Path>,
    payload: &str,
) -> serde_json::Value {
    let mut c = Command::new("sh");
    c.args(["-c", &hook_command(dir, "edit")])
        .current_dir(cwd)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env_remove("CLAUDE_PROJECT_DIR");
    if let Some(p) = project_dir {
        c.env("CLAUDE_PROJECT_DIR", p);
    }
    if let Some(p) = path {
        c.env("PATH", p);
    }
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    json(&out)
}

fn decision(v: &serde_json::Value) -> Option<&str> {
    v["hookSpecificOutput"]["permissionDecision"].as_str()
}

#[test]
fn the_settings_command_finds_the_project_after_a_cd_into_a_nested_repository() {
    let dir = fixture("b1-nest", &config(""));
    install(&dir);
    std::fs::create_dir_all(dir.join("docs/nest")).unwrap();
    git(&dir.join("docs/nest"), &["init", "-q"]);
    let nest = dir.join("docs/nest");
    let target = dir.join(".claude/settings.json");
    let payload = write_payload(&nest, &target, "{}\n");
    // With the variable the agent sets, and without it (the nearest
    // configuration above the directory is the project's).
    for project in [Some(dir.path()), None] {
        let v = run_command(&dir, &nest, project, None, &payload);
        assert_eq!(decision(&v), Some("deny"), "{project:?}: {v}");
    }
}

#[test]
fn the_settings_command_refuses_when_it_cannot_run_and_the_configuration_says_deny() {
    let dir = fixture("b1-closed", &config(""));
    install(&dir);
    let target = dir.join(".claude/settings.json");
    let payload = write_payload(&dir, &target, "{}\n");
    let tools = path_with(&["sh", "git", "awk", "cat"]);
    // No python3: the shell fallback reads the configuration.
    let v = run_command(&dir, &dir, Some(&dir), Some(tools.path()), &payload);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    assert!(v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .contains("on_unjudged"));
    // The translator deleted: the same.
    std::fs::remove_file(dir.join(".claude/hooks/urzua-claude-code.py")).unwrap();
    let v = run_command(&dir, &dir, Some(&dir), None, &payload);
    assert_eq!(decision(&v), Some("deny"), "{v}");
}

#[test]
fn the_settings_command_only_warns_when_the_configuration_allows_an_unjudged_edit() {
    let dir = fixture("b1-open", &config(", on_unjudged: allow"));
    install(&dir);
    std::fs::remove_file(dir.join(".claude/hooks/urzua-claude-code.py")).unwrap();
    let payload = write_payload(&dir, &dir.join(".claude/settings.json"), "{}\n");
    let v = run_command(&dir, &dir, Some(&dir), None, &payload);
    assert_eq!(decision(&v), None, "{v}");
    assert!(v["systemMessage"]
        .as_str()
        .unwrap()
        .contains("could NOT run"));
}

// ------------------------------------------------- B2. a dangling symlink

#[cfg(unix)]
#[test]
fn a_write_through_a_dangling_symlink_is_judged_as_the_file_it_will_create() {
    let dir = fixture("b2", &config(""));
    // A new protected file, a new workflow, a new Accepted record.
    symlink("../.claude/hooks/evil.sh", &dir.join("docs/hook"));
    symlink("../.github/workflows/evil.yml", &dir.join("docs/wf"));
    symlink("adr/ADR-998-x.md", &dir.join("docs/rec"));
    symlink(
        &dir.join(".claude/local.json").to_string_lossy(),
        &dir.join("docs/abs"),
    );
    // A chain of dangling links, and one that loops.
    symlink("hop2", &dir.join("docs/hop1"));
    symlink("../.claude/chain.json", &dir.join("docs/hop2"));
    symlink("loop", &dir.join("docs/loop"));
    for link in ["hook", "wf", "abs", "hop1"] {
        let (code, v) = run_edit(&dir, &dir.join("docs").join(link), "#!/bin/sh\n");
        assert_eq!(code, 1, "{link}: {v}");
        assert_eq!(v["verdict"], "block", "{link}: {v}");
        assert!(msg(&v).contains("protected path"), "{link}: {v}");
    }
    let (code, v) = run_edit(&dir, &dir.join("docs/rec"), ACCEPTED);
    assert_eq!(code, 1, "{v}");
    assert!(msg(&v).contains("acceptance"), "{v}");
    // A link to a new ordinary file is not governed; a loop is not a panic.
    symlink("../notes.txt", &dir.join("docs/ok"));
    let (code, v) = run_edit(&dir, &dir.join("docs/ok"), "hi\n");
    assert_eq!(code, 0, "{v}");
    let (code, _) = run_edit(&dir, &dir.join("docs/loop"), "x\n");
    assert!(code == 0 || code == 1);
}

// ------------------------------------------------- B3. a nested repository

#[cfg(unix)]
#[test]
fn a_nested_repository_does_not_take_a_symlink_out_of_the_outer_gates() {
    let dir = fixture("b3", &config(""));
    std::fs::create_dir_all(dir.join("zz")).unwrap();
    git(&dir.join("zz"), &["init", "-q"]);
    symlink("../.claude/settings.json", &dir.join("zz/lnk"));
    symlink("../docs/adr/ADR-1-a.md", &dir.join("zz/rec"));
    // From the project root, and from a session that cd'd into the nested repo.
    for process_cwd in [dir.path().to_path_buf(), dir.join("zz")] {
        let (code, v) = run_edit_from(
            &process_cwd,
            &process_cwd,
            &dir.join("zz/lnk").to_string_lossy(),
            "{}\n",
        );
        assert_eq!(code, 1, "{process_cwd:?}: {v}");
        assert!(msg(&v).contains("protected path"), "{v}");
        let (code, v) = run_edit_from(
            &process_cwd,
            &process_cwd,
            &dir.join("zz/rec").to_string_lossy(),
            ACCEPTED,
        );
        assert_eq!(code, 1, "{process_cwd:?}: {v}");
    }
    // A relative path from the nested directory is the same file.
    let (code, v) = run_edit_from(&dir.join("zz"), &dir.join("zz"), "lnk", "{}\n");
    assert_eq!(code, 1, "{v}");
    // An ordinary file in the nested repository is not governed.
    let (code, v) = run_edit(&dir, &dir.join("zz/plain.txt"), "x\n");
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "not-governed", "{v}");
}

// ------------------------------------------------- S1. MCP path discovery

fn mcp(dir: &Path, tool: &str, input: serde_json::Value) -> String {
    serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": tool, "cwd": dir.to_string_lossy(), "tool_input": input}).to_string()
}

fn translate(dir: &Path, payload: &str) -> serde_json::Value {
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("bundle/claude-code/files/.claude/hooks/urzua-claude-code.py"),
        )
        .args(["edit", "--", "urzua", "harness", "run", "edit"])
        .current_dir(dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    json(&out)
}

#[test]
fn an_mcp_call_is_judged_on_every_path_it_names_wherever_it_names_it() {
    let dir = fixture("s1", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let uri = format!("file://{}", abs(".claude/settings.json"));
    let cases = [
        (
            "mcp__fs__remove",
            serde_json::json!({"from": abs(".claude/settings.json")}),
        ),
        (
            "mcp__fs__move_file",
            serde_json::json!({"source": abs(".claude/settings.json"), "destination": abs("moved.json")}),
        ),
        (
            "mcp__fs__move_file",
            serde_json::json!({"source": abs("notes.txt"), "destination": abs(".claude/settings.json")}),
        ),
        (
            "mcp__fs__write_files",
            serde_json::json!({"files": [{"path": abs("ok.txt"), "content": "a"}, {"path": abs(".claude/settings.json"), "content": "{}"}]}),
        ),
        (
            "mcp__fs__write_file",
            serde_json::json!({"target": {"path": abs(".claude/settings.json")}, "content": "{}"}),
        ),
        (
            "mcp__fs__write_file",
            serde_json::json!({"uri": uri, "content": "{}"}),
        ),
        (
            "mcp__fs__batch",
            serde_json::json!({"operations": [{"op": "rm", "paths": [abs(".claude/settings.json")]}]}),
        ),
        (
            "mcp__x__do",
            serde_json::json!({"a": {"b": {"c": [abs(PATH_1)]}}}),
        ),
    ];
    for (tool, input) in cases {
        let v = translate(&dir, &mcp(&dir, tool, input.clone()));
        assert_eq!(decision(&v), Some("deny"), "{tool} {input}: {v}");
    }
    // Reads, calls that name no governed file, and a write elsewhere say nothing.
    let silent = [
        (
            "mcp__fs__read_file",
            serde_json::json!({"path": abs(".claude/settings.json")}),
        ),
        (
            "mcp__fs__list_directory",
            serde_json::json!({"from": abs(".claude")}),
        ),
        (
            "mcp__fs__move_file",
            serde_json::json!({"source": abs("a.txt"), "destination": abs("b.txt")}),
        ),
        (
            "mcp__github__create_issue",
            serde_json::json!({"title": "t", "body": "b", "labels": ["x"]}),
        ),
        (
            "mcp__fs__write_files",
            serde_json::json!({"files": [{"path": abs("ok.txt"), "content": "a"}]}),
        ),
    ];
    for (tool, input) in silent {
        let v = translate(&dir, &mcp(&dir, tool, input.clone()));
        if tool == "mcp__fs__move_file" {
            // An unjudgeable move is stated, not refused.
            assert_eq!(decision(&v), None, "{tool}: {v}");
        } else {
            assert!(v.is_null(), "{tool} {input}: {v}");
        }
    }
    // A call nested deeper than the translator reads is an edit that was not
    // judged: refused under on_unjudged: deny.
    let mut deep = serde_json::json!({"path": abs("ok.txt"), "content": "x"});
    for _ in 0..40 {
        deep = serde_json::json!({"wrap": deep});
    }
    let v = translate(&dir, &mcp(&dir, "mcp__fs__write_file", deep.clone()));
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let open = fixture("s1-open", &config(", on_unjudged: allow"));
    let v = translate(&open, &mcp(&open, "mcp__fs__write_file", deep));
    assert_eq!(decision(&v), None, "{v}");
}

// ------------------------------------------------- S2. session start

fn problems(v: &serde_json::Value) -> Vec<(String, String)> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["check"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn session(dir: &Path) -> serde_json::Value {
    let ok = path_with(&["urzua", "python3", "git", "sh"]);
    let mut c = urzua();
    c.args(["harness", "run", "session-start"])
        .current_dir(dir)
        .env("PATH", ok.path())
        .env("CLAUDE_PROJECT_DIR", dir);
    let out = run_with_stdin(
        &mut c,
        &serde_json::json!({"event": "session-start", "cwd": dir.to_string_lossy()}).to_string(),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    json(&out)
}

fn wire_git_hooks(dir: &Path) {
    let hooks = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    for layer in ["pre-commit", "pre-push"] {
        let p = hooks.join(layer);
        std::fs::write(&p, "#!/bin/sh\nurzua check\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

#[test]
fn session_start_compares_the_installed_files_the_hook_entries_and_the_kill_switch_with_the_lock() {
    let dir = fixture("s2", &config(""));
    install(&dir);
    wire_git_hooks(&dir);
    let v = session(&dir);
    assert_eq!(v["verdict"], "allow", "installed and wired: {v}");
    let translator = dir.join(".claude/hooks/urzua-claude-code.py");
    let original = std::fs::read_to_string(&translator).unwrap();

    // A neutered translator.
    std::fs::write(&translator, "import sys\nsys.exit(0)\n").unwrap();
    let v = session(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let p = problems(&v);
    assert_eq!(p.len(), 1, "{v}");
    assert_eq!(p[0].0, "file");
    assert!(p[0].1.contains("urzua-claude-code.py"), "{v}");
    // A deleted one.
    std::fs::remove_file(&translator).unwrap();
    let v = session(&dir);
    assert!(problems(&v)[0].1.contains("is missing"), "{v}");
    std::fs::write(&translator, &original).unwrap();
    assert_eq!(session(&dir)["verdict"], "allow", "restored");

    // An edited hook entry, still present by handler.
    let settings = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    let edited = settings.replacen(
        "urzua harness run edit; c=$?;",
        "urzua harness run edit; c=0;",
        1,
    );
    assert_ne!(edited, settings, "the fixture's command changed shape");
    std::fs::write(dir.join(".claude/settings.json"), &edited).unwrap();
    let v = session(&dir);
    let p = problems(&v);
    assert_eq!(p.len(), 1, "{v}");
    assert_eq!(p[0].0, "hook-entry");
    assert!(p[0].1.contains("not the one the install wrote"), "{v}");
    std::fs::write(dir.join(".claude/settings.json"), &settings).unwrap();

    // disableAllHooks.
    let off = settings.replacen('{', "{\n  \"disableAllHooks\": true,", 1);
    std::fs::write(dir.join(".claude/settings.json"), off).unwrap();
    let v = session(&dir);
    let p = problems(&v);
    assert_eq!(p.len(), 1, "{v}");
    assert_eq!(p[0].0, "kill-switch");
    assert!(p[0].1.contains("disableAllHooks"), "{v}");
}

// ------------------------------------------------- S3. what the harness keeps

#[test]
fn the_edit_tools_cannot_write_what_the_harness_keeps_for_itself() {
    let dir = fixture("s3", &config(""));
    install(&dir);
    for rel in [
        ".urzua/cache/post-command.json",
        ".urzua/cache/protected-hashes.json",
        ".urzua/cache/post-command-head",
        ".urzua/harness.lock",
        ".urzua/bin/urzua",
        ".urzua/bin/other",
        ".urzua/override-log.md",
        ".git/hooks/pre-commit",
        ".git/config",
        ".URZUA/Cache/x",
    ] {
        let (code, v) = run_edit(&dir, &dir.join(rel), "x\n");
        assert_eq!(code, 1, "{rel}: {v}");
        assert!(msg(&v).contains("kept by urzua itself"), "{rel}: {v}");
    }
    // The override path still appends to the log: a blocked record edit with a
    // named override and a reason is let through and recorded.
    std::fs::write(dir.join(".urzua/override-log.md"), "").unwrap();
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [dir.join("docs/adr/ADR-2-b.md").to_string_lossy()],
        "content": "---\nOther: x\n---\n# ADR-2: T\n\nprose\n", "actor": "agent",
        "cwd": dir.to_string_lossy(),
    });
    let mut c = urzua();
    c.args(["harness", "run", "edit"])
        .current_dir(&dir)
        .env("URZUA_HOOK_OVERRIDE", "me")
        .env("URZUA_HOOK_REASON", "a reason");
    let out = run_with_stdin(&mut c, &request.to_string());
    let v = json(&out);
    assert_eq!(v["verdict"], "allow-override", "{v}");
    let log = std::fs::read_to_string(dir.join(".urzua/override-log.md")).unwrap();
    assert!(log.contains("| me |") && log.contains("a reason"), "{log}");
}

// ------------------------------------------------- S4. a hidden change

#[test]
fn a_record_moved_after_assume_unchanged_or_skip_worktree_is_still_reported() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let dir = fixture("s4", &config(""));
        run_event(&dir, "session-start");
        assert_eq!(run_event(&dir, "post-command")["verdict"], "allow");
        git(&dir, &["update-index", flag, PATH_1]);
        bash(
            &dir,
            &format!("sed -i 's/^Status: Proposed/Status: Accepted/' {PATH_1}"),
        );
        assert!(
            git(&dir, &["status", "--porcelain"]).trim().is_empty(),
            "{flag}: git shows nothing"
        );
        let v = run_event(&dir, "post-command");
        assert_eq!(v["verdict"], "allow-with-warnings", "{flag}: {v}");
        let moves = v["gated_moves"].as_array().unwrap();
        assert_eq!(moves.len(), 1, "{flag}: {v}");
        assert!(moves[0].as_str().unwrap().contains(PATH_1), "{v}");
        // An unchanged flagged file is quiet.
        let dir = fixture("s4-quiet", &config(""));
        git(&dir, &["update-index", flag, PATH_1]);
        assert_eq!(
            run_event(&dir, "post-command")["verdict"],
            "allow",
            "{flag}"
        );
    }
}

// ------------------------------------------------- N1. an over-deep settings file

#[test]
fn a_settings_file_nested_a_hundred_thousand_deep_is_refused_not_a_crash() {
    let dir = fixture("n1", &config(""));
    std::fs::write(
        dir.join(".claude/settings.json"),
        format!("{}{}", "[".repeat(100_000), "]".repeat(100_000)),
    )
    .unwrap();
    let out = urzua()
        .args(["harness", "install", "--adapter", "claude-code"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let v = json(&out);
    assert_eq!(v["status"], "not-run", "{v}");
    assert!(v.to_string().contains("nests deeper"), "{v}");
}

// ------------------------------------------------- N4. a command the person already wrote

#[test]
fn a_drafting_command_that_is_the_persons_own_is_reported_modified_not_missing() {
    let dir = fixture("n4", &config(""));
    install(&dir);
    // A record type added since the install, whose command file already exists.
    let with_note = config("").replace(
        LOG,
        &format!("  note:\n    dir: \"docs/notes\"\n    required_fields: [\"Status\"]\n{LOG}"),
    );
    std::fs::write(dir.join(".urzua/config.yaml"), with_note).unwrap();
    std::fs::write(dir.join(".claude/commands/note.md"), "my own command\n").unwrap();
    let run = |args: &[&str]| {
        let out = urzua()
            .arg("harness")
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
        json(&out)
    };
    let update = run(&["update"]);
    let state = |v: &serde_json::Value| {
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["path"] == ".claude/commands/note.md")
            .map(|i| i["state"].as_str().unwrap().to_string())
    };
    assert_eq!(state(&update).as_deref(), Some("kept-modified"), "{update}");
    let status = run(&["status"]);
    assert_eq!(state(&status).as_deref(), Some("modified"), "{status}");
}

// ------------------------------- round 4: the repository an edit is judged in

/// A project at `P/proj` under a writable parent `P`, as the review's repro has it.
fn nested_project(name: &str) -> (common::TempDir, PathBuf) {
    let parent = common::TempDir::new(&format!("urzua-r4-{name}"));
    let proj = parent.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    populate(&proj, &config(""));
    (parent, proj)
}

fn run_edit_in(
    process_cwd: &Path,
    project_dir: Option<&Path>,
    path: &Path,
    content: &str,
) -> (i32, serde_json::Value) {
    let mut request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path.to_string_lossy()], "content": content,
        "actor": "agent", "cwd": process_cwd.to_string_lossy(),
    });
    if let Some(p) = project_dir {
        request["project_dir"] = serde_json::json!(p.to_string_lossy());
    }
    let mut c = urzua();
    c.args(["harness", "run", "edit"]).current_dir(process_cwd);
    let out = run_with_stdin(&mut c, &request.to_string());
    (out.status.code().unwrap(), json(&out))
}

/// The four edits the review denied before step 1 of its repro.
fn assert_protected_edits_deny(proj: &Path, project_dir: Option<&Path>, why: &str) {
    for (rel, content) in [
        (".claude/settings.json", "{}\n"),
        (".github/workflows/x.yml", "name: x\n"),
        (".claude/hooks/urzua-claude-code.py", "import sys\n"),
        (PATH_1, ACCEPTED),
    ] {
        let (code, v) = run_edit_in(proj, project_dir, &proj.join(rel), content);
        assert_eq!(code, 1, "{why}: {rel}: {v}");
    }
}

#[test]
fn an_outer_repository_made_of_two_writes_does_not_replace_the_sessions_own_gates() {
    // Observed before the fix: every one of these was allow-unjudged.
    let (parent, proj) = nested_project("b1-minted");
    assert_protected_edits_deny(&proj, Some(&proj), "before the writes");
    // Step 1: an empty file named .git. Step 2: a configuration with no gates.
    std::fs::write(parent.join(".git"), "").unwrap();
    std::fs::create_dir_all(parent.join(".urzua")).unwrap();
    std::fs::write(parent.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
    // Whether the request names the project, or only its directory is the cwd.
    assert_protected_edits_deny(&proj, Some(&proj), "empty .git file, project named");
    assert_protected_edits_deny(&proj, None, "empty .git file, cwd only");
    // The process runs somewhere else under the minted parent.
    std::fs::create_dir_all(parent.join("elsewhere")).unwrap();
    for rel in [".claude/settings.json", PATH_1] {
        let content = if rel == PATH_1 { ACCEPTED } else { "{}\n" };
        let (code, v) = run_edit_in(
            &parent.join("elsewhere"),
            Some(&proj),
            &proj.join(rel),
            content,
        );
        assert_eq!(code, 1, "{rel}: {v}");
    }
}

#[test]
fn an_empty_git_directory_does_not_make_a_repository_either() {
    let (parent, proj) = nested_project("b1-gitdir");
    std::fs::create_dir_all(parent.join(".git")).unwrap();
    std::fs::create_dir_all(parent.join(".urzua")).unwrap();
    std::fs::write(parent.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
    assert_protected_edits_deny(&proj, Some(&proj), "empty .git directory");
}

#[test]
fn a_real_outer_repository_with_no_gates_still_leaves_the_sessions_gates_in_force() {
    let (parent, proj) = nested_project("b1-real-outer");
    git(&parent, &["init", "-q"]);
    std::fs::create_dir_all(parent.join(".urzua")).unwrap();
    std::fs::write(parent.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
    assert_protected_edits_deny(&proj, Some(&proj), "real outer repository without gates");
}

#[test]
fn every_governed_repository_that_holds_the_file_can_deny_it_inner_or_outer() {
    // Outer session, inner governed repository with its own protected path.
    let (_outer, proj) = nested_project("b1-inner");
    let inner = proj.join("vendor");
    std::fs::create_dir_all(&inner).unwrap();
    populate(&inner, &config(""));
    let (code, v) = run_edit_in(
        &proj,
        Some(&proj),
        &inner.join(".claude/settings.json"),
        "{}\n",
    );
    assert_eq!(code, 1, "the inner repository's own gate: {v}");
    // Outer repository protects a path the session's repository does not.
    let (parent, proj) = nested_project("b1-outer-more");
    git(&parent, &["init", "-q"]);
    std::fs::create_dir_all(parent.join(".urzua")).unwrap();
    std::fs::write(
        parent.join(".urzua/config.yaml"),
        format!("{TYPES}harness:\n  guardrails:\n    protected-paths: {{enabled: true}}\nauthority:\n  deciders:\n    maintainers: {{logins: [Alice]}}\n  gates:\n    - {{paths: [\"proj/notes.txt\"], by: maintainers}}\n"),
    )
    .unwrap();
    let (code, v) = run_edit_in(&proj, Some(&proj), &proj.join("notes.txt"), "x\n");
    assert_eq!(code, 1, "the outer repository adds a restriction: {v}");
}

#[test]
fn the_installed_settings_command_denies_the_minted_outer_repository_repro() {
    let (parent, proj) = nested_project("b1-command");
    install(&proj);
    std::fs::write(parent.join(".git"), "").unwrap();
    std::fs::create_dir_all(parent.join(".urzua")).unwrap();
    std::fs::write(parent.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
    let payload = write_payload(&proj, &proj.join(".claude/settings.json"), "{}\n");
    let v = run_command(&proj, &proj, Some(&proj), None, &payload);
    assert_eq!(decision(&v), Some("deny"), "{v}");
}

// ------------------------------------- round 4 S1: the translator's own hash

fn sha256_of(path: &Path) -> String {
    let out = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())",
        ])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn status_of(dir: &Path) -> serde_json::Value {
    let out = urzua()
        .args(["harness", "status"])
        .current_dir(dir)
        .output()
        .unwrap();
    json(&out)
}

fn command_for(dir: &Path, handler: &str, cwd: &Path, payload: &str) -> String {
    let mut c = Command::new("sh");
    c.args(["-c", &hook_command(dir, handler)])
        .current_dir(cwd)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env("CLAUDE_PROJECT_DIR", dir);
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn every_hook_command_embeds_the_translators_hash_and_refuses_a_translator_that_differs() {
    let dir = fixture("s1", &config(""));
    install(&dir);
    let translator = dir.join(".claude/hooks/urzua-claude-code.py");
    let hash = sha256_of(&translator);
    for handler in ["edit", "post-command", "session-start"] {
        assert!(hook_command(&dir, handler).contains(&hash), "{handler}");
    }
    assert_eq!(status_of(&dir)["status"], "ok", "installed");

    // A translator that exits 0 without doing anything, and leaves a mark if it runs.
    let mark = dir.join("translator-ran");
    let original = std::fs::read_to_string(&translator).unwrap();
    std::fs::write(
        &translator,
        format!(
            "open({:?}, 'w').write('x')\nimport sys\nsys.exit(0)\n",
            mark.to_string_lossy()
        ),
    )
    .unwrap();
    let edit = write_payload(&dir, &dir.join(".claude/settings.json"), "{}\n");
    let out = command_for(&dir, "edit", &dir, &edit);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(decision(&v), Some("deny"), "{out}");
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap();
    assert!(reason.contains("does not match the sha256"), "{reason}");
    let bash = serde_json::json!({"hook_event_name": "PostToolUse", "tool_name": "Bash", "cwd": dir.to_string_lossy(), "tool_input": {"command": "true"}}).to_string();
    let out = command_for(&dir, "post-command", &dir, &bash);
    assert!(
        out.contains("does not match the sha256") && out.contains("NOT judged"),
        "{out}"
    );
    let start =
        serde_json::json!({"hook_event_name": "SessionStart", "cwd": dir.to_string_lossy()})
            .to_string();
    let out = command_for(&dir, "session-start", &dir, &start);
    assert!(out.contains("does not match the sha256"), "{out}");
    assert!(!mark.exists(), "the changed translator must not run");

    // Where the configuration allows an unjudged edit the command states it.
    // (The same configuration text is read by the shell, so rewrite it.)
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config(", on_unjudged: allow"),
    )
    .unwrap();
    let out = command_for(&dir, "edit", &dir, &edit);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(decision(&v), None, "{out}");
    assert!(v["systemMessage"]
        .as_str()
        .unwrap()
        .contains("does not match the sha256"));
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();

    // Restored, it runs again; `update` restores a changed one the same way.
    std::fs::write(&translator, &original).unwrap();
    let out = command_for(&dir, "edit", &dir, &edit);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        decision(&v),
        Some("deny"),
        "judged by the engine again: {out}"
    );
    assert!(!v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .contains("does not match"));
}

#[test]
fn install_update_and_uninstall_keep_the_hash_consistent() {
    let dir = fixture("s1-roundtrip", &config(""));
    install(&dir);
    let before = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    // update with nothing changed rewrites nothing.
    let out = urzua()
        .args(["harness", "update"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        before,
        std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap()
    );
    // A translator changed by hand is drift; update --force restores both file and hash.
    let translator = dir.join(".claude/hooks/urzua-claude-code.py");
    let hash = sha256_of(&translator);
    std::fs::write(&translator, "import sys\nsys.exit(0)\n").unwrap();
    assert_ne!(status_of(&dir)["status"], "ok", "drift is reported");
    let out = urzua()
        .args(["harness", "update", "--force"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(sha256_of(&translator), hash);
    assert!(hook_command(&dir, "edit").contains(&hash));
    assert_eq!(status_of(&dir)["status"], "ok");
    let out = urzua()
        .args(["harness", "uninstall"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let left = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap_or_default();
    assert!(!left.contains(&hash), "{left}");
}

// ---------------------------- round 4 S2 and S3: MCP shapes outside the name

fn denies(dir: &Path, tool: &str, input: serde_json::Value) -> bool {
    decision(&translate(dir, &mcp(dir, tool, input))) == Some("deny")
}

#[test]
fn a_tool_named_for_reading_is_still_judged_when_the_call_names_an_output_or_carries_content() {
    let dir = fixture("s2", &config(""));
    let p = dir
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    for (tool, input) in [
        (
            "mcp__x__fetch",
            serde_json::json!({"url": "https://example.com/a", "output_path": p}),
        ),
        (
            "mcp__x__get_file",
            serde_json::json!({"id": "1", "save_to": p}),
        ),
        (
            "mcp__x__query",
            serde_json::json!({"sql": "select 1", "outfile": p}),
        ),
        (
            "mcp__x__get",
            serde_json::json!({"path": p, "content": "{}\n"}),
        ),
        (
            "mcp__x__log",
            serde_json::json!({"path": p, "content": "{}\n"}),
        ),
        (
            "mcp__x__check_in",
            serde_json::json!({"path": p, "content": "{}\n"}),
        ),
    ] {
        assert!(denies(&dir, tool, input.clone()), "{tool}: {input}");
    }
    // Plain reads stay quiet.
    for (tool, input) in [
        ("mcp__x__get_file", serde_json::json!({"path": p})),
        (
            "mcp__x__fetch",
            serde_json::json!({"url": "https://example.com/a"}),
        ),
        ("mcp__x__info", serde_json::json!({"path": p, "limit": 3})),
    ] {
        assert!(
            translate(&dir, &mcp(&dir, tool, input.clone())).is_null(),
            "{tool}: {input}"
        );
    }
}

#[test]
fn an_mcp_path_is_read_after_trimming_and_normalising_whatever_the_wrapping() {
    let dir = fixture("s3", &config(""));
    let p = dir
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    let long = format!("{}{}", "/.".repeat(2100), p);
    assert!(long.len() > 4096);
    for (label, path) in [
        ("trailing newline", format!("{p}\n")),
        ("leading space", format!(" {p}")),
        ("trailing space and tab", format!("{p} \t")),
        ("nul", format!("{p}\0ignored")),
        ("over-long, collapses", long),
        (
            "dot segments",
            format!("{}/docs/../.claude/./settings.json", dir.to_string_lossy()),
        ),
    ] {
        assert!(
            denies(
                &dir,
                "mcp__x__write_file",
                serde_json::json!({"path": path, "content": "{}\n"})
            ),
            "{label}"
        );
    }
    // An ordinary file with the same wrapping is not refused.
    let ok = dir.join("notes.txt").to_string_lossy().to_string();
    assert!(!denies(
        &dir,
        "mcp__x__write_file",
        serde_json::json!({"path": format!(" {ok}\n"), "content": "x\n"})
    ));
}

#[test]
fn with_the_project_variable_unset_a_minted_outer_configuration_cannot_take_the_wrapper() {
    for real_git in [false, true] {
        let (parent, proj) = nested_project(if real_git {
            "unset-real"
        } else {
            "unset-empty"
        });
        install(&proj);
        if real_git {
            git(&parent, &["init", "-q"]);
        } else {
            std::fs::write(parent.join(".git"), "").unwrap();
        }
        std::fs::create_dir_all(parent.join(".urzua")).unwrap();
        std::fs::write(parent.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
        std::fs::create_dir_all(proj.join("docs/sub")).unwrap();
        // From the project root and from a subdirectory, variable unset.
        for cwd in [proj.clone(), proj.join("docs/sub")] {
            for rel in [".claude/settings.json", PATH_1] {
                let content = if rel == PATH_1 { ACCEPTED } else { "{}\n" };
                let payload = write_payload(&cwd, &proj.join(rel), content);
                let v = run_command(&proj, &cwd, None, None, &payload);
                assert_eq!(
                    decision(&v),
                    Some("deny"),
                    "real_git={real_git} {cwd:?} {rel}: {v}"
                );
            }
        }
    }
}

#[test]
fn a_read_named_mcp_tool_with_body_data_payload_or_value_is_judged_as_a_write() {
    let dir = fixture("s2-keys", &config(""));
    let p = dir
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    for key in ["body", "data", "payload", "value"] {
        assert!(
            denies(
                &dir,
                "mcp__x__get",
                serde_json::json!({"path": p, key: "{}\n"})
            ),
            "{key}"
        );
    }
}

// ------------------------------------------------- review 7: status

#[test]
fn status_reports_the_kill_switch_and_compares_versions_numerically() {
    let dir = fixture("status-kill", &config(""));
    install(&dir);
    let v = status_of(&dir);
    assert_eq!(v["status"], "ok", "{v}");
    let settings = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    let off = settings.replacen('{', "{\n  \"disableAllHooks\": true,", 1);
    std::fs::write(dir.join(".claude/settings.json"), off).unwrap();
    let v = status_of(&dir);
    assert_eq!(v["status"], "drift", "{v}");
    let problems = v["problems"].as_array().unwrap();
    assert!(
        problems
            .iter()
            .any(|p| p.as_str().unwrap().contains("disableAllHooks")),
        "{v}"
    );
    // A string value is read as the setting reads it.
    let off = settings.replacen('{', "{\n  \"disableAllHooks\": \"TRUE\",", 1);
    std::fs::write(dir.join(".claude/settings.json"), off).unwrap();
    assert_eq!(status_of(&dir)["status"], "drift");
    // Off again: ok.
    std::fs::write(dir.join(".claude/settings.json"), &settings).unwrap();
    assert_eq!(status_of(&dir)["status"], "ok");

    // A lock whose minor version has more digits than this binary's is newer,
    // though it sorts before it as text ("0.10.0" < "0.3.0").
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    let major = parts.next().unwrap();
    let minor = parts.next().unwrap();
    let longer = format!("{major}.1{}.0", "0".repeat(minor.len()));
    let lock = std::fs::read_to_string(dir.join(".urzua/harness.lock")).unwrap();
    let bundle = lock
        .lines()
        .find_map(|l| l.trim().strip_prefix("\"bundle_version\": \""))
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    for (locked, newer) in [(longer.as_str(), true), ("0.0.1", false)] {
        std::fs::write(
            dir.join(".urzua/harness.lock"),
            lock.replace(
                &format!("\"bundle_version\": \"{bundle}\""),
                &format!("\"bundle_version\": \"{locked}\""),
            ),
        )
        .unwrap();
        let v = status_of(&dir);
        let notices = v["notices"].to_string();
        assert_eq!(
            notices.contains("this binary is only"),
            newer,
            "{locked}: {v}"
        );
        let session = session(&dir);
        let text = session["message"].as_str().unwrap();
        assert_eq!(
            text.contains("is newer than this binary"),
            newer,
            "{locked}: {session}"
        );
    }
}
