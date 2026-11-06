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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
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
    let mut c = Command::new("python3");
    c.arg("-I")
        .arg(translator())
        .args(["edit", "--"])
        .args(engine)
        .current_dir(dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, payload);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
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

#[test]
fn an_edit_the_translator_cannot_place_is_refused_when_the_configuration_denies() {
    let payload = |dir: &Path| {
        serde_json::json!({
            "tool_name": "Edit", "cwd": dir.to_string_lossy(),
            "tool_input": {"file_path": dir.join(PATH_1).to_string_lossy(), "old_string": "absent text", "new_string": "x"},
        })
        .to_string()
    };
    let dir = fixture("closed-place", &config(""));
    let v = translate(&dir, &payload(&dir), &ENGINE);
    assert_eq!(decision(&v), Some("deny"), "{v}");
    let dir = fixture("open-place", &config(", on_unjudged: allow"));
    let v = translate(&dir, &payload(&dir), &ENGINE);
    assert_eq!(decision(&v), None, "{v}");
    assert!(
        v["systemMessage"].as_str().unwrap().contains("NOT judge"),
        "{v}"
    );
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
