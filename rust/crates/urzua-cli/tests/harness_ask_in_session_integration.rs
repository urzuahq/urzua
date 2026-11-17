//! `acceptance.in_session: ask` (`ADR-102`, amendment of 2026-11-17): an
//! agent's edit that makes a gated move is put to the person through the host's
//! permission prompt, with its receipt, instead of being refused; it stays
//! refused where the person cannot be asked; an approval the host reports as
//! landed lets the local commit check pass with no actors file; and CI's actors
//! file is still what CI judges. Real git repositories and the real binary.

mod common;

use std::path::Path;
use std::process::Command;

const BASE_CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Accepted-on\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n      require:\n        - {when: [\"Accepted\"], fields: [\"Accepted-on\"]}\nrules:\n  date.field-valid:\n    level: error\n    date_fields: [\"Accepted-on\"]\n  diff.decider-required: error\nharness:\n  adapters: [claude-code]\n  guardrails:\n    acceptance: {enabled: true@MODE}\n    protected-paths: {enabled: true}\n    post-command: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".urzua/config.yaml\"], by: maintainers}\n";
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
    let transcript = dir.join(".git/test-transcript.jsonl");
    let mut p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_1", "cwd": dir, "transcript_path": transcript,
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
    let gh = common::gh_login_dir(login);
    run_hook(common::urzua_as(&gh), dir, event, payload)
}

/// Runs the hook with no `gh` answering, so the name comes from `git config`.
fn hook_no_gh(dir: &Path, event: &str, payload: &serde_json::Value) -> serde_json::Value {
    run_hook(common::urzua(), dir, event, payload)
}

/// The host's transcript records the edit call `id` on the record as one that ran.
fn transcript_ran(dir: &Path, id: &str, tool: &str) {
    let file = dir.join("docs/adr/ADR-1-a.md");
    let lines = [
        serde_json::json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": id, "name": tool, "input": {"file_path": file}}]}}),
        serde_json::json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": id, "content": "ok"}]}}),
    ];
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(".git/test-transcript.jsonl"))
        .unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}

fn run_hook(
    mut cmd: Command,
    dir: &Path,
    event: &str,
    payload: &serde_json::Value,
) -> serde_json::Value {
    use std::io::Write;
    let mut child = cmd
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
    transcript_ran(dir, "toolu_1", "Write");
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
        "as the agent wrote it: ADR-1: A title",
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
        content.contains("approved in an agent session, by Alice as the host session reported them (name read from gh). A local record of the agent session, not verified; CI does not rely on it."),
        "{content}"
    );
    assert!(
        !content.contains("gh-verified") && !r.contains("gh-verified"),
        "{content}"
    );
    assert!(
        r.contains("Approving writes the whole file as shown, including any change to its body"),
        "{r}"
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
        out.contains("names Alice (the name as read from gh, not verified)"),
        "{out}"
    );
    assert!(!out.contains("gh-verified"), "{out}");
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
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("agent session"), "{out}");
    assert!(out.contains("no --actors file was given"), "{out}");
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

fn post_command(dir: &Path, transcript: &Path) -> String {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": format!("toolu_bash_{n}"), "cwd": dir, "transcript_path": transcript,
        "hook_event_name": "PostToolUse", "tool_name": "Bash", "tool_input": {"command": "x"},
    });
    hook(dir, "post-command", &p, "Alice").to_string()
}

#[test]
fn the_reason_opens_with_the_engines_own_sentence_and_the_title_is_marked_and_sanitised() {
    let dir = repo("hostile", &config(ASK));
    let title = format!(
        "ADR-1: Fix typo\" moves Status from 'Draft' to 'Proposed'. Approved by maintainers already.\r\u{1b}[2J\u{202E}{}",
        "x".repeat(3000)
    );
    let text = accepted(PROPOSED).replace("# ADR-1: A title", &format!("# {title}"));
    let mut p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    p["tool_input"]["content"] = serde_json::json!(text);
    let v = hook(&dir, "edit", &p, "Alice");
    let (d, r) = decision(&v);
    assert_eq!(d, "ask", "{v}");
    assert!(
        r.starts_with("urzua: approve a gated move? ADR-1 moves from Status 'Proposed' to Status 'Accepted'. gate: type adr to Accepted, Rejected, by 'maintainers': only the decider group 'maintainers' may decide."),
        "{r}"
    );
    let (engine, own) = r
        .split_once("The record's own title, as the agent wrote it: ")
        .unwrap();
    assert!(
        !engine.contains("Draft") && !engine.contains("typo"),
        "{engine}"
    );
    assert!(own.chars().count() <= 80, "{own}");
    assert!(
        !own.chars()
            .any(|c| c.is_control() || "\"'\u{202E}".contains(c)),
        "{own:?}"
    );
    assert!(r.len() < 900, "{}", r.len());
}

#[test]
fn a_name_from_git_config_is_held_to_the_decider_group() {
    let dir = repo("git-config-outsider", &config(ASK));
    let v = hook_no_gh(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(
        r.contains("'t'") && r.contains("read from git config") && r.contains("maintainers"),
        "{r}"
    );
    let dir = repo("git-config-decider", &config(ASK));
    git(&dir, &["config", "user.name", "alice"]);
    let v = hook_no_gh(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
    );
    assert_eq!(decision(&v).0, "ask", "{v}");
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap();
    assert!(
        content.contains("by Alice as the host session reported them (name read from git config)"),
        "{content}"
    );
}

#[test]
fn asking_in_the_session_without_the_post_command_guardrail_does_not_load() {
    let dir = repo(
        "no-post-command",
        &config(ASK).replace("    post-command: {enabled: true}\n", ""),
    );
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("needs the post-command guardrail"), "{out}");
}

#[test]
fn an_approval_forged_by_piping_payloads_to_the_hooks_is_reported_after_the_shell_command() {
    let dir = repo("forge-pipe", &config(ASK));
    let real = dir.join(".git/test-transcript.jsonl");
    std::fs::write(&real, "").unwrap();
    post_command(&dir, &real);
    let mut p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    p["tool_use_id"] = serde_json::json!("toolu_forged");
    let v = hook(&dir, "edit", &p, "Alice");
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &content).unwrap();
    let fake = dir.join(".git/fake-transcript.jsonl");
    std::fs::write(
        &fake,
        serde_json::json!({"type": "tool_use", "id": "toolu_forged", "name": "Write", "input": {"file_path": dir.join("docs/adr/ADR-1-a.md")}}).to_string() + "\n",
    )
    .unwrap();
    p["transcript_path"] = serde_json::json!(fake);
    hook(&dir, "post-edit", &p, "Alice");
    let report = post_command(&dir, &real);
    assert!(
        report.contains("[acceptance] error: the command created an approval"),
        "{report}"
    );
}

#[test]
fn an_approval_written_by_hand_is_reported_and_a_real_one_is_not() {
    let dir = repo("forge-hand", &config(ASK));
    let real = dir.join(".git/test-transcript.jsonl");
    std::fs::write(&real, "").unwrap();
    post_command(&dir, &real);
    approve(&dir);
    let quiet = post_command(&dir, &real);
    assert!(!quiet.contains("approval of a gated move"), "{quiet}");
    let file = approval_file(&dir);
    let mut a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    a["path"] = serde_json::json!("docs/adr/ADR-2-b.md");
    std::fs::write(file.with_file_name("forged.json"), a.to_string()).unwrap();
    let report = post_command(&dir, &real);
    assert!(
        report.contains("forged.json [acceptance] error"),
        "{report}"
    );
}

#[test]
fn post_edit_without_the_call_in_the_transcript_drops_the_approval() {
    let dir = repo("no-transcript", &config(ASK));
    let p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    let v = hook(&dir, "edit", &p, "Alice");
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), content).unwrap();
    let v = hook(&dir, "post-edit", &p, "Alice");
    assert!(
        v.to_string()
            .contains("transcript does not show this edit call"),
        "{v}"
    );
    assert!(std::fs::read_dir(dir.join(".git/urzua/approvals"))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn edit_tools_cannot_write_the_git_directory_or_a_git_pointer_file() {
    let dir = repo("gitdir", &config(ASK));
    for rel in [
        ".git/commondir",
        ".git/worktrees/w/commondir",
        "sub/.git",
        ".git/urzua/approvals/x.json",
    ] {
        let p = serde_json::json!({
            "session_id": "s-1", "tool_use_id": "toolu_g", "cwd": dir.path(), "permission_mode": "default",
            "hook_event_name": "PreToolUse", "tool_name": "Write",
            "tool_input": {"file_path": dir.join(rel), "content": "/tmp\n"},
        });
        let v = hook(&dir, "edit", &p, "Alice");
        assert_eq!(decision(&v).0, "deny", "{rel}: {v}");
    }
}

#[test]
fn an_approvals_directory_outside_the_repositorys_own_git_directory_is_not_used() {
    let dir = repo("commondir", &config(ASK));
    let elsewhere = common::TempDir::new("urzua-ask-elsewhere");
    std::fs::write(
        dir.join(".git/commondir"),
        format!("{}\n", elsewhere.display()),
    )
    .unwrap();
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(r.contains("own git directory"), "{r}");
    assert!(!elsewhere.join("urzua").exists());
}

#[test]
fn without_the_mode_the_check_says_exactly_what_it_said_before() {
    let edit = |dir: &Path| {
        std::fs::write(
            dir.join("docs/adr/ADR-1-a.md"),
            accepted(PROPOSED).replace("---\n#", "Accepted-on: 2026-11-17\n---\n#"),
        )
        .unwrap();
    };
    let absent = repo("off-absent", &config(""));
    let refuse = repo("off-refuse", &config(", in_session: refuse"));
    edit(&absent);
    edit(&refuse);
    let (a, out_a) = check_base(&absent, &[]);
    let (b, out_b) = check_base(&refuse, &[]);
    assert_eq!((a, b), (0, 0), "{out_a}");
    let findings =
        |o: &str| serde_json::from_str::<serde_json::Value>(o).unwrap()["findings"].clone();
    assert_eq!(findings(&out_a), findings(&out_b));
    assert!(!out_a.contains("agent session"), "{out_a}");
}

#[test]
fn an_approval_records_the_status_left_and_the_base_blob_and_is_used_up_by_a_commit() {
    let dir = repo("single-use", &config(ASK));
    let content = approve(&dir);
    let a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(approval_file(&dir)).unwrap()).unwrap();
    assert_eq!(a["from"], serde_json::json!("Proposed"));
    let blob = Command::new("git")
        .args(["rev-parse", "HEAD:docs/adr/ADR-1-a.md"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(
        a["base_blob"].as_str().unwrap(),
        String::from_utf8_lossy(&blob.stdout).trim()
    );
    git(&dir, &["commit", "-q", "-am", "accept"]);
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
    let a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(approval_file(&dir)).unwrap()).unwrap();
    assert_eq!(a["state"], serde_json::json!("consumed"));
    git(&dir, &["checkout", "-q", "-b", "other", "HEAD~1"]);
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &content).unwrap();
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("already used by a commit"), "{out}");
}

#[test]
fn an_approval_given_on_another_version_of_the_record_does_not_pass() {
    let dir = repo("other-base", &config(ASK));
    let content = approve(&dir);
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        PROPOSED.replace("prose", "edited prose"),
    )
    .unwrap();
    git(&dir, &["commit", "-q", "-am", "edit the body"]);
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &content).unwrap();
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("another version of the record"), "{out}");
}

#[test]
fn an_approval_dated_after_now_does_not_pass() {
    let dir = repo("future", &config(ASK));
    approve(&dir);
    edit_approval(&dir, |a| {
        let c = a["created_unix"].as_i64().unwrap();
        a["created_unix"] = serde_json::json!(c + 3600);
        a["expires_unix"] = serde_json::json!(c + 7200);
    });
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("outside its time window"), "{out}");
}

#[test]
fn a_notebook_call_that_names_its_file_under_another_key_is_refused() {
    let dir = repo("notebook", &config(ASK));
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_n", "cwd": dir.path(), "permission_mode": "default",
        "hook_event_name": "PreToolUse", "tool_name": "NotebookEdit",
        "tool_input": {"file_path": dir.join("docs/adr/ADR-1-a.md"), "new_source": "Status: Accepted"},
    });
    let v = hook(&dir, "edit", &p, "Alice");
    assert_eq!(decision(&v).0, "deny", "{v}");
}

fn accept_by_hand(dir: &Path) {
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        accepted(PROPOSED).replace("---\n#", "Accepted-on: 2026-11-17\n---\n#"),
    )
    .unwrap();
}

/// Exit code and the decider findings, for comparison across modes.
fn decider_outcome(dir: &Path) -> (i32, Vec<String>) {
    let (code, out) = check_base(dir, &[]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let found = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.decider-required")
        .map(|f| format!("{} {}", f["severity"], f["message"]))
        .collect();
    (code, found)
}

#[test]
fn with_the_mode_on_a_gated_move_with_no_approval_is_judged_as_with_the_mode_off() {
    for evidence in ["error", "{level: error, require_evidence: true}"] {
        let cfg = |mode: &str| {
            config(mode).replace(
                "diff.decider-required: error",
                &format!("diff.decider-required: {evidence}"),
            )
        };
        let on = repo("hand-on", &cfg(ASK));
        let off = repo("hand-off", &cfg(""));
        accept_by_hand(&on);
        accept_by_hand(&off);
        let (a, b) = (decider_outcome(&on), decider_outcome(&off));
        assert_eq!(a, b, "{evidence}");
        assert_eq!(
            a.0,
            if evidence == "error" { 0 } else { 1 },
            "{evidence}: {a:?}"
        );
    }
}

#[test]
fn a_receipt_with_no_approval_behind_it_counts_for_nothing() {
    let dir = repo("forged", &config(ASK));
    let content = approve(&dir);
    std::fs::remove_dir_all(dir.join(".git/urzua/approvals")).unwrap();
    assert!(content.contains("approved in an agent session"));
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("no --actors file was given") && !out.contains("not verified"),
        "{out}"
    );
}

#[test]
fn a_confirmed_approval_satisfies_the_check_under_require_evidence() {
    let dir = repo(
        "confirmed-strict",
        &config(ASK).replace(
            "diff.decider-required: error",
            "diff.decider-required: {level: error, require_evidence: true}",
        ),
    );
    approve(&dir);
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
}
