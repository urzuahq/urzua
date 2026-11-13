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

/// A pull request cut before the base had an `authority` section declares one
/// whose group does not hold its author: the base names no decider and no
/// gate, so judging against it would let anything through (the bypass). The
/// head table is judged instead, and an info notice says why.
fn assert_stale_base_is_judged_against_the_head(name: &str, base: Option<&str>, want: &[&str]) {
    for mode in ["base", "both"] {
        let head = config(&format!("  decider_table: {mode}\n"), "Alice", GATES);
        let dir = fixture(&format!("{name}-{mode}"), base);
        let v = check_pr(&dir, &head, "mallory", &[]);
        assert_eq!(files(&v), want, "{mode}: {v}");
        let message = findings(&v)[0]["message"].as_str().unwrap();
        assert!(message.contains("the head's table"), "{message}");
        assert!(
            v["notices"].as_array().unwrap().iter().any(|n| {
                n["subject"] == "diff.decider-required"
                    && n["severity"] == "info"
                    && n["message"]
                        .as_str()
                        .unwrap()
                        .contains("the base revision has no config file or no authority section")
                    && n["message"]
                        .as_str()
                        .unwrap()
                        .contains("head table is judged")
            }),
            "{mode}: {v}"
        );
        // A decider of the head's group still passes.
        let v = check_pr(&dir, &head, "alice", &[]);
        assert!(findings(&v).is_empty(), "{mode}: {v}");
    }
}

#[test]
fn a_base_with_no_config_is_judged_against_the_head_table_and_says_so() {
    // The config file is new, not committed, so it is not in the diff.
    assert_stale_base_is_judged_against_the_head("no-base-config", None, &[PATH_1]);
}

#[test]
fn a_stale_base_with_a_config_but_no_authority_is_judged_against_the_head_table() {
    assert_stale_base_is_judged_against_the_head(
        "no-base-authority",
        Some(TYPES),
        &[CONFIG, PATH_1],
    );
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
        // A plain local check has nothing for the rule to judge: no notice.
        let plain = json(&run(&dir, &["check"]));
        assert!(
            !plain["notices"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .any(|n| n["message"].as_str().unwrap_or("").contains("no --base")),
            "{mode}: {plain}"
        );
        // `--actors` without `--base` names who made a change there is no base for.
        let file = actors(&dir, "mallory", &[]);
        let v = json(&run(&dir, &["check", "--actors", &file]));
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

/// The pull request's config is `head_config`, the base's is `base_config`;
/// returns the report and the exit code.
fn check_pr_exit(dir: &Path, head_config: &str, who: &str) -> (serde_json::Value, Option<i32>) {
    std::fs::write(dir.join(CONFIG), head_config).unwrap();
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let file = actors(dir, who, &[]);
    let out = run(dir, &["check", "--base", "HEAD", "--actors", &file]);
    (json(&out), out.status.code())
}

fn severities(v: &serde_json::Value) -> Vec<String> {
    findings(v)
        .iter()
        .map(|f| f["severity"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn turning_the_rule_off_in_the_pull_request_is_judged_under_the_base_setting() {
    for mode in ["base", "both"] {
        let extra = format!("  decider_table: {mode}\n");
        let base = config(&extra, "Alice", GATES);
        let off = config(&extra, "Alice", GATES)
            .replace("diff.decider-required: error", "diff.decider-required: off");
        let dir = fixture(&format!("rule-off-{mode}"), Some(&base));
        let (v, code) = check_pr_exit(&dir, &off, "mallory");
        let mut want = vec![CONFIG, PATH_1];
        if mode == "both" {
            want = vec![CONFIG, CONFIG, PATH_1, PATH_1];
        }
        assert_eq!(files(&v), want, "{mode}: {v}");
        assert!(severities(&v).iter().all(|s| s == "error"), "{mode}: {v}");
        assert_eq!(code, Some(1), "{mode}: {v}");
    }
}

#[test]
fn deleting_the_rule_entry_in_the_pull_request_is_judged_under_the_base_setting() {
    let extra = BASE;
    let base = config(extra, "Alice", GATES);
    let gone = base.replace(
        "  diff.decider-required: error\n",
        "  field.quality: error\n",
    );
    assert_ne!(gone, base);
    let dir = fixture("rule-deleted", Some(&base));
    let (v, code) = check_pr_exit(&dir, &gone, "mallory");
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    assert_eq!(code, Some(1), "{v}");
}

#[test]
fn dropping_the_authority_section_and_the_rule_together_is_still_judged_under_the_base() {
    let base = config(BASE, "Alice", GATES);
    let dir = fixture("authority-dropped", Some(&base));
    let head = TYPES.replace("diff.decider-required: error", "diff.decider-required: off");
    let (v, code) = check_pr_exit(&dir, &head, "mallory");
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    assert_eq!(code, Some(1), "{v}");
}

#[test]
fn lowering_the_rule_to_warn_and_fail_on_to_error_still_blocks() {
    // The base gates at warn under fail_on: warning; the pull request keeps the
    // rule at warn and drops fail_on, which would exit 0.
    let strict = "fail_on: warning\n";
    let base_cfg = config(BASE, "Alice", GATES).replace(
        "diff.decider-required: error",
        "diff.decider-required: warn",
    );
    let base = format!("{strict}{base_cfg}");
    let dir = fixture("fail-on-lowered", Some(&base));
    let (v, code) = check_pr_exit(&dir, &base_cfg, "mallory");
    assert_eq!(files(&v), [CONFIG, PATH_1], "{v}");
    assert_eq!(severities(&v), ["warning", "warning"], "{v}");
    assert_eq!(code, Some(1), "{v}");
    // Control: the same head with no base gate at all exits 0.
    let loose_base = base_cfg.replace("decider_table: base", "decider_table: head");
    let dir = fixture("fail-on-lowered-head", Some(&loose_base));
    let (v, code) = check_pr_exit(&dir, &base_cfg, "mallory");
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn a_base_gate_that_is_looser_than_the_head_does_not_loosen_the_head() {
    let base = config(BASE, "Alice", GATES).replace(
        "diff.decider-required: error",
        "diff.decider-required: warn",
    );
    let head = config(BASE, "Alice", GATES);
    let dir = fixture("base-looser", Some(&base));
    let (v, code) = check_pr_exit(&dir, &head, "mallory");
    assert_eq!(severities(&v), ["error", "error"], "{v}");
    assert_eq!(code, Some(1), "{v}");
}

#[test]
fn doctor_says_an_authority_with_no_gates_is_declared_but_gates_nothing() {
    let doctor = |cfg: &str, name: &str| {
        let dir = fixture(name, Some(cfg));
        let v = json(&run(&dir, &["doctor"]));
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["check"] == "authority-host")
            .cloned()
            .expect("an authority-host check")
    };
    let empty = doctor(&config(BASE, "Alice", ""), "doctor-no-gates");
    assert_eq!(empty["status"], "warn", "{empty}");
    assert!(empty["message"]
        .as_str()
        .unwrap()
        .contains("declared, no gates"));
    let gated = doctor(&config(BASE, "Alice", GATES), "doctor-gates");
    assert_eq!(gated["status"], "ok", "{gated}");
}

const WAIVER_TYPES: &str = "  waiver:\n    dir: \"docs/waiver\"\n";
const ADR_GATE: &str = "  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n";
const WAIVER_PATH: &str = "docs/waiver/WAIVER-9-x.md";
const WAIVER: &str = "---\nRule: diff.decider-required\nScope: docs/adr/ADR-1-a.md\nReason: accepted\nExpires: 2099-12-31\n---\n# WAIVER-9: x\n";

fn waiver_config(extra: &str) -> String {
    format!(
        "{TYPES}{WAIVER_TYPES}authority:\n{extra}  deciders:\n    maintainers: {{logins: [Alice]}}\n{ADR_GATE}"
    )
}

/// The PR moves the record to Accepted as `mallory` and, when `add_waiver`,
/// adds the waiver in the same change. `base_waiver` commits it at the base.
fn waiver_pr(
    name: &str,
    base_cfg: &str,
    head_cfg: &str,
    base_waiver: bool,
    add_waiver: bool,
) -> (serde_json::Value, Option<i32>) {
    let dir = common::TempDir::new(&format!("urzua-waivable-{name}"));
    for d in ["docs/adr", "docs/waiver", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    std::fs::write(dir.join(CONFIG), base_cfg).unwrap();
    if base_waiver {
        std::fs::write(dir.join(WAIVER_PATH), WAIVER).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    std::fs::write(dir.join(CONFIG), head_cfg).unwrap();
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    if add_waiver {
        std::fs::write(dir.join(WAIVER_PATH), WAIVER).unwrap();
        git(&dir, &["add", WAIVER_PATH]);
    }
    let file = actors(&dir, "mallory", &[]);
    let out = run(&dir, &["check", "--base", "HEAD", "--actors", &file]);
    (json(&out), out.status.code())
}

#[test]
fn a_waiver_added_in_the_pull_request_clears_the_gate_under_any_and_undeclared() {
    for extra in ["", "  decider_table: base\n  waivable: any\n"] {
        let cfg = waiver_config(extra);
        let (v, code) = waiver_pr("any", &cfg, &cfg, false, true);
        assert_eq!(code, Some(0), "{extra:?}: {v}");
        assert_eq!(findings(&v).len(), 1, "{v}");
        assert!(!findings(&v)[0]["waived"].is_null(), "{v}");
    }
}

#[test]
fn a_waiver_added_in_the_pull_request_does_not_clear_the_gate_under_base_or_never() {
    for w in ["base", "never"] {
        let cfg = waiver_config(&format!("  decider_table: base\n  waivable: {w}\n"));
        let (v, code) = waiver_pr(&format!("pr-{w}"), &cfg, &cfg, false, true);
        assert_eq!(code, Some(1), "{w}: {v}");
        assert!(findings(&v)[0]["waived"].is_null(), "{w}: {v}");
        assert!(
            v["notices"].as_array().unwrap().iter().any(|n| n["message"]
                .as_str()
                .unwrap()
                .contains("authority.waivable")),
            "{w}: {v}"
        );
    }
}

#[test]
fn a_waiver_the_base_already_holds_still_waives_under_base_but_not_never() {
    let base = waiver_config("  decider_table: base\n  waivable: base\n");
    let (v, code) = waiver_pr("held-base", &base, &base, true, false);
    assert_eq!(code, Some(0), "{v}");
    let never = waiver_config("  decider_table: base\n  waivable: never\n");
    let (v, code) = waiver_pr("held-never", &never, &never, true, false);
    assert_eq!(code, Some(1), "{v}");
}

#[test]
fn a_pull_request_cannot_loosen_waivable_by_editing_the_key() {
    let base = waiver_config("  decider_table: base\n  waivable: never\n");
    for head_extra in [
        "  decider_table: base\n  waivable: any\n",
        "  decider_table: base\n",
    ] {
        let head = waiver_config(head_extra);
        let (v, code) = waiver_pr("loosen", &base, &head, false, true);
        assert_eq!(code, Some(1), "{head_extra:?}: {v}");
    }
}

#[test]
fn the_pull_request_cannot_drop_require_evidence_at_the_same_level() {
    let base = config(BASE, "Alice", GATES).replace(
        "diff.decider-required: error",
        "diff.decider-required: {level: error, require_evidence: true}",
    );
    let head = config(BASE, "Alice", GATES);
    let dir = fixture("evidence-dropped", Some(&base));
    std::fs::write(dir.join(CONFIG), &head).unwrap();
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    // No actors file: with the option kept this is an error, and must stay one.
    let out = run(&dir, &["check", "--base", "HEAD"]);
    let v = json(&out);
    assert_eq!(severities(&v), ["error", "error"], "{v}");
    assert_eq!(out.status.code(), Some(1), "{v}");
}

#[test]
fn doctor_warns_on_an_undeclared_waivable_and_on_an_ungated_config() {
    let doctor = |cfg: &str, name: &str, check: &str| {
        let dir = fixture(name, Some(cfg));
        let v = json(&run(&dir, &["doctor"]));
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["check"] == check)
            .cloned()
    };
    let loose = waiver_config(BASE);
    let w = doctor(&loose, "doc-waivable", "authority-waivable").expect("a check");
    assert_eq!(w["status"], "warn", "{w}");
    let c = doctor(&loose, "doc-config", "authority-config-gated").expect("a check");
    assert_eq!(c["status"], "warn", "{c}");
    let gated = config("  decider_table: base\n  waivable: never\n", "Alice", GATES);
    let w = doctor(&gated, "doc-waivable-ok", "authority-waivable").expect("a check");
    assert_eq!(w["status"], "ok", "{w}");
    assert!(doctor(&gated, "doc-config-ok", "authority-config-gated").is_none());
}

// ----------------------------- deletions use the base revision's type config

/// `TYPES` with `adr` declaring `extra` (indented type-body lines) as well.
fn config_with_adr(extra: &str, mode: &str) -> String {
    format!(
        "{}{extra}authority:\n{mode}  deciders:\n    maintainers: {{logins: [Alice]}}\n{GATES}",
        TYPES
    )
}

/// A repository whose base holds an Accepted `ADR-1` and `base_config`; the
/// working tree is the pull request.
fn accepted_fixture(name: &str, base_config: &str) -> common::TempDir {
    let dir = fixture(name, Some(base_config));
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "accepted"]);
    dir
}

/// The pull request deletes the Accepted record and writes `head_config`.
fn delete_pr(dir: &Path, head_config: &str) -> serde_json::Value {
    std::fs::write(dir.join(CONFIG), head_config).unwrap();
    std::fs::remove_file(dir.join(PATH_1)).unwrap();
    let file = actors(dir, "mallory", &[]);
    json(&run(dir, &["check", "--base", "HEAD", "--actors", &file]))
}

#[test]
fn deleting_an_accepted_record_and_excluding_it_in_the_same_pr_is_a_deletion_under_base() {
    for mode in ["base", "both"] {
        let m = format!("  decider_table: {mode}\n");
        let base = config_with_adr("", &m);
        let head = config_with_adr("    exclude_files: [\"ADR-1-a.md\"]\n", &m);
        let dir = accepted_fixture(&format!("del-exclude-{mode}"), &base);
        let v = delete_pr(&dir, &head);
        assert!(files(&v).contains(&PATH_1.to_string()), "{mode}: {v}");
    }
}

#[test]
fn deleting_an_accepted_record_and_moving_the_type_dir_in_the_same_pr_is_a_deletion_under_base() {
    let m = "  decider_table: base\n";
    let base = config_with_adr("", m);
    let head = config_with_adr("", m).replace("dir: \"docs/adr\"", "dir: \"docs/decisions\"");
    let dir = accepted_fixture("del-dir", &base);
    let v = delete_pr(&dir, &head);
    assert!(files(&v).contains(&PATH_1.to_string()), "{v}");
}

#[test]
fn the_same_pull_requests_are_not_deletions_under_head_or_when_undeclared() {
    for m in ["  decider_table: head\n", ""] {
        let base = config_with_adr("", m);
        let excluded = config_with_adr("    exclude_files: [\"ADR-1-a.md\"]\n", m);
        let moved = config_with_adr("", m).replace("dir: \"docs/adr\"", "dir: \"docs/decisions\"");
        for (label, head) in [("exclude", excluded), ("dir", moved)] {
            let dir = accepted_fixture(&format!("del-head-{label}"), &base);
            let v = delete_pr(&dir, &head);
            assert!(
                !files(&v).contains(&PATH_1.to_string()),
                "{m:?} {label}: {v}"
            );
        }
    }
}
