//! The output contract of `check`, `waive`, `new` and `rules` (ADR-23, ADR-46,
//! ADR-101 "machine-first"): `blocking` agrees with the exit code, a path that
//! selects nothing is reported, a `not-run` carries a reason, what `waive` and
//! `new` say on stderr is also in the JSON, and `rules` has a `status`. Real
//! binary against fixture repositories.
//!
//! Verifies: ADR-46

mod common;

use std::path::Path;

use serde_json::Value;

const WARN_ONLY: &str = "---\nDeciders: Ana\nReviewers:\n---\n# ADR-1: First\n";

fn fixture(name: &str, top_level: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-contract-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    common::waive::git(&dir, &["init", "-q"]);
    common::waive::git(&dir, &["config", "user.email", "t@example.com"]);
    common::waive::git(&dir, &["config", "user.name", "t"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{top_level}rules:\n  field.quality: warn\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Deciders, Reviewers]\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-first.md"), WARN_ONLY).unwrap();
    std::fs::write(dir.join("README.md"), "# readme\n").unwrap();
    common::waive::git(&dir, &["add", "-A"]);
    common::waive::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, Value, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    (
        out.status.code().unwrap_or(-1),
        json,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn notice_messages(report: &Value) -> Vec<String> {
    report["notices"]
        .as_array()
        .map(|n| {
            n.iter()
                .map(|n| n["message"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// 5.1: `blocking` is true exactly when the run exits 1, under either threshold.
#[test]
fn blocking_agrees_with_the_exit_code_under_every_fail_on_observed_failing() {
    for (config, flag) in [
        ("", None),
        ("fail_on: warning\n", None),
        ("fail_on: warning\n", Some("error")),
        ("", Some("warning")),
    ] {
        let dir = fixture("blocking", config);
        let mut args = vec!["check", "docs/"];
        if let Some(f) = flag {
            args.extend(["--fail-on", f]);
        }
        let (code, report, _) = run(&dir, &args);
        assert_eq!(
            report["blocking"],
            Value::Bool(code == 1),
            "config {config:?} flag {flag:?}: exit {code}, {report}"
        );
    }
    let strict = fixture("blocking-strict", "fail_on: warning\n");
    let (code, report, _) = run(&strict, &["check", "docs/"]);
    assert_eq!((code, &report["blocking"]), (1, &Value::Bool(true)));
    assert_eq!(report["status"], "findings-present");
}

/// 5.2: an argument that selects no record is reported even when another matches.
#[test]
fn a_path_argument_that_matches_nothing_is_reported_beside_one_that_matches_observed_failing() {
    let dir = fixture("unmatched", "");
    let (code, report, _) = run(&dir, &["check", "docs/adr", "README.md"]);
    assert_eq!(code, 0, "{report}");
    let notices = report["notices"].as_array().unwrap();
    let hit = notices
        .iter()
        .find(|n| n["message"].as_str().unwrap().contains("README.md"))
        .unwrap_or_else(|| panic!("no notice names README.md: {report}"));
    assert_eq!(hit["severity"], "warning");
    assert_eq!(hit["subject"], "scope");
    assert!(
        !notice_messages(&report)
            .iter()
            .any(|m| m.contains("docs/adr")),
        "the argument that matched is not reported: {report}"
    );
    let (_, clean, _) = run(&dir, &["check", "docs/adr"]);
    assert!(
        !notice_messages(&clean)
            .iter()
            .any(|m| m.contains("matches")),
        "{clean}"
    );
}

/// 5.3: every full-report `not-run` says why.
#[test]
fn every_not_run_check_report_carries_a_reason_observed_failing() {
    let dir = fixture("not-run", "");
    for args in [
        &["check", "README.md"][..],
        &["check", "--rules", "diff.decider-required"][..],
        &["check", "--rules", "field.quality", "README.md"][..],
    ] {
        let (code, report, _) = run(&dir, args);
        assert_eq!(code, 2, "{args:?}: {report}");
        assert_eq!(report["status"], "not-run", "{args:?}: {report}");
        let reasons = report["error"].is_string()
            || report["notices"]
                .as_array()
                .is_some_and(|n| n.iter().any(|n| n["severity"] == "warning"));
        assert!(reasons, "{args:?} has no reason: {report}");
    }
}

/// 5.4: the identity substitution `waive` prints on stderr is a notice in the JSON.
#[test]
fn waive_reports_the_identity_substitution_and_numbering_notices_in_json_observed_failing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = common::waive::fixture("contract-waive", common::waive::two_noisy());
    let stub = common::TempDir::new("urzua-contract-stub-gh");
    std::fs::write(stub.join("gh"), "#!/bin/sh\necho gh-login\n").unwrap();
    std::fs::set_permissions(stub.join("gh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = common::urzua()
        .args(["waive", "--from-report", "--by", "ana"])
        .env("PATH", path)
        .current_dir(&dir)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        stderr.contains("differs from the authenticated gh login"),
        "{stderr}"
    );
    let notices = notice_messages(&report);
    for line in stderr.lines().filter(|l| !l.trim().is_empty()) {
        let text = line.trim_start_matches("urzua: ");
        assert!(
            notices.iter().any(|m| m == text),
            "stderr line {line:?} is not a notice: {report}"
        );
    }
}

/// 5.4: what `new` prints on stderr is a notice in the JSON.
#[test]
fn new_reports_what_it_prints_to_stderr_in_json_observed_failing() {
    let dir = common::TempDir::new("urzua-contract-new");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    common::waive::git(&dir, &["init", "-q"]);
    common::waive::git(&dir, &["config", "user.email", "t@example.com"]);
    common::waive::git(&dir, &["config", "user.name", "t"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  bug:\n    dir: docs/bug\n    prefix: BUG\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n",
    )
    .unwrap();
    common::waive::git(&dir, &["add", "-A"]);
    common::waive::git(&dir, &["commit", "-q", "-m", "fixture"]);
    let (code, report, stderr) = run(&dir, &["new", "bug", "First", "--by", "t"]);
    assert_eq!(code, 0, "{report} {stderr}");
    assert!(stderr.contains("does not exist yet"), "{stderr}");
    let notices = notice_messages(&report);
    for line in stderr.lines().filter(|l| !l.trim().is_empty()) {
        let text = line.trim_start_matches("urzua: ");
        assert!(
            notices.iter().any(|m| m == text),
            "stderr line {line:?} is not a notice: {report}"
        );
    }
}

/// 5.5: `rules` carries a `status` like its siblings.
#[test]
fn rules_carries_a_status_observed_failing() {
    let dir = fixture("rules", "");
    let (code, report, _) = run(&dir, &["rules"]);
    assert_eq!(code, 0);
    assert_eq!(report["status"], "ok", "{}", report.to_string().len());
}
