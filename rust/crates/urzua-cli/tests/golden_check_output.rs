//! Golden `check` output, produced before three behaviour-preserving changes and
//! asserted unchanged after them (MILE-111, MILE-112, MILE-131).
//!
//! `fixtures/golden-check/corpus` is a small corpus whose config enables every
//! rule that existed before the earliest of the three changes, with records that
//! trip a spread of them (a missing and a blank field, a placeholder, an
//! unresolved pointer, a missing locator, an undeclared field, an untrimmed value,
//! an identity collision, a record outside its type's directory, an unclean
//! pointer and several schema-inventory findings). `fixtures/golden-check/golden.json`
//! is what `urzua check docs/` printed over it, reduced to the fields every
//! release has had (`rule`, `severity`, `file`, `line`, `message`), the repository
//! repository path removed.
//!
//! The golden was written by the binary built from each change's parent commit,
//! never by the code under test: the three binaries printed identical bytes, and
//! so does the current one. If a later change moves a finding on purpose, the
//! golden is edited in that change, where a reviewer sees the new text beside it.

mod common;

use std::path::Path;
use std::process::Command;

fn fixture_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-check")
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// `check docs/` over the fixture corpus, reduced to the golden's shape.
fn current_output() -> serde_json::Value {
    let dir = common::TempDir::new("urzua-golden-check");
    copy_tree(&fixture_dir().join("corpus"), &dir);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);

    let output = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .expect("urzua binary should run");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "check must print JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let root = std::fs::canonicalize(&*dir).unwrap();
    let root = root.to_string_lossy();
    let findings: Vec<serde_json::Value> = report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .map(|f| {
            let mut kept = serde_json::Map::new();
            for key in ["file", "line", "message", "rule", "severity"] {
                kept.insert(key.to_string(), f[key].clone());
            }
            serde_json::Value::Object(kept)
        })
        .collect();
    let reduced = serde_json::json!({
        "blocking": report["blocking"],
        "files_examined": report["files_examined"],
        "findings": findings,
        "status": report["status"],
    });
    let text = serde_json::to_string(&reduced)
        .unwrap()
        .replace(&format!("{root}/"), "");
    serde_json::from_str(&text).unwrap()
}

fn golden() -> serde_json::Value {
    let text = std::fs::read_to_string(fixture_dir().join("golden.json")).expect("golden.json");
    serde_json::from_str(&text).expect("golden.json is JSON")
}

fn assert_unchanged(change: &str) {
    let want = golden();
    assert!(
        want["findings"].as_array().is_some_and(|f| f.len() >= 10),
        "a golden with next to nothing in it proves nothing"
    );
    let got = current_output();
    assert_eq!(
        got,
        want,
        "`check` output changed ({change}); the golden was written before it:\n{}",
        serde_json::to_string_pretty(&got).unwrap()
    );
}

/// MILE-111: disclosing unreadable headers on their own (`Population::unreadable`)
/// and adding the opt-in `relation.target-status-undeclared` change no finding
/// over a corpus with no unreadable header and no undeclared-target reference.
/// Golden written by the binary built from the parent of the change (#110).
#[test]
fn check_output_is_unchanged_by_the_unreadable_disclosure_and_the_undeclared_target_rule() {
    assert_unchanged("MILE-111, #110");
}

/// MILE-112: the duplicated-logic sweep (`BUG-139`, `BUG-140`, `BUG-141`) moved
/// the filename-id extraction, the declared-field guard of the four reference
/// rules and the schema-inventory skeleton of the eight config rules into shared
/// helpers; the corpus trips each of those families. Golden written by the binary
/// built from the parent of the sweep (#124).
#[test]
fn check_output_is_unchanged_by_the_duplicated_logic_sweep() {
    assert_unchanged("MILE-112, #124");
}

/// MILE-131: splitting `rules.rs` into one module per rule family is a pure move.
/// Golden written by the binary built from the parent of the split (#152).
#[test]
fn check_output_is_unchanged_by_the_rules_module_split() {
    assert_unchanged("MILE-131, #152");
}
