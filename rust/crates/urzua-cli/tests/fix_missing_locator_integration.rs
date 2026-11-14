//! `urzua fix` counts only the `Realized-by` locators that resolve: a locator
//! naming a file that is not there is no evidence, so `fix --apply` never
//! writes `Embodiment` or `Status` from it (external review, item 1.1; SPEC-8).
//! Real git-repo fixtures and the real binary; all output is JSON.

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

fn run(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("json: {e}: {out:?}"))
}

const REVISION_LOG: &str =
    "\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";

fn fixture(name: &str, config: &str, path: &str, record: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fix-missing-{name}"));
    for d in ["docs/adr", "docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(path), record).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

const EMBODIMENT_CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const ADR: &str = "docs/adr/ADR-1-first.md";

fn adr(embodiment: &str, realized_by: &str) -> String {
    format!(
        "---\nEmbodiment: {embodiment}\nRealized-by: {realized_by}\n---\n# ADR-1: First\n{REVISION_LOG}"
    )
}

#[test]
fn a_locator_naming_a_missing_test_file_is_not_written_as_verified() {
    let dir = fixture(
        "embodiment-missing-test",
        EMBODIMENT_CONFIG,
        ADR,
        &adr("Not started", "test:tests/missing.rs"),
    );
    let v = run(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    let after = std::fs::read_to_string(dir.join(ADR)).unwrap();
    assert!(
        after.contains("\nEmbodiment: Not started\n"),
        "{v}\n{after}"
    );
    assert!(!after.contains("Verified"), "{v}\n{after}");
}

#[test]
fn a_missing_locator_does_not_lift_the_tier_above_what_the_existing_ones_give() {
    // `src/a.rs` exists, the test file does not: the evidence is code tier.
    let dir = fixture(
        "embodiment-mixed",
        EMBODIMENT_CONFIG,
        ADR,
        &adr("Implemented", "code:src/a.rs, test:tests/missing.rs"),
    );
    let v = run(&dir, &["fix"]);
    assert!(v["repairs"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn a_repair_that_lowers_a_claim_rests_only_on_locators_that_resolve_and_says_which_do_not() {
    let dir = fixture(
        "embodiment-lowering",
        EMBODIMENT_CONFIG,
        ADR,
        &adr("Verified", "code:src/a.rs, test:tests/missing.rs"),
    );
    let v = run(&dir, &["fix"]);
    let repairs = v["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{v}");
    assert_eq!(repairs[0]["computed_value"], "Implemented", "{v}");
    let evidence = repairs[0]["evidence"].as_str().unwrap();
    assert!(
        evidence.contains("tests/missing.rs") && evidence.contains("does not resolve"),
        "{v}"
    );
}

#[test]
fn a_resolving_locator_still_computes_as_before() {
    let dir = fixture(
        "embodiment-resolving",
        EMBODIMENT_CONFIG,
        ADR,
        &adr("Not started", "code:src/a.rs"),
    );
    let v = run(&dir, &["fix"]);
    let repairs = v["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{v}");
    assert_eq!(repairs[0]["computed_value"], "Implemented", "{v}");
}

const STATUS_CONFIG: &str = "schema_version: 2\nrules:\n  status.delivery-agreement: error\n  embodiment.locator-exists: error\nrecord_types:\n  milestone:\n    dir: \"docs/milestones\"\n    prefix: \"MILE\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Realized-by\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Planned, InProgress, Blocked, Done, WontDo]\n      success: [Done]\n      failure: [WontDo]\n      delivery:\n        delivered_tiers: [Implemented, Verified]\n        unevidenced_tiers: [\"Not started\"]\n        not_done: [Planned, InProgress, Blocked]\n        set_status: Done\n";
const MILE: &str = "docs/milestones/MILE-1-x.md";

fn milestone(realized_by: &str) -> String {
    format!(
        "---\nStatus: Planned\nRealized-by: {realized_by}\n---\n# X\n\n## What\n\nx\n{REVISION_LOG}"
    )
}

#[test]
fn a_status_is_not_set_to_done_from_a_locator_naming_a_missing_file() {
    let dir = fixture(
        "status-missing",
        STATUS_CONFIG,
        MILE,
        &milestone("code:src/missing.rs"),
    );
    let v = run(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    let after = std::fs::read_to_string(dir.join(MILE)).unwrap();
    assert!(after.contains("\nStatus: Planned\n"), "{v}\n{after}");
    let check = run(&dir, &["check"]);
    let found: Vec<_> = check["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "status.delivery-agreement")
        .collect();
    assert!(found.is_empty(), "{check}");
}

#[test]
fn a_status_is_still_set_to_done_from_a_locator_that_resolves() {
    let dir = fixture(
        "status-resolving",
        STATUS_CONFIG,
        MILE,
        &milestone("code:src/a.rs"),
    );
    run(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    let after = std::fs::read_to_string(dir.join(MILE)).unwrap();
    assert!(after.contains("\nStatus: Done\n"), "{after}");
}
