//! The question route of `acceptance.in_session: ask` (`ADR-105`): where the
//! host shows no prompt, a gated edit is refused with urzua's question, the
//! hooks on the question tool record an approving answer, and the same edit is
//! then written once with its receipt. Real git repositories and the real binary.

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
    let dir = common::TempDir::new(&format!("urzua-askq-{name}"));
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
    let gh = common::gh_login_dir(login);
    run_hook(common::urzua_as(&gh), dir, event, payload)
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
        .env_remove("URZUA_HOOK_OVERRIDE")
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

const BYPASS: Option<&str> = Some("bypassPermissions");

/// Each call has its own id, as the host gives a retry a new one.
fn edit(dir: &Path) -> serde_json::Value {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let call = serde_json::json!({"tool_use_id": format!("toolu_edit_{n}")});
    hook(dir, "edit", &payload(dir, "Write", BYPASS, call), "Alice")
}

/// The question input the refusal hands the agent.
fn question_of(refusal: &str) -> serde_json::Value {
    let from = refusal.find("no answers field: ").expect(refusal) + "no answers field: ".len();
    let to = refusal[from..].find(" If the person picks").expect(refusal) + from;
    serde_json::from_str(&refusal[from..to]).unwrap()
}

fn text(input: &serde_json::Value) -> String {
    input["questions"][0]["question"]
        .as_str()
        .unwrap()
        .to_string()
}

fn approve_label(input: &serde_json::Value) -> String {
    input["questions"][0]["options"][0]["label"]
        .as_str()
        .unwrap()
        .to_string()
}

fn ask(dir: &Path, input: &serde_json::Value, call: &str) -> serde_json::Value {
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": call, "cwd": dir, "permission_mode": "bypassPermissions",
        "hook_event_name": "PreToolUse", "tool_name": "AskUserQuestion", "tool_input": input,
    });
    hook(dir, "question", &p, "Alice")
}

/// The answer as the host reports it after the call: the result carries it,
/// and the input is rewritten to match.
fn answer(
    dir: &Path,
    input: &serde_json::Value,
    call: &str,
    answer: &str,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut response = input.clone();
    response["answers"] = serde_json::json!({ text(input): answer });
    response["annotations"] = serde_json::json!({});
    if let serde_json::Value::Object(m) = extra {
        for (k, v) in m {
            response[k] = v;
        }
    }
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": call, "cwd": dir, "permission_mode": "bypassPermissions",
        "hook_event_name": "PostToolUse", "tool_name": "AskUserQuestion",
        "tool_input": response.clone(), "tool_response": response,
    });
    hook(dir, "answer", &p, "Alice")
}

fn refused_with_question(dir: &Path) -> serde_json::Value {
    let v = edit(dir);
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(r.contains("bypassPermissions"), "{r}");
    question_of(r)
}

#[test]
fn in_bypass_mode_an_approving_answer_lets_the_same_edit_through_once_with_its_receipt() {
    let dir = repo("q-flow", &config(ASK));
    let input = refused_with_question(&dir);
    assert!(text(&input).starts_with("urzua approval "), "{input}");
    assert!(
        text(&input).contains("ADR-1 moving Status from 'Proposed' to 'Accepted'"),
        "{input}"
    );
    assert_eq!(ask(&dir, &input, "toolu_q"), serde_json::Value::Null);
    let said = answer(
        &dir,
        &input,
        "toolu_q",
        &approve_label(&input),
        serde_json::json!({}),
    );
    assert!(
        said.to_string().contains("recorded the person's approval"),
        "{said}"
    );
    let v = edit(&dir);
    let h = &v["hookSpecificOutput"];
    assert_eq!(h["permissionDecision"], "allow", "{v}");
    let written = h["updatedInput"]["content"].as_str().unwrap();
    assert!(written.contains("Status: Accepted"), "{written}");
    assert!(written.contains("Recorded as approved in an agent session, not verified: ADR-1 Status moved from 'Proposed' to 'Accepted'"), "{written}");
    assert!(written.contains("answering urzua's question"), "{written}");
    let again = edit(&dir);
    assert_eq!(decision(&again).0, "deny", "an approval is used once");
}

#[test]
fn answers_already_in_the_question_input_are_refused_and_record_nothing() {
    let dir = repo("q-prefill", &config(ASK));
    let mut input = refused_with_question(&dir);
    input["answers"] = serde_json::json!({ text(&input): approve_label(&input) });
    let asked = ask(&dir, &input, "toolu_q");
    let (d, r) = decision(&asked);
    assert_eq!(d, "deny", "{r}");
    assert!(r.contains("answers of its own"), "{r}");
    answer(
        &dir,
        &input,
        "toolu_q",
        &approve_label(&input),
        serde_json::json!({}),
    );
    assert_eq!(decision(&edit(&dir)).0.to_string(), "deny");
}

#[test]
fn a_decline_free_text_or_idle_timeout_approves_nothing() {
    for (name, said, extra) in [
        ("decline", "Decline", serde_json::json!({})),
        ("free", "Approve", serde_json::json!({})),
        ("timeout", "", serde_json::json!({"afkTimeoutMs": 60000})),
    ] {
        let dir = repo(&format!("q-{name}"), &config(ASK));
        let input = refused_with_question(&dir);
        ask(&dir, &input, "toolu_q");
        let said = if name == "timeout" {
            approve_label(&input)
        } else {
            said.to_string()
        };
        answer(&dir, &input, "toolu_q", &said, extra);
        assert_eq!(decision(&edit(&dir)).0.to_string(), "deny", "{name}");
    }
}

#[test]
fn an_answer_to_a_question_urzuas_hook_did_not_see_first_records_nothing() {
    let dir = repo("q-unseen", &config(ASK));
    let input = refused_with_question(&dir);
    answer(
        &dir,
        &input,
        "toolu_q",
        &approve_label(&input),
        serde_json::json!({}),
    );
    assert_eq!(decision(&edit(&dir)).0.to_string(), "deny");
}

#[test]
fn an_approval_does_not_cover_a_different_proposed_content_or_another_record() {
    let dir = repo("q-body", &config(ASK));
    let input = refused_with_question(&dir);
    ask(&dir, &input, "toolu_q");
    answer(
        &dir,
        &input,
        "toolu_q",
        &approve_label(&input),
        serde_json::json!({}),
    );
    let other = accepted(PROPOSED).replace("prose", "prose changed after the question");
    let p = payload(
        &dir,
        "Write",
        BYPASS,
        serde_json::json!({"tool_use_id": "toolu_other"}),
    );
    let mut p2 = p.clone();
    p2["tool_input"]["content"] = serde_json::json!(other);
    assert_eq!(
        decision(&hook(&dir, "edit", &p2, "Alice")).0.to_string(),
        "deny"
    );
    let mut p3 = p;
    p3["tool_use_id"] = serde_json::json!("toolu_other_2");
    p3["tool_input"]["file_path"] = serde_json::json!(dir.join("docs/adr/ADR-2-b.md"));
    p3["tool_input"]["content"] =
        serde_json::json!(accepted(PROPOSED).replace("ADR-1: A title", "ADR-2: Another"));
    assert_eq!(
        decision(&hook(&dir, "edit", &p3, "Alice")).0.to_string(),
        "deny"
    );
}

#[test]
fn without_the_mode_no_question_is_offered() {
    let dir = repo("q-off", &config(""));
    let v = edit(&dir);
    let (d, r) = decision(&v);
    assert_eq!(d, "deny");
    assert!(!r.contains("AskUserQuestion"), "{r}");
}

#[test]
fn the_answer_is_read_from_the_tools_result_never_its_input() {
    let dir = repo("q-result", &config(ASK));
    let input = refused_with_question(&dir);
    ask(&dir, &input, "toolu_q");
    let mut response = input.clone();
    response["answers"] = serde_json::json!({ text(&input): "Decline" });
    let mut claimed = input.clone();
    claimed["answers"] = serde_json::json!({ text(&input): approve_label(&input) });
    let p = serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_q", "cwd": &*dir, "permission_mode": "bypassPermissions",
        "hook_event_name": "PostToolUse", "tool_name": "AskUserQuestion",
        "tool_input": claimed, "tool_response": response,
    });
    hook(&dir, "answer", &p, "Alice");
    assert_eq!(decision(&edit(&dir)).0.to_string(), "deny");
}
