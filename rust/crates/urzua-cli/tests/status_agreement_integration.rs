//! MILE-144: a written `Status` is held to the derived `blocked` value
//! (`RFC-48`, `ADR-64`). A record is blocked when any `Depends-on` target is
//! not in a status its own type declares `success`.

use std::path::Path;
use std::process::Command;

const CONFIG: &str = "schema_version: 2
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, acyclic: true, cross_type: true}
rules:
  sequence.status-agreement: error
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    statuses:
      values: [Planned, InProgress, Blocked, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      implementable: [Planned, InProgress, Done]
      blocked: [Blocked]
  bug:
    dir: docs/bugs
    prefix: BUG
    required_fields: [Status]
    known_fields: [Depends-on]
    pointer_fields: [Depends-on]
    statuses:
      values: [Open, Fixed]
      success: [Fixed]
";

fn repo(name: &str, config: &str, records: &[(&str, &str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-agree-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/milestones")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (path, status, extra) in records {
        std::fs::write(
            dir.join(path),
            format!("---\nStatus: {status}\n{extra}\n---\n# x\n"),
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

fn agreement(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "sequence.status-agreement")
        .collect()
}

#[test]
fn a_written_blocked_with_nothing_blocking_is_an_error() {
    let dir = repo(
        "stale",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "Done", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "Blocked",
                "Depends-on: MILE-1",
            ),
            ("docs/milestones/MILE-3-c.md", "Blocked", ""),
        ],
    );
    let (code, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 2, "{report}");
    assert_eq!(code, 1, "{report}");
    assert!(found.iter().all(|f| f["severity"] == "error"));
    let msg = found[0]["message"].as_str().unwrap();
    assert!(msg.contains("nothing blocks MILE-"), "{msg}");
    assert_eq!(found[0]["suggested_action"]["kind"], "edit-field");
    assert_eq!(found[0]["suggested_action"]["op"], "set");
    assert_eq!(found[0]["suggested_action"]["target"]["field"], "Status");
    assert!(found[0]["suggested_action"].get("value").is_none());
}

#[test]
fn a_blocked_record_that_states_an_implementable_status_is_an_error() {
    let dir = repo(
        "derived",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "InProgress", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "Planned",
                "Depends-on: MILE-1",
            ),
            ("docs/milestones/MILE-3-c.md", "Done", "Depends-on: MILE-1"),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 2, "{report}");
    let first = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("MILE-2"))
        .unwrap();
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("MILE-1 (InProgress)"));
    // One declared blocked status is the computed answer.
    assert_eq!(first["suggested_action"]["value"], "Blocked");
}

#[test]
fn blocked_and_done_agree_with_the_derived_value() {
    let dir = repo(
        "agree",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "InProgress", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "Blocked",
                "Depends-on: MILE-1",
            ),
            ("docs/milestones/MILE-3-c.md", "Done", ""),
            (
                "docs/milestones/MILE-4-d.md",
                "Planned",
                "Depends-on: MILE-3",
            ),
            // Not examined as blocked-but-implementable: a failure status is not implementable.
            (
                "docs/milestones/MILE-5-e.md",
                "WontDo",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (code, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "sequence.status-agreement")
        .expect("the rule reports its execution");
    // The five milestones that declare `statuses.blocked` are all examined.
    assert_eq!(rule["population"]["examined"], 5, "{rule}");
}

#[test]
fn a_dependency_in_a_failure_status_still_blocks() {
    let dir = repo(
        "failure",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "WontDo", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "Planned",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("MILE-1 (WontDo)"));
}

#[test]
fn the_dependency_is_judged_by_its_own_types_success_list() {
    // A Fixed bug is finished; an Open one is not. The bug type declares no
    // `blocked`, so it is a target here and is not itself examined.
    let dir = repo(
        "cross",
        CONFIG,
        &[
            ("docs/bugs/BUG-1-a.md", "Fixed", ""),
            ("docs/bugs/BUG-2-b.md", "Open", ""),
            (
                "docs/milestones/MILE-1-a.md",
                "Planned",
                "Depends-on: BUG-1",
            ),
            (
                "docs/milestones/MILE-2-b.md",
                "Planned",
                "Depends-on: BUG-2",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"].as_str().unwrap().contains("MILE-2"));
}

#[test]
fn a_written_blocks_counts_as_the_dependency_it_states() {
    // MILE-1 writes `Blocks: MILE-2`, so MILE-2 depends on MILE-1.
    let dir = repo(
        "inverse",
        CONFIG,
        &[
            (
                "docs/milestones/MILE-1-a.md",
                "InProgress",
                "Blocks: MILE-2",
            ),
            ("docs/milestones/MILE-2-b.md", "Planned", ""),
        ],
    );
    let (_, report) = check(&dir);
    assert_eq!(agreement(&report).len(), 1, "{report}");
}

#[test]
fn enabling_the_rule_with_no_type_declaring_blocked_fails_to_load() {
    let config = CONFIG.replace("      blocked: [Blocked]\n", "");
    let dir = repo(
        "undeclared",
        &config,
        &[("docs/milestones/MILE-1-a.md", "Planned", "")],
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
    assert!(text.contains("statuses.blocked"), "{text}");
}

#[test]
fn a_repository_that_does_not_enable_the_rule_is_unchanged() {
    let config = CONFIG.replace("  sequence.status-agreement: error\n", "");
    let config = config.replace("rules:\n", "rules: {}\n");
    let dir = repo(
        "off",
        &config,
        &[("docs/milestones/MILE-1-a.md", "Blocked", "")],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
}
