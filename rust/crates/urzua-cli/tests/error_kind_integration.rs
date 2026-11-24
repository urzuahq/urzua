//! A refusal states its kind where it is made (`MILE-365`): an argument that is wrong is `usage`
//! whatever command's code noticed it, and a `not-run` report that carries an `error` carries
//! `error_kind` beside it.

mod common;

use common::waive::*;

#[test]
fn every_argument_error_is_a_usage_not_run() {
    let dir = fixture("usage", two_noisy());
    let cases: &[&[&str]] = &[
        &["check", "--rules", "nope.rule"],
        &["check", "--rules", "header.required-field"],
        &["check", "--rules", ""],
        &["waive", "--from-report", "--rule", "nonexistent.rule"],
        &["explain", "--rule", "nope.rule"],
        &["explain", "--rule", "header.required-field"],
        &["explain", "docs/adr/ADR-1-a.md#nope#more"],
        &["new", "nosuchtype", "x"],
        &["new", "adr", ""],
        &[
            "reverify",
            "--apply",
            "--by",
            "tester",
            "--actor",
            "agent",
            "--reason",
            "x",
            "--ids",
            "docs/adr/ADR-1-a.md",
        ],
        &["fix", "--pin", "--replace", "a=b"],
    ];
    let mut wrong = Vec::new();
    for args in cases {
        let (code, stdout, stderr) = run(&dir, args);
        let report: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("{args:?}: not JSON ({e}): {stdout} / {stderr}"));
        if code != 2 || report["status"] != "not-run" || report["error_kind"] != "usage" {
            wrong.push(format!(
                "{args:?}: exit {code}, status {}, error_kind {}",
                report["status"], report["error_kind"]
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "not a usage not-run:\n{}",
        wrong.join("\n")
    );
}

#[cfg(unix)]
#[test]
fn a_waive_apply_that_wrote_nothing_is_a_not_run_with_an_error_kind() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fixture("readonly", two_noisy());
    let waivers = dir.join("docs/waiver");
    std::fs::set_permissions(&waivers, std::fs::Permissions::from_mode(0o555)).unwrap();
    let (code, report) = waive(&dir, &["--apply"]);
    std::fs::set_permissions(&waivers, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(report["error"].is_string(), "{report}");
    assert_eq!(report["error_kind"], "environment", "{report}");
}
