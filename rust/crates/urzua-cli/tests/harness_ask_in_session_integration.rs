//! `acceptance.in_session: ask` (`ADR-102`, amendment of 2026-11-17): an
//! agent's edit that makes a gated move is put to the person through the host's
//! permission prompt, with its receipt, instead of being refused; it stays
//! refused where the person cannot be asked; an approval the host reports as
//! landed lets the local commit check pass with no actors file; and CI's actors
//! file is still what CI judges. Real git repositories and the real binary.

mod common;

use std::path::Path;
use std::process::Command;

const BASE_CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Accepted-on\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n      require:\n        - {when: [\"Accepted\"], fields: [\"Accepted-on\"]}\nrules:\n  date.field-valid:\n    level: error\n    date_fields: [\"Accepted-on\"]\n  diff.decider-required: {level: error, require_evidence: true}\nharness:\n  adapters: [claude-code]\n  guardrails:\n    acceptance: {enabled: true@MODE}\n    protected-paths: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".urzua/config.yaml\"], by: maintainers}\n";
const ASK: &str = ", in_session: ask";

const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: A title\n\nprose\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-01 | Drafted. | **substantive** |\n";

fn config(mode: &str) -> String {
    BASE_CONFIG.replace("@MODE", mode)
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-ask-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), PROPOSED).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        PROPOSED.replace("ADR-1: A title", "ADR-2: Another"),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn accepted(text: &str) -> String {
    text.replace("Status: Proposed", "Status: Accepted")
}

fn payload(
    dir: &Path,
    tool: &str,
    mode: Option<&str>,
    extra: serde_json::Value,
) -> serde_json::Value {
    let file = dir.join("docs/adr/ADR-1-a.md");
    let input = match tool {
        "Write" => serde_json::json!({"file_path": file, "content": accepted(PROPOSED)}),
        _ => {
            serde_json::json!({"file_path": file, "old_string": "Status: Proposed", "new_string": "Status: Accepted"})
        }
    };
    let mut p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_1", "cwd": dir,
        "hook_event_name": "PreToolUse", "tool_name": tool, "tool_input": input,
    });
    if let Some(m) = mode {
        p["permission_mode"] = serde_json::json!(m);
    }
    if let serde_json::Value::Object(m) = extra {
        for (k, v) in m {
            p[k] = v;
        }
    }
    p
}

/// Runs the hook with `gh` answering `login`, as the person's machine does.
fn hook(dir: &Path, event: &str, payload: &serde_json::Value, login: &str) -> serde_json::Value {
    use std::io::Write;
    let gh = common::gh_login_dir(login);
    let mut child = common::urzua_as(&gh)
        .args(["harness", "hook", "claude-code", event])
        .env("CLAUDE_PROJECT_DIR", dir)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("{e}: {text}"))
}

fn decision(v: &serde_json::Value) -> (&str, &str) {
    let h = &v["hookSpecificOutput"];
    (
        h["permissionDecision"].as_str().unwrap_or(""),
        h["permissionDecisionReason"].as_str().unwrap_or(""),
    )
}

fn check_base(dir: &Path, extra: &[&str]) -> (i32, String) {
    let out = common::urzua()
        .args(["check", "--base", "HEAD"])
        .args(extra)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

/// The ask, then the host writing the approved input, then its post-edit event.
fn approve(dir: &Path) -> String {
    let p = payload(dir, "Write", Some("default"), serde_json::json!({}));
    let v = hook(dir, "edit", &p, "Alice");
    let (d, r) = decision(&v);
    assert_eq!(d, "ask", "{v}");
    assert!(!r.is_empty());
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &content).unwrap();
    let mut post = p.clone();
    post["hook_event_name"] = serde_json::json!("PostToolUse");
    let v = hook(dir, "post-edit", &post, "Alice");
    assert!(v.to_string().contains("the person approved ADR-1"), "{v}");
    content
}

fn approval_file(dir: &Path) -> std::path::PathBuf {
    let d = dir.join(".git/urzua/approvals");
    std::fs::read_dir(&d)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .next()
        .expect("an approval")
}

fn edit_approval(dir: &Path, f: impl FnOnce(&mut serde_json::Value)) {
    let path = approval_file(dir);
    let mut a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    f(&mut a);
    std::fs::write(&path, a.to_string()).unwrap();
}

#[test]
fn without_the_mode_the_gated_edit_is_refused_as_before_and_the_refusal_names_the_key() {
    let dir = repo("off", &config(""));
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(
        r.contains("in_session: ask") && r.contains("outside the agent harness"),
        "{r}"
    );
    assert!(!dir.join(".git/urzua/approvals").exists());
}

#[test]
fn in_the_manual_mode_the_hook_asks_with_a_reason_it_wrote_and_the_edit_carries_its_receipt() {
    let dir = repo("ask", &config(ASK));
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "ask", "{v}");
    for want in [
        "ADR-1",
        "\"ADR-1: A title\"",
        "'Proposed'",
        "'Accepted'",
        "'maintainers'",
    ] {
        assert!(r.contains(want), "{want} not in {r}");
    }
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap();
    let today = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .unwrap();
    let today = String::from_utf8_lossy(&today.stdout).trim().to_string();
    assert!(
        content.contains(&format!("Accepted-on: {today}")),
        "{content}"
    );
    assert!(
        content.contains("approved in an agent session by Alice (identity: gh-verified)"),
        "{content}"
    );
    assert!(
        !content.to_lowercase().contains("edited by alice"),
        "{content}"
    );
    assert!(approval_file(&dir).exists());
}

#[test]
fn an_edit_is_put_to_the_person_as_one_replacement_of_the_whole_file() {
    let dir = repo("edit", &config(ASK));
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Edit", Some("default"), serde_json::json!({})),
        "Alice",
    );
    assert_eq!(decision(&v).0, "ask", "{v}");
    let u = &v["hookSpecificOutput"]["updatedInput"];
    assert_eq!(u["old_string"].as_str().unwrap(), PROPOSED);
    assert!(u["new_string"]
        .as_str()
        .unwrap()
        .contains("Status: Accepted"));
    assert_eq!(u["replace_all"], serde_json::json!(false));
}

#[test]
fn every_mode_that_does_not_show_the_prompt_and_an_unknown_or_missing_mode_refuse() {
    let dir = repo("modes", &config(ASK));
    for mode in [
        Some("acceptEdits"),
        Some("auto"),
        Some("dontAsk"),
        Some("bypassPermissions"),
        Some("plan"),
        Some("someNewMode"),
        None,
    ] {
        let v = hook(
            &dir,
            "edit",
            &payload(&dir, "Write", mode, serde_json::json!({})),
            "Alice",
        );
        let (d, r) = decision(&v);
        assert_eq!(d, "deny", "{mode:?}: {v}");
        assert!(
            r.contains("Manual (default) permission mode"),
            "{mode:?}: {r}"
        );
        if let Some(m) = mode {
            assert!(r.contains(m), "{r}");
        }
    }
    assert!(!dir.join(".git/urzua/approvals").exists());
}

#[test]
fn a_subagent_is_told_to_hand_back_to_the_main_session() {
    let dir = repo("subagent", &config(ASK));
    let p = payload(
        &dir,
        "Write",
        Some("default"),
        serde_json::json!({"agent_id": "a-1", "agent_type": "Explore"}),
    );
    let (d, r) = {
        let v = hook(&dir, "edit", &p, "Alice");
        let (d, r) = decision(&v);
        (d.to_string(), r.to_string())
    };
    assert_eq!(d, "deny");
    assert!(
        r.contains("hand back to the main session") || r.contains("Hand back to the main session"),
        "{r}"
    );
}

#[test]
fn a_verified_login_outside_the_decider_group_is_not_asked() {
    let dir = repo("outsider", &config(ASK));
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "mallory",
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(r.contains("'mallory'") && r.contains("maintainers"), "{r}");
}

#[test]
fn an_approved_move_reported_as_landed_passes_the_local_commit_check_without_an_actors_file() {
    let dir = repo("landed", &config(ASK));
    approve(&dir);
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("approved in an agent session by Alice"),
        "{out}"
    );
}

#[test]
fn the_same_bytes_written_without_the_host_reporting_the_call_do_not_pass() {
    let dir = repo("pending", &config(ASK));
    let p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    let v = hook(&dir, "edit", &p, "Alice");
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap();
    // A shell write of exactly what the prompt offered, after a decline.
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), content).unwrap();
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("has not reported that edit call as landed"),
        "{out}"
    );
}

#[test]
fn a_receipt_with_no_approval_behind_it_does_not_pass() {
    let dir = repo("forged", &config(ASK));
    let content = approve(&dir);
    std::fs::remove_dir_all(dir.join(".git/urzua/approvals")).unwrap();
    assert!(content.contains("approved in an agent session"));
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("no --actors file was given"), "{out}");
}

#[test]
fn an_approval_for_another_record_does_not_pass() {
    let dir = repo("other-record", &config(ASK));
    approve(&dir);
    edit_approval(&dir, |a| a["record_id"] = serde_json::json!("ADR-2"));
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("names the record ADR-2"), "{out}");
}

#[test]
fn an_approval_for_another_status_does_not_pass() {
    let dir = repo("other-status", &config(ASK));
    approve(&dir);
    edit_approval(&dir, |a| a["status"] = serde_json::json!("Rejected"));
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("a move to 'Rejected'"), "{out}");
}

#[test]
fn a_record_changed_after_the_approval_does_not_pass() {
    let dir = repo("changed", &config(ASK));
    let content = approve(&dir);
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        content.replace("prose", "other prose"),
    )
    .unwrap();
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("changed after the person approved it"),
        "{out}"
    );
}

#[test]
fn an_expired_approval_does_not_pass() {
    let dir = repo("expired", &config(ASK));
    approve(&dir);
    edit_approval(&dir, |a| a["expires_unix"] = serde_json::json!(1));
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("expired"), "{out}");
}

#[test]
fn ci_judges_the_actors_file_and_a_receipt_does_not_stand_in_for_it() {
    let dir = repo("ci", &config(ASK));
    approve(&dir);
    let outside = common::TempDir::new("urzua-ask-actors");
    let actors = |author: &str| {
        let p = outside.join(format!("{author}.json"));
        std::fs::write(
            &p,
            serde_json::json!({"schema": 1, "pr_author": author}).to_string(),
        )
        .unwrap();
        p.to_string_lossy().to_string()
    };
    let (code, out) = check_base(&dir, &["--actors", &actors("Alice")]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = check_base(&dir, &["--actors", &actors("bob")]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("none is in the decider group 'maintainers'"),
        "{out}"
    );
}

#[test]
fn a_receipt_counts_for_nothing_when_the_mode_is_off() {
    let dir = repo("receipt-off", &config(ASK));
    approve(&dir);
    std::fs::write(dir.join(".urzua/config.yaml"), config("")).unwrap();
    git(&dir, &["add", ".urzua/config.yaml"]);
    git(&dir, &["commit", "-q", "-m", "mode off"]);
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_agent_cannot_write_an_approval_with_its_edit_tools() {
    let dir = repo("protected", &config(ASK));
    approve(&dir);
    let file = approval_file(&dir);
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_9", "cwd": dir.path(), "permission_mode": "default",
        "hook_event_name": "PreToolUse", "tool_name": "Write",
        "tool_input": {"file_path": file, "content": "{}"},
    });
    let v = hook(&dir, "edit", &p, "Alice");
    assert_eq!(decision(&v).0, "deny", "{v}");
}

#[test]
fn with_the_editor_hook_on_the_receipt_passes_the_record_check_before_the_person_is_asked() {
    let cfg = config(ASK).replace(
        "    protected-paths: {enabled: true}\n",
        "    protected-paths: {enabled: true}\n    editor-hook: {enabled: true}\n",
    );
    let dir = repo("editor-hook", &cfg);
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    assert_eq!(decision(&v).0, "ask", "{v}");
}

#[test]
fn the_installed_instructions_say_a_gated_edit_prompts_the_person() {
    let dir = repo(
        "instructions",
        &config(ASK).replace(
            "  adapters: [claude-code]\n",
            "  adapters: [claude-code]\n  instructions_file: AGENTS.md\n",
        ),
    );
    std::fs::write(dir.join("AGENTS.md"), "# Agents\n").unwrap();
    let out = common::urzua()
        .args(["harness", "install"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(text.contains("prompts the person to approve it"), "{text}");
    assert!(
        text.contains("never change a gated status without that answer"),
        "{text}"
    );
}
