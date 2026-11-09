//! ADR-89, amendment of 2026-11-09 (decider table): `authority.decider_table`
//! says which revision's table `diff.decider-required` reads. A pull request
//! that adds its author to the decider group, or deletes the gates, in the
//! change that moves a record to an accepting status is reported under `base`
//! and `both` and judged as before under `head` and when undeclared.
//! Real git-repo fixtures and the real binary; all output is JSON.

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const TYPES: &str = "schema_version: 2\nrules:\n  diff.decider-required: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const GATES: &str = "  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";
const CONFIG: &str = ".urzua/config.yaml";

/// The configuration: the types, then `authority` with `extra` lines (the
/// mode, the evidence), the groups naming `logins`, and the gates.
fn config(extra: &str, logins: &str, gates: &str) -> String {
    format!(
        "{TYPES}authority:\n{extra}  deciders:\n    maintainers: {{logins: [{logins}]}}\n{gates}"
    )
}

/// A repository whose HEAD commits `base_config` (when given) and a proposed
/// record. The working tree is then the pull request's head.
fn fixture(name: &str, base_config: Option<&str>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-decider-table-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    git(&dir, &["add", PATH_1]);
    if let Some(base) = base_config {
        std::fs::write(dir.join(CONFIG), base).unwrap();
        git(&dir, &["add", CONFIG]);
    }
    git(&dir, &["commit", "-q", "-m", "base", "--allow-empty"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn actors(dir: &Path, author: &str, approvers: &[&str]) -> String {
    let path = dir.join("actors.json");
    let body = serde_json::json!({"schema": 1, "pr_author": author, "approvers": approvers});
    std::fs::write(&path, body.to_string()).unwrap();
    path.to_string_lossy().to_string()
}

/// The pull request: `head_config` written over the configuration and the
/// record moved to `Accepted`, checked against `HEAD` as the base.
fn check_pr(dir: &Path, head_config: &str, who: &str, approvers: &[&str]) -> serde_json::Value {
    std::fs::write(dir.join(CONFIG), head_config).unwrap();
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let file = actors(dir, who, approvers);
    json(&run(dir, &["check", "--base", "HEAD", "--actors", &file]))
}

fn findings(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.decider-required")
        .collect()
}

fn files(v: &serde_json::Value) -> Vec<String> {
    let mut out: Vec<String> = findings(v)
        .iter()
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    out.sort();
    out
}

const BASE: &str = "  decider_table: base\n";

#[test]
fn the_repro_is_reported_under_base_and_both() {
    for mode in ["base", "both"] {
        let base = config(&format!("  decider_table: {mode}\n"), "Alice", GATES);
        let head = config(
            &format!("  decider_table: {mode}\n"),
            "Alice, mallory",
            GATES,
        );
        let dir = fixture(&format!("repro-{mode}"), Some(&base));
        let v = check_pr(&dir, &head, "mallory", &[]);
        assert_eq!(files(&v), [CONFIG, PATH_1], "{mode}: {v}");
        let message = findings(&v)[0]["message"].as_str().unwrap();
        assert!(message.contains("the base revision's table"), "{message}");
    }
}

#[test]
fn the_repro_is_unchanged_under_head_and_when_undeclared() {
    for mode in ["  decider_table: head\n", ""] {
        let base = config(mode, "Alice", GATES);
        let head = config(mode, "Alice, mallory", GATES);
        let dir = fixture("repro-head", Some(&base));
        let v = check_pr(&dir, &head, "mallory", &[]);
        assert!(findings(&v).is_empty(), "{mode:?}: {v}");
    }
}

#[test]
fn undeclared_and_head_judge_the_head_table_with_the_old_message() {
    // A genuine non-decider is reported by the head table, with no table named.
    for mode in ["", "  decider_table: head\n"] {
        let cfg = config(mode, "Alice", GATES);
        let dir = fixture("old-message", Some(&cfg));
        let v = check_pr(&dir, &cfg, "mallory", &[]);
        let message = findings(&v)[0]["message"].as_str().unwrap().to_string();
        assert!(!message.contains("table"), "{message}");
        assert!(
            message.contains("(gate: type adr to Accepted, Rejected)"),
            "{message}"
        );
    }
}

#[test]
fn declaring_base_in_the_pull_request_itself_applies_at_once() {
    // The base declares nothing; the head adds the key and the author.
    let base = config("", "Alice", GATES);
    let head = config(BASE, "Alice, mallory", GATES);
    let dir = fixture("declared-in-pr", Some(&base));
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
}

#[test]
fn the_pull_request_cannot_loosen_the_mode_by_editing_the_key() {
    let base = config(BASE, "Alice", GATES);
    for head_mode in ["  decider_table: head\n", ""] {
        let head = config(head_mode, "Alice, mallory", GATES);
        let dir = fixture("loosen", Some(&base));
        let v = check_pr(&dir, &head, "mallory", &[]);
        assert_eq!(files(&v), [CONFIG, PATH_1], "{head_mode:?}: {v}");
    }
}

#[test]
fn adding_a_maintainer_under_base_needs_an_existing_deciders_approval() {
    let base = config(
        &format!("{BASE}  evidence: [pr-author, pr-approver]\n"),
        "Alice",
        GATES,
    );
    let head = config(
        &format!("{BASE}  evidence: [pr-author, pr-approver]\n"),
        "Alice, mallory",
        GATES,
    );
    let dir = fixture("approval", Some(&base));
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    let v = check_pr(&dir, &head, "mallory", &["mallory"]);
    assert_eq!(
        files(&v),
        [CONFIG, PATH_1],
        "an own approval is not an existing decider's: {v}"
    );
    let v = check_pr(&dir, &head, "mallory", &["Alice"]);
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn deleting_the_gates_in_the_pull_request_under_base_still_gates() {
    let base = config(BASE, "Alice", GATES);
    // The head keeps the group and the key and drops every gate.
    let head = config(BASE, "Alice, mallory", "");
    let dir = fixture("delete-gates", Some(&base));
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    // Under head the same pull request passes: the head has no gate.
    let base_head = config("", "Alice", GATES);
    let head_head = config("", "Alice, mallory", "");
    let dir = fixture("delete-gates-head", Some(&base_head));
    let v = check_pr(&dir, &head_head, "mallory", &[]);
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn a_decider_of_the_base_table_passes_under_base() {
    let base = config(BASE, "Alice", GATES);
    let dir = fixture("decider-passes", Some(&base));
    let v = check_pr(&dir, &base, "alice", &[]);
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn both_needs_a_decider_under_each_table() {
    let mode = "  decider_table: both\n";
    let base = config(mode, "Alice", GATES);
    // Bob is a decider only in the head's table.
    let head = config(mode, "Bob", GATES);
    let dir = fixture("both", Some(&base));
    let v = check_pr(&dir, &head, "bob", &[]);
    assert_eq!(findings(&v).len(), 2, "{v}");
    assert!(findings(&v).iter().all(|f| f["message"]
        .as_str()
        .unwrap()
        .contains("the base revision's table")));
    let v = check_pr(&dir, &base, "alice", &[]);
    assert!(
        findings(&v).is_empty(),
        "alice decides under both tables: {v}"
    );
}

#[test]
fn a_base_with_no_config_holds_no_deciders_and_no_gates() {
    let head = config(BASE, "Alice", GATES);
    let dir = fixture("no-base-config", None);
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert!(findings(&v).is_empty(), "{v}");
    let status = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "diff.decider-required")
        .unwrap()["status"]
        .clone();
    assert_eq!(status, "ran", "{v}");
}

#[test]
fn a_base_with_a_config_but_no_authority_holds_no_deciders_and_no_gates() {
    let head = config(BASE, "Alice", GATES);
    let dir = fixture("no-base-authority", Some(TYPES));
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn a_base_whose_authority_does_not_load_falls_back_to_the_head_table_and_says_so() {
    let head = config(BASE, "Alice", GATES);
    let broken = format!("{TYPES}authority:\n  colour: blue\n");
    let dir = fixture("broken-base", Some(&broken));
    let v = check_pr(&dir, &head, "mallory", &[]);
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    let notices = v["notices"].as_array().unwrap();
    assert!(
        notices
            .iter()
            .any(|n| n["subject"] == "diff.decider-required"
                && n["message"]
                    .as_str()
                    .unwrap()
                    .contains("head table is used")),
        "{v}"
    );
}

#[test]
fn a_run_with_no_base_says_the_head_table_is_used_under_base_and_both() {
    for mode in ["base", "both"] {
        let cfg = config(&format!("  decider_table: {mode}\n"), "Alice", GATES);
        let dir = fixture(&format!("no-base-{mode}"), Some(&cfg));
        std::fs::write(dir.join(CONFIG), &cfg).unwrap();
        let v = json(&run(&dir, &["check"]));
        let notices = v["notices"].as_array().unwrap();
        assert!(
            notices
                .iter()
                .any(|n| n["subject"] == "diff.decider-required"
                    && n["message"].as_str().unwrap().contains("no --base")
                    && n["message"].as_str().unwrap().contains(mode)),
            "{mode}: {v}"
        );
    }
    for mode in ["", "  decider_table: head\n"] {
        let cfg = config(mode, "Alice", GATES);
        let dir = fixture("no-base-quiet", Some(&cfg));
        let v = json(&run(&dir, &["check"]));
        let notices = v["notices"].as_array().cloned().unwrap_or_default();
        assert!(
            !notices.iter().any(|n| n["message"]
                .as_str()
                .unwrap_or("")
                .contains("decider_table")),
            "{mode:?}: {v}"
        );
    }
}

#[test]
fn an_unknown_decider_table_value_is_a_load_error() {
    let cfg = config("  decider_table: neither\n", "Alice", GATES);
    let dir = fixture("bad-value", Some(&cfg));
    let out = run(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(json(&out)["error"]
        .as_str()
        .unwrap()
        .contains("decider_table"));
}

#[test]
fn doctor_warns_when_gates_are_declared_without_a_decider_table() {
    let check = |cfg: &str, name: &str| {
        let dir = fixture(name, Some(cfg));
        let v = json(&run(&dir, &["doctor"]));
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["check"] == "authority-decider-table")
            .cloned()
    };
    let warned = check(&config("", "Alice", GATES), "doctor-warn").expect("a check");
    assert_eq!(warned["status"], "warn");
    assert!(warned["message"]
        .as_str()
        .unwrap()
        .contains("decider_table: base"));
    let ok = check(&config(BASE, "Alice", GATES), "doctor-ok").expect("a check");
    assert_eq!(ok["status"], "ok");
}

#[test]
fn init_proposes_decider_table_base_and_the_config_loads() {
    let dir = common::TempDir::new("urzua-decider-table-init");
    git(&dir, &["init", "-q"]);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-a.md"),
        "---\nStatus: Accepted\n---\n# A\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let out = run(&dir, &["init"]);
    assert!(out.status.success(), "{out:?}");
    let written = std::fs::read_to_string(dir.join(CONFIG)).unwrap();
    assert!(
        written.contains("authority:\n  decider_table: base"),
        "{written}"
    );
    let doctor = json(&run(&dir, &["doctor"]));
    assert_eq!(doctor["checks"][1]["status"], "ok", "{doctor}");
}
