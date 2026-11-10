//! MILE-202, ADR-78 (amendment) and ADR-89: after any Bash command the harness
//! runs `check` on the records git shows as changed (untracked included) and
//! reports; it never blocks. Real git-repo fixtures, the real binary, real
//! shell commands as the writers. Needs `git`, `python3` and `sh`; a missing
//! tool fails a test, never skips it.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const CONFIG: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules: {embodiment.consistency: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\nharness:\n  adapters: [claude-code]\n  instructions_file: AGENTS.md\n  guardrails:\n    editor-hook: {enabled: true}\n    post-command: {enabled: true}\n";
const GOOD: &str =
    "---\nEmbodiment: Implemented\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";
const BAD: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A clean git repo: a config with the guardrail on, one clean record, a
/// source file, an instructions file; everything committed.
fn fixture(name: &str, config: &str, record: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-postcmd-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH_1), record).unwrap();
    std::fs::write(dir.join("AGENTS.md"), "# Agents\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// Runs a shell command the way the agent's Bash tool does, in the repository.
fn bash(dir: &Path, script: &str) {
    let status = Command::new("sh")
        .args(["-c", script])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "{script}");
}

fn run_with_stdin(c: &mut Command, stdin: &str) -> Output {
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn request(dir: &Path) -> String {
    serde_json::json!({
        "event": "post-command", "tool": "Bash", "paths": [], "actor": "agent",
        "cwd": dir.to_string_lossy(),
    })
    .to_string()
}

/// `urzua harness run post-command`; the exit code is asserted 0 here, since
/// the check reports and never blocks.
fn post(dir: &Path) -> serde_json::Value {
    post_with(dir, None)
}

fn post_with(dir: &Path, path: Option<&Path>) -> serde_json::Value {
    let mut c = common::urzua();
    c.args(["harness", "run", "post-command"]).current_dir(dir);
    if let Some(p) = path {
        c.env("PATH", p);
    }
    let out = run_with_stdin(&mut c, &request(dir));
    assert_eq!(out.status.code(), Some(0), "never blocks: {out:?}");
    json(&out)
}

fn msg(v: &serde_json::Value) -> &str {
    v["message"].as_str().unwrap()
}

fn checked(v: &serde_json::Value) -> Vec<String> {
    v["checked"]
        .as_array()
        .unwrap_or_else(|| panic!("no checked list: {v}"))
        .iter()
        .map(|p| p.as_str().unwrap().to_string())
        .collect()
}

fn assert_reports_the_bad_record(v: &serde_json::Value, path: &str) {
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{v}");
    assert_eq!(findings[0]["rule"], "embodiment.consistency", "{v}");
    assert!(findings[0]["file"].as_str().unwrap().ends_with(path), "{v}");
    let m = msg(v);
    assert!(
        m.contains(path) && m.contains("[embodiment.consistency]") && m.contains("'Implemented'"),
        "{m}"
    );
    assert!(
        m.contains("error") && m.contains("was not prevented") && m.contains("pre-commit"),
        "the report says the command already ran and the commit hook and CI stay the gate: {m}"
    );
    assert_eq!(checked(v), [path], "{v}");
}

// ------------------------------------------------- a gated move by shell

/// MILE-200, ADR-89 section 4: a Bash command that moves a record to a status
/// the `authority` table gates is reported, as an error of its own, and never
/// blocked. `check` has no rule to run here, so the report does not depend on it.
#[test]
fn a_bash_edit_that_moves_a_record_to_a_gated_status_is_reported_and_not_blocked() {
    let config = "schema_version: 2\nrules: {embodiment.consistency: error}\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nharness:\n  adapters: [claude-code]\n  guardrails:\n    acceptance: {enabled: true}\n    post-command: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n";
    let proposed = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
    let dir = fixture("gated", config, proposed);
    bash(
        &dir,
        &format!("sed -i 's/^Status: Proposed/Status: Accepted/' {PATH_1}"),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let moves = v["gated_moves"].as_array().unwrap();
    assert_eq!(moves.len(), 1, "{v}");
    let line = moves[0].as_str().unwrap();
    assert!(
        line.contains(PATH_1)
            && line.contains("'Proposed'")
            && line.contains("'Accepted'")
            && line.contains("[acceptance] error"),
        "{line}"
    );
    assert!(
        msg(&v).contains("1 gated move(s)") && msg(&v).contains("was not prevented"),
        "{v}"
    );
}

const GATED_CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nharness:\n  adapters: [claude-code]\n  guardrails:\n    acceptance: {enabled: true}\n    post-command: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n";

/// ADR-89 amendment of 2026-11-10: removing a record that held a gated status
/// is reported as a gated move, and a rename (same Stable-Id and record id) is
/// not.
#[test]
fn a_bash_rm_of_a_gated_record_is_reported_and_a_rename_is_not() {
    let sid = "01M6R3NK40JBCPESD89J6FN865";
    let accepted = format!("---\nStable-Id: {sid}\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n");
    let dir = fixture("gated-rm", GATED_CONFIG, &accepted);
    bash(&dir, &format!("git mv {PATH_1} docs/adr/ADR-1-b.md"));
    let v = post(&dir);
    assert!(
        !v["gated_moves"].to_string().contains("deleted"),
        "a rename is not a deletion: {v}"
    );
    bash(&dir, "git reset -q --hard");

    bash(&dir, &format!("rm {PATH_1}"));
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let moves = v["gated_moves"].as_array().unwrap();
    assert_eq!(moves.len(), 1, "{v}");
    let line = moves[0].as_str().unwrap();
    assert!(
        line.contains(PATH_1) && line.contains("deleted") && line.contains("'Accepted'"),
        "{line}"
    );
}

// ----------------------------------------------------- the three writers

#[test]
fn a_sed_in_place_edit_of_a_record_is_reported_with_file_rule_and_message() {
    let dir = fixture("sed", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    assert_reports_the_bad_record(&post(&dir), PATH_1);
}

#[test]
fn a_heredoc_that_creates_an_untracked_record_is_reported_and_staging_is_untouched() {
    let dir = fixture("heredoc", CONFIG, GOOD);
    let new = "docs/adr/ADR-3-c.md";
    bash(
        &dir,
        &format!(
            "cat > {new} <<'EOF'\n---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-3: V\n\nprose\nEOF"
        ),
    );
    let (index_before, status_before) = (
        git(&dir, &["ls-files", "-s"]),
        git(&dir, &["status", "--porcelain"]),
    );
    assert!(
        status_before.contains("?? docs/adr/ADR-3-c.md"),
        "{status_before}"
    );
    assert_reports_the_bad_record(&post(&dir), new);
    assert_eq!(
        git(&dir, &["ls-files", "-s"]),
        index_before,
        "the index is unchanged"
    );
    assert_eq!(
        git(&dir, &["status", "--porcelain"]),
        status_before,
        "the working tree state is unchanged"
    );
}

#[test]
fn an_interpreter_rewrite_of_a_record_is_reported() {
    let dir = fixture("python", CONFIG, GOOD);
    bash(
        &dir,
        &format!(
            "python3 -c \"import pathlib; p = pathlib.Path('{PATH_1}'); p.write_text(p.read_text().replace('Implemented', 'Not started'))\""
        ),
    );
    assert_reports_the_bad_record(&post(&dir), PATH_1);
}

#[test]
fn several_changed_records_are_checked_in_one_call() {
    let dir = fixture("several", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    bash(
        &dir,
        "printf -- '---\\nEmbodiment: Not started\\nRealized-by: code:src/a.rs\\n---\\n# ADR-3: V\\n' > docs/adr/ADR-3-c.md",
    );
    let v = post(&dir);
    let mut c = checked(&v);
    c.sort();
    assert_eq!(c, [PATH_1, "docs/adr/ADR-3-c.md"], "{v}");
    assert_eq!(v["findings"].as_array().unwrap().len(), 2, "{v}");
}

// ------------------------------------------------- when nothing is judged

#[test]
fn a_command_that_changes_no_governed_record_reports_nothing_and_runs_no_check() {
    let dir = fixture("quiet", CONFIG, GOOD);
    let v = post(&dir);
    assert_eq!(
        (v["verdict"].as_str(), checked(&v).len()),
        (Some("allow"), 0),
        "{v}"
    );
    // A source file and a markdown file outside every record type change.
    bash(&dir, "echo 'fn b() {}' > src/b.rs; echo notes > NOTES.md; mkdir -p scratch && echo x > scratch/x.md");
    let v = post(&dir);
    assert_eq!(
        (v["verdict"].as_str(), checked(&v).len()),
        (Some("allow"), 0),
        "{v}"
    );
    assert!(v["findings"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn a_deleted_record_is_not_judged() {
    let dir = fixture("deleted", CONFIG, BAD);
    bash(&dir, &format!("rm {PATH_1}"));
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(checked(&v).is_empty(), "{v}");
    // Staged deletion too.
    git(&dir, &["rm", "-q", "--cached", PATH_1]);
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(checked(&v).is_empty(), "{v}");
}

#[test]
fn a_record_that_is_clean_after_the_command_is_checked_and_says_nothing() {
    let dir = fixture("clean", CONFIG, BAD);
    bash(
        &dir,
        &format!("sed -i 's/Not started/Implemented/' {PATH_1}"),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
    assert_eq!(checked(&v), [PATH_1], "it was examined: {v}");
    assert!(v["findings"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn the_guardrail_is_opt_in_and_a_repository_that_did_not_name_it_is_left_alone() {
    let off = CONFIG.replace("    post-command: {enabled: true}\n", "");
    let dir = fixture("off", &off, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(checked(&v).is_empty(), "{v}");
    assert!(msg(&v).contains("not enabled"), "{v}");
    let none = CONFIG.split("harness:").next().unwrap().to_string();
    let dir = fixture("nokey", &none, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    assert_eq!(post(&dir)["verdict"], "allow");
}

// ------------------------------------- pre-existing and the content hash

#[test]
fn a_record_already_failing_at_head_is_listed_as_pre_existing() {
    let dir = fixture("preexisting", CONFIG, BAD);
    bash(&dir, &format!("echo 'more prose' >> {PATH_1}"));
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    assert!(
        v["findings"].as_array().unwrap().is_empty(),
        "nothing new: {v}"
    );
    let old = v["pre_existing"].as_array().unwrap();
    assert_eq!(old.len(), 1, "{v}");
    assert_eq!(old[0]["rule"], "embodiment.consistency");
    assert!(msg(&v).contains("Already failing at HEAD"), "{v}");
    assert_eq!(checked(&v), [PATH_1]);
}

#[test]
fn an_unchanged_record_is_not_reported_again_and_a_new_change_is() {
    let dir = fixture("hash", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    assert_reports_the_bad_record(&post(&dir), PATH_1);
    // A second command that does not touch the record: git still shows it
    // changed, but its content hash is the one already reported.
    bash(&dir, "echo 'fn b() {}' > src/b.rs");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(checked(&v).is_empty(), "no check ran: {v}");
    assert!(v["findings"].as_array().unwrap().is_empty(), "{v}");
    // A change to the record is judged and reported afresh.
    bash(&dir, &format!("echo 'more' >> {PATH_1}"));
    assert_reports_the_bad_record(&post(&dir), PATH_1);
    // The memory lives under the ignored cache, so it is never itself a change.
    assert!(!git(&dir, &["status", "--porcelain"]).contains("cache"));
}

#[test]
fn reverting_a_record_forgets_it_so_the_same_bad_edit_is_reported_again() {
    let dir = fixture("revert", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    assert_reports_the_bad_record(&post(&dir), PATH_1);
    git(&dir, &["checkout", "--", PATH_1]);
    assert_eq!(post(&dir)["verdict"], "allow");
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    assert_reports_the_bad_record(&post(&dir), PATH_1);
}

// ------------------------------------------------ what it cannot judge

fn path_with(names: &[&str]) -> common::TempDir {
    let dir = common::TempDir::new("urzua-postcmd-path");
    for name in names {
        let target = which(name);
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
    }
    dir
}

fn which(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} must be installed"))
}

fn assert_not_judged(v: &serde_json::Value, why: &str) {
    assert_eq!(v["verdict"], "allow-unjudged", "{v}");
    let m = msg(v);
    assert!(
        m.contains("NOT judged") && m.contains(why),
        "the report says the command's effect on records was NOT judged ({why}): {m}"
    );
    assert!(
        v["findings"].as_array().unwrap().is_empty(),
        "no success is claimed: {v}"
    );
}

#[test]
fn with_git_absent_the_report_says_the_effect_on_records_was_not_judged() {
    let dir = fixture("nogit", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    let no_git = path_with(&["sh"]);
    let v = post_with(&dir, Some(&no_git));
    assert_not_judged(&v, "git");
}

#[test]
fn outside_a_git_repository_the_report_says_the_effect_on_records_was_not_judged() {
    let dir = fixture("norepo", CONFIG, GOOD);
    std::fs::remove_dir_all(dir.join(".git")).unwrap();
    let v = post(&dir);
    assert_not_judged(&v, "git");
}

#[test]
fn when_check_cannot_run_the_report_says_the_effect_on_records_was_not_judged() {
    let dir = fixture("nocheck", CONFIG, GOOD);
    // A record whose bytes are not text: check cannot read it, so it cannot
    // judge it, and the report must not read as a pass.
    std::fs::write(dir.join(PATH_1), [0xff, 0xfe, 0xfd, b'\n']).unwrap();
    let v = post(&dir);
    assert_not_judged(&v, "check");
    // A broken configuration is the same loud state.
    let dir = fixture("badconfig", CONFIG, GOOD);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    std::fs::write(dir.join(".urzua/config.yaml"), "schema_version: [\n").unwrap();
    let v = post(&dir);
    assert_not_judged(&v, "configuration");
}

#[test]
fn it_never_blocks_whatever_it_finds_or_cannot_judge() {
    // Every `post` above asserts exit 0; this pins the unreadable request.
    let dir = fixture("garbage", CONFIG, GOOD);
    let mut c = common::urzua();
    c.args(["harness", "run", "post-command"]).current_dir(&dir);
    let out = run_with_stdin(&mut c, "not json");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_not_judged(&json(&out), "request");
}

// ------------------------------------------------------ the adapter wiring

fn install(dir: &Path) {
    let out = common::urzua()
        .args(["harness", "install"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

fn hook_command(dir: &Path, matcher: &str) -> String {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap())
            .unwrap();
    let groups = v["hooks"]["PostToolUse"]
        .as_array()
        .expect("a PostToolUse event");
    let group = groups
        .iter()
        .find(|g| g["matcher"] == matcher)
        .unwrap_or_else(|| panic!("no PostToolUse matcher {matcher}: {v}"));
    group["hooks"][0]["command"].as_str().unwrap().to_string()
}

fn payload(dir: &Path, tool: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PostToolUse", "tool_name": tool, "cwd": dir.to_string_lossy(),
        "tool_input": {"command": "true"}, "tool_response": {"stdout": ""},
    })
    .to_string()
}

fn run_hook(dir: &Path, cmd: &str, stdin: &str, path: Option<&Path>) -> Output {
    let mut c = Command::new("sh");
    c.args(["-c", cmd])
        .current_dir(dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    if let Some(p) = path {
        c.env("PATH", p).env_remove("URZUA_BIN");
    }
    run_with_stdin(&mut c, stdin)
}

#[test]
fn the_claude_code_adapter_wires_a_post_tool_use_hook_on_bash_and_relays_the_report() {
    let dir = fixture("adapter", CONFIG, GOOD);
    install(&dir);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "install"]);
    let cmd = hook_command(&dir, "Bash");
    assert!(cmd.contains("urzua harness run post-command"), "{cmd}");

    // A clean tree: the hook says nothing.
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        out.stdout.is_empty(),
        "nothing changed, nothing said: {out:?}"
    );

    // The Bash command rewrites a record; the agent is told, as context, and
    // nothing is denied.
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    assert_eq!(
        v["hookSpecificOutput"]["hookEventName"], "PostToolUse",
        "{v}"
    );
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        ctx.contains(PATH_1) && ctx.contains("embodiment.consistency"),
        "{ctx}"
    );
    assert!(v.get("decision").is_none(), "a report never blocks: {v}");
    assert!(
        v["hookSpecificOutput"].get("permissionDecision").is_none(),
        "{v}"
    );

    // The same record on the next command is not repeated.
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert!(out.stdout.is_empty(), "{out:?}");

    // Another tool is not this hook's business, even with a changed record.
    bash(&dir, &format!("echo more >> {PATH_1}"));
    let out = run_hook(&dir, &cmd, &payload(&dir, "Read"), None);
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn without_the_engine_the_adapter_states_that_the_command_was_not_judged() {
    let dir = fixture("adapter-missing", CONFIG, GOOD);
    install(&dir);
    let cmd = hook_command(&dir, "Bash");
    let only_python = path_with(&["python3", "git", "sh"]);
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), Some(&only_python));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    let m = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        m.contains("NOT judged") || m.contains("did NOT judge"),
        "{m}"
    );
    assert_eq!(
        v["hookSpecificOutput"]["hookEventName"], "PostToolUse",
        "{v}"
    );
}
