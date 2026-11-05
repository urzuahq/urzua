//! MILE-159: `urzua fix [PATHS...]` examines and repairs only those paths.
//! Real git-repo fixtures, real binary; all output is JSON.

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

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";

fn record(n: u32, embodiment: &str) -> String {
    format!(
        "---\nEmbodiment: {embodiment}\nRealized-by: code:src/a.rs\n---\n# ADR-{n}: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// Two records, both stating `Not started` against an implemented locator,
/// so each has one pending repair.
fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fix-paths-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/other")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), record(1, "Not started")).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-2-b.md"), record(2, "Not started")).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn a_path_limits_the_examination_and_the_repairs_to_that_record() {
    let dir = fixture("one");
    let all = json(&run_urzua(&dir, &["fix"]));
    assert_eq!(all["repairs"].as_array().unwrap().len(), 2, "{all}");

    let v = json(&run_urzua(&dir, &["fix", "docs/adr/ADR-1-a.md"]));
    assert_eq!(v["status"], "repairs-available", "{v}");
    let repairs = v["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{v}");
    assert_eq!(repairs[0]["record"], "docs/adr/ADR-1-a.md", "{v}");
    assert_eq!(v["population"]["examined"], 1, "{v}");
}

#[test]
fn a_directory_path_selects_every_record_under_it() {
    let dir = fixture("dir");
    let v = json(&run_urzua(&dir, &["fix", "docs/adr"]));
    assert_eq!(v["repairs"].as_array().unwrap().len(), 2, "{v}");
}

#[test]
fn a_path_matching_no_record_is_reported_not_skipped() {
    let dir = fixture("nomatch");
    let v_out = run_urzua(&dir, &["fix", "docs/other", "docs/adr/ADR-9-missing.md"]);
    let v = json(&v_out);
    assert_eq!(v["status"], "partial-failure", "{v}");
    assert_eq!(v_out.status.code(), Some(1), "{v}");
    let failed = v["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 2, "{v}");
    assert!(
        failed[0]["error"]
            .as_str()
            .unwrap()
            .contains("matched no record"),
        "{v}"
    );
}

#[test]
fn apply_with_a_path_writes_only_that_record() {
    let dir = fixture("apply");
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "docs/adr/ADR-2-b.md",
            "--apply",
            "--force",
            "--by",
            "me",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let a = std::fs::read_to_string(dir.join("docs/adr/ADR-1-a.md")).unwrap();
    let b = std::fs::read_to_string(dir.join("docs/adr/ADR-2-b.md")).unwrap();
    assert!(a.contains("Embodiment: Not started"), "{a}");
    assert!(b.contains("Embodiment: Implemented"), "{b}");
}

#[test]
fn ids_outside_the_given_paths_are_reported_as_matching_no_repair() {
    let dir = fixture("ids");
    let v = json(&run_urzua(
        &dir,
        &[
            "fix",
            "docs/adr/ADR-1-a.md",
            "--apply",
            "--ids",
            "docs/adr/ADR-2-b.md",
            "--by",
            "me",
        ],
    ));
    assert_eq!(v["status"], "partial-failure", "{v}");
    let b = std::fs::read_to_string(dir.join("docs/adr/ADR-2-b.md")).unwrap();
    assert!(b.contains("Not started"), "{b}");
}
