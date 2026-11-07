//! M6: a `--rules` selection reads only what its rules read, and a rule that
//! is selected still reads everything it needs.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-rules-reads-{name}"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  header.required-fields: warn\n  embodiment.back-pointer-resolution: {level: error, back_pointer_paths: [\"src\"]}\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Draft\n---\n# A\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/a.rs"), "// Implements: ADR-99\nfn a() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn a_selected_back_pointer_rule_still_reads_the_source_files() {
    let dir = repo("selected");
    let report = check(&dir, &["--rules", "embodiment.back-pointer-resolution"]);
    let findings = report["findings"].to_string();
    assert!(findings.contains("ADR-99"), "{report}");
}

#[test]
fn a_selection_that_leaves_the_back_pointer_rules_out_does_not_run_them() {
    let dir = repo("unselected");
    let report = check(&dir, &["--rules", "header.required-fields"]);
    let executed: Vec<&str> = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["rule"].as_str().unwrap())
        .collect();
    assert_eq!(executed, ["header.required-fields"], "{report}");
    assert_eq!(report["status"], "ok", "{report}");
}
