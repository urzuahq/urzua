//! ADR-89, amendment of 2026-11-14 (leaving a gate): blanking an Accepted
//! record, stripping its header, or moving it from a gate status to a
//! non-gate status (an un-accept) is a gated move like deleting it, because
//! the outcome is the same. Real git-repo fixtures and the real binary.

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const CONFIG_YAML: &str = "schema_version: 2\nrules:\n  diff.decider-required: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nauthority:\n  decider_table: base\n  deciders:\n    maintainers: {logins: [alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

/// A repository whose HEAD holds an Accepted `ADR-1`; the working tree is the
/// pull request.
fn accepted_fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-gate-leaving-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG_YAML).unwrap();
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    dir
}

fn check(dir: &Path, who: &str) -> serde_json::Value {
    let path = dir.join("actors.json");
    let body = serde_json::json!({"schema": 1, "pr_author": who, "approvers": []});
    std::fs::write(&path, body.to_string()).unwrap();
    let out: Output = common::urzua()
        .args(["check", "--base", "HEAD", "--actors"])
        .arg(&path)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn decider_findings(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.decider-required")
        .collect()
}

fn flagged(v: &serde_json::Value) -> bool {
    decider_findings(v).iter().any(|f| f["file"] == PATH_1)
}

#[test]
fn blanking_an_accepted_record_needs_a_decider() {
    let dir = accepted_fixture("empty");
    std::fs::write(dir.join(PATH_1), "").unwrap();
    let v = check(&dir, "mallory");
    assert!(flagged(&v), "{v}");
    assert!(!flagged(&check(&dir, "alice")), "a decider may blank it");
}

#[test]
fn stripping_an_accepted_records_header_needs_a_decider() {
    let dir = accepted_fixture("stripped");
    std::fs::write(dir.join(PATH_1), "# ADR-1: T\n\nprose\n").unwrap();
    let v = check(&dir, "mallory");
    assert!(flagged(&v), "{v}");
    assert!(!flagged(&check(&dir, "alice")), "a decider may strip it");
}

#[test]
fn moving_an_accepted_record_to_a_non_gate_status_needs_a_decider() {
    let dir = accepted_fixture("unaccept");
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    let v = check(&dir, "mallory");
    assert!(flagged(&v), "{v}");
    assert!(!flagged(&check(&dir, "alice")), "a decider may un-accept");
}

#[test]
fn an_unchanged_accepted_record_and_a_move_between_gate_statuses_are_not_departures() {
    let dir = accepted_fixture("stay");
    std::fs::write(dir.join(PATH_1), format!("{ACCEPTED}more prose\n")).unwrap();
    assert!(!flagged(&check(&dir, "mallory")));
}

#[test]
fn un_accepting_then_deleting_across_two_pull_requests_is_stopped_at_the_first() {
    let dir = accepted_fixture("two-prs");
    // First pull request: a non-decider un-accepts the record.
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    assert!(flagged(&check(&dir, "mallory")), "the un-accept is gated");
    // It lands only with a decider; the second pull request deletes the
    // now-Proposed record, which is no longer a gated one.
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "un-accept, by a decider"]);
    std::fs::remove_file(dir.join(PATH_1)).unwrap();
    assert!(!flagged(&check(&dir, "mallory")));
}
