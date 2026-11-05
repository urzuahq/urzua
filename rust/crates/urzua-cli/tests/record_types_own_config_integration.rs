//! The record types and read-only commands SPEC-6, 9, 11, 13, 15, 16 and 17
//! describe, exercised end to end by the real binary under this repository's own
//! `.urzua/config.yaml`: the declarations it ships are what these tests run.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run")
        .success());
}

/// A fresh git repo holding the repository's own config and `files`.
fn repo(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-types-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    let cfg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::copy(cfg, dir.join(".urzua/config.yaml")).unwrap();
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("JSON on stdout")
}

/// Rules of the findings `args` reports against files whose path contains `file`.
fn rules_for(dir: &Path, args: &[&str], file: &str) -> Vec<String> {
    let report = json(&urzua(dir, args));
    report["findings"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter(|f| f["file"].as_str().is_some_and(|p| p.contains(file)))
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

fn check_rules(dir: &Path, file: &str) -> Vec<String> {
    rules_for(dir, &["check", "docs/"], file)
}

fn milestone(status: &str, extra: &str, done_means: bool) -> String {
    let section = if done_means {
        "\n## Done means\n\nIt works.\n"
    } else {
        ""
    };
    format!(
        "---\nStatus: {status}\nPhase: '1'\nTrack: t\n{extra}---\n# 1 — A milestone\n\n## What\n\nx\n{section}\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

// ---- SPEC-6: the milestone type

#[test]
fn a_milestone_must_state_its_done_means_unless_it_is_wontdo() {
    let path = "docs/milestones/MILE-1-a-milestone.md";
    let bare = repo("mile-bare", &[(path, &milestone("Planned", "", false))]);
    assert!(check_rules(&bare, "MILE-1-").contains(&"section.required".to_string()));
    let full = repo("mile-full", &[(path, &milestone("Planned", "", true))]);
    assert!(check_rules(&full, "MILE-1-").is_empty());
    let wont = repo("mile-wont", &[(path, &milestone("WontDo", "", false))]);
    assert!(check_rules(&wont, "MILE-1-").is_empty());
}

#[test]
fn a_done_milestone_needs_delivery_evidence_and_evidence_under_planned_is_a_disagreement() {
    let path = "docs/milestones/MILE-1-a-milestone.md";
    let done = repo("mile-done", &[(path, &milestone("Done", "", true))]);
    assert!(
        check_rules(&done, "MILE-1-").contains(&"status.delivery-agreement".to_string()),
        "{:?}",
        check_rules(&done, "MILE-1-")
    );
    let evidenced = repo(
        "mile-evidenced",
        &[
            (
                path,
                &milestone("Planned", "Realized-by: code:src/lib.rs\n", true),
            ),
            ("src/lib.rs", "// code\n"),
        ],
    );
    assert!(
        check_rules(&evidenced, "MILE-1-").contains(&"status.delivery-agreement".to_string()),
        "{:?}",
        check_rules(&evidenced, "MILE-1-")
    );
}

#[test]
fn a_milestone_status_outside_the_declared_vocabulary_is_a_finding() {
    let bad = repo(
        "mile-bad-status",
        &[(
            "docs/milestones/MILE-1-a-milestone.md",
            &milestone("Shipping", "", true),
        )],
    );
    assert!(check_rules(&bad, "MILE-1-").contains(&"status.undeclared-value".to_string()));
}

// ---- SPEC-9: the bug type

fn bug(status: &str, header_extra: &str, heading: &str) -> String {
    format!("---\nStatus: {status}\n{header_extra}---\n# 1 — A bug\n\n## {heading}\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n")
}

#[test]
fn a_bug_needs_found_in_and_regression_test_and_a_what_was_wrong_section() {
    let path = "docs/bugs/BUG-1-a-bug.md";
    let missing = repo("bug-missing", &[(path, &bug("Open", "", "What was wrong"))]);
    assert!(check_rules(&missing, "BUG-1-").contains(&"header.required-fields".to_string()));
    let ok = "Found-in: a review\nRegression-test: pending\n";
    let good = repo("bug-good", &[(path, &bug("Open", ok, "What was wrong"))]);
    assert!(check_rules(&good, "BUG-1-").is_empty());
    let alias = repo("bug-alias", &[(path, &bug("Open", ok, "What is wrong"))]);
    assert!(check_rules(&alias, "BUG-1-").is_empty());
    let none = repo("bug-nosection", &[(path, &bug("Open", ok, "Notes"))]);
    assert!(check_rules(&none, "BUG-1-").contains(&"section.required".to_string()));
}

#[test]
fn a_bug_status_outside_the_declared_vocabulary_is_a_finding_and_fixed_may_be_filed() {
    let ok = "Found-in: a review\nRegression-test: x::y\n";
    let closed = repo(
        "bug-closed",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug("Closed", ok, "What was wrong"),
        )],
    );
    assert!(check_rules(&closed, "BUG-1-").contains(&"status.undeclared-value".to_string()));
    let fixed = repo(
        "bug-fixed",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug("Fixed", ok, "What was wrong"),
        )],
    );
    assert!(check_rules(&fixed, "BUG-1-").is_empty());
}

// ---- SPEC-16: the ADR type

fn n2(record: String) -> String {
    record.replace("# 1 —", "# 2 —")
}

fn adr(status: &str, extra: &str) -> String {
    format!(
        "---\nStatus: {status}\nDate: 2026-01-01\nAuthor: a\n{extra}---\n# 1 — A decision\n\n## Context\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

#[test]
fn an_accepted_adr_must_name_its_deciders_and_enters_proposed() {
    let path = "docs/adr/ADR-1-a-decision.md";
    let no_deciders = repo("adr-nodec", &[(path, &adr("Proposed", ""))]);
    assert!(check_rules(&no_deciders, "ADR-1-").contains(&"header.required-fields".to_string()));
    let accepted = repo(
        "adr-accepted",
        &[(path, &adr("Accepted", "Deciders: unknown\n"))],
    );
    let rules = check_rules(&accepted, "ADR-1-");
    assert!(
        rules.contains(&"status.required-field-missing".to_string()),
        "{rules:?}"
    );
}

#[test]
fn adr_supersession_must_be_reciprocated_and_a_retired_decision_is_a_declared_status() {
    let old = |link: &str| {
        adr(
            "Superseded",
            &format!("Deciders: a\nSupersedes / Superseded-by: {link}\n"),
        )
    };
    let new = |link: &str| {
        adr(
            "Accepted",
            &format!("Deciders: a\nSupersedes / Superseded-by: {link}\n"),
        )
    };
    let one_way = repo(
        "adr-oneway",
        &[
            ("docs/adr/ADR-1-a-decision.md", &old("ADR-2")),
            ("docs/adr/ADR-2-a-decision.md", &n2(new("—"))),
        ],
    );
    assert!(
        check_rules(&one_way, "ADR-").contains(&"relation.supersession-reciprocity".to_string())
    );
    let both = repo(
        "adr-both",
        &[
            ("docs/adr/ADR-1-a-decision.md", &old("ADR-2")),
            ("docs/adr/ADR-2-a-decision.md", &n2(new("ADR-1"))),
        ],
    );
    assert!(
        !check_rules(&both, "ADR-").contains(&"relation.supersession-reciprocity".to_string()),
        "{:?}",
        check_rules(&both, "ADR-")
    );
    let retired = repo(
        "adr-retired",
        &[(
            "docs/adr/ADR-1-a-decision.md",
            &adr("Retired", "Deciders: a\n"),
        )],
    );
    assert!(!check_rules(&retired, "ADR-1-").contains(&"status.undeclared-value".to_string()));
}

// ---- SPEC-17: the RFC type

fn rfc(status: &str, extra: &str) -> String {
    format!(
        "---\nStatus: {status}\nDate: 2026-01-01\nAuthor: a\n{extra}---\n# 1 — A proposal\n\n## Summary\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

#[test]
fn an_rfc_follows_its_declared_lifecycle_and_vocabulary() {
    let path = "docs/rfc/RFC-1-a-proposal.md";
    let draft = repo("rfc-draft", &[(path, &rfc("Draft", ""))]);
    assert!(
        check_rules(&draft, "RFC-1-").is_empty(),
        "{:?}",
        check_rules(&draft, "RFC-1-")
    );
    let bad = repo("rfc-bad", &[(path, &rfc("Proposed", ""))]);
    assert!(check_rules(&bad, "RFC-1-").contains(&"status.undeclared-value".to_string()));
}

#[test]
fn an_rfc_amending_another_must_be_echoed_by_the_amended_rfc() {
    let amended = |extra: &str| rfc("Accepted", extra);
    let one_way = repo(
        "rfc-amends",
        &[
            ("docs/rfc/RFC-1-base.md", &amended("")),
            ("docs/rfc/RFC-2-amender.md", &amended("Amends: RFC-1\n")),
        ],
    );
    let rules = rules_for(&one_way, &["check", "docs/"], "RFC-");
    assert!(
        rules.contains(&"relation.pair-reciprocity".to_string()),
        "{rules:?}"
    );
    let both = repo(
        "rfc-amends-echoed",
        &[
            ("docs/rfc/RFC-1-base.md", &amended("Amended-by: RFC-2\n")),
            ("docs/rfc/RFC-2-amender.md", &amended("Amends: RFC-1\n")),
        ],
    );
    assert!(
        !check_rules(&both, "RFC-").contains(&"relation.pair-reciprocity".to_string()),
        "{:?}",
        check_rules(&both, "RFC-")
    );
}

// ---- SPEC-11: audit

#[test]
fn audit_reports_a_one_directional_supersession_and_passes_a_reciprocated_one() {
    let old = adr(
        "Superseded",
        "Deciders: a\nSupersedes / Superseded-by: ADR-2\n",
    );
    let new_oneway = adr("Accepted", "Deciders: a\nSupersedes / Superseded-by: —\n");
    let new_both = adr(
        "Accepted",
        "Deciders: a\nSupersedes / Superseded-by: ADR-1\n",
    );
    let bad = repo(
        "audit-bad",
        &[
            ("docs/adr/ADR-1-a-decision.md", &old),
            ("docs/adr/ADR-2-a-decision.md", &n2(new_oneway.clone())),
        ],
    );
    let out = urzua(&bad, &["audit"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("relation.supersession-reciprocity"));
    let good = repo(
        "audit-good",
        &[
            ("docs/adr/ADR-1-a-decision.md", &old),
            ("docs/adr/ADR-2-a-decision.md", &n2(new_both)),
        ],
    );
    let out = urzua(&good, &["audit"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// ---- SPEC-13: explain and graph

#[test]
fn explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge() {
    let cited = adr(
        "Accepted",
        "Deciders: a\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n",
    );
    let derived = adr("Accepted", "Deciders: a\nDerives-from: ADR-1, ADR-9\n");
    let dir = repo(
        "explain-graph",
        &[
            ("docs/adr/ADR-1-a-decision.md", &cited),
            ("docs/adr/ADR-2-a-decision.md", &n2(derived)),
            ("src/lib.rs", "// code\n"),
        ],
    );
    let out = urzua(&dir, &["explain", "src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0));
    let governing = json(&out)["governing_records"].clone();
    assert_eq!(governing.as_array().unwrap().len(), 1, "{governing}");
    assert_eq!(governing[0]["record"], "docs/adr/ADR-1-a-decision.md");
    assert_eq!(governing[0]["via"], "code");
    let graph = urzua(&dir, &["graph"]);
    assert_eq!(graph.status.code(), Some(0));
    let edges = json(&graph)["edges"].as_array().unwrap().clone();
    let edge = |to: &str| {
        edges
            .iter()
            .find(|e| e["from"] == "ADR-2" && e["to"] == to)
            .unwrap_or_else(|| panic!("no ADR-2 -> {to} edge in {edges:?}"))
            .clone()
    };
    for to in ["ADR-1", "ADR-9"] {
        assert_eq!(edge(to)["relation"], "Derives-from");
        assert_eq!(edge(to)["kind"], "pointer");
    }
    assert_eq!(edge("ADR-1")["dangling"], false);
    assert_eq!(edge("ADR-9")["dangling"], true);
}

// ---- SPEC-15: doctor

#[test]
fn doctor_reads_the_repositorys_own_config_and_reports_the_unwired_hook_as_a_warning() {
    let dir = repo(
        "doctor",
        &[(
            "docs/adr/ADR-1-a-decision.md",
            &adr("Proposed", "Deciders: a\n"),
        )],
    );
    let report = json(&urzua(&dir, &["doctor"]));
    let checks = report["checks"].as_array().expect("checks array");
    let find = |name: &str| checks.iter().find(|c| c["check"] == name);
    assert_eq!(
        find("config-exists").expect("config-exists")["status"],
        "ok"
    );
    assert_eq!(
        find("config-parses").expect("config-parses")["status"],
        "ok"
    );
    let hook = find("hook-wired").expect("hook-wired check");
    assert_eq!(hook["status"], "warn", "{hook}");
    assert!(
        hook["message"]
            .as_str()
            .unwrap()
            .contains("no pre-commit hook"),
        "{hook}"
    );
}
