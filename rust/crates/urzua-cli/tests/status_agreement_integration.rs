//! MILE-144: a written `Status` is held to the derived `blocked` value
//! (`RFC-48`, `ADR-83`, `ADR-64`). A record is blocked when any dependency over
//! the pairs `roadmap.pairs` declares is not in a status its own type declares
//! `success`. A blocked record that states a `progress` status is an error; a
//! type may also opt in to a written blocked status, checked the other way.

use std::path::Path;
use std::process::Command;

const CONFIG: &str = "schema_version: 2
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, acyclic: true, cross_type: true}
roadmap:
  pairs: [sequencing]
rules:
  sequence.status-agreement: error
  sequence.unresolved-blocker: error
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
      progress: [InProgress, Done]
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

/// A repository that follows RFC-48 as urzua does: `Blocked` is not a status,
/// and the milestone type declares only `progress`.
fn derived_only() -> String {
    CONFIG
        .replace(
            "Planned, InProgress, Blocked, Done, WontDo",
            "Planned, InProgress, Done, WontDo",
        )
        .replace("      blocked: [Blocked]\n", "")
}

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

fn examined_and_unreadable(report: &serde_json::Value) -> (u64, u64) {
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "sequence.status-agreement")
        .expect("the rule reports its execution");
    (
        rule["population"]["examined"].as_u64().unwrap(),
        rule["population"]["unreadable"].as_u64().unwrap(),
    )
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
fn a_blocked_record_that_claims_progress_is_an_error_and_planned_is_not() {
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
            (
                "docs/milestones/MILE-4-d.md",
                "InProgress",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    let files: Vec<&str> = found.iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert_eq!(found.len(), 2, "{report}");
    assert!(files.iter().any(|f| f.contains("MILE-3")), "{files:?}");
    assert!(files.iter().any(|f| f.contains("MILE-4")), "{files:?}");
    assert!(
        !files.iter().any(|f| f.contains("MILE-2")),
        "Planned is not progress: {files:?}"
    );
    let in_progress = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("MILE-4"))
        .unwrap();
    assert!(in_progress["message"]
        .as_str()
        .unwrap()
        .contains("MILE-1 (InProgress)"));
    // One declared blocked status, and no declared lifecycle: it is the answer.
    assert_eq!(in_progress["suggested_action"]["value"], "Blocked");
    // A finished record never moves back to a waiting state.
    let done = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("MILE-3"))
        .unwrap();
    assert!(done["suggested_action"].get("value").is_none(), "{done}");
}

#[test]
fn the_suggested_status_is_offered_only_when_the_lifecycle_allows_it() {
    let allowed = CONFIG.replace(
        "      blocked: [Blocked]\n",
        "      blocked: [Blocked]\n      initial: [Planned]\n      transitions:\n        Planned: [InProgress]\n        InProgress: [Blocked, Done]\n        Blocked: [InProgress]\n        Done: [Planned]\n",
    );
    let dir = repo(
        "lifecycle",
        &allowed,
        &[
            ("docs/milestones/MILE-1-a.md", "Planned", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["suggested_action"]["value"], "Blocked");
    // The same declaration without InProgress -> Blocked offers no value.
    let denied = allowed.replace("InProgress: [Blocked, Done]", "InProgress: [Done]");
    let dir = repo(
        "lifecycle-denied",
        &denied,
        &[
            ("docs/milestones/MILE-1-a.md", "Planned", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["suggested_action"].get("value").is_none(),
        "{report}"
    );
}

#[test]
fn a_repository_that_declares_no_written_blocked_status_still_reports_progress() {
    let dir = repo(
        "derived-only",
        &derived_only(),
        &[
            ("docs/milestones/MILE-1-a.md", "Planned", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
                "Depends-on: MILE-1",
            ),
            (
                "docs/milestones/MILE-3-c.md",
                "Planned",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"].as_str().unwrap().contains("MILE-2"));
    assert!(
        found[0]["suggested_action"].get("value").is_none(),
        "no blocked status to propose"
    );
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
                "InProgress",
                "Depends-on: MILE-3",
            ),
            // A failure status is not progress.
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
    assert_eq!(examined_and_unreadable(&report), (5, 0), "{report}");
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
                "InProgress",
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
    // A Fixed bug is finished; an Open one is not.
    let dir = repo(
        "cross",
        CONFIG,
        &[
            ("docs/bugs/BUG-1-a.md", "Fixed", ""),
            ("docs/bugs/BUG-2-b.md", "Open", ""),
            (
                "docs/milestones/MILE-1-a.md",
                "InProgress",
                "Depends-on: BUG-1",
            ),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
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
fn a_dependency_whose_type_declares_no_success_list_blocks() {
    let config = CONFIG.replace(
        "    statuses:\n      values: [Open, Fixed]\n      success: [Fixed]\n",
        "",
    );
    let dir = repo(
        "no-success",
        &config,
        &[
            ("docs/bugs/BUG-1-a.md", "Fixed", ""),
            (
                "docs/milestones/MILE-1-a.md",
                "InProgress",
                "Depends-on: BUG-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    let found = agreement(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("BUG-1 (Fixed)"));
}

#[test]
fn a_dependency_with_no_status_blocks() {
    let dir = repo(
        "no-status",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
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
        .contains("MILE-1 (no status)"));
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
            ("docs/milestones/MILE-2-b.md", "InProgress", ""),
        ],
    );
    let (_, report) = check(&dir);
    assert_eq!(agreement(&report).len(), 1, "{report}");
}

#[test]
fn an_acyclic_pair_the_roadmap_does_not_declare_blocks_nothing() {
    // A containment pair is checked for cycles but a child does not wait on
    // its parent finishing.
    let config = CONFIG
        .replace(
            "relation_pairs:\n",
            "relation_pairs:\n  - {name: containment, forward: Parent, inverse: Children, reciprocity: none, acyclic: true, cross_type: true}\n",
        )
        .replace("known_fields: [Depends-on, Blocks]", "known_fields: [Depends-on, Blocks, Parent, Children]")
        .replace("pointer_fields: [Depends-on, Blocks]", "pointer_fields: [Depends-on, Blocks, Parent, Children]");
    let dir = repo(
        "containment",
        &config,
        &[
            ("docs/milestones/MILE-1-a.md", "InProgress", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
                "Parent: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
}

#[test]
fn a_dangling_depends_on_with_a_written_blocked_status_is_not_called_unblocked() {
    let dir = repo(
        "dangling",
        CONFIG,
        &[(
            "docs/milestones/MILE-2-b.md",
            "Blocked",
            "Depends-on: MILE-999",
        )],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(examined_and_unreadable(&report), (0, 1), "{report}");
    // The reference is the one finding to fix.
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["rule"] == "sequence.unresolved-blocker"));
}

#[test]
fn a_dependency_on_an_unreadable_record_is_not_called_unblocked() {
    let dir = repo(
        "unreadable-target",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "[unclosed", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "Blocked",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(examined_and_unreadable(&report), (0, 2), "{report}");
}

#[test]
fn an_unreadable_record_that_may_write_blocks_does_not_make_a_record_unblocked() {
    // MILE-1's header cannot be read, so a `Blocks: MILE-2` it may write is
    // invisible; MILE-2 is not reported as blocked by nothing.
    let dir = repo(
        "unreadable-writer",
        CONFIG,
        &[
            ("docs/milestones/MILE-1-a.md", "[unclosed", "Blocks: MILE-2"),
            ("docs/milestones/MILE-2-b.md", "Blocked", ""),
        ],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(examined_and_unreadable(&report), (0, 2), "{report}");
}

// ADR-83: a value that cannot be computed is counted unreadable, not reported
// as unblocked. A type that declares only `progress` (no written `blocked`) is
// held to the same guard: a progress-status record with an unresolved or
// unreadable dependency is not examined and clean.
#[test]
fn a_progress_record_with_a_dangling_dependency_is_unreadable_when_no_blocked_status_is_declared() {
    for (name, status) in [
        ("derived-dangling-inprogress", "InProgress"),
        ("derived-dangling-done", "Done"),
    ] {
        let dir = repo(
            name,
            &derived_only(),
            &[(
                "docs/milestones/MILE-2-b.md",
                status,
                "Depends-on: MILE-999",
            )],
        );
        let (_, report) = check(&dir);
        assert!(agreement(&report).is_empty(), "{report}");
        assert_eq!(
            examined_and_unreadable(&report),
            (0, 1),
            "{status}: {report}"
        );
    }
}

#[test]
fn a_progress_record_depending_on_an_unreadable_record_is_unreadable_when_no_blocked_status_is_declared(
) {
    let dir = repo(
        "derived-unreadable-target",
        &derived_only(),
        &[
            ("docs/milestones/MILE-1-a.md", "[unclosed", ""),
            (
                "docs/milestones/MILE-2-b.md",
                "InProgress",
                "Depends-on: MILE-1",
            ),
        ],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(examined_and_unreadable(&report), (0, 2), "{report}");
}

#[test]
fn a_progress_record_is_unreadable_when_an_unreadable_record_may_write_blocks_and_no_blocked_status_is_declared(
) {
    let dir = repo(
        "derived-unreadable-writer",
        &derived_only(),
        &[
            ("docs/milestones/MILE-1-a.md", "[unclosed", "Blocks: MILE-2"),
            ("docs/milestones/MILE-2-b.md", "InProgress", ""),
        ],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
    assert_eq!(examined_and_unreadable(&report), (0, 2), "{report}");
}

#[test]
fn enabling_the_rule_with_no_type_declaring_blocked_or_progress_fails_to_load() {
    let config = derived_only().replace("      progress: [InProgress, Done]\n", "");
    assert_load_error("undeclared", &config, "statuses.progress");
}

#[test]
fn enabling_the_rule_with_no_roadmap_pairs_fails_to_load() {
    let config = CONFIG.replace("roadmap:\n  pairs: [sequencing]\n", "");
    assert_load_error("no-pairs", &config, "roadmap.pairs");
}

#[test]
fn a_roadmap_pair_must_be_declared_and_acyclic() {
    let config = CONFIG.replace("pairs: [sequencing]", "pairs: [nope]");
    assert_load_error("unknown-pair", &config, "not a relation_pairs entry");
    let config = CONFIG.replace("reciprocity: none, acyclic: true", "reciprocity: none");
    assert_load_error("not-acyclic", &config, "acyclic");
}

#[test]
fn a_blocked_status_that_is_also_a_progress_status_fails_to_load() {
    let config = CONFIG.replace(
        "progress: [InProgress, Done]",
        "progress: [InProgress, Blocked]",
    );
    assert_load_error("overlap", &config, "both `progress` and `blocked`");
}

fn assert_load_error(name: &str, config: &str, needle: &str) {
    let dir = repo(
        name,
        config,
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
    assert!(text.contains(needle), "{text}");
}

#[test]
fn a_repository_that_does_not_enable_the_rule_is_unchanged() {
    let config = CONFIG.replace("  sequence.status-agreement: error\n", "");
    let dir = repo(
        "off",
        &config,
        &[("docs/milestones/MILE-1-a.md", "Blocked", "")],
    );
    let (_, report) = check(&dir);
    assert!(agreement(&report).is_empty(), "{report}");
}
