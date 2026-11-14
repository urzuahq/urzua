//! `embodiment.pin-stale` (external review, item 1.2; maintainer decision,
//! 2026-11-14: on by default). A confirmed `@sha256` pin mismatch on any
//! record's evidence is a `check` warning, bugs and milestones included, not
//! only records that state an `Embodiment`. Real git fixtures and the real
//! binary; the configuration declares no rule at all, so everything below
//! comes from the rule being on by default. All output is JSON.

mod common;

use common::{commit_all, git};
use std::path::Path;

const RULE: &str = "embodiment.pin-stale";
const V1: &str = "fn a() {}\n";
const V2: &str = "fn a() { let _ = 1; }\n";
const BUG: &str = "docs/bugs/BUG-1-x.md";
const MILE: &str = "docs/milestones/MILE-1-x.md";

const CONFIG: &str = "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    prefix: BUG\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Realized-by]\n  milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Realized-by]\n  waiver:\n    dir: docs/waiver\n    prefix: WAIVER\n    role: suppression\n    header_shape: yaml-frontmatter\n    required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires]\n";

fn sha256(content: &str) -> String {
    use std::io::Write;
    let mut child = std::process::Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn record(status: &str, locator: &str) -> String {
    format!("---\nStatus: {status}\nRealized-by: {locator}\n---\n# 1: T\n")
}

/// A bug and a milestone each pinning `src/a.rs` at the hash of `V1`.
fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-pin-stale-{name}"));
    for d in [
        "docs/bugs",
        "docs/milestones",
        "docs/waiver",
        "src",
        ".urzua",
    ] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    let pin = format!("code:src/a.rs@sha256:{}", sha256(V1));
    std::fs::write(dir.join(BUG), record("Fixed", &pin)).unwrap();
    std::fs::write(dir.join(MILE), record("Done", &pin)).unwrap();
    commit_all(&dir, 0);
    dir
}

fn check(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("json: {e}: {out:?}"))
}

fn stale(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .collect()
}

#[test]
fn a_fresh_pin_on_a_bug_and_a_milestone_is_not_reported_and_the_rule_ran() {
    let dir = fixture("fresh");
    let report = check(&dir, &[]);
    assert!(stale(&report).is_empty(), "{report}");
    let exec = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == RULE)
        .unwrap_or_else(|| panic!("{RULE} missing: {report}"));
    assert_eq!(exec["status"], "ran", "{exec}");
    assert_eq!(exec["population"]["examined"], 2, "{exec}");
}

#[test]
fn a_stale_pin_on_a_bug_and_on_a_milestone_is_a_warning_with_the_rebaseline_action() {
    let dir = fixture("stale");
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, 0);
    let report = check(&dir, &[]);
    let found = stale(&report);
    assert_eq!(found.len(), 2, "{report}");
    for (finding, path) in found.iter().zip([BUG, MILE]) {
        assert_eq!(finding["file"], path, "{finding}");
        assert_eq!(finding["severity"], "warning", "{finding}");
        let message = finding["message"].as_str().unwrap();
        assert!(
            message.contains("src/a.rs") && message.contains(&sha256(V1)[..12]),
            "{message}"
        );
        assert_eq!(
            finding["suggested_action"]["kind"], "run-command",
            "{finding}"
        );
        assert_eq!(
            finding["suggested_action"]["target"]["argv"],
            serde_json::json!(["urzua", "fix", "--rebaseline", "--apply", "--ids", path]),
            "{finding}"
        );
    }
    // A warning does not block under the default `fail_on`.
    assert_eq!(report["blocking"], false, "{report}");
}

#[test]
fn a_pin_that_cannot_be_read_is_not_a_confirmed_mismatch() {
    let dir = fixture("missing");
    git(&dir, &["rm", "-q", "src/a.rs"]);
    commit_all(&dir, 0);
    assert!(stale(&check(&dir, &[])).is_empty());
}

#[test]
fn a_waived_stale_pin_stays_waived() {
    let dir = fixture("waived");
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-pin.md"),
        format!("---\nRule: {RULE}\nScope: {BUG}\nReason: Reviewed; the claim holds for the new code.\nExpires: 2099-01-01\n---\n# 1: Waiver\n"),
    )
    .unwrap();
    commit_all(&dir, 0);
    let report = check(&dir, &[]);
    let found = stale(&report);
    assert_eq!(found.len(), 2, "{report}");
    let waived: Vec<_> = found.iter().filter(|f| f["waived"].is_string()).collect();
    assert_eq!(waived.len(), 1, "only the bug is waived: {report}");
    assert_eq!(waived[0]["file"], BUG);
}

#[test]
fn a_repository_may_turn_it_off_and_a_declared_level_stands() {
    let dir = fixture("levels");
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    let off = format!("{CONFIG}rules:\n  {RULE}: off\n");
    std::fs::write(dir.join(".urzua/config.yaml"), off).unwrap();
    commit_all(&dir, 0);
    assert!(stale(&check(&dir, &[])).is_empty());
    let error = format!("{CONFIG}rules:\n  {RULE}: error\n");
    std::fs::write(dir.join(".urzua/config.yaml"), error).unwrap();
    commit_all(&dir, 0);
    let report = check(&dir, &[]);
    assert_eq!(stale(&report).len(), 2, "{report}");
    assert!(
        stale(&report).iter().all(|f| f["severity"] == "error"),
        "{report}"
    );
    assert_eq!(report["blocking"], true);
}

#[test]
fn a_change_to_the_pinned_file_surfaces_the_stale_pin_on_a_base_run() {
    let dir = fixture("base");
    git(&dir, &["tag", "base"]);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, 0);
    let report = check(&dir, &["--base", "base"]);
    assert_eq!(stale(&report).len(), 2, "{report}");
}
