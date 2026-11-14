//! A tracked record file that is a symlink resolving outside the repository, or
//! outside its record type's directory, is refused by every corpus command
//! (ADR-89 amendment, 2026-11-14; SPEC-2 `identity.record-symlink-escapes`).
//! The reviewer's two-PR attack is reproduced end to end: the first PR moves a
//! decision record out of the governed tree and leaves a symlink in its place,
//! the second flips the moved copy to `Accepted`, which no gate on the governed
//! path would otherwise see.

mod common;

use std::path::Path;
use std::process::Command;

const RULE: &str = "identity.record-symlink-escapes";
const PROPOSED: &str = "---\nStatus: Proposed\n---\n# ADR-1: T\n\nprose\n";
const ACCEPTED: &str = "---\nStatus: Accepted\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";

/// No `identity.record-symlink-escapes` entry under `rules:`: the refusal is on
/// for every configuration.
const CONFIG: &str = "schema_version: 2\noverride_log: .urzua/override-log.md\nrules:\n  diff.decider-required: error\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    statuses:\n      values: [Proposed, Accepted, Rejected]\nauthority:\n  deciders:\n    maintainers: {logins: [Alice]}\n  gates:\n    - {type: adr, to: [Accepted, Rejected], by: maintainers}\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-symlink-escape-{name}"));
    for d in ["docs/adr", ".urzua", "notes"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join(PATH_1), PROPOSED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let json = serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code().unwrap_or(-1), json, stdout)
}

fn escapes(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|f| f.iter().filter(|f| f["rule"] == RULE).collect())
        .unwrap_or_default()
}

fn commit_all(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn actors(dir: &Path, login: &str) -> String {
    let path = dir.join("actors.json");
    std::fs::write(
        &path,
        serde_json::json!({"schema": 1, "pr_author": login, "approvers": []}).to_string(),
    )
    .unwrap();
    path.to_string_lossy().to_string()
}

/// Moves the record out of the governed tree and leaves a symlink in its place.
fn move_out_and_link(dir: &Path) {
    git(dir, &["mv", PATH_1, "notes/x.md"]);
    std::os::unix::fs::symlink("../../notes/x.md", dir.join(PATH_1)).unwrap();
}

#[test]
fn a_record_symlinked_outside_the_repository_is_a_blocking_finding_naming_the_file() {
    let dir = fixture("outside");
    let secret = common::TempFile::new("urzua-symlink-secret");
    std::fs::write(
        &secret,
        "---\nStatus: Accepted\n---\n# 9: secret contents\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&secret, dir.join("docs/adr/ADR-9-link.md")).unwrap();
    commit_all(&dir, "link");
    let (code, report, stdout) = run(&dir, &["check"]);
    let found = escapes(&report);
    assert_eq!(found.len(), 1, "{stdout}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-9-link.md");
    assert_eq!(found[0]["severity"], "error");
    assert_eq!(code, 1, "{stdout}");
    assert_eq!(report["blocking"], true, "{stdout}");
    assert!(!stdout.contains("secret contents"), "{stdout}");
}

#[test]
fn a_record_symlinked_inside_the_repository_but_outside_its_type_directory_is_refused() {
    let dir = fixture("outside-tree");
    std::fs::write(dir.join("notes/y.md"), PROPOSED).unwrap();
    std::os::unix::fs::symlink("../../notes/y.md", dir.join("docs/adr/ADR-2-b.md")).unwrap();
    commit_all(&dir, "link");
    let (code, report, stdout) = run(&dir, &["check"]);
    let found = escapes(&report);
    assert_eq!(found.len(), 1, "{stdout}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-2-b.md");
    assert_eq!(code, 1, "{stdout}");
}

#[test]
fn a_symlink_to_a_sibling_record_in_the_same_directory_is_refused_too() {
    // Aliases have no use a record needs, and a link to a file no type governs
    // is the same attack (see the same-directory test below).
    let dir = fixture("inside-tree");
    std::os::unix::fs::symlink("ADR-1-a.md", dir.join("docs/adr/ADR-3-alias.md")).unwrap();
    commit_all(&dir, "alias");
    let (code, report, stdout) = run(&dir, &["check"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(escapes(&report)[0]["file"], "docs/adr/ADR-3-alias.md");
    assert_eq!(code, 1, "{stdout}");
}

#[test]
fn a_configuration_may_set_the_level_or_turn_the_refusal_off() {
    let dir = fixture("levels");
    std::fs::write(dir.join("notes/y.md"), PROPOSED).unwrap();
    std::os::unix::fs::symlink("../../notes/y.md", dir.join("docs/adr/ADR-2-b.md")).unwrap();
    commit_all(&dir, "link");
    let with = |rule: &str| {
        let config = CONFIG.replace("rules:\n", &format!("rules:\n  {RULE}: {rule}\n"));
        std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    };
    // A declared `warn` still reports the link and no longer blocks.
    with("warn");
    let (_, report, stdout) = run(&dir, &["check"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(escapes(&report)[0]["severity"], "warning", "{stdout}");
    assert_eq!(report["blocking"], false, "{stdout}");
    // `off` reads the record through the link, as before, and reports nothing: the
    // risk the repository accepts (SPEC-2).
    with("off");
    let (_, report, stdout) = run(&dir, &["check"]);
    assert!(escapes(&report).is_empty(), "{stdout}");
    assert_eq!(report["blocking"], false, "{stdout}");
    let (code, _, stdout) = run(&dir, &["graph"]);
    assert_eq!(code, 0, "{stdout}");
    // A selection does not bring it back once it is off.
    let (_, report, stdout) = run(&dir, &["check", "--rules", "diff.decider-required"]);
    assert!(escapes(&report).is_empty(), "{stdout}");
}

#[test]
fn every_other_corpus_command_refuses_the_escaping_record_naming_it() {
    let dir = fixture("commands");
    let secret = common::TempFile::new("urzua-symlink-secret-cmds");
    std::fs::write(
        &secret,
        "---\nStatus: Accepted\n---\n# 9: secret contents\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&secret, dir.join("docs/adr/ADR-9-link.md")).unwrap();
    commit_all(&dir, "link");
    for args in [
        &["graph"][..],
        &["export", "--format", "json"][..],
        &["fix"][..],
        &["explain", "docs/adr/ADR-1-a.md"][..],
        &["migrate", "ids"][..],
    ] {
        let (code, _, stdout) = run(&dir, args);
        assert_eq!(code, 2, "{args:?}: {stdout}");
        assert!(
            stdout.contains("docs/adr/ADR-9-link.md")
                && stdout.contains("regular file at its own path"),
            "{args:?}: {stdout}"
        );
        assert!(!stdout.contains("secret contents"), "{args:?}: {stdout}");
    }
}

#[test]
fn audit_is_check_and_reports_the_finding_too() {
    let dir = fixture("audit");
    std::fs::write(dir.join("notes/y.md"), PROPOSED).unwrap();
    std::os::unix::fs::symlink("../../notes/y.md", dir.join("docs/adr/ADR-2-b.md")).unwrap();
    commit_all(&dir, "link");
    let (code, report, stdout) = run(&dir, &["audit"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "{stdout}");
}

#[test]
fn the_refusal_runs_under_every_rules_selection_and_is_listed_in_a_narrowed_run() {
    let dir = fixture("selection");
    std::fs::write(dir.join("notes/y.md"), PROPOSED).unwrap();
    std::os::unix::fs::symlink("../../notes/y.md", dir.join("docs/adr/ADR-2-b.md")).unwrap();
    commit_all(&dir, "link");
    let (code, report, stdout) = run(&dir, &["check", "--rules", "diff.decider-required"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "{stdout}");
    let ran: Vec<&str> = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["rule"].as_str())
        .collect();
    assert!(ran.contains(&RULE), "{ran:?}");
    // Naming it needs no declaration either.
    let (_, report, stdout) = run(&dir, &["check", "--rules", RULE]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
}

/// The review's bypass: the target sits in the same directory under a name no
/// type governs, so a target-directory test passed it.
fn alias_step_one(dir: &Path) {
    git(dir, &["mv", PATH_1, "docs/adr/draft.txt"]);
    std::os::unix::fs::symlink("draft.txt", dir.join(PATH_1)).unwrap();
    commit_all(dir, "pr1");
}

#[test]
fn the_same_directory_alias_attack_is_blocked_at_both_steps_on_base_and_whole_runs() {
    let dir = fixture("alias-attack");
    let mallory = actors(&dir, "mallory");

    alias_step_one(&dir);
    let (code, report, stdout) = run(&dir, &["check", "--base", "base", "--actors", &mallory]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(escapes(&report)[0]["file"], PATH_1);
    assert_eq!(code, 1, "PR 1 must block (--base): {stdout}");
    let (code, report, stdout) = run(&dir, &["check"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "PR 1 must block (whole corpus): {stdout}");
    git(&dir, &["tag", "after-pr1"]);

    std::fs::write(dir.join("docs/adr/draft.txt"), ACCEPTED).unwrap();
    commit_all(&dir, "pr2");
    let (code, report, stdout) = run(
        &dir,
        &["check", "--base", "after-pr1", "--actors", &mallory],
    );
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "PR 2 must block (--base): {stdout}");
    let (code, report, stdout) = run(&dir, &["check"]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "PR 2 must block (whole corpus): {stdout}");
}

#[test]
fn the_two_pr_symlink_attack_is_blocked_at_both_steps() {
    let dir = fixture("attack");
    let mallory = actors(&dir, "mallory");

    // PR 1: the record leaves the governed tree and a symlink stands in for it.
    move_out_and_link(&dir);
    commit_all(&dir, "pr1");
    let (code, report, stdout) = run(&dir, &["check", "--base", "base", "--actors", &mallory]);
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(escapes(&report)[0]["file"], PATH_1);
    assert_eq!(code, 1, "PR 1 must block: {stdout}");
    git(&dir, &["tag", "after-pr1"]);

    // PR 2 (had PR 1 merged): the moved copy becomes Accepted by a non-decider.
    std::fs::write(dir.join("notes/x.md"), ACCEPTED).unwrap();
    commit_all(&dir, "pr2");
    let (code, report, stdout) = run(
        &dir,
        &["check", "--base", "after-pr1", "--actors", &mallory],
    );
    assert_eq!(escapes(&report).len(), 1, "{stdout}");
    assert_eq!(code, 1, "PR 2 must block: {stdout}");
}

#[test]
fn an_honest_move_with_no_symlink_left_behind_is_not_refused() {
    let dir = fixture("honest-move");
    git(&dir, &["rm", "-q", PATH_1]);
    std::fs::write(dir.join("notes/x.md"), PROPOSED).unwrap();
    commit_all(&dir, "move");
    let (_, report, stdout) = run(&dir, &["check"]);
    assert!(escapes(&report).is_empty(), "{stdout}");
}
