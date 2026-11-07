//! MILE-159: `revision_log.required` governs whether `fix --apply` needs a log section.
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
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn config(revision_log: &str) -> String {
    format!("schema_version: 2\nrules: {{embodiment.consistency: error}}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n{revision_log}")
}
const PATH: &str = "docs/adr/ADR-1-a.md";
const LOGLESS: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nBody.\n";

fn fixture(name: &str, revision_log: &str, record: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fix-log-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(revision_log)).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH), record).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn not_required_apply_writes_the_field_without_a_log_row() {
    let dir = fixture("optional", "    revision_log: {required: false}\n", LOGLESS);
    let out = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    let v = json(&out);
    assert_eq!(v["status"], "applied", "{v}");
    let after = std::fs::read_to_string(dir.join(PATH)).unwrap();
    assert!(after.contains("Embodiment: Implemented"), "{after}");
    assert_eq!(
        after.replace("Embodiment: Implemented", "Embodiment: Not started"),
        LOGLESS
    );
}

#[test]
fn required_by_default_a_missing_section_fails_naming_the_record() {
    let dir = fixture("default", "", LOGLESS);
    let out = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    let v = json(&out);
    assert_eq!(v["status"], "partial-failure", "{v}");
    assert_eq!(v["failed"][0]["record"], PATH, "{v}");
    assert!(
        v["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("Revision log"),
        "{v}"
    );
    assert!(std::fs::read_to_string(dir.join(PATH))
        .unwrap()
        .contains("Not started"));
}

#[test]
fn required_true_declared_with_a_heading_fails_without_the_section() {
    let dir = fixture(
        "explicit",
        "    revision_log: {heading: History, required: true}\n",
        LOGLESS,
    );
    let v = json(&run_urzua(
        &dir,
        &["fix", "--apply", "--force", "--by", "me"],
    ));
    assert_eq!(v["status"], "partial-failure", "{v}");
    assert!(
        v["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("History"),
        "{v}"
    );
}

#[test]
fn a_non_boolean_required_is_a_config_error() {
    let dir = fixture("bad", "    revision_log: {required: maybe}\n", LOGLESS);
    let out = run_urzua(&dir, &["fix"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}
