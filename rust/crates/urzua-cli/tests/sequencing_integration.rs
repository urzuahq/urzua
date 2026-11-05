//! MILE-27: the declared sequencing relation (`RFC-48`). A pair with
//! `acyclic: true` is checked for loops and for values that name no record.

mod common;

use std::path::Path;
use std::process::Command;

const CONFIG: &str = "schema_version: 2
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, acyclic: true}
rules:
  sequence.cycle: error
  sequence.unresolved-blocker: error
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
";

fn repo(name: &str, config: &str, milestones: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-seq-{name}"));
    std::fs::create_dir_all(dir.join("docs/milestones")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (file, header) in milestones {
        std::fs::write(
            dir.join("docs/milestones").join(file),
            format!("---\nStatus: Planned\n{header}\n---\n# x\n"),
        )
        .unwrap();
    }
    for args in [&["init", "-q"][..], &["add", "-A"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let json =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), json)
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

#[test]
fn a_planted_cycle_fails_check_and_names_its_path() {
    let dir = repo(
        "cycle",
        CONFIG,
        &[
            ("MILE-1-a.md", "Depends-on: MILE-2"),
            ("MILE-2-b.md", "Depends-on: MILE-3"),
            ("MILE-3-c.md", "Depends-on: MILE-1"),
            ("MILE-4-d.md", "Depends-on: MILE-1"),
        ],
    );
    let (code, report) = check(&dir);
    let found = findings(&report, "sequence.cycle");
    assert_eq!(
        found.len(),
        3,
        "MILE-4 depends on the cycle but is not on it: {report}"
    );
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("MILE-1 -> MILE-2 -> MILE-3 -> MILE-1"),
        "{found:?}"
    );
    assert_ne!(code, 0, "a cycle is an error: {report}");
}

#[test]
fn a_dangling_depends_on_target_fails_check() {
    let dir = repo(
        "dangling",
        CONFIG,
        &[
            ("MILE-1-a.md", "Depends-on: MILE-2, MILE-90"),
            ("MILE-2-b.md", "Blocks: MILE-91"),
        ],
    );
    let (code, report) = check(&dir);
    let found = findings(&report, "sequence.unresolved-blocker");
    assert_eq!(found.len(), 2, "{report}");
    assert!(found
        .iter()
        .any(|f| f["message"].as_str().unwrap().contains("MILE-90")));
    assert!(found
        .iter()
        .any(|f| f["message"].as_str().unwrap().contains("MILE-91")));
    assert!(findings(&report, "sequence.cycle").is_empty());
    assert_ne!(code, 0, "{report}");
}

#[test]
fn an_ordered_corpus_passes_and_the_rules_state_what_they_examined() {
    let dir = repo(
        "clean",
        CONFIG,
        &[
            ("MILE-1-a.md", "Depends-on: MILE-2"),
            ("MILE-2-b.md", "Title: leaf"),
        ],
    );
    let (code, report) = check(&dir);
    assert!(findings(&report, "sequence.cycle").is_empty(), "{report}");
    assert!(
        findings(&report, "sequence.unresolved-blocker").is_empty(),
        "{report}"
    );
    let ran = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "sequence.cycle")
        .expect("sequence.cycle executed");
    assert_eq!(ran["population"]["examined"], 1, "{ran}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_sequence_rule_with_no_acyclic_pair_declared_fails_to_load() {
    let config = CONFIG.replace(", acyclic: true", "");
    let dir = repo(
        "no-acyclic",
        &config,
        &[("MILE-1-a.md", "Depends-on: MILE-2")],
    );
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("acyclic"), "{text}");
}
