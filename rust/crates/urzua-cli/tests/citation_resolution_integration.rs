//! `citation.resolution` (MILE-136) end to end: the real binary against an
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
    let dir = std::env::temp_dir().join(format!("urzua-citation-{name}-{}", std::process::id()));
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

const CITES_MISSING: &str = "# 1 — X\n\n> Status: Accepted\n\nSee ADR-99.\n\n```\nADR-98\n```\n";
const CITES_PRESENT: &str = "# 2 — Y\n\n> Status: Accepted\n\nSee ADR-0001.\n";

#[test]
fn a_declared_rule_reports_a_dangling_citation_and_counts_citations() {
    let dir = fixture(
        "dangling",
        "{ citation.resolution: error }",
        &[("ADR-1-x.md", CITES_MISSING), ("ADR-2-y.md", CITES_PRESENT)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "citation.resolution");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"].as_str().unwrap().ends_with("ADR-1-x.md"));
    assert_eq!(found[0]["line"], 5);
    let pop = &execution(&report, "citation.resolution")["population"];
    assert_eq!(pop["unit"], "citation");
    assert_eq!(
        (pop["eligible"].as_u64(), pop["examined"].as_u64()),
        (Some(2), Some(2))
    );
}

#[test]
fn an_undeclared_rule_does_not_run() {
    let dir = fixture("optin", "{}", &[("ADR-1-x.md", CITES_MISSING)]);
    let (_, report) = check(&dir);
    assert_eq!(
        execution(&report, "citation.resolution")["status"],
        "not-enabled"
    );
    assert!(findings_of(&report, "citation.resolution").is_empty());
}
