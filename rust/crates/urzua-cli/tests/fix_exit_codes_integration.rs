//! `urzua fix` never reads as success when it wrote nothing: an `--ids` entry
//! is matched by the record's repo-relative path whatever form it is written
//! in, and a run that fails exits nonzero and names the command that lists
//! what is pending. Real git repositories and the real binary.

mod common;

use std::path::Path;

const A: &str = "docs/adr/ADR-1-new.md";
const B: &str = "docs/adr/ADR-2-old.md";
const LOG: &str = "\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
const CONFIG: &str = "schema_version: 2
rules:
  header.required-fields: error
  relation.pair-reciprocity: error
record_types:
  adr:
    dir: \"docs/adr\"
    required_fields: [\"Status\"]
    known_fields: [\"Supersedes\", \"Superseded-by\"]
    statuses:
      values: [Proposed, Accepted, Superseded]
relation_pairs:
  - name: supersession
    forward: \"Supersedes\"
    inverse: \"Superseded-by\"
    reciprocity: required
    types: [adr]
";

fn record(n: u32, fields: &str) -> String {
    format!("---\nStatus: Proposed\n{fields}---\n# ADR-{n}: T\n\nBody.\n{LOG}")
}

/// A repository whose first ADR supersedes the second without the inverse.
fn project(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fix-exit-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "t@example.com"]);
    common::git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join(A), record(1, "Supersedes: ADR-2\n")).unwrap();
    std::fs::write(dir.join(B), record(2, "")).unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn apply(dir: &Path, id: &str) -> (i32, serde_json::Value) {
    run(dir, &["fix", "--apply", "--ids", id, "--actor", "agent"])
}

fn written(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(B))
        .unwrap()
        .contains("Superseded-by: ADR-1")
}

#[test]
fn a_dot_slash_ids_entry_writes_the_repair() {
    let dir = project("dot-slash", CONFIG);
    let (code, v) = apply(&dir, &format!("./{B}"));
    assert_eq!((code, v["status"].as_str().unwrap()), (0, "applied"), "{v}");
    assert!(written(&dir), "the inverse is written: {v}");
}

#[test]
fn an_absolute_ids_entry_inside_the_repository_writes_the_repair() {
    let dir = project("absolute", CONFIG);
    let abs = dir.canonicalize().unwrap().join(B);
    let (code, v) = apply(&dir, abs.to_str().unwrap());
    assert_eq!((code, v["status"].as_str().unwrap()), (0, "applied"), "{v}");
    assert!(written(&dir), "{v}");
}

#[test]
fn a_dot_slash_entry_reads_a_dry_run_selection_too() {
    let dir = project("dry-run", CONFIG);
    let (code, v) = run(&dir, &["fix", "--ids", &format!("./{B}")]);
    assert_eq!(
        (code, v["status"].as_str().unwrap()),
        (0, "repairs-available"),
        "{v}"
    );
    assert_eq!(v["repairs"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn a_dot_slash_entry_selects_the_record_for_fix_pin() {
    let dir = project("pin", CONFIG);
    let (_, v) = run(&dir, &["fix", "--pin", "--ids", &format!("./{B}")]);
    let errors = v["failed"].to_string();
    assert!(
        !errors.contains("names no record"),
        "the record is found, not unknown: {v}"
    );
}

#[test]
fn an_absolute_entry_outside_the_repository_fails_naming_the_entry() {
    let dir = project("outside", CONFIG);
    let (code, v) = apply(&dir, "/elsewhere/docs/adr/ADR-2-old.md");
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["status"], "partial-failure", "{v}");
    assert!(
        v["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("/elsewhere/docs/adr/ADR-2-old.md"),
        "{v}"
    );
    assert!(!written(&dir), "{v}");
}

#[test]
fn a_failed_apply_exits_1_and_names_the_dry_run_that_lists_what_is_pending() {
    let dir = project("unmatched", CONFIG);
    let (code, v) = apply(&dir, "docs/adr/ADR-9-nothing.md");
    assert_eq!(
        (code, v["status"].as_str().unwrap()),
        (1, "partial-failure"),
        "{v}"
    );
    assert_eq!(v["next_action"], "urzua fix", "{v}");
    assert!(
        v["next_action_reason"]
            .as_str()
            .unwrap()
            .contains("pending"),
        "{v}"
    );
}

#[test]
fn a_config_that_fails_to_load_exits_2_in_every_fix_mode() {
    let bad = format!("{CONFIG}bogus_key: 1\n");
    let dir = project("bad-config", &bad);
    let (check_code, _) = run(&dir, &["check"]);
    assert_eq!(check_code, 2);
    for args in [
        vec!["fix"],
        vec!["fix", "--apply", "--ids", B, "--actor", "agent"],
        vec!["fix", "--rebaseline"],
        vec!["fix", "--conflicts"],
        vec!["fix", "--pin"],
    ] {
        let (code, v) = run(&dir, &args);
        assert_eq!(
            (code, v["status"].as_str().unwrap()),
            (2, "not-run"),
            "{args:?}: {v}"
        );
    }
}
