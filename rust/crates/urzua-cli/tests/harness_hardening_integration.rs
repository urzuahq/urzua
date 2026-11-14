//! The second-pass review of the harness (ADR-89): a Bash edit of a protected
//! path is reported, a commit made inside the same command is reported, an
//! adapter failure fails closed where the configuration says deny, an edit
//! from a cwd outside the repository is still judged, paths match case
//! insensitively, an edit through a symlink is judged as its target, and
//! `export`/`import` name their format. Real git repositories, the real
//! binary and the real translator. Needs `git`, `python3` and `sh`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

fn harness(extra: &str) -> String {
    format!("harness:\n  guardrails:\n    acceptance: {{enabled: true{extra}}}\n    protected-paths: {{enabled: true{extra}}}\n    post-command: {{enabled: true}}\n    editor-hook: {{enabled: true}}\n    session-start: {{enabled: true}}\n")
}

fn config(extra: &str) -> String {
    format!("{TYPES}{}{AUTHORITY}", harness(extra))
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
    let dir = common::TempDir::new(&format!("urzua-hardening-{name}"));
    for d in ["docs/adr", ".urzua", ".claude"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn bash(dir: &Path, script: &str) {
    let status = Command::new("sh")
        .args(["-c", script])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "{script}");
}

fn run_with_stdin(c: &mut Command, stdin: &str) -> Output {
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn urzua() -> Command {
    common::urzua()
}

/// `urzua harness run <event>` with a request for `dir`.
fn run_event(dir: &Path, event: &str) -> serde_json::Value {
    let request = serde_json::json!({
        "event": event, "tool": "Bash", "paths": [], "actor": "agent",
        "cwd": dir.to_string_lossy(),
    });
    let mut c = urzua();
    c.args(["harness", "run", event]).current_dir(dir);
    let out = run_with_stdin(&mut c, &request.to_string());
    assert_eq!(out.status.code(), Some(0), "never blocks: {out:?}");
    json(&out)
}

fn post(dir: &Path) -> serde_json::Value {
    run_event(dir, "post-command")
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

fn run_edit(dir: &Path, path: &str, content: &str) -> (i32, serde_json::Value) {
    run_edit_from(dir, dir, path, content)
}

// ------------------------------------------- 1. a protected path by shell

#[test]
fn a_bash_edit_of_a_protected_path_is_reported_once_and_never_blocked() {
    let dir = fixture("protected-bash", &config(""));
    assert_eq!(post(&dir)["verdict"], "allow", "a clean tree is quiet");
    bash(&dir, "sed -i 's/{}/{\"x\": 1}/' .claude/settings.json");
    bash(&dir, "echo '# changed' >> .urzua/config.yaml");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let lines: Vec<&str> = v["protected_changes"]
        .as_array()
        .unwrap_or_else(|| panic!("no protected_changes: {v}"))
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{v}");
    assert!(
        lines.iter().any(|l| l.contains(".claude/settings.json")
            && l.contains("[protected-paths]")
            && l.contains("not prevented")),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l.contains(".urzua/config.yaml")));
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("2 protected path change(s)"),
        "{v}"
    );
    // The same change is not reported again.
    assert_eq!(post(&dir)["verdict"], "allow", "{}", post(&dir));
}

#[test]
fn a_deleted_protected_path_is_reported_and_an_unprotected_change_is_not() {
    let dir = fixture("protected-delete", &config(""));
    bash(&dir, "rm .claude/settings.json; echo x > notes.txt");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let lines = v["protected_changes"].as_array().unwrap();
    assert_eq!(lines.len(), 1, "{v}");
    assert!(lines[0].as_str().unwrap().contains("deleted"), "{v}");
}

#[test]
fn with_the_protected_paths_guardrail_off_a_changed_protected_path_is_not_reported() {
    let cfg = config("").replace(
        "    protected-paths: {enabled: true}\n",
        "    protected-paths: {enabled: false}\n",
    );
    let dir = fixture("protected-off", &cfg);
    bash(&dir, "echo x >> .claude/settings.json");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
}

// ---------------------------------- 2. a commit made inside the command

#[test]
fn a_gated_move_committed_in_the_same_command_is_reported() {
    let dir = fixture("commit-gated", &config(""));
    // The first check of the session records the commit the next one starts from.
    assert_eq!(post(&dir)["verdict"], "allow");
    bash(
        &dir,
        &format!("sed -i 's/^Status: Proposed/Status: Accepted/' {PATH_1} && git commit -q -am x"),
    );
    assert!(git(&dir, &["status", "--porcelain"]).trim().is_empty());
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let moves = v["gated_moves"].as_array().unwrap();
    assert_eq!(moves.len(), 1, "{v}");
    let line = moves[0].as_str().unwrap();
    assert!(
        line.contains("'Proposed'") && line.contains("'Accepted'") && line.contains(PATH_1),
        "{line}"
    );
    // Reported once: the commit is behind the remembered one now.
    assert_eq!(post(&dir)["verdict"], "allow", "{}", post(&dir));
}

#[test]
fn a_protected_path_committed_in_the_same_command_is_reported() {
    let dir = fixture("commit-protected", &config(""));
    assert_eq!(post(&dir)["verdict"], "allow");
    bash(
        &dir,
        "echo '{\"a\": 1}' > .claude/settings.json && git commit -q -am x",
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn session_start_records_the_commit_so_the_first_command_is_covered() {
    let dir = fixture("commit-seeded", &config(""));
    run_event(&dir, "session-start");
    bash(
        &dir,
        &format!("sed -i 's/^Status: Proposed/Status: Accepted/' {PATH_1} && git commit -q -am x"),
    );
    let v = post(&dir);
    assert_eq!(v["gated_moves"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn a_forgotten_commit_falls_back_to_head_without_failing() {
    let dir = fixture("commit-gone", &config(""));
    std::fs::create_dir_all(dir.join(".urzua/cache")).unwrap();
    std::fs::write(
        dir.join(".urzua/cache/post-command-head"),
        "0123456789012345678901234567890123456789\n",
    )
    .unwrap();
    bash(&dir, "echo x >> .claude/settings.json");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
}

// ------------------------------------------- 3. the adapter fails closed

fn translator() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bundle/claude-code/files/.claude/hooks/urzua-claude-code.py")
}

/// The translator run the way the settings command runs it, with `engine` as the
/// command after `--`.
fn translate(dir: &Path, payload: &str, engine: &[&str]) -> serde_json::Value {
    translate_home(dir, payload, engine, None)
}

/// [`translate`] with `HOME` set to `home`, for a path that starts with `~`.
fn translate_home(
    dir: &Path,
    payload: &str,
    engine: &[&str],
    home: Option<&Path>,
) -> serde_json::Value {
    let mut c = Command::new("python3");
    if let Some(home) = home {
        c.env("HOME", home);
    }
    c.arg("-I")
        .arg(translator())
        .args(["edit", "--"])
        .args(engine)
        .current_dir(dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    // An allowed call prints nothing.
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    json(&out)
}

fn decision(v: &serde_json::Value) -> Option<&str> {
    v["hookSpecificOutput"]["permissionDecision"].as_str()
}

const ENGINE: [&str; 4] = ["urzua", "harness", "run", "edit"];

#[test]
fn an_unreadable_payload_is_refused_when_acceptance_or_protected_paths_deny_unjudged() {
    let dir = fixture("closed-payload", &config(""));
    let v = translate(&dir, "garbage{", &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    assert!(
        v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("on_unjudged"),
        "{v}"
    );
}

#[test]
fn an_unreadable_payload_is_allowed_with_a_notice_when_the_configuration_allows() {
    let dir = fixture("open-payload", &config(", on_unjudged: allow"));
    let v = translate(&dir, "garbage{", &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judged"),
        "{v}"
    );
    // No gate on at all: the same.
    let off = format!("{TYPES}harness:\n  guardrails:\n    editor-hook: {{enabled: true}}\n    session-start: {{enabled: true}}\n");
    let dir = fixture("off-payload", &off);
    let v = translate(&dir, "garbage{", &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
}

#[test]
fn a_failed_engine_is_refused_when_the_configuration_denies_and_allowed_when_it_allows() {
    let payload = |dir: &Path| {
        serde_json::json!({
            "tool_name": "Write", "cwd": dir.to_string_lossy(),
            "tool_input": {"file_path": dir.join("notes.txt").to_string_lossy(), "content": "x"},
        })
        .to_string()
    };
    let dir = fixture("closed-engine", &config(""));
    for engine in [&["urzua-no-such-binary"][..], &["false"][..]] {
        let v = translate(&dir, &payload(&dir), engine);
        assert_eq!(decision(&v), Some("deny"), "{engine:?}: {v}");
    }
    let dir = fixture("open-engine", &config(", on_unjudged: allow"));
    for engine in [&["urzua-no-such-binary"][..], &["false"][..]] {
        let v = translate(&dir, &payload(&dir), engine);
        assert_eq!(decision(&v), None, "{engine:?}: {v}");
        assert!(
            v["systemMessage"].as_str().unwrap().contains("NOT judge"),
            "{v}"
        );
    }
    // Block style, and a comment: the text fallback reads the same settings.
    let block = format!("{TYPES}harness:\n  guardrails:\n    acceptance:\n      enabled: true\n      on_unjudged: allow\n    protected-paths:\n      enabled: true\n{AUTHORITY}");
    let dir = fixture("block-engine", &block);
    let v = translate(&dir, &payload(&dir), &["false"]);
    assert_eq!(
        decision(&v),
        Some("deny"),
        "protected-paths still denies: {v}"
    );
}

/// An `Edit` whose `old_string` is not in `file`, from `dir`.
fn unplaceable(dir: &Path, file: &str) -> String {
    serde_json::json!({
        "tool_name": "Edit", "cwd": dir.to_string_lossy(),
        "tool_input": {"file_path": dir.join(file).to_string_lossy(), "old_string": "absent text", "new_string": "x"},
    })
    .to_string()
}

#[test]
fn an_edit_the_translator_cannot_place_is_refused_on_a_gated_record_or_protected_path_when_the_configuration_denies(
) {
    let dir = fixture("closed-place", &config(""));
    // A gated record, a protected path and the configuration itself: refused.
    for file in [PATH_1, ".claude/settings.json", ".urzua/config.yaml"] {
        let v = translate(&dir, &unplaceable(&dir, file), &ENGINE);
        assert_eq!(decision(&v), Some("deny"), "{file}: {v}");
    }
    // Case-variant spellings and a path through a symlink are the same files.
    std::os::unix::fs::symlink("../.claude", dir.join("docs/cl")).unwrap();
    for file in [
        "Docs/ADR/ADR-1-a.md",
        ".CLAUDE/settings.json",
        "docs/cl/settings.json",
    ] {
        let v = translate(&dir, &unplaceable(&dir, file), &ENGINE);
        assert_eq!(decision(&v), Some("deny"), "{file}: {v}");
    }
    let dir = fixture("open-place", &config(", on_unjudged: allow"));
    let v = translate(&dir, &unplaceable(&dir, PATH_1), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

#[test]
fn an_edit_the_translator_cannot_place_is_allowed_with_a_notice_on_any_other_file() {
    let dir = fixture("ungated-place", &config(""));
    std::fs::write(dir.join("notes.txt"), "hello\n").unwrap();
    // An existing ungated file, one that does not exist, and a record directory
    // no gate names: the edit tool's own failure handles an absent old_string.
    for file in ["notes.txt", "missing/nothing.txt", "README.md"] {
        let v = translate(&dir, &unplaceable(&dir, file), &ENGINE);
        assert_eq!(decision(&v), None, "{file}: {v}");
        assert!(
            v["systemMessage"].as_str().unwrap().contains("NOT judge"),
            "{file}: {v}"
        );
    }
    // An unreadable payload names no file, so it is still refused.
    let v = translate(&dir, "garbage{", &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
}

// ------------------------------------ 4. an edit from outside the repo

#[test]
fn an_edit_from_a_cwd_outside_the_repository_is_judged_by_the_files_repository() {
    let dir = fixture("outside", &config(""));
    let outside = common::TempDir::new("urzua-hardening-elsewhere");
    for path in [dir.join(".claude/settings.json"), dir.join(PATH_1)] {
        let content = if path.ends_with("ADR-1-a.md") {
            ACCEPTED
        } else {
            "{}\n"
        };
        let (code, v) = run_edit_from(&outside, &outside, &path.to_string_lossy(), content);
        assert_eq!(code, 1, "{}: {v}", path.display());
        assert_eq!(v["verdict"], "block", "{v}");
    }
    // An ordinary file of that repository is judged, not blocked.
    let (code, v) = run_edit_from(
        &outside,
        &outside,
        &dir.join("notes.txt").to_string_lossy(),
        "x\n",
    );
    assert_eq!(code, 0, "{v}");
}

// --------------------------------------------- 5. letter case

#[test]
fn a_protected_path_or_record_directory_spelled_in_another_case_is_still_gated() {
    let dir = fixture("case", &config(""));
    for path in [
        ".CLAUDE/settings.json",
        ".Claude/hooks/x.py",
        ".urzua/CONFIG.yaml",
    ] {
        let (code, v) = run_edit(&dir, path, "x\n");
        assert_eq!(code, 1, "{path}: {v}");
        assert!(
            v["message"].as_str().unwrap().contains("protected-paths"),
            "{v}"
        );
    }
    let (code, v) = run_edit(&dir, "Docs/ADR/ADR-1-a.md", ACCEPTED);
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("acceptance"), "{v}");
}

#[test]
fn a_protected_path_changed_by_shell_in_another_case_is_reported() {
    let dir = fixture("case-bash", &config(""));
    bash(&dir, "mkdir -p .Claude && echo x > .Claude/new.json");
    let v = post(&dir);
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
}

// --------------------------------------------------- 6. symlinks

#[cfg(unix)]
#[test]
fn an_edit_through_a_symlink_is_judged_as_the_file_it_writes() {
    let dir = fixture("symlink", &config(""));
    std::os::unix::fs::symlink("adr/ADR-1-a.md", dir.join("docs/link.md")).unwrap();
    std::os::unix::fs::symlink(".claude", dir.join("cl")).unwrap();
    let (code, v) = run_edit(&dir, "docs/link.md", ACCEPTED);
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("acceptance"), "{v}");
    let (code, v) = run_edit(&dir, "cl/settings.json", "{}\n");
    assert_eq!(code, 1, "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("protected-paths"),
        "{v}"
    );
    // A new file under a linked directory too.
    let (code, v) = run_edit(&dir, "cl/new.json", "{}\n");
    assert_eq!(code, 1, "{v}");
}

// ----------------------------------------------- 6. the templates

#[test]
fn this_repositorys_templates_are_a_protected_path() {
    let own = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml"),
    )
    .unwrap();
    let dir = fixture("templates", &own);
    let (code, v) = run_edit(&dir, ".urzua/templates/adr.md", "x\n");
    assert_eq!(code, 1, "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("protected-paths"),
        "{v}"
    );
}

// ---------------------------------------- 7. export, import, fix help

#[test]
fn export_and_import_require_a_format_and_the_help_says_which_is_implemented() {
    let dir = fixture("formats", &config(""));
    for args in [&["export"][..], &["import"][..]] {
        let out = urzua().args(args).current_dir(&dir).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let v = json(&out);
        assert!(v["error"].as_str().unwrap().contains("--format"), "{v}");
        assert!(
            !v["error"].as_str().unwrap().contains("not implemented"),
            "{v}"
        );
    }
    let help = urzua().args(["fix", "--help"]).output().unwrap();
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(!text.contains("Only Tier 1"), "{text}");
    assert!(text.contains("--rebaseline"), "{text}");
}

// --------------------------- the drafting helper's binary variable, the docs

#[test]
fn the_drafting_helper_honours_urzua_bin_and_still_accepts_urzua() {
    let dir = fixture("helper-bin", &config(""));
    let helper = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bundle/claude-code/files/.claude/hooks/urzua-record.sh");
    let fake = dir.join("fake-urzua");
    std::fs::write(
        &fake,
        "#!/bin/sh\necho called >> \"$0.log\"\necho '{\"path\": \"x.md\"}'\n",
    )
    .unwrap();
    bash(&dir, "chmod +x fake-urzua");
    for var in ["URZUA_BIN", "URZUA"] {
        let _ = std::fs::remove_file(dir.join("fake-urzua.log"));
        let out = Command::new("bash")
            .arg(&helper)
            .args(["new", "adr", "T"])
            .current_dir(&dir)
            .env("PATH", "/usr/bin:/bin")
            .env_remove("URZUA_BIN")
            .env_remove("URZUA")
            .env(var, &fake)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!err.contains("no urzua binary"), "{var}: {err}");
        assert!(
            dir.join("fake-urzua.log").exists(),
            "{var} was not used: {err}"
        );
    }
}

#[test]
fn the_installed_instructions_say_the_post_command_check_reports_a_shell_edit_of_a_guardrail() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("bundle/instructions.md"),
    )
    .unwrap();
    assert!(
        text.contains("post-command check reports it after the command"),
        "{text}"
    );
    // And it is true: the shell edit of a guardrail is reported (see the first test).
    let dir = fixture("instructions-true", &config(""));
    bash(&dir, "echo '# x' >> .urzua/config.yaml");
    assert_eq!(post(&dir)["protected_changes"].as_array().unwrap().len(), 1);
}

// ============================================================ third pass
// The gaps #378 left: NotebookEdit and file-writing MCP tools, a gitignored
// protected file written by shell, Unicode case, and hardlinks.

/// A fake engine that saves the request it is sent to `capture` and allows.
fn capturing_engine(capture: &Path) -> Vec<String> {
    vec![
        "sh".into(),
        "-c".into(),
        "cat > \"$0\"; echo '{\"verdict\":\"allow\",\"message\":\"\"}'".into(),
        capture.to_string_lossy().to_string(),
    ]
}

fn translate_with(dir: &Path, payload: &str, engine: &[String]) -> serde_json::Value {
    let engine: Vec<&str> = engine.iter().map(String::as_str).collect();
    translate(dir, payload, &engine)
}

fn notebook(cells: serde_json::Value) -> String {
    serde_json::json!({"cells": cells, "metadata": {}, "nbformat": 4, "nbformat_minor": 5})
        .to_string()
}

fn notebook_payload(dir: &Path, file: &str) -> String {
    serde_json::json!({
        "tool_name": "NotebookEdit", "cwd": dir.to_string_lossy(),
        "tool_input": {
            "notebook_path": dir.join(file).to_string_lossy(),
            "cell_id": "c1", "new_source": "print('edited')", "edit_mode": "replace",
        },
    })
    .to_string()
}

#[test]
fn a_notebook_edit_is_judged_as_the_notebook_it_writes() {
    let dir = fixture("notebook", &config(""));
    let nb = notebook(serde_json::json!([
        {"id": "c1", "cell_type": "code", "source": "print(1)", "metadata": {}, "outputs": [], "execution_count": null}
    ]));
    std::fs::write(dir.join(".claude/n.ipynb"), &nb).unwrap();
    std::fs::write(dir.join("n.ipynb"), &nb).unwrap();
    // A protected notebook is refused, an ordinary one is not.
    let v = translate(&dir, &notebook_payload(&dir, ".claude/n.ipynb"), &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let v = translate(&dir, &notebook_payload(&dir, "n.ipynb"), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    // The engine is sent the notebook as it would be after the edit.
    let capture = dir.join("request.json");
    translate_with(
        &dir,
        &notebook_payload(&dir, "n.ipynb"),
        &capturing_engine(&capture),
    );
    let request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&capture).unwrap()).unwrap();
    assert_eq!(request["tool"], "NotebookEdit", "{request}");
    assert!(
        request["paths"][0].as_str().unwrap().ends_with("n.ipynb"),
        "{request}"
    );
    let after: serde_json::Value =
        serde_json::from_str(request["content"].as_str().unwrap()).unwrap();
    assert_eq!(after["cells"][0]["source"], "print('edited')", "{request}");
}

#[test]
fn a_notebook_edit_that_cannot_be_placed_is_refused_only_on_a_protected_path() {
    let dir = fixture("notebook-unplaced", &config(""));
    // The notebook does not exist, so the edit cannot be applied to it.
    let v = translate(&dir, &notebook_payload(&dir, ".claude/none.ipynb"), &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let v = translate(&dir, &notebook_payload(&dir, "none.ipynb"), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

fn mcp(dir: &Path, tool: &str, input: serde_json::Value) -> String {
    serde_json::json!({"tool_name": tool, "cwd": dir.to_string_lossy(), "tool_input": input})
        .to_string()
}

#[test]
fn an_mcp_tool_that_writes_a_file_is_judged_by_its_path_and_content_fields() {
    let dir = fixture("mcp-write", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    for (tool, path_key, content_key) in [
        ("mcp__fs__write_file", "path", "content"),
        ("mcp__files__create", "filePath", "contents"),
        ("mcp__other__put", "file", "text"),
    ] {
        let v = translate(
            &dir,
            &mcp(
                &dir,
                tool,
                serde_json::json!({path_key: abs(".claude/x.json"), content_key: "{}"}),
            ),
            &ENGINE,
        );
        assert_eq!(decision(&v), Some("deny"), "{tool}: {v}");
        // A gated record's move to an accepting status is refused as well.
        let v = translate(
            &dir,
            &mcp(
                &dir,
                tool,
                serde_json::json!({path_key: abs(PATH_1), content_key: ACCEPTED}),
            ),
            &ENGINE,
        );
        assert_eq!(decision(&v), Some("deny"), "{tool}: {v}");
        let v = translate(
            &dir,
            &mcp(
                &dir,
                tool,
                serde_json::json!({path_key: abs("notes.txt"), content_key: "hi"}),
            ),
            &ENGINE,
        );
        assert_eq!(decision(&v), None, "{tool}: {v}");
    }
    // The engine is sent the path and the content.
    let capture = dir.join("request.json");
    translate_with(
        &dir,
        &mcp(
            &dir,
            "mcp__fs__write_file",
            serde_json::json!({"path": abs("notes.txt"), "content": "hi"}),
        ),
        &capturing_engine(&capture),
    );
    let request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&capture).unwrap()).unwrap();
    assert_eq!(request["content"], "hi", "{request}");
    assert!(request["paths"][0].as_str().unwrap().ends_with("notes.txt"));
}

#[test]
fn an_mcp_write_it_cannot_judge_applies_on_unjudged_only_to_a_gated_or_protected_target() {
    let dir = fixture("mcp-unjudged", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let edit = |f: &str| {
        mcp(
            &dir,
            "mcp__fs__edit_file",
            serde_json::json!({"path": abs(f), "edits": [{"oldText": "a", "newText": "b"}]}),
        )
    };
    // Edit-shaped input, or a move/delete by name, cannot be turned into the
    // file's text: refused on a protected path or a gated record only.
    for f in [".claude/settings.json", PATH_1, ".CLAUDE/settings.json"] {
        let v = translate(&dir, &edit(f), &ENGINE);
        assert_eq!(decision(&v), Some("deny"), "{f}: {v}");
    }
    let v = translate(&dir, &edit("notes.txt"), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__fs__delete_file",
            serde_json::json!({"path": abs(".claude/settings.json")}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    // Reads and tools with no file in them are not edits and say nothing.
    for (tool, input) in [
        (
            "mcp__fs__read_file",
            serde_json::json!({"path": abs(".claude/settings.json")}),
        ),
        (
            "mcp__github__create_issue",
            serde_json::json!({"title": "t", "body": "b"}),
        ),
    ] {
        let mut c = Command::new("python3");
        c.arg("-I")
            .arg(translator())
            .args(["edit", "--"])
            .args(ENGINE)
            .current_dir(&dir)
            .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
        let out = run_with_stdin(&mut c, &mcp(&dir, tool, input));
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty(), "{tool}: {out:?}");
    }
    // With on_unjudged: allow the unjudged protected target is allowed, loudly.
    let dir = fixture("mcp-unjudged-open", &config(", on_unjudged: allow"));
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__fs__edit_file",
            serde_json::json!({"path": dir.join(".claude/settings.json").to_string_lossy(), "edits": [{}]}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    assert!(v["systemMessage"].as_str().unwrap().contains("NOT judge"));
}

const PROTECTED: &str = ".claude/settings.json";

fn denied_mcp(dir: &Path, tool: &str, input: serde_json::Value) -> serde_json::Value {
    let v = translate(dir, &mcp(dir, tool, input), &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{tool}: {v}");
    v
}

#[test]
fn a_path_under_a_non_path_key_is_read_however_many_other_strings_the_call_carries() {
    let dir = fixture("mcp-many-strings", &config(""));
    let abs = dir.join(PROTECTED).to_string_lossy().to_string();
    for junk in [0usize, 70, 300] {
        let mut input = serde_json::Map::new();
        for i in 0..junk {
            input.insert(format!("junk{i}"), format!("value {i}").into());
        }
        input.insert("location".into(), abs.clone().into());
        input.insert("content".into(), "x".into());
        denied_mcp(&dir, "mcp__x__save_note", input.into());
    }
    // A long list of tags before it is not a way past the scan either.
    let tags: Vec<String> = (0..200).map(|i| format!("tag {i}")).collect();
    denied_mcp(
        &dir,
        "mcp__x__save_note",
        serde_json::json!({"tags": tags, "location": abs, "content": "x"}),
    );
}

#[test]
fn a_call_with_more_strings_than_the_translator_reads_is_unjudged_and_refused_under_deny() {
    let dir = fixture("mcp-too-many-strings", &config(""));
    let mut input = serde_json::Map::new();
    for i in 0..700 {
        input.insert(format!("junk{i}"), format!("value {i}").into());
    }
    input.insert("content".into(), "x".into());
    denied_mcp(&dir, "mcp__x__save_note", input.clone().into());
    // Where the configuration allows an unjudged edit, it is allowed, loudly.
    let open = fixture("mcp-too-many-strings-open", &config(", on_unjudged: allow"));
    let v = translate(
        &open,
        &mcp(&open, "mcp__x__save_note", input.into()),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

#[test]
fn a_path_used_as_a_dictionary_key_is_read_as_a_path() {
    let dir = fixture("mcp-dict-keys", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    denied_mcp(
        &dir,
        "mcp__x__write_files",
        serde_json::json!({"files": {abs(PROTECTED): "x"}}),
    );
    denied_mcp(
        &dir,
        "mcp__x__write_files",
        serde_json::json!({abs(PROTECTED): "x"}),
    );
    // A gated record's move to an accepting status, written as a map entry.
    denied_mcp(
        &dir,
        "mcp__x__write_files",
        serde_json::json!({"files": {abs(PATH_1): ACCEPTED}}),
    );
    denied_mcp(
        &dir,
        "mcp__x__write_files",
        serde_json::json!({"files": {abs(PROTECTED): {"mode": "0644"}}}),
    );
    // An ordinary file in a map is allowed.
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_files",
            serde_json::json!({"files": {abs("notes.txt"): "hi"}}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
}

#[test]
fn a_file_uri_is_a_path_whatever_its_case_and_however_many_slashes() {
    let dir = fixture("mcp-file-uri", &config(""));
    let abs = dir.join(PROTECTED).to_string_lossy().to_string();
    for uri in [
        format!("file:{abs}"),
        format!("FILE://{abs}"),
        format!("File:///{}", abs.trim_start_matches('/')),
        format!("file://localhost{abs}"),
        format!("file://{abs}"),
    ] {
        denied_mcp(
            &dir,
            "mcp__x__write_note",
            serde_json::json!({"uri": uri, "content": "x"}),
        );
    }
    // Another scheme is not a path.
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_note",
            serde_json::json!({"uri": format!("https://example.com{abs}"), "content": "x"}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
}

/// An engine that logs every request and blocks any naming `.claude`; a
/// `gates_only` request is answered as a record, for every path it names.
fn logging_engine(log: &Path) -> Vec<String> {
    let script = "import json,sys\nraw=sys.stdin.read()\nopen(sys.argv[1],'a').write(raw.replace('\\n',' ')+'\\n')\nr=json.loads(raw)\nif '.claude' in raw:\n print(json.dumps({'verdict':'block','message':'protected'}))\nelif r.get('gates_only'):\n print(json.dumps({'verdict':'allow','message':'','record':True,'records':[True]*len(r['paths'])}))\nelse:\n print(json.dumps({'verdict':'allow','message':''}))";
    vec![
        "python3".into(),
        "-c".into(),
        script.into(),
        log.to_string_lossy().to_string(),
    ]
}

fn logged(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_multi_file_call_is_blocked_by_its_protected_file_before_any_record_is_checked() {
    let dir = fixture("mcp-early-exit", &config(""));
    let log = dir.join("requests.log");
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let items: Vec<serde_json::Value> = [
        "docs/adr/ADR-2.md",
        "docs/adr/ADR-3.md",
        PROTECTED,
        "docs/adr/ADR-4.md",
    ]
    .iter()
    .map(|f| serde_json::json!({"path": abs(f), "content": PROPOSED}))
    .collect();
    let v = translate_with(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_files",
            serde_json::json!({"files": items}),
        ),
        &logging_engine(&log),
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let requests = logged(&log);
    assert!(!requests.is_empty());
    assert!(
        requests.iter().all(|r| r.contains("gates_only")),
        "no full record judgement before the denial: {requests:?}"
    );
    assert_eq!(
        requests.len(),
        1,
        "every file is asked of the engine at once: {requests:?}"
    );
}

#[test]
fn record_items_are_judged_in_full_only_after_every_gate_has_passed_and_stop_at_a_block() {
    let dir = fixture("mcp-record-order", &config(""));
    let log = dir.join("requests.log");
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let items: Vec<serde_json::Value> = ["docs/adr/ADR-2.md", "docs/adr/ADR-3.md"]
        .iter()
        .map(|f| serde_json::json!({"path": abs(f), "content": PROPOSED}))
        .collect();
    translate_with(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_files",
            serde_json::json!({"files": items}),
        ),
        &logging_engine(&log),
    );
    let requests = logged(&log);
    let (gates, full): (Vec<_>, Vec<_>) = requests.iter().partition(|r| r.contains("gates_only"));
    assert_eq!(
        (gates.len(), full.len()),
        (1, 2),
        "one gates request for the whole call, then one per record: {requests:?}"
    );
}

#[test]
fn a_call_with_more_record_writes_than_the_limit_is_unjudged_and_refused_under_deny() {
    let dir = fixture("mcp-record-cap", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let items: Vec<serde_json::Value> = (2..12)
        .map(|n| serde_json::json!({"path": abs(&format!("docs/adr/ADR-{n}.md")), "content": PROPOSED}))
        .collect();
    let input = serde_json::json!({"files": items});
    let v = denied_mcp(&dir, "mcp__x__write_files", input.clone());
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap();
    assert!(reason.contains("more than the 8"), "{reason}");
    // Eight are judged.
    let eight = serde_json::json!({"files": &items[..8]});
    let v = translate(&dir, &mcp(&dir, "mcp__x__write_files", eight), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    // With on_unjudged: allow the large call is allowed, loudly.
    let open = fixture("mcp-record-cap-open", &config(", on_unjudged: allow"));
    let abs = |f: &str| open.join(f).to_string_lossy().to_string();
    let items: Vec<serde_json::Value> = (2..12)
        .map(|n| serde_json::json!({"path": abs(&format!("docs/adr/ADR-{n}.md")), "content": PROPOSED}))
        .collect();
    let v = translate(
        &open,
        &mcp(
            &open,
            "mcp__x__write_files",
            serde_json::json!({"files": items}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

#[test]
fn the_engine_says_for_a_gates_only_request_whether_a_record_would_be_checked() {
    let dir = fixture("gates-only", &config(""));
    let ask = |file: &str, content: &str| {
        let request = serde_json::json!({
            "event": "edit", "tool": "mcp__x__w", "paths": [dir.join(file).to_string_lossy()],
            "content": content, "actor": "agent", "cwd": dir.to_string_lossy(),
            "gates_only": true,
        });
        let mut c = urzua();
        c.args(["harness", "run", "edit"]).current_dir(&dir);
        json(&run_with_stdin(&mut c, &request.to_string()))
    };
    assert_eq!(ask("docs/adr/ADR-9.md", PROPOSED)["record"], true);
    assert_eq!(ask("notes.txt", "hi")["record"], false);
    let v = ask(PROTECTED, "x");
    assert_eq!(v["verdict"], "block", "{v}");
    let v = ask(PATH_1, ACCEPTED);
    assert_eq!(v["verdict"], "block", "{v}");
}

#[test]
fn a_request_whose_contents_do_not_match_its_paths_is_refused_and_a_null_content_is_judged_by_path()
{
    let dir = fixture("contents-mismatch", &config(""));
    let ask =
        |paths: serde_json::Value, contents: serde_json::Value, content: serde_json::Value| {
            let request = serde_json::json!({
                "event": "edit", "tool": "mcp__x__w", "paths": paths, "contents": contents,
                "content": content, "actor": "agent", "cwd": dir.to_string_lossy(),
                "gates_only": true,
            });
            let mut c = urzua();
            c.args(["harness", "run", "edit"]).current_dir(&dir);
            json(&run_with_stdin(&mut c, &request.to_string()))
        };
    let benign = dir.join("notes.txt").to_string_lossy().to_string();
    let protected = dir.join(PROTECTED).to_string_lossy().to_string();
    let v = ask(
        serde_json::json!([benign, protected]),
        serde_json::json!(["x"]),
        serde_json::Value::Null,
    );
    assert_eq!(v["verdict"], "block", "{v}");
    let v = ask(
        serde_json::json!([benign, protected]),
        serde_json::json!([null, null]),
        serde_json::Value::Null,
    );
    assert_eq!(v["verdict"], "block", "{v}");
    let v = ask(
        serde_json::json!([benign, benign]),
        serde_json::json!(["a", "b"]),
        serde_json::Value::Null,
    );
    assert_eq!(v["verdict"], "allow", "{v}");
}

#[test]
fn the_installed_hook_entry_matches_notebook_edit_and_mcp_tools() {
    let fragment = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("bundle/claude-code/settings.fragment.json"),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&fragment).unwrap();
    let matcher = v["hooks"]["PreToolUse"][0]["matcher"].as_str().unwrap();
    assert_eq!(matcher, "Write|Edit|MultiEdit|NotebookEdit|mcp__.*");
    // An explicit timeout, longer than the host's 60 second default: a call may
    // carry up to eight record writes at several seconds each.
    let timeout = v["hooks"]["PreToolUse"][0]["hooks"][0]["timeout"].as_u64();
    assert_eq!(timeout, Some(120), "{v}");
}

// ------------------------- a gitignored protected file written by shell

fn ignored_fixture(name: &str) -> common::TempDir {
    let dir = fixture(name, &config(""));
    std::fs::write(
        dir.join(".gitignore"),
        ".urzua/cache/\n.claude/settings.local.json\n",
    )
    .unwrap();
    std::fs::write(dir.join(".claude/settings.local.json"), "{}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "ignore"]);
    dir
}

#[test]
fn a_bash_write_to_a_gitignored_protected_file_is_reported_once() {
    let dir = ignored_fixture("ignored");
    run_event(&dir, "session-start");
    assert_eq!(post(&dir)["verdict"], "allow", "nothing changed yet");
    bash(&dir, "echo '{\"hooks\": {}}' > .claude/settings.local.json");
    assert!(
        git(&dir, &["status", "--porcelain"]).trim().is_empty(),
        "git does not see it"
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let lines = v["protected_changes"].as_array().unwrap();
    assert_eq!(lines.len(), 1, "{v}");
    assert!(
        lines[0]
            .as_str()
            .unwrap()
            .contains(".claude/settings.local.json"),
        "{v}"
    );
    // The same change is not reported again (documented).
    assert_eq!(post(&dir)["verdict"], "allow", "{}", post(&dir));
    bash(&dir, "rm .claude/settings.local.json");
    let v = post(&dir);
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
    assert!(
        v["protected_changes"][0]
            .as_str()
            .unwrap()
            .contains("deleted"),
        "{v}"
    );
}

#[test]
fn a_new_gitignored_file_under_a_protected_directory_is_reported() {
    let dir = ignored_fixture("ignored-new");
    std::fs::write(
        dir.join(".gitignore"),
        ".urzua/cache/\n.claude/*.local.json\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "ignore more"]);
    run_event(&dir, "session-start");
    bash(&dir, "echo x > .claude/new.local.json");
    assert!(git(&dir, &["status", "--porcelain"]).trim().is_empty());
    let v = post(&dir);
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn an_unchanged_protected_set_is_quiet_and_off_without_the_guardrail() {
    let dir = ignored_fixture("ignored-quiet");
    run_event(&dir, "session-start");
    for _ in 0..3 {
        let v = post(&dir);
        assert_eq!(v["verdict"], "allow", "{v}");
        assert!(v["protected_changes"].as_array().unwrap().is_empty());
    }
    let cfg = config("").replace(
        "    protected-paths: {enabled: true}\n",
        "    protected-paths: {enabled: false}\n",
    );
    let dir = ignored_fixture("ignored-off");
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    git(&dir, &["commit", "-q", "-am", "off"]);
    run_event(&dir, "session-start");
    bash(&dir, "echo x > .claude/settings.local.json");
    assert_eq!(post(&dir)["verdict"], "allow");
}

// ------------------------- a command that removes the gate on the configuration

const SED_REMOVE_CONFIG: &str = "sed -i 's/, \".urzua\\/config.yaml\"//' .urzua/config.yaml";
const SED_REMOVE_CLAUDE: &str = "sed -i 's/\".claude\", //' .urzua/config.yaml";

fn lines_of(v: &serde_json::Value) -> Vec<String> {
    v["protected_changes"]
        .as_array()
        .unwrap_or_else(|| panic!("no protected_changes: {v}"))
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_command_that_removes_the_configuration_from_its_own_gate_is_reported() {
    let dir = fixture("config-self-removal", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, SED_REMOVE_CONFIG);
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let lines = lines_of(&v);
    assert!(
        lines.iter().any(|l| l.contains(".urzua/config.yaml")
            && l.contains("[protected-paths]")
            && l.contains("changed")),
        "{v}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("protected set shrank") && l.contains(".urzua/config.yaml")),
        "{v}"
    );
    // Reported once; the baseline moved.
    assert_eq!(post(&dir)["verdict"], "allow", "{}", post(&dir));
}

#[test]
fn the_removal_is_reported_when_git_is_told_to_ignore_the_configuration() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let dir = fixture("config-self-removal-hidden", &config(""));
        run_event(&dir, "session-start");
        git(&dir, &["update-index", flag, ".urzua/config.yaml"]);
        bash(&dir, SED_REMOVE_CONFIG);
        assert!(git(&dir, &["status", "--porcelain"]).trim().is_empty());
        let v = post(&dir);
        assert!(
            lines_of(&v)
                .iter()
                .any(|l| l.contains(".urzua/config.yaml") && l.contains("changed")),
            "{flag}: {v}"
        );
    }
}

#[test]
fn removing_a_gate_and_editing_under_it_in_one_command_reports_both() {
    let dir = fixture("gate-removal-and-edit", &config(""));
    run_event(&dir, "session-start");
    bash(
        &dir,
        &format!("{SED_REMOVE_CLAUDE} && echo '{{\"x\": 1}}' > .claude/settings.json"),
    );
    let lines = lines_of(&post(&dir));
    assert!(
        lines.iter().any(|l| l.contains(".claude/settings.json")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains(".urzua/config.yaml") && l.contains("changed")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("protected set shrank") && l.contains(".claude")),
        "{lines:?}"
    );
}

#[test]
fn a_gitignored_file_under_a_removed_gate_is_still_reported() {
    let dir = ignored_fixture("gate-removal-ignored");
    run_event(&dir, "session-start");
    bash(
        &dir,
        &format!("{SED_REMOVE_CLAUDE} && echo x > .claude/settings.local.json"),
    );
    let lines = lines_of(&post(&dir));
    assert!(
        lines
            .iter()
            .any(|l| l.contains(".claude/settings.local.json")),
        "{lines:?}"
    );
}

#[test]
fn session_start_reports_a_protected_set_that_shrank_before_it_reseeds() {
    let dir = fixture("session-shrink", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, SED_REMOVE_CONFIG);
    let v = run_event(&dir, "session-start");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("protected set shrank"),
        "{v}"
    );
    // Reseeded: a later session does not repeat it.
    let v = run_event(&dir, "session-start");
    assert!(
        !v["message"]
            .as_str()
            .unwrap()
            .contains("protected set shrank"),
        "{v}"
    );
}

// ------------------------- a command that removes a status gate or the acceptance guardrail

/// Two status gates, so removing one leaves a configuration that still loads.
fn two_gate_config() -> String {
    config("").replace(
        "    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n",
        "    - {type: adr, to: [Accepted], by: maintainers}\n    - {type: adr, to: [Rejected], by: maintainers}\n",
    )
}

const SED_REMOVE_STATUS_GATE: &str = "sed -i '/type: adr, to: \\[Accepted\\]/d' .urzua/config.yaml";
const SED_NARROW_STATUS_GATE: &str =
    "sed -i 's/to: \\[Accepted, Rejected\\]/to: [Rejected]/' .urzua/config.yaml";
const SED_DISABLE_ACCEPTANCE: &str =
    "sed -i 's/acceptance: {enabled: true}/acceptance: {enabled: false}/' .urzua/config.yaml";
const SED_ACCEPT: &str = "sed -i 's/Status: Proposed/Status: Accepted/' docs/adr/ADR-1-a.md";

fn gated_moves(v: &serde_json::Value) -> Vec<String> {
    v["gated_moves"]
        .as_array()
        .unwrap_or_else(|| panic!("no gated_moves: {v}"))
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn removing_a_status_gate_and_moving_a_record_in_one_command_reports_the_move() {
    let dir = fixture("status-gate-removal", &two_gate_config());
    run_event(&dir, "session-start");
    bash(&dir, &format!("{SED_REMOVE_STATUS_GATE} && {SED_ACCEPT}"));
    let v = post(&dir);
    let moves = gated_moves(&v);
    assert!(
        moves
            .iter()
            .any(|l| l.contains("ADR-1-a.md") && l.contains("[acceptance]")),
        "{v}"
    );
    assert!(
        lines_of(&v)
            .iter()
            .any(|l| l.contains("status gate") && l.contains("type adr")),
        "{v}"
    );
}

#[test]
fn narrowing_a_status_gate_and_moving_a_record_to_the_dropped_status_reports_the_move() {
    let dir = fixture("status-gate-narrow", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, &format!("{SED_NARROW_STATUS_GATE} && {SED_ACCEPT}"));
    let v = post(&dir);
    assert_eq!(gated_moves(&v).len(), 1, "{v}");
    assert!(
        lines_of(&v).iter().any(|l| l.contains("status gate")),
        "{v}"
    );
}

#[test]
fn disabling_acceptance_and_moving_a_record_in_one_command_reports_the_move() {
    let dir = fixture("acceptance-off", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, &format!("{SED_DISABLE_ACCEPTANCE} && {SED_ACCEPT}"));
    let v = post(&dir);
    assert_eq!(gated_moves(&v).len(), 1, "{v}");
    assert!(lines_of(&v).iter().any(|l| l.contains("acceptance")), "{v}");
}

#[test]
fn a_status_gate_removed_in_an_earlier_command_is_reported_as_shrunk_once() {
    let dir = fixture("status-gate-earlier", &two_gate_config());
    run_event(&dir, "session-start");
    bash(&dir, SED_REMOVE_STATUS_GATE);
    let v = post(&dir);
    assert!(
        lines_of(&v).iter().any(|l| l.contains("status gate")),
        "{v}"
    );
    assert_eq!(post(&dir)["verdict"], "allow");
}

#[test]
fn session_start_reports_a_status_gate_that_shrank_before_it_reseeds() {
    let dir = fixture("status-session-shrink", &two_gate_config());
    run_event(&dir, "session-start");
    bash(&dir, SED_REMOVE_STATUS_GATE);
    let v = run_event(&dir, "session-start");
    assert!(
        v["message"].as_str().unwrap().contains("status gate"),
        "{v}"
    );
}

#[test]
fn an_unchanged_status_gate_reports_no_shrink() {
    let dir = fixture("status-gate-kept", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, "echo '# note' >> .urzua/config.yaml");
    let v = post(&dir);
    assert!(!lines_of(&v).iter().any(|l| l.contains("shrank")), "{v}");
}

#[test]
fn widening_the_decider_group_of_a_status_gate_is_not_shrinkage_but_the_move_and_the_configuration_are_reported(
) {
    let cfg = config("").replace(
        "maintainers: {logins: [Alice]}",
        "maintainers: {logins: [Alice]}\n    everyone: {logins: [Alice, Bob]}",
    );
    let dir = fixture("status-gate-widened", &cfg);
    run_event(&dir, "session-start");
    bash(
        &dir,
        &format!(
            "sed -i 's/to: \\[Accepted, Rejected\\], by: maintainers/to: [Accepted, Rejected], by: everyone/' .urzua/config.yaml && {SED_ACCEPT}"
        ),
    );
    let v = post(&dir);
    // The move is reported, naming the widened group the gate now has.
    let moves = gated_moves(&v);
    assert_eq!(moves.len(), 1, "{v}");
    assert!(moves[0].contains("everyone"), "{moves:?}");
    // The configuration is a protected path, so its change is reported.
    let protected: Vec<&str> = v["protected_changes"]
        .as_array()
        .unwrap_or_else(|| panic!("no protected_changes: {v}"))
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(
        protected.iter().any(|l| l.contains(".urzua/config.yaml")),
        "{v}"
    );
    // The gate still covers the status, so nothing is said to have shrunk.
    assert!(
        !lines_of(&v).iter().any(|l| l.contains("shrank")),
        "a change of `by` is not shrinkage: {v}"
    );
}

#[test]
fn a_change_reported_in_an_earlier_session_and_redone_in_the_next_is_reported_again() {
    let dir = fixture("session-forgets", &config(""));
    run_event(&dir, "session-start");
    bash(&dir, SED_ACCEPT);
    assert_eq!(gated_moves(&post(&dir)).len(), 1);
    // A person reverts it outside the agent; no post-command check sees that.
    git(&dir, &["checkout", "--", PATH_1]);
    run_event(&dir, "session-start");
    bash(&dir, SED_ACCEPT);
    let v = post(&dir);
    assert_eq!(gated_moves(&v).len(), 1, "{v}");
}

// ----------------------------------------------- Unicode case folding

const UNICODE_TYPES: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/décisions\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const UNICODE_AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \"café\", \"Straße\"], by: maintainers}\n";

fn unicode_fixture(name: &str) -> common::TempDir {
    let cfg = format!("{UNICODE_TYPES}{}{UNICODE_AUTHORITY}", harness(""));
    let dir = common::TempDir::new(&format!("urzua-hardening-{name}"));
    for d in ["docs/décisions", ".urzua", ".claude", "café"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    std::fs::write(dir.join("café/a.txt"), "x\n").unwrap();
    std::fs::write(dir.join("docs/décisions/ADR-1-a.md"), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn a_protected_path_or_record_directory_in_another_unicode_case_is_still_gated() {
    let dir = unicode_fixture("unicode");
    // Non-ASCII letters, and a sharp s that folds to "ss".
    for path in ["CAFÉ/b.txt", "Café/b.txt", "STRASSE/x", "STRASSE"] {
        let (code, v) = run_edit(&dir, path, "x\n");
        assert_eq!(code, 1, "{path}: {v}");
        assert!(
            v["message"].as_str().unwrap().contains("protected-paths"),
            "{v}"
        );
    }
    let (code, v) = run_edit(&dir, "DOCS/DÉCISIONS/ADR-1-a.md", ACCEPTED);
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("acceptance"), "{v}");
    // A different letter is a different path.
    let (code, v) = run_edit(&dir, "cafe/b.txt", "x\n");
    assert_eq!(code, 0, "{v}");
}

#[test]
fn a_protected_path_changed_by_shell_in_another_unicode_case_is_reported() {
    let dir = unicode_fixture("unicode-bash");
    bash(&dir, "mkdir -p CAFÉ && echo x > CAFÉ/new.txt");
    let v = post(&dir);
    assert_eq!(v["protected_changes"].as_array().unwrap().len(), 1, "{v}");
}

// ----------------------------------------------------------- hardlinks

#[cfg(unix)]
#[test]
fn an_edit_through_a_hardlink_is_judged_as_the_file_it_writes() {
    let dir = fixture("hardlink", &config(""));
    std::fs::write(dir.join(".claude/settings.local.json"), "{}\n").unwrap();
    std::fs::hard_link(dir.join(".claude/settings.json"), dir.join("alias.json")).unwrap();
    std::fs::hard_link(
        dir.join(".claude/settings.local.json"),
        dir.join("alias2.json"),
    )
    .unwrap();
    std::fs::hard_link(dir.join(PATH_1), dir.join("docs/alias.md")).unwrap();
    for path in ["alias.json", "alias2.json"] {
        let (code, v) = run_edit(&dir, path, "{}\n");
        assert_eq!(code, 1, "{path}: {v}");
        assert!(
            v["message"].as_str().unwrap().contains("protected-paths"),
            "{v}"
        );
    }
    let (code, v) = run_edit(&dir, "docs/alias.md", ACCEPTED);
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("acceptance"), "{v}");
    // A file with other links that are not gated is judged as itself.
    std::fs::write(dir.join("a.txt"), "x\n").unwrap();
    std::fs::hard_link(dir.join("a.txt"), dir.join("b.txt")).unwrap();
    let (code, v) = run_edit(&dir, "b.txt", "y\n");
    assert_eq!(code, 0, "{v}");
}

fn many_files(dir: &Path, count: usize, protected_at: usize) -> serde_json::Value {
    let items: Vec<serde_json::Value> = (0..count)
        .map(|n| {
            let file = if n == protected_at {
                PROTECTED.to_string()
            } else {
                format!("notes/n{n}.txt")
            };
            serde_json::json!({"path": dir.join(file).to_string_lossy(), "content": "x"})
        })
        .collect();
    serde_json::json!({"files": items})
}

#[test]
fn the_gates_of_a_call_with_hundreds_of_files_are_one_engine_process() {
    let dir = fixture("mcp-one-gates-process", &config(""));
    let log = dir.join("requests.log");
    // 500 ordinary files and the protected one last: judged by one request.
    let v = translate_with(
        &dir,
        &mcp(&dir, "mcp__x__write_files", many_files(&dir, 500, 499)),
        &logging_engine(&log),
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let requests = logged(&log);
    assert_eq!(requests.len(), 1, "one gates request: {}", requests.len());
    assert!(requests[0].contains("gates_only"), "{}", requests[0]);
    // Each file carries its own content in the request.
    assert!(requests[0].contains("\"contents\""), "{}", requests[0]);
}

#[test]
fn two_thousand_files_with_the_protected_one_last_cost_one_gates_request_and_one_for_the_overflow()
{
    // The cost of a call is the number of engine requests it makes, not the
    // time they take: one gates request for the files that are read, one
    // request for the rest, never one per file.
    let dir = fixture("mcp-2200", &config(""));
    let log = dir.join("requests.log");
    let v = translate_with(
        &dir,
        &mcp(&dir, "mcp__x__write_files", many_files(&dir, 2200, 2199)),
        &logging_engine(&log),
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let requests = logged(&log);
    assert_eq!(requests.len(), 2, "{} requests", requests.len());
    assert!(requests[0].contains("gates_only"), "{}", requests[0]);
    assert!(
        requests[1].contains("adapter_error") && !requests[1].contains("gates_only"),
        "the overflow is one request: {}",
        requests[1]
    );
    // The protected file inside the files that are read is refused by the
    // first request, and nothing else is asked.
    let log = dir.join("requests-inside.log");
    let v = translate_with(
        &dir,
        &mcp(&dir, "mcp__x__write_files", many_files(&dir, 2200, 100)),
        &logging_engine(&log),
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    assert!(
        v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("protected"),
        "{v}"
    );
    let requests = logged(&log);
    assert_eq!(requests.len(), 1, "{} requests", requests.len());
    assert!(requests[0].contains("gates_only"), "{}", requests[0]);
}

#[test]
fn files_beyond_the_cap_are_unjudged_and_refused_under_deny_allowed_loudly_otherwise() {
    let dir = fixture("mcp-item-cap", &config(""));
    // 513 files with the protected one as the last: it is beyond what is read,
    // so the call is an edit that was not judged, and the protected path
    // it names makes the guardrail refuse it.
    let v = translate(
        &dir,
        &mcp(&dir, "mcp__x__write_files", many_files(&dir, 513, 512)),
        &ENGINE,
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap();
    assert!(reason.contains("NOT judge"), "{reason}");
    // The 512 that are read are judged in full, and none is protected.
    let v = translate(
        &dir,
        &mcp(&dir, "mcp__x__write_files", many_files(&dir, 512, 9999)),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    assert!(v.is_null(), "{v}");
    // Where unjudged edits are allowed, the overflow is allowed with a notice.
    let open = fixture("mcp-item-cap-open", &config(", on_unjudged: allow"));
    let v = translate(
        &open,
        &mcp(&open, "mcp__x__write_files", many_files(&open, 513, 512)),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

#[test]
fn a_gated_status_among_many_files_is_judged_with_its_own_content() {
    let dir = fixture("mcp-batch-contents", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    // Another file's content must not stand in for the record's.
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_files",
            serde_json::json!({"files": [
                {"path": abs("notes.txt"), "content": "hi"},
                {"path": abs(PATH_1), "content": ACCEPTED},
            ]}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_files",
            serde_json::json!({"files": [
                {"path": abs("notes.txt"), "content": ACCEPTED},
                {"path": abs(PATH_1), "content": PROPOSED},
            ]}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
}

#[test]
fn an_unjudged_notice_names_urzua_once() {
    let dir = fixture("mcp-notice-once", &config(", on_unjudged: allow"));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__fs__edit_file",
            serde_json::json!({"path": abs("notes.txt"), "edits": [{"oldText": "a", "newText": "b"}]}),
        ),
        &ENGINE,
    );
    let message = v["systemMessage"].as_str().unwrap();
    assert_eq!(
        message.matches("urzua did NOT judge").count(),
        1,
        "{message}"
    );
    assert!(!message.contains("(urzua harness"), "{message}");
    // Refused under deny, the reason reads once too.
    let closed = fixture("mcp-notice-once-deny", &config(""));
    let v = translate(
        &closed,
        &mcp(
            &closed,
            "mcp__fs__edit_file",
            serde_json::json!({"path": closed.join(PROTECTED).to_string_lossy(), "edits": [{"oldText": "a", "newText": "b"}]}),
        ),
        &ENGINE,
    );
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap();
    assert_eq!(reason.matches("urzua did NOT judge").count(), 1, "{reason}");
    assert!(!reason.contains("(urzua harness"), "{reason}");
}

// ============================================================ sixth pass

#[test]
fn an_mcp_path_starting_with_tilde_or_home_is_the_home_directory() {
    // The project sits under HOME, as a filesystem MCP server would resolve it.
    let dir = fixture("mcp-tilde", &config(""));
    let home = dir.parent().unwrap().to_path_buf();
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    for spelled in [
        format!("~/{name}/.claude/settings.json"),
        format!("$HOME/{name}/.claude/settings.json"),
        format!("${{HOME}}/{name}/.claude/./settings.json"),
        format!("~/{name}/docs/../.claude/settings.json"),
    ] {
        let payload = mcp(
            &dir,
            "mcp__fs__write_file",
            serde_json::json!({"path": spelled, "content": "{}"}),
        );
        let v = translate_home(&dir, &payload, &ENGINE, Some(&home));
        assert_eq!(decision(&v), Some("deny"), "{spelled}: {v}");
        // The same text in a list of paths.
        let payload = mcp(
            &dir,
            "mcp__fs__move_file",
            serde_json::json!({"source": "a.txt", "destination": spelled}),
        );
        let v = translate_home(&dir, &payload, &ENGINE, Some(&home));
        assert_eq!(decision(&v), Some("deny"), "move to {spelled}: {v}");
    }
    // An ordinary file under the home directory stays allowed.
    let payload = mcp(
        &dir,
        "mcp__fs__write_file",
        serde_json::json!({"path": format!("~/{name}/notes.txt"), "content": "x"}),
    );
    let v = translate_home(&dir, &payload, &ENGINE, Some(&home));
    assert_eq!(decision(&v), None, "{v}");
}

#[test]
fn a_path_nested_under_a_content_key_is_judged_and_a_string_there_is_still_content() {
    let dir = fixture("mcp-nested-content", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    for key in [
        "data", "payload", "value", "body", "contents", "text", "content",
    ] {
        let as_dict = serde_json::json!({key: {"path": abs(".claude/x.json"), "content": "{}"}});
        let as_list = serde_json::json!({key: [{"path": abs(".claude/x.json"), "content": "{}"}]});
        for input in [as_dict, as_list] {
            let v = translate(&dir, &mcp(&dir, "mcp__x__save", input.clone()), &ENGINE);
            assert_eq!(decision(&v), Some("deny"), "{key}: {input}: {v}");
        }
        // A gated record's move nested the same way.
        let input = serde_json::json!({key: {"path": abs(PATH_1), "content": ACCEPTED}});
        let v = translate(&dir, &mcp(&dir, "mcp__x__save", input.clone()), &ENGINE);
        assert_eq!(decision(&v), Some("deny"), "{key}: {input}: {v}");
        // A string under the key is content, not a path to judge.
        let input = serde_json::json!({"path": abs("notes.txt"), key: abs(".claude/x.json")});
        let v = translate(&dir, &mcp(&dir, "mcp__x__save", input.clone()), &ENGINE);
        assert_eq!(decision(&v), None, "{key}: {input}: {v}");
    }
}

#[test]
fn a_gated_record_with_an_upper_case_extension_is_judged_as_a_record() {
    let dir = fixture("record-extension-case", &config(""));
    for name in ["ADR-2-b.MD", "ADR-2-b.Md", "ADR-2-b.mD"] {
        let (code, v) = run_edit(&dir, &format!("docs/adr/{name}"), ACCEPTED);
        assert_eq!(code, 1, "{name}: {v}");
        assert_eq!(v["verdict"], "block", "{name}: {v}");
    }
    let (code, v) = run_edit(&dir, "docs/adr/ADR-2-b.MD", PROPOSED);
    assert_eq!(code, 0, "{v}");
}

// `urzua mcp` and the editor hook (SPEC-26, SPEC-19): the hook reads an MCP call
// as a write by the words in its tool name, so the server's names are load-bearing.

/// The tool names `urzua mcp` advertises, read from the real server.
fn urzua_mcp_tool_names(dir: &Path) -> Vec<String> {
    let mut c = common::urzua();
    c.arg("mcp").current_dir(dir);
    let out = run_with_stdin(
        &mut c,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
    );
    let reply: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    reply["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn every_urzua_mcp_tool_is_left_alone_by_the_editor_hook_even_when_it_names_a_gated_path() {
    let dir = fixture("mcp-urzua-read", &config(""));
    let names = urzua_mcp_tool_names(&dir);
    assert_eq!(names.len(), 5, "{names:?}");
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    for name in names {
        let tool = format!("mcp__urzua__{name}");
        for target in [PATH_1, ".claude/settings.json"] {
            let mut c = Command::new("python3");
            c.arg("-I")
                .arg(translator())
                .args(["edit", "--"])
                .args(ENGINE)
                .current_dir(&dir)
                .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
            let input = serde_json::json!({"path": abs(target), "id": "ADR-1"});
            let out = run_with_stdin(&mut c, &mcp(&dir, &tool, input));
            assert_eq!(out.status.code(), Some(0), "{tool}: {out:?}");
            assert!(
                out.stdout.is_empty(),
                "{tool} on {target} was judged: {out:?}"
            );
        }
    }
}

#[test]
fn a_planted_write_named_urzua_mcp_tool_is_judged_as_a_write() {
    // The other half of the pair above: it shows the hook does read the name, so
    // the read-named tools passing is not the hook ignoring `mcp__urzua__*`.
    let dir = fixture("mcp-urzua-write", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    for tool in [
        "mcp__urzua__urzua_write_record",
        "mcp__urzua__urzua_fix_apply",
    ] {
        let v = translate(
            &dir,
            &mcp(
                &dir,
                tool,
                serde_json::json!({"path": abs(PATH_1), "content": ACCEPTED}),
            ),
            &ENGINE,
        );
        assert_eq!(decision(&v), Some("deny"), "{tool}: {v}");
        let v = translate(
            &dir,
            &mcp(
                &dir,
                tool,
                serde_json::json!({"path": abs(".claude/settings.json"), "content": "{}"}),
            ),
            &ENGINE,
        );
        assert_eq!(decision(&v), Some("deny"), "{tool}: {v}");
    }
}

// ================================================================ review 7
// The harness findings of an external review: a request the engine cannot
// parse, an MCP decoy content key, a working directory that cannot be entered,
// the kill switch in `status`, and the cost of judging a call with many paths.

/// `urzua harness run edit` with `raw` on stdin, from `dir`.
fn run_edit_raw(dir: &Path, raw: &str) -> (i32, serde_json::Value) {
    let mut c = urzua();
    c.args(["harness", "run", "edit"]).current_dir(dir);
    let out = run_with_stdin(&mut c, raw);
    (out.status.code().unwrap(), json(&out))
}

/// A copy of the translator installed in `dir` (`<dir>/.claude/hooks/`), so its
/// project directory is `dir`.
fn install_translator(dir: &Path) -> PathBuf {
    let hooks = dir.join(".claude/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let script = hooks.join("urzua-claude-code.py");
    std::fs::copy(translator(), &script).unwrap();
    script
}

/// The installed translator run with `payload`, from `process_cwd`.
fn translate_installed(script: &Path, process_cwd: &Path, payload: &str) -> serde_json::Value {
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(script)
        .args(["edit", "--"])
        .args(ENGINE)
        .current_dir(process_cwd)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    json(&out)
}

#[test]
fn a_request_the_engine_cannot_parse_is_refused_where_a_gate_denies_and_allowed_loudly_otherwise() {
    // A lone surrogate escape is valid for the translator's JSON and not for the
    // engine's: the request cannot be read, so nothing was judged.
    let raw = |dir: &Path| {
        format!(
            "{{\"event\":\"edit\",\"tool\":\"Write\",\"paths\":[\"{}\"],\"content\":\"x\\udfff\",\"actor\":\"agent\",\"cwd\":\"{}\"}}",
            dir.join("notes.txt").to_string_lossy(),
            dir.to_string_lossy()
        )
    };
    let dir = fixture("surrogate-closed", &config(""));
    let (code, v) = run_edit_raw(&dir, &raw(&dir));
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("on_unjudged"),
        "{v}"
    );
    // Where the configuration allows an unjudged edit, it is allowed, loudly.
    let dir = fixture("surrogate-open", &config(", on_unjudged: allow"));
    let (code, v) = run_edit_raw(&dir, &raw(&dir));
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    // No denying gate: the editor hook's own policy stands (allow by default).
    let off = format!("{TYPES}harness:\n  guardrails:\n    editor-hook: {{enabled: true}}\n");
    let dir = fixture("surrogate-off", &off);
    let (code, v) = run_edit_raw(&dir, &raw(&dir));
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
}

#[test]
fn a_lone_surrogate_in_a_tool_input_is_refused_by_the_translator_where_a_gate_denies() {
    let payload = |dir: &Path| {
        format!(
            "{{\"tool_name\":\"Write\",\"cwd\":\"{}\",\"tool_input\":{{\"file_path\":\"{}\",\"content\":\"x\\udfff\"}}}}",
            dir.to_string_lossy(),
            dir.join("notes.txt").to_string_lossy()
        )
    };
    let dir = fixture("surrogate-translated", &config(""));
    let v = translate(&dir, &payload(&dir), &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let open = fixture("surrogate-translated-open", &config(", on_unjudged: allow"));
    let v = translate(&open, &payload(&open), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
}

#[test]
fn an_mcp_call_with_more_than_one_candidate_content_string_is_unplaceable() {
    let dir = fixture("mcp-decoy", &config(""));
    let abs = |f: &str| dir.join(f).to_string_lossy().to_string();
    // The accepted status is in the string a server may write; the one the
    // translator would judge is harmless.
    for (first, second) in [("content", "text"), ("text", "content"), ("body", "data")] {
        let mut input = serde_json::Map::new();
        input.insert("path".into(), abs(PATH_1).into());
        input.insert(first.into(), PROPOSED.into());
        input.insert(second.into(), ACCEPTED.into());
        let v = denied_mcp(&dir, "mcp__x__write_file", input.into());
        let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap();
        assert!(reason.contains("NOT judge"), "{first}/{second}: {reason}");
    }
    // The same string twice is one candidate: judged as the whole-file write.
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_file",
            serde_json::json!({"path": abs(PATH_1), "content": ACCEPTED, "text": ACCEPTED}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), Some("deny"), "judged as the gated write: {v}");
    assert!(
        v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("refused"),
        "{v}"
    );
    // Not a gated target: allowed with a notice. Where unjudged edits are
    // allowed, a gated one is too.
    let v = translate(
        &dir,
        &mcp(
            &dir,
            "mcp__x__write_file",
            serde_json::json!({"path": abs("notes.txt"), "content": "a", "text": "b"}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
    let open = fixture("mcp-decoy-open", &config(", on_unjudged: allow"));
    let v = translate(
        &open,
        &mcp(
            &open,
            "mcp__x__write_file",
            serde_json::json!({"path": open.join(PATH_1).to_string_lossy(), "content": PROPOSED, "text": ACCEPTED}),
        ),
        &ENGINE,
    );
    assert_eq!(decision(&v), None, "{v}");
}

#[test]
fn a_cwd_that_cannot_be_entered_is_replaced_by_the_project_directory_or_refused_under_deny() {
    let dir = fixture("bad-cwd", &config(""));
    let elsewhere = common::TempDir::new("urzua-hardening-bad-cwd-elsewhere");
    let protected = dir
        .join(".claude/settings.json")
        .to_string_lossy()
        .to_string();
    let request = |cwd: &str, project_dir: Option<&Path>, path: &str| {
        let mut r = serde_json::json!({
            "event": "edit", "tool": "Write", "paths": [path], "content": "{}\n",
            "actor": "agent", "cwd": cwd,
        });
        if let Some(p) = project_dir {
            r["project_dir"] = p.to_string_lossy().to_string().into();
        }
        r.to_string()
    };
    // With the project directory, the edit is judged there: the gate refuses.
    let (code, v) = run_edit_raw(
        &elsewhere,
        &request("/nonexistent-urzua-cwd", Some(&dir), &protected),
    );
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("refused"), "{v}");
    // A relative path is the project directory's file, not the process's.
    let (code, v) = run_edit_raw(
        &elsewhere,
        &request(
            "/nonexistent-urzua-cwd",
            Some(&dir),
            ".claude/settings.json",
        ),
    );
    assert_eq!(code, 1, "{v}");
    // With neither, the edit was not judged: refused where a gate denies.
    let (code, v) = run_edit_raw(&dir, &request("/nonexistent-urzua-cwd", None, "notes.txt"));
    assert_eq!(code, 1, "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("cannot be entered"),
        "{v}"
    );
    // Allowed loudly where the configuration allows an unjudged edit.
    let open = fixture("bad-cwd-open", &config(", on_unjudged: allow"));
    let (code, v) = run_edit_raw(&open, &request("/nonexistent-urzua-cwd", None, "notes.txt"));
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
}

#[test]
fn the_translator_runs_the_engine_in_the_project_directory_when_the_payload_cwd_is_unusable() {
    let dir = fixture("bad-cwd-translated", &config(""));
    let script = install_translator(&dir);
    let elsewhere = common::TempDir::new("urzua-hardening-bad-cwd-translated-elsewhere");
    for cwd in [
        serde_json::json!("/nonexistent-urzua-cwd"),
        serde_json::json!(dir.join("notes.txt").to_string_lossy()),
    ] {
        let payload = serde_json::json!({
            "tool_name": "Write", "cwd": cwd,
            "tool_input": {"file_path": ".claude/settings.json", "content": "{}\n"},
        })
        .to_string();
        let v = translate_installed(&script, &elsewhere, &payload);
        assert_eq!(decision(&v), Some("deny"), "{cwd}: {v}");
    }
    // The engine cannot run and the cwd is unusable: the configuration text is
    // read from the project directory, so a denying gate still refuses.
    let payload = serde_json::json!({
        "tool_name": "Write", "cwd": "/nonexistent-urzua-cwd",
        "tool_input": {"file_path": "notes.txt", "content": "x"},
    })
    .to_string();
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(&script)
        .args(["edit", "--", "urzua-no-such-binary"])
        .current_dir(&elsewhere);
    let out = run_with_stdin(&mut c, &payload);
    let v = json(&out);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    // An open configuration allows it with a notice.
    let open = fixture("bad-cwd-translated-open", &config(", on_unjudged: allow"));
    let script = install_translator(&open);
    let v = translate_installed(&script, &elsewhere, &payload);
    assert_eq!(decision(&v), None, "{v}");
}

/// A request naming `count` ungated files and, as the last, `last`.
fn engine_paths(dir: &Path, count: usize, last: &str) -> String {
    let mut paths: Vec<String> = (0..count - 1)
        .map(|n| {
            dir.join(format!("notes/n{n}.txt"))
                .to_string_lossy()
                .to_string()
        })
        .collect();
    paths.push(dir.join(last).to_string_lossy().to_string());
    serde_json::json!({
        "event": "edit", "tool": "x", "paths": paths, "content": "x", "actor": "agent",
        "cwd": dir.to_string_lossy(), "gates_only": true,
    })
    .to_string()
}

#[test]
fn the_engine_judges_a_capped_number_of_paths_and_refuses_the_overflow_under_deny() {
    let dir = fixture("engine-cap", &config(""));
    // The protected path is inside the cap: the gate refuses it.
    let (code, v) = run_edit_raw(&dir, &engine_paths(&dir, 512, PROTECTED));
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("refused"), "{v}");
    // Beyond the cap it is not judged by the gates, so the call is an edit that
    // was not judged and the denying guardrail that covers it refuses.
    let (code, v) = run_edit_raw(&dir, &engine_paths(&dir, 513, PROTECTED));
    assert_eq!(code, 1, "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("more than the 512"),
        "{v}"
    );
    // Within the cap and ungated: judged, allowed.
    let (code, v) = run_edit_raw(&dir, &engine_paths(&dir, 512, "notes/last.txt"));
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
    // Over the cap and ungated: allowed, loudly.
    let (code, v) = run_edit_raw(&dir, &engine_paths(&dir, 600, "notes/last.txt"));
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    assert!(
        v["message"].as_str().unwrap().contains("more than the 512"),
        "{v}"
    );
    // Where unjudged edits are allowed, the overflow is allowed, loudly.
    let open = fixture("engine-cap-open", &config(", on_unjudged: allow"));
    let (code, v) = run_edit_raw(&open, &engine_paths(&open, 513, PROTECTED));
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
}
