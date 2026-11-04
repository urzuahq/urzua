//! `header.case-variant-key` (MILE-113) end to end: the real binary against an
//! isolated git fixture. The rule is opt-in, so one test proves it does not
//! run unless declared, and the others prove what it examined and reported.

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

fn fixture(name: &str, rules: &str, records: &[(&str, &str)]) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("urzua-case-variant-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n"
        ),
    )
    .unwrap();
    for (file, content) in records {
        std::fs::write(dir.join("docs/adr").join(file), content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let parsed = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "check must print JSON ({e}): {} / {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap_or(-1), parsed)
}

fn execution<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["rules_executed"]
        .as_array()
        .expect("rules_executed")
        .iter()
        .find(|r| r["rule"] == id)
        .unwrap_or_else(|| panic!("{id} absent from rules_executed: {report}"))
}

fn findings_of<'a>(report: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["rule"] == id)
        .collect()
}

const VARIANT: &str = "---\nStatus: Accepted\nstatus: Draft\n---\n# 1 — X\n";
const CLEAN: &str = "---\nStatus: Accepted\n---\n# 2 — Y\n";

#[test]
fn a_declared_rule_reports_the_case_variant_and_what_it_examined_observed_failing() {
    let dir = fixture(
        "declared",
        "{header.case-variant-key: error}",
        &[("ADR-1-x.md", VARIANT), ("ADR-2-y.md", CLEAN)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "an error-level finding blocks: {report}");

    let exec = execution(&report, "header.case-variant-key");
    assert_eq!(exec["status"], "ran");
    assert_eq!(exec["population"]["eligible"], 2, "{exec}");
    assert_eq!(exec["population"]["examined"], 2, "{exec}");

    let findings = findings_of(&report, "header.case-variant-key");
    assert_eq!(findings.len(), 1, "{report}");
    assert!(findings[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-1-x.md"));
    let message = findings[0]["message"].as_str().unwrap();
    assert!(message.contains("'Status'") && message.contains("'status'"));
}

#[test]
fn an_undeclared_rule_does_not_run() {
    let dir = fixture(
        "undeclared",
        "{header.required-fields: error}",
        &[("ADR-1-x.md", VARIANT)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    let exec = execution(&report, "header.case-variant-key");
    assert_eq!(exec["status"], "not-enabled");
    assert!(exec.get("population").is_none(), "{exec}");
}

#[test]
fn the_declared_level_sets_the_severity() {
    let dir = fixture(
        "warn",
        "{header.case-variant-key: warn}",
        &[("ADR-1-x.md", VARIANT)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "a warning does not block: {report}");
    let findings = findings_of(&report, "header.case-variant-key");
    assert_eq!(findings.len(), 1, "{report}");
    assert_eq!(findings[0]["severity"], "warning", "{report}");
}

#[test]
fn it_composes_beside_the_exact_match_rules_without_changing_them() {
    let dir = fixture(
        "composed",
        "{header.case-variant-key: error, header.required-fields: error, header.field-set-consistency: error}",
        &[("ADR-1-x.md", VARIANT)],
    );
    let (_, report) = check(&dir);
    let rules: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert_eq!(rules, vec!["header.case-variant-key"], "{report}");
}
