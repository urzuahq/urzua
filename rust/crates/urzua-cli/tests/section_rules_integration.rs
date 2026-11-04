//! `section.required` and `section.shape` (MILE-4, MILE-5, MILE-7, MILE-121)
//! end to end: the real binary against an isolated git fixture. Both rules are
//! opt-in and read the type's declared `sections`, so one test proves a type
//! without the declaration is untouched and the rest prove what ran.

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

const SECTIONS: &str = "    sections:
      from: headings
      required:
        - {name: Context}
        - {name: Options, shape: table-min-rows, min_rows: 2}
        - {name: Decision, shape: y-statement, marker: 'TL;DR'}
      exempt_statuses: [Withdrawn]
";

fn fixture(
    name: &str,
    rules: &str,
    sections: &str,
    records: &[(&str, &str)],
) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-section-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n{sections}"
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

const GOOD: &str = "> Status: Accepted\n\n# 1 — Good\n\n## Context\n\nc\n\n## Options\n\n| Option | Pros |\n|---|---|\n| A | x |\n| B | y |\n\n## Decision\n\nTL;DR: In the context of a, facing b, we decided c, to achieve d, accepting e.\n";
const BAD: &str = "> Status: Accepted\n\n# 2 — Bad\n\n## Options\n\n| Option | Pros |\n|---|---|\n| A | x |\n\n## Decision\n\nTL;DR: we decided c.\n";
const WITHDRAWN: &str = "> Status: Withdrawn\n\n# 3 — Gone\n\nVacated.\n";

#[test]
fn declared_rules_report_each_failure_and_count_slots() {
    let dir = fixture(
        "report",
        "{ section.required: error, section.shape: error }",
        SECTIONS,
        &[
            ("ADR-1-good.md", GOOD),
            ("ADR-2-bad.md", BAD),
            ("ADR-3-gone.md", WITHDRAWN),
        ],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");

    let required = findings_of(&report, "section.required");
    assert_eq!(required.len(), 1, "{report}");
    assert!(required[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-2-bad.md"));
    assert!(required[0]["message"].as_str().unwrap().contains("Context"));
    let pop = &execution(&report, "section.required")["population"];
    assert_eq!(pop["unit"], "section");
    // 3 records x 3 required sections; the withdrawn record is out of scope.
    assert_eq!(
        (
            pop["eligible"].as_u64(),
            pop["examined"].as_u64(),
            pop["out_of_scope"].as_u64()
        ),
        (Some(9), Some(6), Some(3))
    );

    let shape = findings_of(&report, "section.shape");
    assert_eq!(shape.len(), 2, "{report}");
    assert!(shape
        .iter()
        .all(|f| f["file"].as_str().unwrap().ends_with("ADR-2-bad.md")));
    let pop = &execution(&report, "section.shape")["population"];
    // 2 shaped sections x 3 records.
    assert_eq!(
        (
            pop["eligible"].as_u64(),
            pop["examined"].as_u64(),
            pop["out_of_scope"].as_u64()
        ),
        (Some(6), Some(4), Some(2))
    );
}

#[test]
fn undeclared_rules_do_not_run() {
    let dir = fixture("optin", "{}", SECTIONS, &[("ADR-2-bad.md", BAD)]);
    let (_, report) = check(&dir);
    for id in ["section.required", "section.shape"] {
        assert_eq!(execution(&report, id)["status"], "not-enabled");
        assert!(findings_of(&report, id).is_empty());
    }
}

#[test]
fn a_type_without_the_declaration_has_nothing_eligible() {
    let dir = fixture(
        "undeclared",
        "{ section.required: error, section.shape: error }",
        "",
        &[("ADR-2-bad.md", BAD)],
    );
    let (_, report) = check(&dir);
    for id in ["section.required", "section.shape"] {
        let pop = &execution(&report, id)["population"];
        assert_eq!(pop["eligible"], 0, "{id}: {report}");
        assert!(findings_of(&report, id).is_empty());
    }
}
