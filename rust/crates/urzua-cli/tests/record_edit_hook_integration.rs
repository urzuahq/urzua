//! MILE-178: the record-edit hook (ADR-69). Recorded Claude Code PreToolUse
//! payloads (`tests/fixtures/hook-payloads`) go through the adapter and the
//! harness-neutral core against the real urzua binary in a git fixture. Needs
//! `bash`, `jq` and `python3`; a missing tool fails the test, never skips it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const RECORD: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";

/// A clean git repo; `Realized-by: code:src/a.rs` computes `Implemented`.
fn fixture(name: &str, record: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-hook-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), record).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn payload(name: &str, root: &Path) -> String {
    let p = repo_root().join(format!(
        "rust/crates/urzua-cli/tests/fixtures/hook-payloads/{name}.json"
    ));
    std::fs::read_to_string(p)
        .unwrap()
        .replace("@ROOT@", root.to_str().unwrap())
}

fn run(cmd: &mut Command, root: &Path, stdin: &str) -> Output {
    let mut child = cmd
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash/python3 must be installed");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// The adapter's stdout as JSON (null when it printed nothing).
fn adapter(root: &Path, name: &str, env: &[(&str, &str)]) -> serde_json::Value {
    let script = repo_root().join("scripts/hooks/claude-code-record-edit.py");
    let mut c = Command::new("python3");
    c.arg("-I").arg(script);
    c.env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env_remove("URZUA_HOOK_OVERRIDE");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = run(&mut c, root, &payload(name, root));
    assert!(out.status.success(), "{out:?}");
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&out.stdout).unwrap()
}

fn core(root: &Path, args: &[&str], env: &[(&str, &str)], stdin: &str) -> (i32, serde_json::Value) {
    let mut c = Command::new("bash");
    c.arg(repo_root().join("scripts/hooks/record-edit-hook.sh"))
        .args(args);
    c.env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env_remove("URZUA_HOOK_OVERRIDE");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = run(&mut c, root, stdin);
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("core stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn hook(v: &serde_json::Value) -> &serde_json::Value {
    &v["hookSpecificOutput"]
}

fn denied(v: &serde_json::Value) -> bool {
    hook(v)["permissionDecision"] == "deny"
}

#[test]
fn a_hand_set_embodiment_is_blocked_with_the_computed_value_and_evidence() {
    let root = fixture("block", RECORD);
    for name in ["write-wrong", "edit-wrong", "multiedit-wrong"] {
        let v = adapter(&root, name, &[]);
        assert!(denied(&v), "{name}: {v}");
        let why = hook(&v)["permissionDecisionReason"].as_str().unwrap();
        assert!(
            why.contains("\"Verified\"") && why.contains("computes \"Implemented\""),
            "{why}"
        );
        assert!(why.contains("Realized-by: code:src/a.rs"), "{why}");
        assert!(why.contains(":2:"), "names the line: {why}");
    }
}

#[test]
fn the_computed_value_and_edits_to_other_fields_pass() {
    let root = fixture("pass", RECORD);
    // The record on disk already disagrees (Not started vs Implemented); an
    // edit that leaves that field alone is not blocked.
    for name in ["write-correct", "edit-prose"] {
        let v = adapter(&root, name, &[]);
        assert!(!denied(&v), "{name}: {v}");
    }
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], RECORD);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("Already disagreeing on disk"),
        "{v}"
    );
}

#[test]
fn an_ungoverned_path_and_other_tools_are_not_touched() {
    let root = fixture("ungoverned", RECORD);
    assert!(adapter(&root, "write-ungoverned", &[]).is_null());
    assert!(adapter(&root, "read-tool", &[]).is_null());
    let (code, v) = core(&root, &["src/a.rs"], &[], "fn a() {}\n");
    assert_eq!((code, v["verdict"].as_str()), (0, Some("not-governed")));
}

#[test]
fn when_urzua_cannot_judge_the_edit_proceeds_and_says_so() {
    let root = fixture("unjudged", RECORD);
    // The record states no Embodiment: urzua answers not-run.
    let v = adapter(&root, "write-no-embodiment", &[]);
    assert!(!denied(&v));
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
    assert!(hook(&v)["additionalContext"]
        .as_str()
        .unwrap()
        .contains("did NOT judge"));
    // No binary at all.
    let v = adapter(
        &root,
        "write-wrong",
        &[
            ("URZUA_BIN", "/nonexistent/urzua"),
            ("PATH", "/usr/bin:/bin"),
        ],
    );
    assert!(!denied(&v), "{v}");
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
    // An Edit whose text is not in the file cannot be turned into a proposal.
    let v = adapter(&root, "edit-missing", &[]);
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
}

#[test]
fn an_unreadable_history_is_cannot_judge() {
    let src = fixture("shallow-src", RECORD);
    std::fs::write(src.join("src/a.rs"), "fn a() { }\n").unwrap();
    git(&src, &["commit", "-q", "-am", "second"]);
    let dst = std::env::temp_dir().join(format!("urzua-hook-shallow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dst);
    assert!(Command::new("git")
        .args(["clone", "-q", "--depth", "1"])
        .arg(format!("file://{}", src.display()))
        .arg(&dst)
        .status()
        .unwrap()
        .success());
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = core(&dst, &["docs/adr/ADR-1-a.md"], &[], &wrong);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow-unjudged", "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("shallow clone"),
        "{v}"
    );
}

#[test]
fn a_named_override_lets_the_edit_through_and_appears_in_the_output() {
    let root = fixture("override", RECORD);
    let v = adapter(
        &root,
        "write-wrong",
        &[("URZUA_HOOK_OVERRIDE", "alice-reviewed")],
    );
    assert!(!denied(&v), "{v}");
    let msg = v["systemMessage"].as_str().unwrap();
    assert!(
        msg.contains("OVERRIDE 'alice-reviewed'") && msg.contains("computes \"Implemented\""),
        "{msg}"
    );

    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = core(
        &root,
        &["--override", "bob", "docs/adr/ADR-1-a.md"],
        &[],
        &wrong,
    );
    assert_eq!(
        (code, v["verdict"].as_str(), v["override"].as_str()),
        (0, Some("allow-override"), Some("bob"))
    );

    // An override without a name is not accepted: the block stands.
    let (code, v) = core(
        &root,
        &["--override", "  ", "docs/adr/ADR-1-a.md"],
        &[],
        &wrong,
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")));
    assert!(
        v["message"].as_str().unwrap().contains("needs a name"),
        "{v}"
    );
}

#[test]
fn a_new_record_is_judged_from_the_proposal_alone() {
    let root = fixture("new", RECORD);
    let new = RECORD.replace("ADR-1: T", "ADR-3: V");
    let (code, v) = core(&root, &["docs/adr/ADR-3-c.md"], &[], &new);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
}
