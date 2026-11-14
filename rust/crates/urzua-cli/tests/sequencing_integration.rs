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
    let out = common::urzua()
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
    let out = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("acyclic"), "{text}");
}

// MILE-192 (ADR-82): with `roadmap.pairs` declared, `sequence.cycle` reads one
// graph over those pairs, so a loop that uses an edge of each is found.

const TWO_PAIRS: &str = "schema_version: 2
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, acyclic: true}
  - {name: gating, forward: Gated-by, inverse: Gates, reciprocity: none, acyclic: true}
rules:
  sequence.cycle: error
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status]
    known_fields: [Depends-on, Blocks, Gated-by, Gates]
    pointer_fields: [Depends-on, Blocks, Gated-by, Gates]
";

fn with_roadmap(pairs: &str) -> String {
    format!("{TWO_PAIRS}roadmap:\n  pairs: {pairs}\n")
}

/// MILE-1 depends on MILE-2 (one pair) and MILE-2 is gated by MILE-1 (the
/// other): neither pair has a loop of its own. MILE-3 only waits on the loop.
const CROSS_PAIR_LOOP: &[(&str, &str)] = &[
    ("MILE-1-a.md", "Depends-on: MILE-2"),
    ("MILE-2-b.md", "Gated-by: MILE-1"),
    ("MILE-3-c.md", "Depends-on: MILE-1"),
];

#[test]
fn a_loop_across_two_declared_pairs_is_reported_on_every_member() {
    let dir = repo(
        "cross",
        &with_roadmap("[sequencing, gating]"),
        CROSS_PAIR_LOOP,
    );
    let (code, report) = check(&dir);
    let found = findings(&report, "sequence.cycle");
    let files: Vec<&str> = found.iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert_eq!(found.len(), 2, "MILE-3 is not on the loop: {report}");
    assert!(
        files.iter().any(|f| f.ends_with("MILE-1-a.md")),
        "{files:?}"
    );
    assert!(
        files.iter().any(|f| f.ends_with("MILE-2-b.md")),
        "{files:?}"
    );
    for f in &found {
        let message = f["message"].as_str().unwrap();
        assert!(message.contains("crosses"), "{message}");
        assert!(message.contains("'sequencing'"), "{message}");
        assert!(message.contains("'gating'"), "{message}");
        assert!(message.contains("roadmap.pairs"), "{message}");
    }
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("MILE-1 -> MILE-2 -> MILE-1"));
    assert_ne!(code, 0, "{report}");
}

#[test]
fn the_cross_pair_loop_is_not_reported_when_roadmap_pairs_is_absent() {
    let dir = repo("cross-absent", TWO_PAIRS, CROSS_PAIR_LOOP);
    let (code, report) = check(&dir);
    assert!(findings(&report, "sequence.cycle").is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_pair_left_out_of_roadmap_pairs_stays_out_of_the_merged_graph() {
    let dir = repo("cross-one", &with_roadmap("[sequencing]"), CROSS_PAIR_LOOP);
    let (code, report) = check(&dir);
    assert!(findings(&report, "sequence.cycle").is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_loop_inside_one_declared_pair_is_reported_once_per_record_as_before() {
    let dir = repo(
        "cross-single",
        &with_roadmap("[sequencing, gating]"),
        &[
            ("MILE-1-a.md", "Depends-on: MILE-2"),
            ("MILE-2-b.md", "Depends-on: MILE-1"),
        ],
    );
    let (_, report) = check(&dir);
    let found = findings(&report, "sequence.cycle");
    assert_eq!(found.len(), 2, "{report}");
    for f in &found {
        let message = f["message"].as_str().unwrap();
        assert!(
            message.contains("in acyclic pair 'sequencing'"),
            "a one-pair loop keeps its message: {message}"
        );
        assert!(!message.contains("crosses"), "{message}");
    }
}

#[test]
fn a_pair_outside_roadmap_pairs_is_still_checked_on_its_own() {
    let dir = repo(
        "cross-outside",
        &with_roadmap("[sequencing]"),
        &[
            ("MILE-1-a.md", "Gated-by: MILE-2"),
            ("MILE-2-b.md", "Gated-by: MILE-1"),
        ],
    );
    let (_, report) = check(&dir);
    let found = findings(&report, "sequence.cycle");
    assert_eq!(found.len(), 2, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("acyclic pair 'gating'"));
}

#[test]
fn a_roadmap_pair_that_is_not_acyclic_fails_check_to_load() {
    let config = with_roadmap("[sequencing, gating]").replace(
        "{name: gating, forward: Gated-by, inverse: Gates, reciprocity: none, acyclic: true}",
        "{name: gating, forward: Gated-by, inverse: Gates, reciprocity: none}",
    );
    let dir = repo("cross-nonacyclic", &config, CROSS_PAIR_LOOP);
    let out = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("gating") && text.contains("acyclic"),
        "{text}"
    );
}
