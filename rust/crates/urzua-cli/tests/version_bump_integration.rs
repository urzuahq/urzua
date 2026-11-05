//! BUG-51: a changed record that adds a revision-log row of a declared class
//! must change its declared version field. Judged against the merge base's
//! copy of the record, so it runs only on a `--base` run.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

const RULE: &str = "revision-log.version-bump-required";

fn config(decl: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  {RULE}: error\nrecord_types:\n  spec:\n    dir: \"docs/specs\"\n    prefix: \"SPEC\"\n    required_fields: [\"Status\", \"Version\"]\n    header_shape: \"yaml-frontmatter\"\n{decl}"
    )
}

const DECL: &str = "    revision_log: {marker: \"**Revision log**\", version_bump: {field: Version, classes: [substantive, structural]}}\n";

fn spec(version: &str, rows: &[&str]) -> String {
    let mut s = format!(
        "---\nStatus: Accepted\nVersion: '{version}'\n---\n# 1 - X\n\nBody.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n"
    );
    for r in rows {
        s.push_str(&format!("> | {r} |\n"));
    }
    s
}

const ROW1: &str = "2026-11-01 | Initial. | **substantive**";
const ROW2: &str = "2026-11-04 | Rewrote the intro. | **substantive**";
const ROW3: &str = "2026-11-04 | Renamed a field. | **structural**";
const ROW_OTHER: &str = "2026-11-04 | Tidied wording. | **editorial**";

fn fixture(name: &str, decl: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-vbump-{name}"));
    std::fs::create_dir_all(dir.join("docs/specs")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(decl)).unwrap();
    std::fs::write(dir.join("docs/specs/SPEC-1-x.md"), spec("0.1", &[ROW1])).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn hits(v: &serde_json::Value) -> usize {
    v["findings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .count()
}

fn write_at(dir: &Path, rel: &str, text: String) {
    std::fs::write(dir.join(rel), text).unwrap();
    git(dir, &["add", "-A"]);
}

fn write(dir: &Path, text: String) {
    std::fs::write(dir.join("docs/specs/SPEC-1-x.md"), text).unwrap();
    git(dir, &["add", "-A"]);
}

#[test]
fn a_substantive_row_without_a_version_bump_is_reported_observed_failing() {
    let dir = fixture("substantive", DECL);
    write(&dir, spec("0.1", &[ROW1, ROW2]));
    let v = run(&dir, &["--base", "base"]);
    assert_eq!(hits(&v), 1, "{v}");
    let f = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == RULE)
        .unwrap();
    // ADR-73: set the version field, with no value (the author picks it).
    assert_eq!(
        f["suggested_action"],
        serde_json::json!({
            "kind": "edit-field",
            "op": "set",
            "target": { "file": "docs/specs/SPEC-1-x.md", "field": "Version" }
        }),
        "{v}"
    );
}

#[test]
fn a_renamed_record_is_unexamined() {
    let dir = fixture("rename", DECL);
    git(
        &dir,
        &["mv", "docs/specs/SPEC-1-x.md", "docs/specs/SPEC-1-y.md"],
    );
    write_at(&dir, "docs/specs/SPEC-1-y.md", spec("0.1", &[ROW1, ROW2]));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0);
}

#[test]
fn a_structural_row_without_a_version_bump_is_reported() {
    let dir = fixture("structural", DECL);
    write(&dir, spec("0.1", &[ROW1, ROW3]));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 1);
}

#[test]
fn a_bumped_version_a_row_of_another_class_or_no_new_row_is_clean() {
    let dir = fixture("clean", DECL);
    write(&dir, spec("0.2", &[ROW1, ROW2]));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0, "bumped");
    write(&dir, spec("0.1", &[ROW1, ROW_OTHER]));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0, "other class");
    write(&dir, spec("0.1", &[ROW1]).replace("Body.", "Body, edited."));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0, "no new row");
}

#[test]
fn a_new_record_has_nothing_to_bump() {
    let dir = fixture("new", DECL);
    std::fs::write(dir.join("docs/specs/SPEC-2-y.md"), spec("0.1", &[ROW2])).unwrap();
    git(&dir, &["add", "-A"]);
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0);
}

#[test]
fn a_whole_repo_run_says_not_applicable_rather_than_clean() {
    let dir = fixture("whole", DECL);
    let v = run(&dir, &[]);
    let status = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == RULE)
        .map(|r| r["status"].clone())
        .unwrap();
    assert_eq!(status, "not-applicable", "{v}");
}

#[test]
fn a_type_that_does_not_declare_it_is_not_judged() {
    // Another type declares the bump so the config loads; `spec` does not.
    let cfg = config("").replace(
        "record_types:\n",
        "record_types:\n  note:\n    dir: \"docs/notes\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    revision_log: {version_bump: {field: Version, classes: [substantive]}}\n",
    );
    let dir = fixture("undeclared", "");
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    write(&dir, spec("0.1", &[ROW1, ROW2]));
    assert_eq!(hits(&run(&dir, &["--base", "base"])), 0);
}

#[test]
fn enabling_the_rule_with_no_type_declaring_it_is_a_config_error() {
    let dir = fixture("none", "");
    let v = run(&dir, &["--base", "base"]);
    assert!(v.to_string().contains("version_bump"), "{v}");
    assert_eq!(v["status"], "not-run", "{v}");
}

#[test]
fn a_malformed_declaration_is_a_config_error() {
    for bad in [
        "revision_log: {version_bump: {field: \"\", classes: [substantive]}}",
        "revision_log: {version_bump: {field: Version, classes: []}}",
        "revision_log: {version_bump: {field: Version, classes: [\" \"]}}",
    ] {
        let dir = fixture("bad", &format!("    {bad}\n"));
        let v = run(&dir, &["--base", "base"]);
        assert_eq!(v["status"], "not-run", "{bad}: {v}");
    }
}
