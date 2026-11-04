//! MILE-15: `urzua fix` writes `Drift detected` (ADR-32's named follow-up).
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

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const RECORD_PATH: &str = "docs/adr/ADR-1-first.md";

fn record_with(embodiment: &str) -> String {
    format!(
        "---\nEmbodiment: {embodiment}\nRealized-by: code:src/a.rs\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// A repo whose cited locator changed in a commit after the `Realized-by`
/// line was last touched -- the shape ADR-32 calls drift.
fn drifted_fixture(name: &str, embodiment: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-fix-drift-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(RECORD_PATH), record_with(embodiment)).unwrap();
    commit_all(&dir);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir);
    dir
}

/// Re-verifies the record: touches the `Realized-by` line in a commit after
/// the locator's last change, which clears the staleness clock.
fn reverify(dir: &Path) {
    let path = dir.join(RECORD_PATH);
    let touched = std::fs::read_to_string(&path)
        .unwrap()
        .replace("code:src/a.rs", "code:src/a.rs ");
    std::fs::write(&path, touched).unwrap();
    commit_all(dir);
}

#[test]
fn detect_reports_drift_detected_as_the_computed_value() {
    let dir = drifted_fixture("detect", "Implemented");
    let out = run_urzua(&dir, &["fix"]);
    let v = json(&out);
    assert_eq!(v["status"], "repairs-available", "{v}");
    let repairs = v["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{v}");
    assert_eq!(repairs[0]["current_value"], "Implemented", "{v}");
    assert_eq!(repairs[0]["computed_value"], "Drift detected", "{v}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn apply_writes_drift_detected_and_a_revision_log_row() {
    let dir = drifted_fixture("apply", "Implemented");
    let out = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(json(&out)["status"], "applied");
    let after = std::fs::read_to_string(dir.join(RECORD_PATH)).unwrap();
    assert!(after.contains("\nEmbodiment: Drift detected\n"), "{after}");
    assert!(!after.contains("Embodiment: Implemented"), "{after}");
    assert!(after.contains("Tool-authored (by "), "{after}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn apply_is_idempotent_and_check_agrees_afterwards() {
    let dir = drifted_fixture("idempotent", "Implemented");
    run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    commit_all(&dir);

    let again = json(&run_urzua(&dir, &["fix"]));
    assert_eq!(again["status"], "ok", "{again}");
    assert!(again["repairs"].as_array().unwrap().is_empty(), "{again}");

    let check = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(
        !check["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "embodiment.consistency"),
        "{check}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_locator_unchanged_since_the_realized_by_line_is_not_drift() {
    let dir = drifted_fixture("no-drift", "Implemented");
    reverify(&dir);
    let v = json(&run_urzua(&dir, &["fix"]));
    assert_eq!(v["status"], "ok", "{v}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_stale_drift_detected_is_recomputed_once_the_line_is_reverified() {
    let dir = drifted_fixture("stale", "Drift detected");
    reverify(&dir);
    let v = json(&run_urzua(&dir, &["fix"]));
    assert_eq!(v["repairs"][0]["current_value"], "Drift detected", "{v}");
    assert_eq!(v["repairs"][0]["computed_value"], "Implemented", "{v}");
    std::fs::remove_dir_all(&dir).ok();
}
