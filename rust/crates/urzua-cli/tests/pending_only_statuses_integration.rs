//! MILE-205: a suppression type (a waiver) declares `statuses: {pending: [...]}` and no
//! `values`. Every reader of `statuses.values` must cope with the empty list: the type
//! loads, and each consumer says plainly that the type declares no vocabulary rather than
//! breaking or reporting a confusing empty "declared:" list. One test per reader.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// A repository with one pending-only type (`waiver`) and one ordinary type (`adr`).
fn fixture(name: &str, config_tail: &str) -> common::TempDir {
    fixture_with(name, config_tail, None)
}

fn fixture_with(name: &str, config_tail: &str, waiver: Option<&str>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-pending-only-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{config_tail}\nrecord_types:\n  waiver:\n    dir: docs/waiver\n    prefix: WAIVER\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    statuses:\n      pending: [pending]\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 - X\n",
    )
    .unwrap();
    if let Some(body) = waiver {
        std::fs::write(dir.join("docs/waiver/WAIVER-1-x.md"), body).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn check_json(dir: &Path) -> (i32, serde_json::Value) {
    let (code, stdout, stderr) = run(dir, &["check"]);
    let report = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("check printed no JSON ({e}): {stdout} / {stderr}"));
    (code, report)
}

#[test]
fn a_pending_only_type_loads_and_check_runs() {
    let dir = fixture("load", "rules: {header.required-fields: error}");
    let (code, report) = check_json(&dir);
    assert_eq!(code, 0, "{report}");
}

/// Reader: `rules/status.rs`, `status.undeclared-value`. A record of a type with no
/// declared vocabulary holds a status nothing declares; it is reported, and the message
/// says the type declares no values instead of printing an empty "declared:" list.
#[test]
fn undeclared_value_reports_a_status_on_a_type_that_declares_no_values() {
    let dir = fixture_with(
        "undeclared",
        "rules: {status.undeclared-value: error}",
        Some("---\nStatus: Open\n---\n# 1 - X\n"),
    );
    let (code, report) = check_json(&dir);
    let found: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "status.undeclared-value")
        .collect();
    assert_eq!(found.len(), 1, "{report}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("declares no values"), "{message}");
    assert!(!message.contains("declared: )"), "{message}");
    assert_eq!(code, 1, "{report}");
}

/// Reader: `authority.rs` gate `to`. A gate on a status of a type that declares no values
/// does not load, and says why.
#[test]
fn an_authority_gate_on_a_type_with_no_values_is_refused_at_load() {
    let dir = fixture(
        "gate",
        "authority:\n  deciders:\n    m: {logins: [a]}\n  gates:\n    - {type: waiver, to: [Accepted], by: m}\n",
    );
    let (code, stdout, stderr) = run(&dir, &["check"]);
    let stderr = format!("{stdout}{stderr}");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("authority.gates[0]"), "{stderr}");
    assert!(stderr.contains("'Accepted'"), "{stderr}");
    assert!(stderr.contains("declares no values"), "{stderr}");
}

/// Reader: `config.rs` `decision_types`. Naming a status of a type that lists no values is
/// still the "does not list" error: it could never match.
#[test]
fn decision_types_naming_a_status_of_a_type_with_no_values_is_refused_at_load() {
    let dir = fixture(
        "decision",
        "rules:\n  diff.source-requires-record:\n    level: error\n    source_paths: [src]\n    decision_types:\n      waiver: [Accepted]\n",
    );
    let (code, stdout, stderr) = run(&dir, &["check"]);
    let stderr = format!("{stdout}{stderr}");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("does not list"), "{stderr}");
    assert!(stderr.contains("Accepted"), "{stderr}");
}

/// Reader: `config.rs` `periods` status list. Same: a status the type does not declare.
#[test]
fn a_claim_period_status_on_a_type_with_no_values_is_refused_at_load() {
    let dir = fixture(
        "periods",
        "rules:\n  embodiment.claim-expired:\n    level: warn\n    anchor_field: Date\n    require_pin: [code]\n    periods:\n      - {type: waiver, status: [Done], days: 30}\n",
    );
    let (code, stdout, stderr) = run(&dir, &["check"]);
    let stderr = format!("{stdout}{stderr}");
    assert_eq!(code, 2, "{stderr}");
    assert!(
        stderr.contains("does not declare in statuses.values"),
        "{stderr}"
    );
}

/// Reader: `commands/new.rs`. There is no first value to start a record at, so the status
/// field is left blank for a human, exactly as for a type declaring no statuses.
#[test]
fn new_leaves_the_status_blank_for_a_type_with_no_values() {
    let dir = fixture("new", "rules: {header.required-fields: error}");
    let (code, stdout, stderr) = run(&dir, &["new", "waiver", "A waiver", "--by", "tester"]);
    assert_eq!(code, 0, "{stdout} / {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let text = std::fs::read_to_string(dir.join(report["path"].as_str().unwrap())).unwrap();
    let status = text
        .lines()
        .find(|l| l.starts_with("Status:"))
        .unwrap_or_else(|| panic!("no Status line: {text}"));
    assert_eq!(status.trim_end(), "Status: null", "{text}");
}

/// A type declaring `values` is unchanged: `new` starts it at its first value.
#[test]
fn a_type_declaring_values_still_starts_at_its_first_value() {
    let dir = fixture("new-values", "rules: {header.required-fields: error}");
    let (code, stdout, stderr) = run(&dir, &["new", "adr", "An adr", "--by", "tester"]);
    assert_eq!(code, 0, "{stdout} / {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let text = std::fs::read_to_string(dir.join(report["path"].as_str().unwrap())).unwrap();
    assert!(text.contains("Status: Proposed"), "{text}");
}
