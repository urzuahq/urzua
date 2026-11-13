//! `level: info` (`ADR-87`, `MILE-194`) against the real binary: a finding at
//! that level is reported and counted with severity `info` and never trips
//! `fail_on`, and a declared severity never lifts a rule set to `info`.
//!
//! Verifies: ADR-87

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

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(
        dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "fixture",
        ],
    );
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let v = serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code().unwrap_or(-1), v, stdout)
}

const REVIEWERS_BLANK: &str = "---\nDeciders: Ana\nReviewers:\n---\n# ADR-1: First\n";
const DECIDERS_BLANK: &str = "---\nDeciders:\nReviewers: Bo\n---\n# ADR-1: First\n";

/// A repository with one ADR and `field.quality` declared as `rule_body`
/// (indented four spaces), under `top` top-level keys.
fn field_repo(name: &str, top: &str, rule_body: &str, record: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-info-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{top}rules:\n  field.quality:\n{rule_body}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Deciders, Reviewers]\n  waiver:\n    dir: \"docs/waiver\"\n    required_fields: []\n    pointer_fields: []\n    narrative_fields: []\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-first.md"), record).unwrap();
    dir
}

fn quality(v: &serde_json::Value) -> Vec<serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no findings array: {v}"))
        .iter()
        .filter(|f| f["rule"] == "field.quality")
        .cloned()
        .collect()
}

#[test]
fn an_info_finding_is_reported_counted_and_never_trips_fail_on() {
    for fail_on in ["", "fail_on: warning\n", "fail_on: error\n"] {
        let dir = field_repo("exit", fail_on, "    level: info\n", REVIEWERS_BLANK);
        commit_all(&dir);
        let (code, v, raw) = run(&dir, &["check", "docs/"]);
        let found = quality(&v);
        assert_eq!(found.len(), 1, "{fail_on}: {raw}");
        assert_eq!(found[0]["severity"], "info", "{raw}");
        assert_eq!(found[0]["file"], "docs/adr/ADR-1-first.md", "{raw}");
        assert_eq!(v["blocking"], false, "{raw}");
        assert_eq!(v["status"], "findings-present", "{raw}");
        assert_eq!(code, 0, "{fail_on}: an info finding must exit 0: {raw}");
        let exec = v["rules_executed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule"] == "field.quality")
            .unwrap();
        assert_eq!(exec["status"], "ran", "{raw}");
        assert!(
            exec["population"]["examined"].as_u64().unwrap() >= 1,
            "{raw}"
        );
    }
}

#[test]
fn the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity() {
    let info = field_repo(
        "cmp-info",
        "fail_on: warning\n",
        "    level: info\n",
        REVIEWERS_BLANK,
    );
    commit_all(&info);
    let warn = field_repo(
        "cmp-warn",
        "fail_on: warning\n",
        "    level: warn\n",
        REVIEWERS_BLANK,
    );
    commit_all(&warn);
    let (info_code, info_v, _) = run(&info, &["check", "docs/"]);
    let (warn_code, warn_v, raw) = run(&warn, &["check", "docs/"]);
    assert_eq!(warn_code, 1, "{raw}");
    assert_eq!(info_code, 0);
    let (mut a, mut b) = (quality(&info_v)[0].clone(), quality(&warn_v)[0].clone());
    assert_eq!(a["severity"], "info");
    assert_eq!(b["severity"], "warning");
    // Rule, file, line, message and suggested action are the same.
    a["severity"] = serde_json::Value::Null;
    b["severity"] = serde_json::Value::Null;
    assert_eq!(a, b);
}

#[test]
fn a_waiver_covers_an_info_finding_like_any_other() {
    let dir = field_repo(
        "waived",
        "fail_on: warning\n",
        "    level: info\n",
        REVIEWERS_BLANK,
    );
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-w.md"),
        "---\nRule: field.quality\nScope: docs/adr/ADR-1-first.md\nReason: advisory\n---\n# WAIVER-1: Waive\n",
    )
    .unwrap();
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check", "docs/"]);
    let found = quality(&v);
    assert_eq!(found.len(), 1, "{raw}");
    assert_eq!(found[0]["severity"], "info", "{raw}");
    assert!(found[0]["waived"].is_string(), "{raw}");
    assert_eq!(code, 0, "{raw}");
}

#[test]
fn field_levels_accepts_info_and_the_other_field_keeps_its_level() {
    let body = "    level: warn\n    field_levels: {Deciders: info, Reviewers: warn}\n";
    let dir = field_repo("fl-info", "fail_on: warning\n", body, DECIDERS_BLANK);
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check", "docs/"]);
    let found = quality(&v);
    assert_eq!(found.len(), 1, "{raw}");
    assert_eq!(found[0]["severity"], "info", "{raw}");
    assert_eq!(code, 0, "{raw}");

    let dir = field_repo("fl-warn", "fail_on: warning\n", body, REVIEWERS_BLANK);
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check", "docs/"]);
    assert_eq!(quality(&v)[0]["severity"], "warning", "{raw}");
    assert_eq!(code, 1, "{raw}");
}

#[test]
fn a_field_level_never_lifts_a_rule_set_to_info() {
    let body = "    level: info\n    field_levels: {Deciders: error, Reviewers: warn}\n";
    let dir = field_repo(
        "clamp-field",
        "fail_on: warning\n",
        body,
        "---\nDeciders:\nReviewers:\n---\n# ADR-1: First\n",
    );
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check", "docs/"]);
    let found = quality(&v);
    assert_eq!(found.len(), 2, "{raw}");
    assert!(found.iter().all(|f| f["severity"] == "info"), "{raw}");
    assert_eq!(v["blocking"], false, "{raw}");
    assert_eq!(code, 0, "{raw}");
}

#[test]
fn any_other_level_and_fail_on_info_are_refused_at_load() {
    let dir = field_repo("bad-level", "", "    level: fatal\n", REVIEWERS_BLANK);
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check", "docs/"]);
    assert_eq!(code, 2, "{raw}");
    let err = v["error"].as_str().unwrap_or_default().to_string();
    for v in ["off", "warn", "error", "info"] {
        assert!(err.contains(&format!("\"{v}\"")), "{err}");
    }

    let dir = field_repo(
        "fail-on-info",
        "fail_on: info\n",
        "    level: warn\n",
        REVIEWERS_BLANK,
    );
    commit_all(&dir);
    assert_eq!(run(&dir, &["check", "docs/"]).0, 2);
}

fn boundary_repo(name: &str, level: &str, body: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-info-b-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("pkg")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nfail_on: warning\nrules:\n  boundary.published-path-reference:\n    level: {level}\n{body}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("pkg/a.rs"), "// see ADR-1\n").unwrap();
    dir
}

fn boundary_findings(v: &serde_json::Value) -> Vec<serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "boundary.published-path-reference")
        .cloned()
        .collect()
}

const ERROR_BOUNDARY: &str =
    "    boundaries:\n      - paths: [\"pkg\"]\n        kind: published\n        reason: \"r\"\n        severity: error\n";

#[test]
fn a_boundarys_error_severity_never_lifts_a_rule_set_to_info() {
    let dir = boundary_repo("clamp", "info", ERROR_BOUNDARY);
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check"]);
    let found = boundary_findings(&v);
    assert_eq!(found.len(), 1, "{raw}");
    assert_eq!(found[0]["severity"], "info", "{raw}");
    assert_eq!(v["blocking"], false, "{raw}");
    assert_eq!(code, 0, "{raw}");
}

#[test]
fn a_boundary_may_declare_info() {
    let body = ERROR_BOUNDARY.replace("severity: error", "severity: info");
    let dir = boundary_repo("declared", "error", &body);
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check"]);
    assert_eq!(boundary_findings(&v)[0]["severity"], "info", "{raw}");
    assert_eq!(code, 0, "{raw}");
}

#[test]
fn published_paths_at_info_is_info_not_an_error() {
    let dir = boundary_repo(
        "published-paths",
        "info",
        "    published_paths: [\"pkg\"]\n",
    );
    commit_all(&dir);
    let (code, v, raw) = run(&dir, &["check"]);
    assert_eq!(boundary_findings(&v)[0]["severity"], "info", "{raw}");
    assert_eq!(v["blocking"], false, "{raw}");
    assert_eq!(code, 0, "{raw}");
}
