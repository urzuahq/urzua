//! `acceptance.in_session: ask` (`ADR-102`, amendment of 2026-11-17): an
//! agent's edit that makes a gated move is put to the person through the host's
//! permission prompt, with its receipt, instead of being refused; it stays
//! refused where the person cannot be asked; nothing is stored; and the local
//! check reads the receipt row only where `require_evidence` is on, while CI's
//! actors file is what CI judges. Real git repositories and the real binary.

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

/// Runs the hook with no `gh` answering, so the name comes from `git config`.
fn hook_no_gh(dir: &Path, event: &str, payload: &serde_json::Value) -> serde_json::Value {
    run_hook(common::urzua(), dir, event, payload)
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

#[test]
fn without_the_mode_the_gated_edit_is_refused_exactly_as_before() {
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
        !r.contains("in_session") && r.contains("outside the agent harness"),
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
    let today = common::date_days_ago(0)[..10].to_string();
    assert!(
        content.contains(&format!("Accepted-on: {today}")),
        "{content}"
    );
    assert!(
        content.contains(&format!("| {} | Recorded as approved in an agent session, not verified: ADR-1 Status moved from 'Proposed' to 'Accepted' (gate: type adr to Accepted, Rejected, by 'maintainers'); by Alice, as the host session reported them (name read from gh); the pull request check does not rely on it. |", today)),
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
    assert!(!dir.join(".git/urzua/approvals").exists());
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
        content.contains("by Alice, as the host session reported them (name read from git config)"),
        "{content}"
    );
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

/// The ask's content written by the host, as on approval.
fn approve(dir: &Path) -> String {
    let v = hook(
        dir,
        "edit",
        &payload(dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    assert_eq!(decision(&v).0, "ask", "{v}");
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &content).unwrap();
    content
}

fn strict(mode: &str) -> String {
    config(mode).replace(
        "diff.decider-required: error",
        "diff.decider-required: {level: error, require_evidence: true}",
    )
}

fn accept_by_hand(dir: &Path) {
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        accepted(PROPOSED).replace("---\n#", "Accepted-on: 2026-11-17\n---\n#"),
    )
    .unwrap();
}

fn decider_findings(out: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(out).unwrap();
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.decider-required")
        .map(|f| format!("{} {}", f["severity"], f["message"]))
        .collect()
}

#[test]
fn asking_stores_nothing_and_needs_no_other_guardrail() {
    let dir = repo(
        "no-store",
        &config(ASK).replace("    post-command: {enabled: true}\n", ""),
    );
    approve(&dir);
    assert!(!dir.join(".git/urzua/approvals").exists());
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn under_require_evidence_the_receipt_row_is_info_and_says_it_is_not_verified() {
    let dir = repo("strict-row", &strict(ASK));
    approve(&dir);
    let (code, out) = check_base(&dir, &[]);
    assert_eq!(code, 0, "{out}");
    let found = decider_findings(&out);
    assert!(
        found.len() == 1 && found[0].starts_with("\"info\""),
        "{found:?}"
    );
    assert!(found[0].contains("not verified"), "{found:?}");
}

#[test]
fn under_require_evidence_a_move_with_no_row_is_the_error_it_was() {
    let on = repo("strict-hand-on", &strict(ASK));
    let off = repo("strict-hand-off", &strict(""));
    accept_by_hand(&on);
    accept_by_hand(&off);
    let (a, out_a) = check_base(&on, &[]);
    let (b, out_b) = check_base(&off, &[]);
    assert_eq!((a, b), (1, 1), "{out_a}");
    assert_eq!(decider_findings(&out_a), decider_findings(&out_b));
}

#[test]
fn without_require_evidence_nothing_changes_with_or_without_the_row() {
    for (name, write) in [("row", true), ("hand", false)] {
        let on = repo(&format!("loose-{name}-on"), &config(ASK));
        let off = repo(&format!("loose-{name}-off"), &config(""));
        if write {
            let content = approve(&on);
            std::fs::write(off.join("docs/adr/ADR-1-a.md"), content).unwrap();
        } else {
            accept_by_hand(&on);
            accept_by_hand(&off);
        }
        let (a, out_a) = check_base(&on, &[]);
        let (b, out_b) = check_base(&off, &[]);
        assert_eq!((a, b), (0, 0), "{out_a}");
        assert_eq!(decider_findings(&out_a), decider_findings(&out_b), "{name}");
    }
}

#[test]
fn ci_judges_the_actors_file_and_the_row_does_not_stand_in_for_it() {
    let dir = repo("ci", &strict(ASK));
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
        out.contains("none is in the decider group 'maintainers'") && !out.contains("not verified"),
        "{out}"
    );
}

#[test]
fn a_name_with_an_invisible_character_is_shown_cleaned_and_said_not_to_match() {
    let dir = repo("zero-width", &config(ASK));
    git(&dir, &["config", "user.name", "Ali\u{200B}ce"]);
    let v = hook_no_gh(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(
        r.contains("'Alice'")
            && r.contains("invisible or control characters removed")
            && !r.contains('\u{200B}'),
        "{r}"
    );
}

fn write_payload(dir: &Path, file: &Path) -> serde_json::Value {
    serde_json::json!({
        "session_id": "s-1", "tool_use_id": "toolu_g", "cwd": dir, "permission_mode": "default",
        "hook_event_name": "PreToolUse", "tool_name": "Write",
        "tool_input": {"file_path": file, "content": "x\n"},
    })
}

#[test]
fn edit_tools_cannot_write_a_git_directory_kept_outside_the_working_tree() {
    // A separate git directory.
    let dir = repo("sep-src", &config(ASK));
    let gitdir = common::TempDir::new("urzua-ask-sepgit");
    std::fs::remove_dir_all(gitdir.path()).unwrap();
    let out = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["init", "-q", "--separate-git-dir"])
        .arg(gitdir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    for rel in ["hooks/pre-commit", "commondir", "config"] {
        let v = hook(
            &dir,
            "edit",
            &write_payload(&dir, &gitdir.join(rel)),
            "Alice",
        );
        assert_eq!(decision(&v).0, "deny", "separate {rel}: {v}");
    }
    // A symlinked `.git`.
    let dir = repo("symlink", &config(ASK));
    let real = common::TempDir::new("urzua-ask-realgit");
    std::fs::remove_dir_all(real.path()).unwrap();
    std::fs::rename(dir.join(".git"), real.path()).unwrap();
    std::os::unix::fs::symlink(real.path(), dir.join(".git")).unwrap();
    for target in [
        real.join("hooks/pre-commit"),
        dir.join(".git/hooks/pre-commit"),
    ] {
        let v = hook(&dir, "edit", &write_payload(&dir, &target), "Alice");
        assert_eq!(decision(&v).0, "deny", "symlink {}: {v}", target.display());
    }
}

#[test]
fn edit_tools_cannot_write_a_bare_repositorys_or_a_submodules_git_directory() {
    // A worktree of a bare repository: its git directory is the bare one's.
    let bare = common::TempDir::new("urzua-ask-bare");
    let src = repo("bare-src", &config(ASK));
    std::fs::remove_dir_all(bare.path()).unwrap();
    git(
        &src,
        &["clone", "-q", "--bare", ".", bare.path().to_str().unwrap()],
    );
    let wt = bare.join("wt");
    let out = Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["worktree", "add", "-q"])
        .arg(&wt)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    for rel in ["hooks/pre-commit", "config", "worktrees/wt/commondir"] {
        let v = hook(&wt, "edit", &write_payload(&wt, &bare.join(rel)), "Alice");
        assert_eq!(decision(&v).0, "deny", "bare {rel}: {v}");
    }
    // A submodule: its git directory sits under the superproject's `.git/modules`.
    let sup = repo("super", &config(ASK));
    let sub = repo("sub", &config(ASK));
    let out = Command::new("git")
        .arg("-C")
        .arg(sup.path())
        .args(["-c", "protocol.file.allow=always", "submodule", "add", "-q"])
        .arg(sub.path())
        .arg("m")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    for rel in [".git/modules/m/hooks/pre-commit", "m/.git"] {
        let v = hook(&sup, "edit", &write_payload(&sup, &sup.join(rel)), "Alice");
        assert_eq!(decision(&v).0, "deny", "submodule {rel}: {v}");
    }
}

/// The row the hook would write for ADR-1, with `edit` applied to the line.
fn with_row(dir: &Path, edit: impl Fn(&str) -> String) -> String {
    let v = hook(
        dir,
        "edit",
        &payload(dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let content = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap()
        .to_string();
    content
        .lines()
        .map(|l| {
            if l.contains("Recorded as approved") {
                edit(l)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn strict_exit(dir: &Path, text: &str) -> (i32, String) {
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), text).unwrap();
    check_base(dir, &[])
}

#[test]
fn a_row_for_another_status_left_does_not_match() {
    let dir = repo("row-from", &strict(ASK));
    let t = with_row(&dir, |l| l.replace("from 'Proposed'", "from 'Rejected'"));
    assert_eq!(strict_exit(&dir, &t).0, 1);
}

#[test]
fn a_row_dated_other_than_its_change_does_not_match() {
    let dir = repo("row-1999", &strict(ASK));
    let today = common::date_days_ago(0)[..10].to_string();
    let t = with_row(&dir, |l| l.replace(&today, "1999-01-01"));
    assert_eq!(strict_exit(&dir, &t).0, 1);
}

#[test]
fn a_row_naming_another_record_does_not_match() {
    let dir = repo("row-id", &strict(ASK));
    let t = with_row(&dir, |l| l.replace("ADR-1 Status", "ADR-2 Status"));
    assert_eq!(strict_exit(&dir, &t).0, 1);
}

#[test]
fn a_row_outside_the_revision_log_does_not_match() {
    for (name, place) in [("prose", "plain"), ("fence", "fence"), ("front", "front")] {
        let dir = repo(&format!("row-{name}"), &strict(ASK));
        let t = with_row(&dir, |l| l.to_string());
        let row = t
            .lines()
            .find(|l| l.contains("Recorded as approved"))
            .unwrap()
            .to_string();
        let without: String = t
            .lines()
            .filter(|l| !l.contains("Recorded as approved"))
            .map(|l| format!("{l}\n"))
            .collect();
        let text = match place {
            "plain" => without.replace("prose\n", &format!("prose\n\n{row}\n")),
            "fence" => without.replace("prose\n", &format!("prose\n\n```\n{row}\n```\n")),
            _ => without.replacen(
                "Status: Accepted\n",
                &format!("Status: Accepted\n# {row}\n"),
                1,
            ),
        };
        let (code, out) = strict_exit(&dir, &text);
        assert_eq!(code, 1, "{name}: {out}");
    }
}

#[test]
fn an_old_row_left_from_an_earlier_move_does_not_match_a_new_one() {
    let dir = repo("row-old", &strict(ASK));
    let accepted_with_row = with_row(&dir, |l| l.to_string());
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &accepted_with_row).unwrap();
    git(&dir, &["commit", "-q", "-am", "accepted"]);
    let back = accepted_with_row
        .replace("Status: Accepted", "Status: Proposed")
        .replace(
            &format!("Accepted-on: {}\n", &common::date_days_ago(0)[..10]),
            "",
        );
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &back).unwrap();
    git(&dir, &["commit", "-q", "-am", "back to proposed"]);
    let (code, out) = strict_exit(&dir, &accepted_with_row);
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_unplaceable_edit_inside_a_git_directory_is_refused_on_every_shape() {
    let dir = repo("sep-unplaced", &config(ASK));
    let gitdir = common::TempDir::new("urzua-ask-sepgit2");
    std::fs::remove_dir_all(gitdir.path()).unwrap();
    let out = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["init", "-q", "--separate-git-dir"])
        .arg(gitdir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let edit = |root: &Path, file: &Path| {
        serde_json::json!({
            "session_id": "s-1", "tool_use_id": "toolu_u", "cwd": root, "permission_mode": "default",
            "hook_event_name": "PreToolUse", "tool_name": "Edit",
            "tool_input": {"file_path": file, "old_string": "no such text", "new_string": "x"},
        })
    };
    let v = hook(&dir, "edit", &edit(&dir, &gitdir.join("config")), "Alice");
    assert_eq!(decision(&v).0, "deny", "separate: {v}");
    let bare = common::TempDir::new("urzua-ask-bare2");
    let src = repo("bare-src2", &config(ASK));
    std::fs::remove_dir_all(bare.path()).unwrap();
    git(
        &src,
        &["clone", "-q", "--bare", ".", bare.path().to_str().unwrap()],
    );
    let wt = bare.join("wt");
    let out = Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["worktree", "add", "-q"])
        .arg(&wt)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v = hook(&wt, "edit", &edit(&wt, &bare.join("config")), "Alice");
    assert_eq!(decision(&v).0, "deny", "bare: {v}");
    let sup = repo("super2", &config(ASK));
    let sub = repo("sub2", &config(ASK));
    let out = Command::new("git")
        .arg("-C")
        .arg(sup.path())
        .args(["-c", "protocol.file.allow=always", "submodule", "add", "-q"])
        .arg(sub.path())
        .arg("m")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v = hook(
        &sup,
        "edit",
        &edit(&sup, &sup.join(".git/modules/m/config")),
        "Alice",
    );
    assert_eq!(decision(&v).0, "deny", "submodule: {v}");
}

#[test]
fn a_path_spelled_in_another_case_is_refused_not_asked() {
    let dir = repo("case", &config(ASK));
    for rel in ["docs/ADR/ADR-1-a.md", "Docs/adr/ADR-1-a.md"] {
        let mut p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
        p["tool_input"]["file_path"] = serde_json::json!(dir.join(rel));
        let (d, r) = {
            let v = hook(&dir, "edit", &p, "Alice");
            let (d, r) = decision(&v);
            (d.to_string(), r.to_string())
        };
        assert_ne!(d, "ask", "{rel}: {r}");
    }
}

#[test]
fn a_move_two_gates_reserve_is_refused_and_sent_to_a_pull_request() {
    let cfg = config(ASK)
        .replace("    maintainers: {logins: [Alice]}\n", "    maintainers: {logins: [Alice]}\n    leads: {logins: [Alice]}\n")
        .replace("    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n", "    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {type: adr, to: [Accepted], by: leads}\n");
    let dir = repo("two-gates", &cfg);
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let (d, r) = decision(&v);
    assert_eq!(d, "deny", "{v}");
    assert!(
        r.contains("More than one decider group") && r.contains("pull request"),
        "{r}"
    );
}

#[test]
fn a_record_filed_straight_at_a_gated_status_is_refused_as_without_the_mode() {
    let on = repo("new-on", &config(ASK));
    let off = repo("new-off", &config(""));
    let reason = |dir: &Path| {
        let mut p = payload(dir, "Write", Some("default"), serde_json::json!({}));
        p["tool_input"]["file_path"] = serde_json::json!(dir.join("docs/adr/ADR-3-c.md"));
        p["tool_input"]["content"] =
            serde_json::json!(accepted(PROPOSED).replace("ADR-1: A title", "ADR-3: New"));
        let v = hook(dir, "edit", &p, "Alice");
        (
            decision(&v).0.to_string(),
            decision(&v).1.replace(dir.to_str().unwrap(), ""),
        )
    };
    let (a, b) = (reason(&on), reason(&off));
    assert_eq!(a.0, "deny");
    assert_eq!(a, b);
}

#[test]
fn a_named_override_means_the_person_is_not_asked() {
    let dir = repo("override", &config(ASK));
    let gh = common::gh_login_dir("Alice");
    let mut cmd = common::urzua_as(&gh);
    cmd.env("URZUA_HOOK_OVERRIDE", "x");
    let mut child = cmd
        .args(["harness", "hook", "claude-code", "edit"])
        .env("CLAUDE_PROJECT_DIR", dir.path())
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            payload(&dir, "Write", Some("default"), serde_json::json!({}))
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(decision(&v).0, "deny", "{v}");
}

#[test]
fn the_reason_says_a_row_is_written_and_names_the_date_field() {
    let dir = repo("reason-row", &config(ASK));
    let v = hook(
        &dir,
        "edit",
        &payload(&dir, "Write", Some("default"), serde_json::json!({})),
        "Alice",
    );
    let r = decision(&v).1;
    assert!(r.contains("Approving also writes a row recording this as approved in an agent session, unverified, and fills Accepted-on with today's date."), "{r}");
}

#[test]
fn the_title_is_the_one_the_title_rules_read() {
    let dir = repo("title-rule", &config(ASK));
    let mut p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    let text = accepted(PROPOSED).replace(
        "---\n# ADR-1: A title",
        "# Fake front\n---\n```\n# Fake fenced\n```\n# ADR-1: A title",
    );
    p["tool_input"]["content"] = serde_json::json!(text.replacen("---\nStatus", "---\nStatus", 1));
    let v = hook(&dir, "edit", &p, "Alice");
    let r = decision(&v).1;
    assert!(r.ends_with("as the agent wrote it: ADR-1: A title"), "{r}");
}

#[test]
fn a_crlf_record_keeps_its_line_endings() {
    let dir = repo("crlf", &config(ASK));
    let crlf = PROPOSED.replace('\n', "\r\n");
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), &crlf).unwrap();
    git(&dir, &["commit", "-q", "-am", "crlf"]);
    let mut p = payload(&dir, "Write", Some("default"), serde_json::json!({}));
    p["tool_input"]["content"] = serde_json::json!(accepted(&crlf));
    let v = hook(&dir, "edit", &p, "Alice");
    let c = v["hookSpecificOutput"]["updatedInput"]["content"]
        .as_str()
        .unwrap();
    assert!(!c.replace("\r\n", "").contains('\n'), "{c:?}");
}
