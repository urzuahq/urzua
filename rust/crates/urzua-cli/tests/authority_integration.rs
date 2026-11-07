//! MILE-200, ADR-89 section 4: who may accept a record. The `authority` table
//! loads and is validated, `urzua harness run edit` refuses an agent's move to
//! an accepting status and an agent's write to a protected path, and
//! `diff.decider-required` reads the actors file the gate action writes.
//! Real git-repo fixtures and the real binary; all output is JSON.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const TYPES: &str = "schema_version: 2\noverride_log: .urzua/override-log.md\nrules:\n  diff.decider-required: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n";
const HARNESS: &str = "harness:\n  guardrails:\n    acceptance: {enabled: true}\n    protected-paths: {enabled: true}\n";
const AUTHORITY: &str = "authority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const REJECTED: &str = "---\nStatus: Rejected\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

fn config() -> String {
    format!("{TYPES}{HARNESS}{AUTHORITY}")
}

fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-authority-{name}"));
    for d in ["docs/adr", ".urzua", ".claude"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_urzua"));
    c.args(args).current_dir(dir);
    c
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

// ------------------------------------------------------------- the table

#[test]
fn a_well_formed_authority_table_loads_and_evidence_defaults_to_the_pr_author() {
    let dir = fixture("loads", &config());
    let out = urzua(&dir, &["check"]).output().unwrap();
    // The fixture holds nothing a rule examines, so the run is `not-run`, but the
    // configuration loaded: a load error is an `error` member.
    assert!(json(&out).get("error").is_none(), "{out:?}");
}

fn must_not_load(name: &str, cfg: &str, needles: &[&str]) {
    let dir = fixture(name, cfg);
    let out = urzua(&dir, &["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
    let error = json(&out)["error"].as_str().unwrap_or("").to_string();
    for needle in needles {
        assert!(error.contains(needle), "{name}: {error}");
    }
}

#[test]
fn a_group_with_no_login_is_a_load_error_naming_the_group() {
    let cfg = config().replace("{logins: [Alice]}", "{logins: []}");
    must_not_load("no-login", &cfg, &["maintainers", "no login"]);
}

#[test]
fn a_login_in_no_group_is_a_load_error_naming_the_login() {
    let cfg = config().replace(
        "    maintainers: {logins: [Alice]}\n",
        "    maintainers: [Alice]\n",
    );
    must_not_load("no-group", &cfg, &["Alice", "no group"]);
    let cfg = config().replace(
        "  deciders:\n    maintainers: {logins: [Alice]}\n",
        "  deciders: [Alice]\n",
    );
    must_not_load("flat-deciders", &cfg, &["Alice", "no group"]);
}

#[test]
fn a_gate_naming_a_group_that_is_not_declared_is_a_load_error() {
    let cfg = config().replace(
        "by: maintainers}\n    - {paths",
        "by: owners}\n    - {paths",
    );
    must_not_load("unknown-group", &cfg, &["authority.gates[0]", "owners"]);
}

#[test]
fn a_gate_whose_type_or_to_the_configuration_does_not_declare_is_a_load_error() {
    let cfg = config().replace("type: adr", "type: rfc");
    must_not_load("unknown-type", &cfg, &["authority.gates[0]", "'rfc'"]);
    let cfg = config().replace("[Accepted, Rejected]", "[Accepted, Approved]");
    must_not_load("unknown-to", &cfg, &["authority.gates[0]", "'Approved'"]);
}

#[test]
fn an_unknown_evidence_kind_and_an_unknown_key_are_load_errors() {
    let cfg = config().replace("  gates:", "  evidence: [pr-author, pr-friend]\n  gates:");
    must_not_load("evidence", &cfg, &["pr-friend"]);
    let cfg = config().replace("  gates:", "  colour: blue\n  gates:");
    must_not_load("key", &cfg, &["colour"]);
}

#[test]
fn an_enabled_guardrail_with_no_gate_to_enforce_is_a_load_error() {
    // `acceptance` needs a gate by type and status, `protected-paths` one by path.
    let only_paths = config().replace(
        "    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n",
        "",
    );
    must_not_load("no-type-gate", &only_paths, &["'acceptance'", "gate"]);
    let only_types = config().replace(
        "    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n",
        "",
    );
    must_not_load("no-path-gate", &only_types, &["'protected-paths'", "gate"]);
}

#[test]
fn the_decider_rule_enabled_with_no_authority_is_a_load_error() {
    let cfg = TYPES.to_string();
    must_not_load(
        "rule-no-authority",
        &cfg,
        &["diff.decider-required", "authority"],
    );
}

// ------------------------------------------------- the local refusal

fn run_edit(
    dir: &Path,
    path: &str,
    content: &str,
    actor: &str,
    env: &[(&str, &str)],
) -> (i32, serde_json::Value) {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path], "content": content,
        "actor": actor, "cwd": dir.to_string_lossy(),
    });
    let mut c = urzua(dir, &["harness", "run", "edit"]);
    c.env_remove("URZUA_HOOK_OVERRIDE")
        .env_remove("URZUA_HOOK_REASON");
    for (k, v) in env {
        c.env(k, v);
    }
    let mut child = c
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap(), json(&out))
}

#[test]
fn an_agent_moving_a_record_to_an_accepting_status_is_denied() {
    let dir = fixture("deny-move", &config());
    for target in [ACCEPTED, REJECTED] {
        let (code, v) = run_edit(&dir, PATH_1, target, "agent", &[]);
        assert_eq!(code, 1, "{v}");
        assert_eq!(v["verdict"], "block", "{v}");
        let message = v["message"].as_str().unwrap();
        assert!(
            message.contains("acceptance") && message.contains("Proposed"),
            "{message}"
        );
        assert!(message.contains("maintainers"), "{message}");
    }
    // A request that names no actor is an agent's.
    let request = serde_json::json!({
        "event": "edit", "paths": [PATH_1], "content": ACCEPTED, "cwd": dir.to_string_lossy(),
    });
    let mut child = urzua(&dir, &["harness", "run", "edit"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    // The file on disk is untouched: the engine judges, it never writes.
    assert_eq!(std::fs::read_to_string(dir.join(PATH_1)).unwrap(), PROPOSED);
}

#[test]
fn a_human_actor_moving_a_record_to_an_accepting_status_is_not_denied() {
    let dir = fixture("human-move", &config());
    let (code, v) = run_edit(&dir, PATH_1, ACCEPTED, "human", &[]);
    assert_eq!(code, 0, "{v}");
    assert_ne!(v["verdict"], "block", "{v}");
}

#[test]
fn the_override_cannot_lift_the_acceptance_denial_and_the_output_says_so() {
    let dir = fixture("override", &config());
    let (code, v) = run_edit(
        &dir,
        PATH_1,
        ACCEPTED,
        "agent",
        &[
            ("URZUA_HOOK_OVERRIDE", "me"),
            ("URZUA_HOOK_REASON", "because"),
        ],
    );
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "block", "{v}");
    assert!(v.get("override").is_none(), "{v}");
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains("override") && message.contains("not accepted"),
        "{message}"
    );
    // No row was written: the override was not applied.
    assert!(!dir.join(".urzua/override-log.md").exists());
}

#[test]
fn an_agent_edit_that_does_not_move_the_status_is_not_refused_by_the_gate() {
    let dir = fixture("no-move", &config());
    // Proposed to Proposed.
    let (_, v) = run_edit(
        &dir,
        PATH_1,
        &PROPOSED.replace("prose", "more"),
        "agent",
        &[],
    );
    assert_ne!(v["verdict"], "block", "{v}");
    // An already Accepted record edited in its body is not a move.
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    git(&dir, &["commit", "-qam", "accepted by a person"]);
    let (_, v) = run_edit(
        &dir,
        PATH_1,
        &ACCEPTED.replace("prose", "more"),
        "agent",
        &[],
    );
    assert_ne!(v["verdict"], "block", "{v}");
    // Moving an Accepted record on to Rejected is a move to a gated status.
    let (code, v) = run_edit(&dir, PATH_1, REJECTED, "agent", &[]);
    assert_eq!(code, 1, "{v}");
}

#[test]
fn an_agent_filing_a_new_record_already_accepted_is_denied() {
    let dir = fixture("new-accepted", &config());
    let (code, v) = run_edit(
        &dir,
        "docs/adr/ADR-2-b.md",
        &ACCEPTED.replace("ADR-1", "ADR-2"),
        "agent",
        &[],
    );
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "block", "{v}");
}

#[test]
fn an_agent_writing_a_protected_path_is_denied_and_a_person_is_not() {
    let dir = fixture("protected", &config());
    for path in [
        ".claude/settings.json",
        ".claude/hooks/new.py",
        ".urzua/config.yaml",
    ] {
        let (code, v) = run_edit(&dir, path, "{}\n", "agent", &[]);
        assert_eq!(code, 1, "{path}: {v}");
        assert_eq!(v["verdict"], "block", "{path}: {v}");
        assert!(
            v["message"].as_str().unwrap().contains("protected-paths"),
            "{path}: {v}"
        );
        let (code, v) = run_edit(&dir, path, "{}\n", "human", &[]);
        assert_eq!(code, 0, "{path}: {v}");
    }
    // An override does not lift it either.
    let (code, v) = run_edit(
        &dir,
        ".claude/settings.json",
        "{}\n",
        "agent",
        &[("URZUA_HOOK_OVERRIDE", "me"), ("URZUA_HOOK_REASON", "r")],
    );
    assert_eq!(code, 1, "{v}");
    // A path no gate names is not protected.
    let (_, v) = run_edit(
        &dir,
        "docs/adr/ADR-9-z.md",
        &PROPOSED.replace("ADR-1", "ADR-9"),
        "agent",
        &[],
    );
    assert_ne!(v["verdict"], "block", "{v}");
}

#[test]
fn a_gate_that_is_not_enabled_refuses_nothing() {
    let cfg = format!("{TYPES}{AUTHORITY}");
    let dir = fixture("not-enabled", &cfg);
    let (code, v) = run_edit(&dir, PATH_1, ACCEPTED, "agent", &[]);
    assert_eq!(code, 0, "{v}");
    let (code, v) = run_edit(&dir, ".claude/settings.json", "{}\n", "agent", &[]);
    assert_eq!(code, 0, "{v}");
}

#[test]
fn a_broken_configuration_still_denies_an_agent_the_protected_paths_and_names_the_way_out() {
    // The record types no longer load, but the raw file still says what is protected.
    let broken = config().replace("record_types:", "record_types_oops:");
    let dir = fixture("broken", &broken);
    let (code, v) = run_edit(&dir, ".urzua/config.yaml", "x: 1\n", "agent", &[]);
    assert_eq!(code, 1, "{v}");
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains("outside the agent harness") && message.contains("on_unjudged"),
        "{message}"
    );
    let (code, v) = run_edit(&dir, ".urzua/config.yaml", "x: 1\n", "human", &[]);
    assert_eq!(code, 0, "{v}");
    // With on_unjudged: allow the repair is open to the agent.
    let relaxed = broken.replace(
        "protected-paths: {enabled: true}",
        "protected-paths: {enabled: true, on_unjudged: allow}",
    );
    let dir = fixture("broken-allow", &relaxed);
    let (code, v) = run_edit(&dir, ".urzua/config.yaml", "x: 1\n", "agent", &[]);
    assert_eq!(code, 0, "{v}");
}

// --------------------------------------------- diff.decider-required

fn write_actors(dir: &Path, body: serde_json::Value) -> String {
    let path = dir.join("actors.json");
    std::fs::write(&path, body.to_string()).unwrap();
    path.to_string_lossy().to_string()
}

fn decider_findings(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.decider-required")
        .collect()
}

fn decider_status(v: &serde_json::Value) -> String {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "diff.decider-required")
        .map(|r| r["status"].as_str().unwrap().to_string())
        .unwrap_or_else(|| panic!("the rule did not appear: {v}"))
}

fn check_base(dir: &Path, extra: &[&str]) -> (i32, serde_json::Value) {
    let mut args = vec!["check", "--base", "HEAD"];
    args.extend_from_slice(extra);
    let out = urzua(dir, &args).output().unwrap();
    (out.status.code().unwrap(), json(&out))
}

#[test]
fn a_gated_move_authored_by_a_decider_passes_with_the_default_evidence() {
    let dir = fixture("author-ok", &config());
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let actors = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "alice", "approvers": []}),
    );
    let (code, v) = check_base(&dir, &["--actors", &actors]);
    assert_eq!(code, 0, "{v}");
    assert!(decider_findings(&v).is_empty(), "{v}");
    assert_eq!(decider_status(&v), "ran");
}

#[test]
fn a_gated_move_authored_by_someone_else_is_an_error_naming_the_record_and_the_group() {
    let dir = fixture("author-bad", &config());
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let actors = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "mallory", "approvers": ["alice"]}),
    );
    // An approval by a decider does not count under the default `pr-author` evidence.
    let (code, v) = check_base(&dir, &["--actors", &actors]);
    assert_eq!(code, 1, "{v}");
    let found = decider_findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["severity"], "error");
    assert_eq!(found[0]["file"], PATH_1);
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("Proposed")
            && message.contains("Accepted")
            && message.contains("maintainers"),
        "{message}"
    );
}

#[test]
fn an_approver_counts_only_when_the_evidence_names_pr_approver() {
    let cfg = config().replace("  gates:", "  evidence: [pr-approver]\n  gates:");
    let dir = fixture("approver", &cfg);
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let approved = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "mallory", "approvers": ["ALICE"]}),
    );
    let (code, v) = check_base(&dir, &["--actors", &approved]);
    assert_eq!(code, 0, "{v}");
    // A stale approval is not in the file the gate action writes, so it is not an approver.
    let stale = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "alice", "approvers": []}),
    );
    let (code, v) = check_base(&dir, &["--actors", &stale]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(decider_findings(&v).len(), 1, "{v}");
}

#[test]
fn a_changed_protected_path_needs_the_group_too() {
    let dir = fixture("path", &config());
    std::fs::write(dir.join(".claude/settings.json"), "{\"a\": 1}\n").unwrap();
    let bad = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "mallory"}),
    );
    let (code, v) = check_base(&dir, &["--actors", &bad]);
    assert_eq!(code, 1, "{v}");
    let found = decider_findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["file"], ".claude/settings.json");
    let good = write_actors(&dir, serde_json::json!({"schema": 1, "pr_author": "Alice"}));
    let (code, v) = check_base(&dir, &["--actors", &good]);
    assert_eq!(code, 0, "{v}");
}

#[test]
fn a_change_that_is_not_gated_is_not_judged() {
    let dir = fixture("not-gated", &config());
    std::fs::write(dir.join(PATH_1), PROPOSED.replace("prose", "more")).unwrap();
    let actors = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "mallory"}),
    );
    let (code, v) = check_base(&dir, &["--actors", &actors]);
    assert_eq!(code, 0, "{v}");
    assert!(decider_findings(&v).is_empty(), "{v}");
}

#[test]
fn a_gated_move_with_no_actors_file_is_info_locally_and_an_error_under_require_evidence() {
    let dir = fixture("no-actors", &config());
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let (code, v) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{v}");
    let found = decider_findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["severity"], "info");
    assert!(
        found[0]["message"].as_str().unwrap().contains("--actors"),
        "{v}"
    );

    let cfg = config().replace(
        "  diff.decider-required: error",
        "  diff.decider-required: {level: error, require_evidence: true}",
    );
    let dir = fixture("require", &cfg);
    std::fs::write(dir.join(PATH_1), ACCEPTED).unwrap();
    let (code, v) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(decider_findings(&v)[0]["severity"], "error");
}

#[test]
fn without_a_base_the_rule_is_not_applicable_not_clean() {
    let dir = fixture("no-base", &config());
    let out = urzua(&dir, &["check"]).output().unwrap();
    assert_eq!(decider_status(&json(&out)), "not-applicable");
}

#[test]
fn an_unreadable_or_malformed_actors_file_is_could_not_run() {
    let dir = fixture("bad-actors", &config());
    let (code, v) = check_base(&dir, &["--actors", "/nonexistent/actors.json"]);
    assert_eq!(code, 2, "{v}");
    std::fs::write(dir.join("actors.json"), "not json").unwrap();
    let path = dir.join("actors.json");
    let (code, v) = check_base(&dir, &["--actors", &path.to_string_lossy()]);
    assert_eq!(code, 2, "{v}");
    let wrong = write_actors(&dir, serde_json::json!({"schema": 2}));
    let (code, v) = check_base(&dir, &["--actors", &wrong]);
    assert_eq!(code, 2, "{v}");
}

#[test]
fn an_actors_file_no_rule_reads_is_reported_not_ignored() {
    let cfg = format!("{TYPES}{HARNESS}{AUTHORITY}").replace(
        "  diff.decider-required: error\n",
        "  diff.decider-required: off\n",
    );
    let dir = fixture("unread-actors", &cfg);
    let actors = write_actors(&dir, serde_json::json!({"schema": 1, "pr_author": "alice"}));
    let (_, v) = check_base(&dir, &["--actors", &actors]);
    let notices = v["notices"].to_string();
    assert!(
        notices.contains("--actors") && notices.contains("diff.decider-required"),
        "{notices}"
    );
}

#[test]
fn actors_cannot_be_combined_with_stdin_path() {
    let dir = fixture("stdin-actors", &config());
    let actors = write_actors(&dir, serde_json::json!({"schema": 1, "pr_author": "alice"}));
    let out = urzua(
        &dir,
        &["check", "--stdin-path", PATH_1, "--actors", &actors],
    )
    .stdin(Stdio::null())
    .output()
    .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

// ------------------------------------------ this repository's own table

fn own_config() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::read_to_string(path).expect("repository config readable")
}

#[test]
fn this_repositorys_own_authority_denies_an_agent_acceptance_and_leaves_a_person_free() {
    let dir = common::TempDir::new("urzua-authority-own");
    for d in ["docs/adr", ".urzua", ".claude", "docs/rfc"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), own_config()).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    let proposed = "---\nStable-Id: 01M6NWW8WX0N9Z2R86HX2E9SBM\nStatus: Proposed\nDate: 2026-11-05\nAuthor: someone\nDeciders: null\n---\n# 1 — T\n\nbody\n";
    std::fs::write(dir.join("docs/adr/ADR-1-t.md"), proposed).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    let accepted = proposed
        .replace("Status: Proposed", "Status: Accepted")
        .replace(
            "Deciders: null",
            "Deciders: beauwilliams\nAccepted-on: 2026-11-06",
        );
    let path = "docs/adr/ADR-1-t.md";
    let (code, v) = run_edit(&dir, path, &accepted, "agent", &[]);
    assert_eq!(code, 1, "{v}");
    assert!(v["message"].as_str().unwrap().contains("acceptance"), "{v}");
    let (code, v) = run_edit(&dir, path, &accepted, "human", &[]);
    assert_eq!(code, 0, "{v}");
    for path in [
        ".claude/settings.json",
        ".git-hooks/pre-commit",
        ".github/workflows/ci.yml",
        ".urzua/config.yaml",
    ] {
        let (code, v) = run_edit(&dir, path, "x\n", "agent", &[]);
        assert_eq!(code, 1, "{path}: {v}");
        let (code, _) = run_edit(&dir, path, "x\n", "human", &[]);
        assert_eq!(code, 0, "{path}");
    }
    // The maintainer, working by hand, accepts and `check --base` is satisfied by the
    // pull request author being in the group, with no other evidence.
    std::fs::write(dir.join("docs/adr/ADR-1-t.md"), &accepted).unwrap();
    let actors = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "beauwilliams"}),
    );
    let (_, v) = check_base(&dir, &["--actors", &actors]);
    assert!(decider_findings(&v).is_empty(), "{v}");
    assert_eq!(decider_status(&v), "ran", "{v}");
}

// ----------------------------------- path entries in a non-canonical spelling

#[test]
fn a_gate_path_written_with_a_leading_dot_slash_gates_what_the_plain_spelling_does() {
    let cfg = config().replace("[\".claude\", ", "[\"./.claude/\", ");
    let dir = fixture("dot-slash", &cfg);
    // The hook core refuses an agent on the path.
    let (code, v) = run_edit(&dir, ".claude/settings.json", "{}\n", "agent", &[]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "block", "{v}");
    // And check --base asks for the group on the changed path.
    std::fs::write(dir.join(".claude/settings.json"), "{\"a\": 1}\n").unwrap();
    let bad = write_actors(
        &dir,
        serde_json::json!({"schema": 1, "pr_author": "mallory"}),
    );
    let (code, v) = check_base(&dir, &["--actors", &bad]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(decider_findings(&v).len(), 1, "{v}");
}

#[test]
fn a_gate_path_that_can_match_nothing_is_a_load_error() {
    for (entry, needle) in [
        ("\".\"", "names no path"),
        ("\"./\"", "names no path"),
        ("\"a\\\\b\"", "backslash"),
        ("\"/abs\"", "absolute"),
    ] {
        let cfg = config().replace("[\".claude\", ", &format!("[{entry}, \".claude\", "));
        must_not_load("unmatchable", &cfg, &[needle, "authority.gates[1]"]);
    }
}
