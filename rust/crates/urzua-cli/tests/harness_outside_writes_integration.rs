//! `harness.outside_writes` (ADR-89 amendment of 2026-11-09): an edit-tool write
//! into a linked worktree of the session's repository at an old commit that
//! holds no gates, a fresh clone, and the `deny` mode with `allow_paths`. Real
//! git repositories and the real binary. Needs `git`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TYPES: &str = "schema_version: 2\nrules:\n  header.required-fields: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".github/workflows\", \".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const ADR: &str = "docs/adr/ADR-1-a.md";

fn session_config(extra: &str) -> String {
    format!("{TYPES}harness:\n{extra}  guardrails:\n    acceptance: {{enabled: true}}\n    protected-paths: {{enabled: true}}\n    editor-hook: {{enabled: true}}\n{AUTHORITY}")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `P/proj`: a first commit with an ungoverned configuration (no `harness`, no
/// `authority`), then a second that adds the gates. Returns the parent, the
/// project and the first commit.
fn project(name: &str, extra: &str) -> (common::TempDir, PathBuf, String) {
    let parent = common::TempDir::new(&format!("urzua-outside-{name}"));
    let proj = parent.join("proj");
    for d in ["docs/adr", ".urzua", ".claude", ".github/workflows"] {
        std::fs::create_dir_all(proj.join(d)).unwrap();
    }
    git(&proj, &["init", "-q"]);
    git(&proj, &["config", "user.email", "t@example.com"]);
    git(&proj, &["config", "user.name", "t"]);
    std::fs::write(proj.join(".urzua/config.yaml"), TYPES).unwrap();
    std::fs::write(proj.join(ADR), PROPOSED).unwrap();
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "old"]);
    let old = git(&proj, &["rev-parse", "HEAD"]);
    std::fs::write(proj.join(".urzua/config.yaml"), session_config(extra)).unwrap();
    std::fs::write(proj.join(".claude/settings.json"), "{}\n").unwrap();
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "gates"]);
    (parent, proj, old)
}

fn worktree(parent: &Path, proj: &Path, old: &str) -> PathBuf {
    let wt = parent.join("wt");
    git(
        proj,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            old,
        ],
    );
    wt
}

fn run_edit(proj: &Path, path: &Path, content: &str) -> (i32, serde_json::Value, String) {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path.to_string_lossy()], "content": content,
        "actor": "agent", "cwd": proj.to_string_lossy(), "project_dir": proj.to_string_lossy(),
    });
    let mut c = common::urzua();
    c.args(["harness", "run", "edit"])
        .current_dir(proj)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    (
        out.status.code().unwrap(),
        serde_json::from_str(&text).unwrap(),
        text,
    )
}

fn verdict(v: &serde_json::Value) -> &str {
    v["verdict"].as_str().unwrap()
}

#[test]
fn shared_denies_protected_paths_and_an_accepted_adr_in_a_linked_worktree() {
    let (parent, proj, old) = project("shared", "  outside_writes: shared\n");
    let wt = worktree(&parent, &proj, &old);
    // The worktree is at a commit without gates; the session's own are applied.
    for (rel, content) in [
        (".github/workflows/x.yml", "name: x\n"),
        (".claude/settings.json", "{}\n"),
        (ADR, ACCEPTED),
    ] {
        let (code, v, _) = run_edit(&proj, &wt.join(rel), content);
        assert_eq!(code, 1, "{rel}: {v}");
        assert_eq!(verdict(&v), "block", "{rel}: {v}");
        assert!(
            v["message"].as_str().unwrap().contains("linked worktree"),
            "{v}"
        );
    }
    // An unprotected file and an unmoved record stay allowed.
    let (code, v, _) = run_edit(&proj, &wt.join("notes.txt"), "x\n");
    assert_eq!((code, verdict(&v)), (0, "allow-unjudged"), "{v}");
    let (code, v, _) = run_edit(&proj, &wt.join(ADR), PROPOSED);
    assert_eq!(code, 0, "{v}");
}

#[test]
fn allow_and_undeclared_leave_a_worktree_write_unjudged_and_agree_with_each_other() {
    let (parent, proj, old) = project("allow", "  outside_writes: allow\n");
    let wt = worktree(&parent, &proj, &old);
    let (c1, v1, t1) = run_edit(&proj, &wt.join(".github/workflows/x.yml"), "name: x\n");
    assert_eq!((c1, verdict(&v1)), (0, "allow-unjudged"), "{v1}");
    // Undeclared: same repository without the key.
    let (parent2, proj2, old2) = project("undeclared", "");
    let wt2 = worktree(&parent2, &proj2, &old2);
    let (c2, v2, t2) = run_edit(&proj2, &wt2.join(".github/workflows/x.yml"), "name: x\n");
    assert_eq!((c2, verdict(&v2)), (0, "allow-unjudged"), "{v2}");
    assert_eq!(
        t1.replace(parent.to_str().unwrap(), "P"),
        t2.replace(parent2.to_str().unwrap(), "P")
    );
}

#[test]
fn a_fresh_clone_is_an_accepted_risk_under_shared() {
    let (parent, proj, old) = project("clone", "  outside_writes: shared\n");
    let clone = parent.join("clone");
    git(
        &parent,
        &[
            "clone",
            "-q",
            proj.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    git(&clone, &["checkout", "-q", "--detach", &old]);
    let (code, v, _) = run_edit(&proj, &clone.join(".github/workflows/x.yml"), "name: x\n");
    assert_eq!((code, verdict(&v)), (0, "allow-unjudged"), "{v}");
}

#[test]
fn deny_refuses_an_outside_write_and_allows_one_under_allow_paths() {
    let (parent, proj, _) = project(
        "deny",
        &format!(
            "  outside_writes: deny\n  allow_paths: [\"{}\"]\n",
            "/placeholder"
        ),
    );
    // Rewrite allow_paths to the real scratch directory.
    let scratch = parent.join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let cfg = session_config(&format!(
        "  outside_writes: deny\n  allow_paths: [\"{}\"]\n",
        scratch.display()
    ));
    std::fs::write(proj.join(".urzua/config.yaml"), cfg).unwrap();
    let other = parent.join("other");
    std::fs::create_dir_all(&other).unwrap();
    let (code, v, _) = run_edit(&proj, &other.join("a.txt"), "x\n");
    assert_eq!((code, verdict(&v)), (1, "block"), "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("allow_paths"),
        "{v}"
    );
    let (code, v, _) = run_edit(&proj, &scratch.join("a.txt"), "x\n");
    assert_eq!(code, 0, "{v}");
    // A write inside the repository is untouched by the mode.
    let (code, v, _) = run_edit(&proj, &proj.join("notes.txt"), "x\n");
    assert_eq!(code, 0, "{v}");
}

#[test]
fn an_unknown_outside_writes_value_stops_the_configuration_loading() {
    let (_parent, proj, _) = project("bad", "  outside_writes: sharde\n");
    let out = common::urzua()
        .args(["check"])
        .current_dir(&proj)
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0), "{out:?}");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("outside_writes"), "{text}");
}

#[test]
fn doctor_warns_when_a_guardrail_is_on_and_the_key_is_undeclared_and_not_otherwise() {
    let doctor = |dir: &Path| {
        let out = common::urzua()
            .arg("doctor")
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    let (_p1, undeclared, _) = project("doctor-undeclared", "");
    let text = doctor(&undeclared);
    assert!(text.contains("harness-outside-writes"), "{text}");
    assert!(text.contains("outside_writes: shared"), "{text}");
    for mode in ["shared", "allow", "deny"] {
        let (_p, proj, _) = project(
            &format!("doctor-{mode}"),
            &format!("  outside_writes: {mode}\n"),
        );
        assert!(!doctor(&proj).contains("harness-outside-writes"), "{mode}");
    }
}

#[test]
fn init_proposes_outside_writes_shared() {
    let dir = common::TempDir::new("urzua-outside-init");
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    let out = common::urzua()
        .args(["init", "--dry-run"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(text.contains("outside_writes: shared"), "{text}");
}

fn nested_worktree(proj: &Path, old: &str) -> PathBuf {
    let wt = proj.join("nested");
    git(
        proj,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            old,
        ],
    );
    wt
}

#[test]
fn shared_judges_a_linked_worktree_nested_inside_the_session_repository() {
    let (_parent, proj, old) = project("nested", "  outside_writes: shared\n");
    let wt = nested_worktree(&proj, &old);
    for (rel, content) in [
        (".github/workflows/x.yml", "name: x\n"),
        (".claude/settings.json", "{}\n"),
        (ADR, ACCEPTED),
    ] {
        let (code, v, _) = run_edit(&proj, &wt.join(rel), content);
        assert_eq!((code, verdict(&v)), (1, "block"), "{rel}: {v}");
    }
    // A plain subdirectory of the session repository is the session's own file:
    // judged by it as before, with no linked-worktree wording.
    std::fs::create_dir_all(proj.join("sub")).unwrap();
    let (code, v, _) = run_edit(&proj, &proj.join("sub/notes.txt"), "x\n");
    assert_eq!((code, verdict(&v)), (0, "not-governed"), "{v}");
    let (code, v, _) = run_edit(&proj, &proj.join(".github/workflows/x.yml"), "name: x\n");
    assert_eq!((code, verdict(&v)), (1, "block"), "{v}");
    assert!(
        !v["message"].as_str().unwrap().contains("linked worktree"),
        "{v}"
    );
}

#[test]
fn shared_judges_the_path_of_a_request_that_carries_no_content() {
    let (parent, proj, old) = project("nocontent", "  outside_writes: shared\n");
    let wt = worktree(&parent, &proj, &old);
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [wt.join(".claude/settings.json").to_string_lossy()],
        "actor": "agent", "cwd": proj.to_string_lossy(), "project_dir": proj.to_string_lossy(),
    });
    let mut c = common::urzua();
    c.args(["harness", "run", "edit"])
        .current_dir(&proj)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verdict(&v), "block", "{v}");
}

fn run_no_content(proj: &Path, path: &Path) -> serde_json::Value {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path.to_string_lossy()],
        "actor": "agent", "cwd": proj.to_string_lossy(), "project_dir": proj.to_string_lossy(),
    });
    let mut c = common::urzua();
    c.args(["harness", "run", "edit"])
        .current_dir(proj)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn the_session_repository_judges_the_path_of_a_request_that_carries_no_content() {
    for extra in ["", "  outside_writes: shared\n", "  outside_writes: deny\n"] {
        let (_parent, proj, _old) = project("session-nocontent", extra);
        let v = run_no_content(&proj, &proj.join(".claude/settings.json"));
        assert_eq!(verdict(&v), "block", "{extra}: {v}");
    }
}

#[test]
fn a_nested_plain_repository_is_outside_under_deny_and_unjudged_under_shared() {
    let (_parent, proj, _old) = project("nested-plain", "  outside_writes: deny\n");
    let nested = proj.join("vendor/inner");
    std::fs::create_dir_all(&nested).unwrap();
    git(&nested, &["init", "-q"]);
    let target = nested.join("notes.txt");
    let (code, v, _) = run_edit(&proj, &target, "x\n");
    assert_eq!((code, verdict(&v)), (1, "block"), "{v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("outside the session repository"),
        "{v}"
    );
    // Listed in allow_paths it is let through.
    std::fs::write(
        proj.join(".urzua/config.yaml"),
        session_config(&format!(
            "  outside_writes: deny\n  allow_paths: [\"{}\"]\n",
            nested.display()
        )),
    )
    .unwrap();
    let (_, v, _) = run_edit(&proj, &target, "x\n");
    assert_ne!(verdict(&v), "block", "{v}");
    // Under shared a separate repository is not the session's: unjudged.
    std::fs::write(
        proj.join(".urzua/config.yaml"),
        session_config("  outside_writes: shared\n"),
    )
    .unwrap();
    let (code, v, _) = run_edit(&proj, &target, "x\n");
    assert_eq!((code, verdict(&v)), (0, "not-governed"), "{v}");
}

/// The installed translator, run as the agent's hook for a Claude Code tool call.
fn translate(proj: &Path, payload: serde_json::Value) -> serde_json::Value {
    let dest = proj.join(".claude/hooks");
    std::fs::create_dir_all(&dest).unwrap();
    let script = dest.join("urzua-claude-code.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bundle/claude-code/files/.claude/hooks/urzua-claude-code.py"),
        &script,
    )
    .unwrap();
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(&script)
        .args(["edit", "--", "urzua", "harness", "run", "edit"])
        .current_dir(proj)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
}

#[test]
fn multiedit_and_notebookedit_into_a_linked_worktree_are_judged_under_shared() {
    let (parent, proj, old) = project("translator", "  outside_writes: shared\n");
    let wt = worktree(&parent, &proj, &old);
    std::fs::create_dir_all(wt.join(".github/workflows")).unwrap();
    std::fs::write(wt.join(".github/workflows/x.yml"), "name: x\n").unwrap();
    std::fs::create_dir_all(wt.join(".claude")).unwrap();
    std::fs::write(
        wt.join(".claude/nb.ipynb"),
        r#"{"cells":[{"cell_type":"code","id":"a","metadata":{},"source":"1","outputs":[],"execution_count":null}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#,
    )
    .unwrap();
    let multi = translate(
        &proj,
        serde_json::json!({
            "tool_name": "MultiEdit", "cwd": proj.to_string_lossy(),
            "tool_input": {
                "file_path": wt.join(".github/workflows/x.yml").to_string_lossy(),
                "edits": [{"old_string": "name: x", "new_string": "name: y"}],
            },
        }),
    );
    assert_eq!(
        multi["hookSpecificOutput"]["permissionDecision"], "deny",
        "{multi}"
    );
    let note = translate(
        &proj,
        serde_json::json!({
            "tool_name": "NotebookEdit", "cwd": proj.to_string_lossy(),
            "tool_input": {
                "notebook_path": wt.join(".claude/nb.ipynb").to_string_lossy(),
                "cell_id": "a", "new_source": "2",
            },
        }),
    );
    assert_eq!(
        note["hookSpecificOutput"]["permissionDecision"], "deny",
        "{note}"
    );
}

#[test]
fn a_fresh_config_with_only_outside_writes_does_not_warn_that_the_harness_is_not_installed() {
    let dir = common::TempDir::new("urzua-outside-doctor-bare");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{TYPES}harness:\n  outside_writes: shared\n"),
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    let out = common::urzua()
        .arg("doctor")
        .current_dir(&dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(!text.contains("harness-installed"), "{text}");
    // Declaring a guardrail makes the missing lock a problem again.
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{TYPES}harness:\n  outside_writes: shared\n  guardrails:\n    editor-hook: {{enabled: true}}\n"),
    )
    .unwrap();
    let out = common::urzua()
        .arg("doctor")
        .current_dir(&dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(text.contains("harness-installed"), "{text}");
}

#[test]
fn shared_compares_a_record_with_the_worktrees_copy_not_the_sessions() {
    let (parent, proj, old) = project("shared-before", "  outside_writes: shared\n");
    let wt = worktree(&parent, &proj, &old);
    // The session's copy is already Accepted; the worktree's is still Proposed.
    std::fs::write(proj.join(ADR), ACCEPTED).unwrap();
    assert_eq!(std::fs::read_to_string(wt.join(ADR)).unwrap(), PROPOSED);
    let (code, v, _) = run_edit(&proj, &wt.join(ADR), ACCEPTED);
    assert_eq!((code, verdict(&v)), (1, "block"), "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("'Proposed'"),
        "the move is from the worktree's status: {v}"
    );
    // Leaving the worktree's status where it is stays allowed.
    let (code, v, _) = run_edit(&proj, &wt.join(ADR), PROPOSED);
    assert_eq!(code, 0, "{v}");
}
