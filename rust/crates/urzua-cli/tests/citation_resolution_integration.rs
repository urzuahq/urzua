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

#[test]
fn a_declared_external_id_is_skipped_and_counted_out_of_scope_end_to_end() {
    let body = "# 1 — X\n\n> Status: Accepted\n\nSee ADR-4122 and ADR-99.\n";
    let dir = fixture(
        "external",
        "{ citation.resolution: { level: error, external_ids: [\"ADR-4122\"] } }",
        &[("ADR-1-x.md", body)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "citation.resolution");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"].as_str().unwrap().contains("ADR-99"));
    let pop = &execution(&report, "citation.resolution")["population"];
    assert_eq!(
        (
            pop["eligible"].as_u64(),
            pop["examined"].as_u64(),
            pop["out_of_scope"].as_u64()
        ),
        (Some(2), Some(1), Some(1))
    );
}

#[test]
fn an_external_id_under_no_declared_prefix_fails_the_config_load() {
    let dir = fixture(
        "external-bad",
        "{ citation.resolution: { level: error, external_ids: [\"FOO-1\"] } }",
        &[("ADR-1-x.md", "# 1 — X\n\n> Status: Accepted\n")],
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(0));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(all.contains("external_ids"), "{all}");
}

fn path_fixture(name: &str, body: &str, extra: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = fixture(
        name,
        "{ citation.path-exists: { level: warn, reference_sections: [\"References\"] } }",
        &[("ADR-1-x.md", body)],
    );
    for (file, content) in extra {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "more"]);
    dir
}

#[test]
fn a_cited_path_that_is_missing_warns_and_an_existing_file_or_directory_does_not() {
    let body = "# 1 — X\n\n> Status: Accepted\n\n## References\n\n- `src/real.rs`, `src/`, `src/gone.rs`\n";
    let dir = path_fixture("paths", body, &[("src/real.rs", "fn main() {}\n")]);
    let (code, report) = check(&dir);
    let found = findings_of(&report, "citation.path-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["severity"], "warning", "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("src/gone.rs"));
    assert_eq!(
        code, 0,
        "a warning does not fail the default gate: {report}"
    );
    let pop = &execution(&report, "citation.path-exists")["population"];
    assert_eq!(
        (pop["unit"].as_str(), pop["examined"].as_u64()),
        (Some("citation"), Some(3))
    );
}

#[test]
fn an_untracked_but_present_file_is_still_missing() {
    let body = "# 1 — X\n\n> Status: Accepted\n\n## References\n\n- `src/local.rs`\n";
    let dir = path_fixture("untracked", body, &[]);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/local.rs"), "x\n").unwrap();
    let (_, report) = check(&dir);
    assert_eq!(
        findings_of(&report, "citation.path-exists").len(),
        1,
        "{report}"
    );
}
