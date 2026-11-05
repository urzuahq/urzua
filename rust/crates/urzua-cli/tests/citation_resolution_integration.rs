//! `citation.resolution` (MILE-136) end to end: the real binary against an
//! isolated git fixture. The rule is opt-in, so one test proves it does not
//! run unless declared, and the others prove what it examined and reported.

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

fn fixture(name: &str, rules: &str, records: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-citation-{name}"));
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

const CITES_MISSING: &str =
    "---\nStatus: Accepted\n---\n# 1 — X\n\nSee ADR-99.\n\n```\nADR-98\n```\n";
const CITES_PRESENT: &str = "---\nStatus: Accepted\n---\n# 2 — Y\n\nSee ADR-0001.\n";

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
    assert_eq!(found[0]["line"], 6);
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
fn a_declared_external_id_is_skipped_and_counted_eligible_not_out_of_scope_end_to_end() {
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\nSee ADR-4122 and ADR-99.\n";
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
        (Some(2), Some(1), Some(0))
    );
}

#[test]
fn a_declared_external_id_is_not_a_misconfiguration_end_to_end_observed_failing() {
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\nSee ADR-4122.\n";
    let dir = fixture(
        "external-scope",
        "{ config.scope-matches-nothing: warn, citation.resolution: { level: error, external_ids: [\"ADR-4122\"] } }",
        &[("ADR-1-x.md", body)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    assert!(
        findings_of(&report, "config.scope-matches-nothing").is_empty(),
        "{report}"
    );
}

#[test]
fn a_skipped_revision_log_hides_revision_log_paths_end_to_end_observed_failing() {
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\n## References\n\n- `src/gone.rs`\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Named `src/old.rs`. | **substantive** |\n";
    let dir = fixture(
        "skip-table",
        "{ citation.path-exists: { level: warn, reference_sections: [\"References\"], skip_revision_log: true } }",
        &[("ADR-1-x.md", body)],
    );
    let (_, report) = check(&dir);
    let found = findings_of(&report, "citation.path-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("src/gone.rs"));
}

#[test]
fn an_external_id_under_no_declared_prefix_fails_the_config_load() {
    let dir = fixture(
        "external-bad",
        "{ citation.resolution: { level: error, external_ids: [\"FOO-1\"] } }",
        &[("ADR-1-x.md", "---\nStatus: Accepted\n---\n# 1 — X\n")],
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

fn path_fixture(name: &str, body: &str, extra: &[(&str, &str)]) -> common::TempDir {
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
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\n## References\n\n- `src/real.rs`, `src/`, `src/gone.rs`\n";
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
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\n## References\n\n- `src/local.rs`\n";
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

#[test]
fn a_declared_revision_log_heading_is_the_log_and_prose_mentions_are_not_observed_failing() {
    let body = "---\nStatus: Accepted\n---\n# 1 — X\n\nSee the **Revision log** below.\n\n## History\n\n| Date | Change | Class |\n|---|---|---|\n| 2026-01-01 | Filed. | — |\n";
    let dir = fixture(
        "revlog-heading",
        "{ revision-log.change-class-required: error }",
        &[("ADR-1-x.md", body)],
    );
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: { revision-log.change-class-required: error }\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    revision_log: { heading: \"History\" }\n",
    )
    .unwrap();
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "revision-log.change-class-required");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"].as_str().unwrap().contains("2026-01-01"));
}

fn nonref_fixture(name: &str, records: &[(&str, &str)]) -> common::TempDir {
    let dir = fixture(
        name,
        "{ citation.resolution: { level: error, non_reference_field: Non-reference-ids } }",
        records,
    );
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: { citation.resolution: { level: error, non_reference_field: Non-reference-ids } }\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Non-reference-ids\"]\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "config"]);
    dir
}

const DECLARES: &str =
    "---\nStatus: Accepted\nNon-reference-ids: ADR-72\n---\n# 1 — X\n\nAn example: ADR-72/73/74.\n";

#[test]
fn a_declared_non_reference_id_survives_the_id_becoming_a_real_record_end_to_end_observed_failing()
{
    let dir = nonref_fixture("nonref", &[("ADR-1-x.md", DECLARES)]);
    let (code, before) = check(&dir);
    assert_eq!(code, 0, "{before}");
    assert!(
        findings_of(&before, "citation.resolution").is_empty(),
        "{before}"
    );

    std::fs::write(
        dir.join("docs/adr/ADR-72-real.md"),
        "---\nStatus: Accepted\n---\n# 72 — Real\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "ADR-72 now exists"]);
    let (code, after) = check(&dir);
    assert_eq!(code, 0, "{after}");
    assert!(
        findings_of(&after, "citation.resolution").is_empty(),
        "{after}"
    );
    let notices = after["notices"].to_string();
    assert!(
        notices.contains("non-reference") && notices.contains("ADR-72"),
        "{after}"
    );
}

#[test]
fn a_declared_id_the_record_never_cites_is_a_finding_end_to_end_observed_failing() {
    let doc = "---\nStatus: Accepted\nNon-reference-ids: ADR-72\n---\n# 1 — X\n\nNothing cited.\n";
    let dir = nonref_fixture("nonref-uncited", &[("ADR-1-x.md", doc)]);
    let (code, report) = check(&dir);
    assert_ne!(code, 0, "{report}");
    let found = findings_of(&report, "citation.resolution");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"].as_str().unwrap().contains("ADR-72"));
}

#[test]
fn a_non_reference_field_no_type_lists_fails_the_config_load() {
    let dir = fixture(
        "nonref-bad",
        "{ citation.resolution: { level: error, non_reference_field: Nope } }",
        &[("ADR-1-x.md", "---\nStatus: Accepted\n---\n# 1 — X\n")],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_urzua"))
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
    assert!(all.contains("non_reference_field"), "{all}");
}

// ---- the field-entry population (MILE-186, ADR-75 clause 3) ---------------------

fn executions_of<'a>(report: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    report["rules_executed"]
        .as_array()
        .expect("rules_executed")
        .iter()
        .filter(|r| r["rule"] == id)
        .collect()
}

#[test]
fn declared_entries_are_their_own_field_entry_population_observed_failing() {
    // Two valid entries (one cited, one not) and one malformed entry.
    let doc = "---\nStatus: Accepted\nNon-reference-ids: ADR-72, ADR-73, nope\n---\n# 1 — X\n\nAn example: ADR-72.\n";
    let dir = nonref_fixture("nonref-population", &[("ADR-1-x.md", doc)]);
    let (_, report) = check(&dir);
    let runs = executions_of(&report, "citation.resolution");
    assert_eq!(runs.len(), 2, "{report}");
    // The first is the citation population, as before.
    assert_eq!(runs[0]["population"]["unit"], "citation", "{report}");
    // The second counts each declared entry: valid, uncited and malformed alike.
    let entries = &runs[1]["population"];
    assert_eq!(entries["unit"], "field-entry", "{report}");
    assert_eq!(entries["eligible"], 3, "{report}");
    assert_eq!(entries["examined"], 3, "{report}");
    assert_eq!(entries["out_of_scope"], 0, "{report}");
    // The uncited and malformed findings are still citation.resolution's.
    assert_eq!(
        findings_of(&report, "citation.resolution").len(),
        2,
        "{report}"
    );
    // And the run is not read as a misconfiguration.
    assert!(
        findings_of(&report, "config.scope-matches-nothing").is_empty(),
        "{report}"
    );
}

#[test]
fn a_run_over_no_declarations_reports_zero_eligible_entries_observed_failing() {
    let dir = nonref_fixture("nonref-none", &[("ADR-1-x.md", CITES_PRESENT)]);
    let (_, report) = check(&dir);
    let runs = executions_of(&report, "citation.resolution");
    assert_eq!(runs.len(), 2, "{report}");
    assert_eq!(runs[1]["population"]["unit"], "field-entry", "{report}");
    assert_eq!(runs[1]["population"]["eligible"], 0, "{report}");
    assert_eq!(runs[1]["status"], "ran", "{report}");
}

#[test]
fn without_non_reference_field_there_is_no_second_population() {
    let dir = fixture(
        "nonref-unset",
        "{ citation.resolution: { level: error } }",
        &[("ADR-1-x.md", CITES_PRESENT)],
    );
    let (_, report) = check(&dir);
    assert_eq!(
        executions_of(&report, "citation.resolution").len(),
        1,
        "{report}"
    );
}

#[test]
fn the_field_entry_population_is_exercised_on_this_repositorys_own_corpus_observed_failing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let output = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check"])
        .current_dir(&root)
        .output()
        .expect("urzua binary should run");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let runs = executions_of(&report, "citation.resolution");
    assert_eq!(runs.len(), 2, "{report}");
    let entries = &runs[1]["population"];
    assert_eq!(entries["unit"], "field-entry");
    // ADR-18 and RFC-5 each declare ADR-72.
    assert!(entries["eligible"].as_u64().unwrap() >= 2, "{entries}");
    assert_eq!(entries["eligible"], entries["examined"], "{entries}");
    assert_eq!(entries["out_of_scope"], 0);
}
