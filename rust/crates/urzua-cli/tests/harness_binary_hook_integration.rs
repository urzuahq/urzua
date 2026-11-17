//! `MILE-339` and `MILE-280`: the harness runs from the binary. An install writes
//! no executable file and no hash; reformatting what it writes is not drift; a
//! script or pinned entry an older bundle left is named with the steps that
//! remove it; `status` says which registration is in effect; a hook registered
//! twice judges a call once; a session rooted elsewhere is judged by the
//! repository that holds the file. Real git repositories and the real binary.

mod common;

use std::path::Path;
use std::process::Command;

const CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nharness:\n  adapters: [claude-code]\n  instructions_file: AGENTS.md\n  guardrails:\n    acceptance: {enabled: true}\n    protected-paths: {enabled: true}\n    session-start: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".claude\", \".urzua/config.yaml\"], by: maintainers}\n";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-binhook-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("AGENTS.md"), "# Agents\n").unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua().args(args).current_dir(dir).output().unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.code().unwrap(), v)
}

fn installed(name: &str) -> common::TempDir {
    let dir = repo(name, CONFIG);
    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    dir
}

fn files_under(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files_under(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn install_writes_no_executable_file_and_no_hash_and_names_what_it_skipped() {
    let dir = repo("plain", CONFIG);
    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    let mut written = Vec::new();
    files_under(&dir.join(".claude"), &mut written);
    for f in &written {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(["md", "json"].contains(&ext), "{}", f.display());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(f).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0, "{} is executable", f.display());
        }
    }
    let settings = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    assert!(!settings.contains("sha256") && !settings.contains("python3"), "{settings}");
    for item in v["items"].as_array().unwrap() {
        assert!(!item["path"].as_str().unwrap().is_empty(), "an item with no path: {v}");
    }
    let notices = v["notices"].to_string();
    assert!(
        notices.contains("skipped by the configuration") && notices.contains("editor-hook") && notices.contains("harness.guardrails.<name>.enabled"),
        "{v}"
    );
    let bare = repo("no-instructions", &CONFIG.replace("  instructions_file: AGENTS.md\n", ""));
    let (_, v) = urzua(&bare, &["harness", "install"]);
    assert!(v["notices"].to_string().contains("set it to the file the agent reads"), "{v}");
}

/// Every file an install writes, reformatted the way a common formatter for its
/// type would: JSON re-indented with four spaces, markdown with its list markers
/// changed, its numbered items renumbered and its lines reflowed with trailing
/// spaces.
fn reformat_everything(dir: &Path) {
    for rel in [".claude/settings.json", ".urzua/harness.lock"] {
        let text = std::fs::read_to_string(dir.join(rel)).unwrap();
        let reindented: String = text
            .lines()
            .map(|l| {
                let lead = l.len() - l.trim_start().len();
                format!("{}{}\n", " ".repeat(lead * 2), l.trim_start())
            })
            .collect();
        std::fs::write(dir.join(rel), reindented).unwrap();
    }
    let mut md = Vec::new();
    files_under(&dir.join(".claude/commands"), &mut md);
    md.push(dir.join("AGENTS.md"));
    for f in md {
        let text = std::fs::read_to_string(&f).unwrap();
        let mut out = String::new();
        for line in text.lines() {
            let lead = &line[..line.len() - line.trim_start().len()];
            let body = line.trim_start();
            let body = if let Some(rest) = body.strip_prefix("- ") {
                format!("* {rest}")
            } else if let Some((n, rest)) = body.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
                let _ = n;
                format!("1)  {rest}")
            } else {
                body.to_string()
            };
            let line = format!("{lead}{body}");
            // Reflow: a long line is broken at its first space after column 40.
            match line.char_indices().find(|(i, c)| *i > 40 && *c == ' ') {
                Some((at, _)) => {
                    out.push_str(&line[..at]);
                    out.push_str("  \n   ");
                    out.push_str(&line[at + 1..]);
                }
                None => out.push_str(&line),
            }
            out.push_str("   \n");
        }
        std::fs::write(&f, out).unwrap();
    }
}

#[test]
fn reformatting_every_file_the_install_wrote_leaves_status_ok_and_the_hook_working() {
    let dir = installed("reformat");
    reformat_everything(&dir);
    let (code, v) = urzua(&dir, &["harness", "status"]);
    assert_eq!((code, v["status"].as_str()), (0, Some("ok")), "{v}");
    // A changed word is drift.
    let done = dir.join(".claude/commands/done.md");
    let text = std::fs::read_to_string(&done).unwrap();
    std::fs::write(&done, text.replacen("engine", "agent", 1)).unwrap();
    let (code, v) = urzua(&dir, &["harness", "status"]);
    assert_eq!(code, 1, "{v}");
    assert!(v["items"].to_string().contains("modified"), "{v}");
    std::fs::write(&done, text).unwrap();
    // The reformatted settings still run the hook, which still refuses.
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap()).unwrap();
    let cmd = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"].as_str().unwrap();
    let payload = serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "tool_input": {"file_path": dir.join(".claude/x.json").to_string_lossy(), "content": "{}"}}).to_string();
    let out = run_sh(&dir, cmd, &payload, Some(dir.path()));
    assert!(out.contains("\"permissionDecision\": \"deny\""), "{out}");
    // The session-start check, the one that compares the files with the lock, agrees.
    let start = serde_json::json!({"hook_event_name": "SessionStart", "cwd": dir.to_string_lossy()}).to_string();
    let cmd = settings["hooks"]["SessionStart"][0]["hooks"][0]["command"].as_str().unwrap();
    let out = run_sh(&dir, cmd, &start, Some(dir.path()));
    assert!(!out.contains("[file]") && !out.contains("[hook-entry]") && !out.contains("[lock]"), "{out}");
}

fn run_sh(dir: &Path, cmd: &str, payload: &str, project: Option<&Path>) -> String {
    use std::io::Write;
    let mut c = Command::new("sh");
    c.args(["-c", cmd])
        .current_dir(dir)
        .env("PATH", common::path_with_urzua())
        .env_remove("CLAUDE_PROJECT_DIR")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if let Some(p) = project {
        c.env("CLAUDE_PROJECT_DIR", p);
    }
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// The settings entries and files an older bundle left, as the bundle of 0.4.7 wrote them.
fn leave_the_old_bundle(dir: &Path) {
    std::fs::create_dir_all(dir.join(".claude/hooks")).unwrap();
    std::fs::write(dir.join(".claude/hooks/urzua-claude-code.py"), "import sys\n").unwrap();
    std::fs::write(dir.join(".claude/hooks/urzua-record.sh"), "#!/bin/sh\n").unwrap();
    let settings = serde_json::json!({"hooks": {"PreToolUse": [{"matcher": "Write|Edit|MultiEdit|NotebookEdit|mcp__.*", "hooks": [{"type": "command", "command": "t=\"$r/.claude/hooks/urzua-claude-code.py\"; python3 -I -c 'import hashlib' \"$t\" 0123 edit -- urzua harness run edit"}]}]}});
    std::fs::write(dir.join(".claude/settings.json"), serde_json::to_string_pretty(&settings).unwrap()).unwrap();
}

#[test]
fn status_doctor_and_install_name_a_leftover_script_and_pinned_entry_with_the_removal_steps() {
    let dir = repo("leftover", CONFIG);
    leave_the_old_bundle(&dir);
    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 2, "the old entry has the key of the new one: {v}");
    let text = v.to_string();
    assert!(text.contains("git rm .claude/hooks/urzua-claude-code.py") && text.contains("git rm .claude/hooks/urzua-record.sh"), "{v}");
    assert!(text.contains("Remove that entry from .claude/settings.json by hand"), "{v}");
    let (_, v) = urzua(&dir, &["doctor"]);
    let leftovers: Vec<&serde_json::Value> = v["checks"].as_array().map(|c| c.iter().filter(|c| c.to_string().contains("harness-leftover")).collect()).unwrap_or_default();
    assert_eq!(leftovers.len(), 3, "{v}");
    // The manual steps, done: the install then succeeds and status is ok.
    std::fs::remove_dir_all(dir.join(".claude/hooks")).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    let (code, v) = urzua(&dir, &["harness", "status"]);
    assert_eq!(code, 0, "{v}");
    assert!(!v.to_string().contains("older bundle"), "{v}");
}

#[test]
fn status_says_which_registration_is_in_effect_and_never_claims_a_plugin_it_cannot_see() {
    let dir = installed("registration");
    let home = common::TempDir::new("urzua-binhook-home");
    let status = |dir: &Path| {
        let out = common::urzua().args(["harness", "status"]).current_dir(dir).env("HOME", home.path()).output().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        v["registration"][0].clone()
    };
    assert_eq!(status(&dir)["in_effect"], "repository-settings");
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    std::fs::write(home.join(".claude/settings.json"), r#"{"enabledPlugins": {"urzua@urzua": true}}"#).unwrap();
    let r = status(&dir);
    assert_eq!(r["in_effect"], "both", "{r}");
    assert_eq!(r["plugin_enabled_in"], "~/.claude/settings.json");
    // A project setting that disables it outranks the user's.
    let settings = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    std::fs::write(dir.join(".claude/settings.local.json"), r#"{"enabledPlugins": {"urzua@urzua": false}}"#).unwrap();
    assert_eq!(status(&dir)["in_effect"], "repository-settings");
    std::fs::remove_file(dir.join(".claude/settings.local.json")).unwrap();
    std::fs::write(dir.join(".claude/settings.json"), "{}\n").unwrap();
    let r = status(&dir);
    assert_eq!(r["in_effect"], "plugin", "{r}");
    assert!(r["note"].as_str().unwrap().contains("not something the binary can read"), "{r}");
    std::fs::write(dir.join(".claude/settings.json"), settings).unwrap();
}

#[test]
fn a_hook_registered_twice_judges_a_call_once_and_both_print_the_same_answer() {
    let dir = installed("twice");
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap()).unwrap();
    let cmd = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"].as_str().unwrap().to_string();
    // A stand-in engine (a debug build's seam) that logs every request it is
    // sent and refuses: how many times the call was judged is the log's length.
    let log = dir.join("engine.log");
    let engine = serde_json::json!(["sh", "-c", "cat >> \"$0\"; echo >> \"$0\"; sleep 1; echo '{\"verdict\":\"block\",\"message\":\"refused once\"}'", log.to_string_lossy()]).to_string();
    let payload = serde_json::json!({"tool_name": "Write", "cwd": dir.to_string_lossy(), "session_id": "s1", "tool_use_id": "toolu_1", "tool_input": {"file_path": dir.join("notes.txt").to_string_lossy(), "content": "x"}}).to_string();
    let run = |payload: String| {
        let (cmd, dir, engine) = (cmd.clone(), dir.to_path_buf(), engine.clone());
        std::thread::spawn(move || {
            use std::io::Write;
            let mut c = Command::new("sh");
            c.args(["-c", &cmd])
                .current_dir(&dir)
                .env("PATH", common::path_with_urzua())
                .env("CLAUDE_PROJECT_DIR", &dir)
                .env("URZUA_HOOK_ENGINE_FOR_TESTS", engine)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped());
            let mut child = c.spawn().unwrap();
            let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
            String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
        })
    };
    let judged = || {
        std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .filter(|l| l.contains("\"event\""))
            .count()
    };
    let (a, b) = (run(payload.clone()), run(payload.clone()));
    let (a, b) = (a.join().unwrap(), b.join().unwrap());
    assert!(a.contains("refused once"), "{a}");
    assert_eq!(a, b, "the two registrations print the same answer");
    assert_eq!(judged(), 1, "the call was judged once");
    // Another call (another id) is judged on its own.
    let c = run(payload.replace("toolu_1", "toolu_2")).join().unwrap();
    assert_eq!(c, a);
    assert_eq!(judged(), 2);
    // A payload with no call id is always judged.
    let bare = payload.replace(",\"tool_use_id\":\"toolu_1\"", "");
    assert_ne!(bare, payload);
    run(bare.clone()).join().unwrap();
    run(bare).join().unwrap();
    assert_eq!(judged(), 4);
}

#[test]
fn a_session_rooted_elsewhere_is_judged_by_the_repository_that_holds_the_file() {
    let dir = installed("elsewhere");
    let elsewhere = common::TempDir::new("urzua-binhook-session-elsewhere");
    let payload = serde_json::json!({"tool_name": "Write", "cwd": elsewhere.to_string_lossy(), "tool_input": {"file_path": dir.join(".claude/settings.json").to_string_lossy(), "content": "{}"}}).to_string();
    // The session's project holds no configuration (a user-level plugin in
    // another directory), and the process runs there too.
    let out = common::run_hook(&elsewhere, "edit", &payload, None, Some(elsewhere.path()), &[]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\"permissionDecision\": \"deny\""), "{text}");
    let accepted = PROPOSED.replace("Proposed", "Accepted");
    let payload = serde_json::json!({"tool_name": "Write", "cwd": elsewhere.to_string_lossy(), "tool_input": {"file_path": dir.join("docs/adr/ADR-1-a.md").to_string_lossy(), "content": accepted}}).to_string();
    let out = common::run_hook(&elsewhere, "edit", &payload, None, None, &[]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("acceptance"), "{out:?}");
}
