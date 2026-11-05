//! This repository's own `.urzua/config.yaml`, exercised end to end: the
//! declarations it ships are what these tests run, not a fixture's.
//! BUG-123 closes here; BUG-57 and BUG-65 are covered end to end.

mod common;

use std::path::Path;
use std::process::Command;

fn own_config() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::read_to_string(path).expect("repository config readable")
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run")
        .success());
}

/// A fresh git repo holding the repository's own config and `files`.
fn repo(name: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-own-config-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    for ty_dir in ["adr", "rfc", "specs", "bugs", "milestones", "waiver"] {
        std::fs::create_dir_all(dir.join("docs").join(ty_dir)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), own_config()).unwrap();
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

/// The H1 of the one file `urzua new <ty>` wrote under `dir_name`.
fn new_title(dir: &Path, ty: &str, dir_name: &str) -> String {
    let out = urzua(dir, &["new", ty, "A fresh record", "--by", "tester"]);
    assert!(
        out.status.success(),
        "{ty}: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_dir(dir.join(dir_name))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().contains("a-fresh-record"))
        .unwrap_or_else(|| panic!("{ty}: no file written under {dir_name}"));
    std::fs::read_to_string(written)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("# "))
        .expect("an H1")
        .to_string()
}

/// BUG-123, observed failing: `new` opened every title with the bare number,
/// while every spec in the corpus carries `SPEC-N`. The spec type now declares
/// its title form through its identity pattern; the other types keep the
/// bare number.
#[test]
fn new_opens_a_spec_title_with_its_declared_prefix_and_other_types_with_the_bare_number() {
    let dir = repo("h1", &[]);
    assert!(
        new_title(&dir, "spec", "docs/specs").starts_with("# SPEC-1 "),
        "spec"
    );
    assert!(
        new_title(&dir, "adr", "docs/adr").starts_with("# 1 "),
        "adr"
    );
    assert!(
        new_title(&dir, "rfc", "docs/rfc").starts_with("# 1 "),
        "rfc"
    );
    assert!(
        new_title(&dir, "bug", "docs/bugs").starts_with("# 1 "),
        "bug"
    );
    assert!(
        new_title(&dir, "milestone", "docs/milestones").starts_with("# 1 "),
        "milestone"
    );
}

fn bug(status: &str, regression_test: &str) -> String {
    format!(
        "---\nStable-Id: 01M6JS58VTV16JK13MJWQZTPG7\nStatus: {status}\nFound-in: a review\nRegression-test: {regression_test}\n---\n# 1 — A bug\n\n## What was wrong\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

fn finding_rules(dir: &Path) -> Vec<String> {
    let out = urzua(dir, &["check", "docs/"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON on stdout");
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["file"].as_str().unwrap().contains("BUG-1-"))
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

/// BUG-57: a `Fixed` bug whose `Regression-test` says no test exists is a
/// finding; an `Open` one, and a `Fixed` one naming a test, are not. The
/// terminal status is declared (`statuses.require`), not inferred, and "not yet
/// written" is in the type's declared pending vocabulary (BUG-65).
#[test]
fn a_fixed_bug_declaring_no_regression_test_is_a_finding_and_an_open_one_is_not() {
    for placeholder in [
        "not yet written -- a planted case",
        "'Not yet written'",
        "pending",
    ] {
        let dir = repo(
            "fixed-placeholder",
            &[("docs/bugs/BUG-1-a-bug.md", &bug("Fixed", placeholder))],
        );
        let rules = finding_rules(&dir);
        assert!(
            rules.contains(&"status.required-field-missing".to_string()),
            "{placeholder}: {rules:?}"
        );
    }
    let open = repo(
        "open-placeholder",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug("Open", "not yet written -- planted"),
        )],
    );
    assert!(
        finding_rules(&open).is_empty(),
        "{:?}",
        finding_rules(&open)
    );
    let real = repo(
        "fixed-real",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug("Fixed", "rust/crates/x/tests/y.rs::a_real_test"),
        )],
    );
    assert!(
        finding_rules(&real).is_empty(),
        "{:?}",
        finding_rules(&real)
    );
}

/// BUG-65: the corpus's commonest way of saying pending is read as pending,
/// and a value that merely mentions the word is a real one.
#[test]
fn the_declared_pending_vocabulary_reads_whole_values_only() {
    let dir = repo(
        "mentions-pending",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug(
                "Fixed",
                "'rust/crates/x/tests/y.rs::a_pending_value_is_real'",
            ),
        )],
    );
    let rules = finding_rules(&dir);
    assert!(rules.is_empty(), "{rules:?}");
}
