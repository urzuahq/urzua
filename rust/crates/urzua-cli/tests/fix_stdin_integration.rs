//! MILE-159: `urzua fix --stdin-path` judges proposed content without writing.
//! Real git-repo fixtures, real binary; all output is JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_stdin(dir: &Path, args: &[&str], stdin: &str) -> std::process::Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("urzua binary should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const PATH: &str = "docs/adr/ADR-1-a.md";

fn record(embodiment: &str, realized_by: &str) -> String {
    format!(
        "---\nEmbodiment: {embodiment}\nRealized-by: {realized_by}\n---\n# ADR-1: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// `src/a.rs` changes in a commit after the committed record's
/// `Realized-by` line: the shape ADR-32 calls drift.
fn fixture(name: &str, on_disk: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-fix-stdin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    if let Some(embodiment) = on_disk {
        std::fs::write(dir.join(PATH), record(embodiment, "code:src/a.rs")).unwrap();
    }
    commit_all(&dir);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn stdin_content_is_judged_and_nothing_is_written() {
    let dir = fixture("judge", Some("Implemented"));
    let before = std::fs::read_to_string(dir.join(PATH)).unwrap();
    // On disk says Implemented (and would be drifted); the proposal says
    // Not started with a fresh Realized-by line.
    let proposed = record("Not started", "code:src/a.rs ");
    let out = run_stdin(&dir, &["fix", "--stdin-path", PATH], &proposed);
    let v = json(&out);
    assert_eq!(v["status"], "repairs-available", "{v}");
    let repairs = v["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{v}");
    assert_eq!(repairs[0]["record"], PATH, "{v}");
    assert_eq!(repairs[0]["field"], "Embodiment", "{v}");
    assert_eq!(repairs[0]["current_value"], "Not started", "{v}");
    assert_eq!(repairs[0]["computed_value"], "Implemented", "{v}");
    assert_eq!(repairs[0]["line"], 2, "{v}");
    assert_eq!(v["population"]["examined"], 1, "{v}");
    assert_eq!(std::fs::read_to_string(dir.join(PATH)).unwrap(), before);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_path_that_does_not_exist_on_disk_yet_is_judged() {
    let dir = fixture("new", None);
    let out = run_stdin(
        &dir,
        &["fix", "--stdin-path", PATH],
        &record("Not started", "code:src/a.rs"),
    );
    let v = json(&out);
    assert_eq!(v["repairs"].as_array().unwrap().len(), 1, "{v}");
    assert!(!dir.join(PATH).exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_realized_by_line_unchanged_since_a_locator_moved_is_drift() {
    let dir = fixture("drift", Some("Implemented"));
    let out = run_stdin(
        &dir,
        &["fix", "--stdin-path", PATH],
        &record("Implemented", "code:src/a.rs"),
    );
    let v = json(&out);
    assert_eq!(v["repairs"][0]["computed_value"], "Drift detected", "{v}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_realized_by_line_edited_in_the_proposal_is_not_drift() {
    let dir = fixture("reverify", Some("Implemented"));
    let out = run_stdin(
        &dir,
        &["fix", "--stdin-path", PATH],
        &record("Implemented", "code:src/a.rs "),
    );
    let v = json(&out);
    assert_eq!(v["status"], "ok", "{v}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_path_no_record_type_governs_is_could_not_run() {
    let dir = fixture("ungoverned", None);
    let out = run_stdin(
        &dir,
        &["fix", "--stdin-path", "docs/other/ADR-1-a.md"],
        &record("Implemented", "code:src/a.rs"),
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("governs"),
        "{out:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn stdin_path_refuses_apply_and_other_paths() {
    let dir = fixture("conflict", Some("Implemented"));
    for extra in [vec!["--apply", "--force"], vec![PATH]] {
        let mut args = vec!["fix", "--stdin-path", PATH];
        args.extend(extra);
        let out = run_stdin(&dir, &args, &record("Implemented", "code:src/a.rs"));
        assert_eq!(out.status.code(), Some(2), "{args:?} {out:?}");
    }
    std::fs::remove_dir_all(&dir).ok();
}
