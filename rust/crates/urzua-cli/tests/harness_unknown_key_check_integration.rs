//! An unknown key inside `harness` or a known guardrail is reported by `urzua check`
//! as a warning notice (`ADR-89` amendment of 2026-11-07: "reported, not an error"),
//! not only by `harness install`, `doctor` and the session hook (`MILE-198`).
//!
//! Verifies: MILE-198

mod common;

use serde_json::Value;

fn repo(harness: &str) -> common::TempDir {
    let dir = common::TempDir::new("urzua-harness-unknown-key");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nfail_on: warning\nrules:\n  header.required-fields: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted]\n{harness}"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n",
    )
    .unwrap();
    for args in [
        &["init", "-q"][..],
        &["add", "-A"],
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
    ] {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }
    dir
}

fn check(dir: &common::TempDir) -> (i32, Value) {
    let out = common::urzua()
        .args(["check", "--config", ".urzua/config.yaml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.code().unwrap(), json)
}

#[test]
fn a_misspelled_key_inside_a_guardrail_is_a_structured_warning_notice_with_the_nearest_key() {
    let dir = repo("harness:\n  guardrails:\n    session-start: {enabeld: true}\n");
    let (code, report) = check(&dir);
    let notices = report["notices"].as_array().expect("notices present");
    let n = notices
        .iter()
        .find(|n| n["subject"] == "harness.guardrails.session-start")
        .unwrap_or_else(|| panic!("{notices:?}"));
    assert_eq!(n["severity"], "warning");
    assert_eq!(n["detail"]["key"], "enabeld");
    assert_eq!(n["detail"]["nearest"], "enabled");
    let valid = n["detail"]["valid"]
        .as_array()
        .unwrap_or_else(|| panic!("valid is a list: {n}"));
    assert!(valid.iter().any(|v| v == "enabled"), "{n}");
    // Reported, not an error: the exit code and the findings do not move.
    assert_eq!(code, 0, "{report}");
    assert!(report["findings"].as_array().unwrap().is_empty());
}

#[test]
fn an_unknown_top_level_harness_key_is_reported_with_its_nearest_key() {
    let dir = repo("harness:\n  adaptors: [example-agent]\n");
    let (_, report) = check(&dir);
    let n = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "harness")
        .expect("a notice for the harness key");
    assert_eq!(n["detail"]["key"], "adaptors");
    assert_eq!(n["detail"]["nearest"], "adapters");
}

#[test]
fn a_key_with_nothing_near_names_no_nearest_key_and_a_clean_harness_adds_no_notice() {
    let dir = repo("harness:\n  guardrails:\n    session-start: {enabled: true, colour: blue}\n");
    let (_, report) = check(&dir);
    let n = &report["notices"][0];
    assert_eq!(n["detail"]["key"], "colour");
    assert!(n["detail"].get("nearest").is_none(), "{n}");

    let dir = repo("harness:\n  guardrails:\n    session-start: {enabled: true}\n");
    let (_, report) = check(&dir);
    assert!(report.get("notices").is_none(), "{report}");
}
