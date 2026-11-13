//! MILE-199, ADR-89 section 1, ADR-70 (amendment of 2026-11-05): the drafting
//! command of every record type is generated from the configuration by
//! `urzua harness install`, and `/amend`, `/repin` and `/done` are thin wrappers
//! over urzua commands. A command carries no record type, field or status of
//! its own; `/done` sets a status only through `urzua fix --apply`, and never
//! one an `authority` gate names; `/repin` and `/reverify` pass `--actor agent`.
//! Real git-repo fixtures and the real binary.

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

fn urzua(dir: &Path, args: &[&str]) -> Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua should run")
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

const HELPER: &str = ".claude/hooks/urzua-record.sh";

const BASE: &str = "schema_version: 2\nrules:\n  status.delivery-agreement: error\n  embodiment.locator-exists: error\nharness:\n  adapters: [claude-code]\n  guardrails:\n    editor-hook: {enabled: true}\nrecord_types:\n  milestone:\n    dir: \"docs/milestones\"\n    prefix: \"MILE\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Realized-by\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Planned, InProgress, Done, WontDo]\n      success: [Done]\n      failure: [WontDo]\n      delivery:\n        delivered_tiers: [Implemented, Verified]\n        unevidenced_tiers: [\"Not started\"]\n        not_done: [Planned, InProgress]\n        set_status: Done\n  note:\n    dir: \"docs/notes\"\n    prefix: \"NOTE\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Open, Closed]\n";

const MILESTONE_BUILT: &str = "---\nStatus: Planned\nRealized-by: code:src/a.rs\n---\n# MILE-1 — X\n\n## What\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n";

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-lifecycle-{name}"));
    for d in ["docs/milestones", "docs/notes", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "tester"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("docs/milestones/MILE-1-x.md"), MILESTONE_BUILT).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn installed(name: &str, config: &str) -> common::TempDir {
    let dir = repo(name, config);
    let out = urzua(&dir, &["harness", "install"]);
    assert_eq!(out.status.code(), Some(0), "{}", json(&out));
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "install"]);
    dir
}

fn helper(dir: &Path, args: &[&str]) -> Output {
    Command::new("bash")
        .arg(dir.join(HELPER))
        .args(args)
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env_remove("URZUA_BY")
        .current_dir(dir)
        .output()
        .expect("bash should run")
}

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

// ------------------------------------------------------------ generation

#[test]
fn install_generates_a_drafting_command_for_every_declared_type_and_the_lifecycle_commands() {
    let dir = installed("generate", BASE);
    for name in ["milestone", "note", "amend", "repin", "reverify", "done"] {
        assert!(
            dir.join(format!(".claude/commands/{name}.md")).is_file(),
            "{name}"
        );
    }
    let milestone = read(&dir, ".claude/commands/milestone.md");
    assert!(
        milestone.contains("milestone") && milestone.contains("docs/milestones"),
        "{milestone}"
    );
    let note = read(&dir, ".claude/commands/note.md");
    assert!(
        note.contains("note") && note.contains("docs/notes"),
        "{note}"
    );
    assert!(!note.contains("milestone"), "{note}");
    let status = urzua(&dir, &["harness", "status"]);
    assert_eq!(status.status.code(), Some(0), "{}", json(&status));
}

#[test]
fn adding_a_type_to_the_configuration_and_updating_adds_its_command() {
    let dir = installed("update", BASE);
    assert!(!dir.join(".claude/commands/waiver.md").exists());
    let cfg = format!(
        "{BASE}  waiver:\n    dir: \"docs/waivers\"\n    prefix: \"WAIV\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
    );
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    let before = urzua(&dir, &["harness", "status"]);
    assert_eq!(before.status.code(), Some(1), "a missing command is drift");
    let out = urzua(&dir, &["harness", "update"]);
    assert_eq!(out.status.code(), Some(0), "{}", json(&out));
    let waiver = read(&dir, ".claude/commands/waiver.md");
    assert!(waiver.contains("docs/waivers"), "{waiver}");
    let after = urzua(&dir, &["harness", "status"]);
    assert_eq!(after.status.code(), Some(0), "{}", json(&after));
}

#[test]
fn a_type_named_like_a_lifecycle_command_is_refused_at_install() {
    let cfg = BASE.replace("  note:\n", "  done:\n");
    let dir = repo("collide", &cfg);
    let out = urzua(&dir, &["harness", "install"]);
    assert_eq!(out.status.code(), Some(2), "{}", json(&out));
    assert!(json(&out).to_string().contains("done"), "{}", json(&out));
}

#[test]
fn no_generated_command_names_a_field_or_status_of_the_configuration() {
    let dir = installed("vocabulary", BASE);
    let mut literals = vec![
        "Status",
        "Realized-by",
        "Planned",
        "InProgress",
        "Done",
        "WontDo",
        "Open",
        "Closed",
        "Accepted",
        "Fixed",
        "Proposed",
        "Draft",
        "Rejected",
        "Deciders",
        "Accepted-on",
        "Embodiment",
        "Implements",
        "Stable-Id",
    ];
    literals.sort();
    for name in ["milestone", "note", "amend", "repin", "reverify", "done"] {
        let text = read(&dir, &format!(".claude/commands/{name}.md"));
        for word in &literals {
            assert!(!text.contains(word), "{name}.md restates `{word}`");
        }
    }
    let script = read(&dir, HELPER);
    for word in &literals {
        assert!(!script.contains(word), "{HELPER} restates `{word}`");
    }
}

// ------------------------------------------------------------------ /done

#[test]
fn done_sets_the_status_only_through_the_engines_computation() {
    let dir = installed("done", BASE);
    let before = read(&dir, "docs/milestones/MILE-1-x.md");
    let out = Command::new("bash")
        .arg(dir.join(HELPER))
        .args(["done", "docs/milestones/MILE-1-x.md"])
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env("URZUA_BY", "tester")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let after = read(&dir, "docs/milestones/MILE-1-x.md");
    assert_ne!(before, after);
    assert!(after.contains("Status: Done"), "{after}");
    assert!(after.contains("actor: agent"), "{after}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("examined 1 record"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn done_refuses_when_the_engine_computes_no_status() {
    let dir = installed("done-nothing", BASE);
    // Without evidence there is nothing computed to set, and the command writes no status itself.
    let path = "docs/milestones/MILE-1-x.md";
    let text = read(&dir, path).replace("Realized-by: code:src/a.rs\n", "");
    std::fs::write(dir.join(path), &text).unwrap();
    let out = Command::new("bash")
        .arg(dir.join(HELPER))
        .args(["done", path])
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env("URZUA_BY", "tester")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("computed no"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(read(&dir, path), text);
}

#[test]
fn a_status_an_authority_gate_names_is_never_written_by_fix_or_done() {
    let cfg = format!(
        "{BASE}authority:\n  deciders:\n    m: {{logins: [a]}}\n  gates:\n    - {{type: milestone, to: [Done], by: m}}\n"
    );
    let dir = installed("gated", &cfg);
    let path = "docs/milestones/MILE-1-x.md";
    let before = read(&dir, path);
    let out = urzua(&dir, &["fix", "--apply", "--ids", path, "--by", "tester"]);
    let report = json(&out);
    assert!(
        report["failed"].to_string().contains("authority"),
        "{report}"
    );
    assert_eq!(read(&dir, path), before);
    let out = Command::new("bash")
        .arg(dir.join(HELPER))
        .args(["done", path])
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env("URZUA_BY", "tester")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(read(&dir, path), before);
}

// ----------------------------------------------------------------- /repin

const STALE: &str =
    "code:src/a.rs@sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn with_stale_pin(name: &str) -> common::TempDir {
    let dir = installed(name, BASE);
    let path = "docs/milestones/MILE-1-x.md";
    let text = read(&dir, path)
        .replace(
            "Realized-by: code:src/a.rs",
            &format!("Realized-by: {STALE}"),
        )
        .replace("Status: Planned", "Status: Done");
    std::fs::write(dir.join(path), text).unwrap();
    git(&dir, &["commit", "-qam", "stale pin"]);
    dir
}

fn repin(dir: &Path, reason: &str) -> Output {
    Command::new("bash")
        .arg(dir.join(HELPER))
        .args(["repin", "docs/milestones/MILE-1-x.md", reason])
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env("URZUA_BY", "tester")
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn repin_refuses_an_empty_reason_and_changes_nothing() {
    let dir = with_stale_pin("repin-empty");
    let before = read(&dir, "docs/milestones/MILE-1-x.md");
    for reason in ["", "   "] {
        let out = repin(&dir, reason);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("reason"));
    }
    assert_eq!(read(&dir, "docs/milestones/MILE-1-x.md"), before);
}

#[test]
fn repin_passes_actor_agent_and_prints_the_line_for_the_pr_body() {
    let dir = with_stale_pin("repin-ok");
    let out = repin(
        &dir,
        "the helper gained a return value; the claim still holds",
    );
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read(&dir, "docs/milestones/MILE-1-x.md");
    assert!(!text.contains(&"0".repeat(64)), "{text}");
    assert!(text.contains("actor: agent"), "{text}");
    assert!(text.contains("the helper gained a return value"), "{text}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Re-pin: docs/milestones/MILE-1-x.md")
            && stderr.contains("the helper gained"),
        "{stderr}"
    );
}

// -------------------------------------------------------------- /reverify

fn with_expiry_rule(name: &str) -> common::TempDir {
    let config = BASE
        .replace(
            "rules:\n",
            "rules:\n  embodiment.claim-expired:\n    level: warn\n    anchor_field: Last-audited\n    require_pin: [code]\n    periods:\n      - {type: milestone, status: [Done], days: 1}\n",
        )
        .replace(
            "known_fields: [\"Realized-by\"]",
            "known_fields: [\"Realized-by\", \"Last-audited\"]",
        );
    let dir = installed(name, &config);
    let path = "docs/milestones/MILE-1-x.md";
    let text = read(&dir, path).replace("Status: Planned", "Status: Done");
    std::fs::write(dir.join(path), text).unwrap();
    git(&dir, &["commit", "-qam", "done"]);
    dir
}

fn reverify(dir: &Path, reason: &str) -> Output {
    Command::new("bash")
        .arg(dir.join(HELPER))
        .args(["reverify", "docs/milestones/MILE-1-x.md", reason])
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .env("URZUA_BY", "tester")
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn reverify_refuses_an_empty_reason_and_changes_nothing() {
    let dir = with_expiry_rule("reverify-empty");
    let before = read(&dir, "docs/milestones/MILE-1-x.md");
    for reason in ["", "   "] {
        let out = reverify(&dir, reason);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("reason"));
    }
    assert_eq!(read(&dir, "docs/milestones/MILE-1-x.md"), before);
}

#[test]
fn reverify_lists_the_review_passes_actor_agent_and_prints_the_lines_for_the_pr_body() {
    let dir = with_expiry_rule("reverify-ok");
    let out = reverify(&dir, "read a.rs against the item; the claim holds");
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read(&dir, "docs/milestones/MILE-1-x.md");
    assert!(text.contains("Last-audited: "), "{text}");
    assert!(text.contains("@sha256:"), "{text}");
    assert!(text.matches("actor: agent").count() == 2, "{text}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("will-pin")
            && stderr.contains("Pinned: docs/milestones/MILE-1-x.md")
            && stderr.contains("Re-verified: docs/milestones/MILE-1-x.md")
            && stderr.contains("read a.rs against the item"),
        "{stderr}"
    );
}

/// The helper runs under `set -o pipefail`. `printf ... | grep -q` there is a race: `grep -q`
/// exits on its first match and a `printf` still writing is killed by SIGPIPE, so the whole
/// condition reads false although the text matched (the `Pinned:` line went missing about once
/// in a thousand runs under load). A match is tested on a here-string, which has no writer to kill.
#[test]
fn the_helper_never_tests_a_match_through_a_pipe_into_grep_q() {
    let script = include_str!("../bundle/claude-code/files/.claude/hooks/urzua-record.sh");
    let piped: Vec<&str> = script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && l.contains("| grep -q"))
        .collect();
    assert!(piped.is_empty(), "{piped:#?}");
}

#[test]
fn reverify_refuses_changed_evidence_and_says_nothing_was_renewed() {
    let dir = with_expiry_rule("reverify-changed");
    let path = "docs/milestones/MILE-1-x.md";
    let text = read(&dir, path).replace(
        "Realized-by: code:src/a.rs",
        &format!("Realized-by: {STALE}"),
    );
    std::fs::write(dir.join(path), &text).unwrap();
    git(&dir, &["commit", "-qam", "stale pin"]);
    let out = reverify(&dir, "looked at it");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("nothing was renewed"));
    assert_eq!(read(&dir, path), text);
}

// ----------------------------------------------------------------- /amend

const DECIDED: &str = "---\nStatus: Open\n---\n# NOTE-1 — N\n\n## Decision\n\nWe do it this way.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n";

fn note_repo(name: &str) -> common::TempDir {
    let dir = installed(name, BASE);
    std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), DECIDED).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "a decided note"]);
    dir
}

fn amend_finish(dir: &Path) -> Output {
    helper(dir, &["amend-finish", "docs/notes/NOTE-1-n.md"])
}

#[test]
fn amend_accepts_an_added_dated_section_and_a_row_and_refuses_a_changed_decision() {
    let dir = note_repo("amend");
    let good = DECIDED.replace(
        "\n> **Revision log**",
        "\n## Amendment (2026-11-06): a correction\n\nThe wording was wrong.\n\n> **Revision log**",
    ) + "> | 2026-11-06 | Amendment added: a correction. | **substantive** |\n";
    std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), &good).unwrap();
    let out = amend_finish(&dir);
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("examined 1 record"));

    let dir = note_repo("amend-refused");
    let bad = good.replace("We do it this way.", "We do it another way.");
    std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), bad).unwrap();
    let out = amend_finish(&dir);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("refusing"));

    let dir = note_repo("amend-none");
    let no_section = DECIDED.to_string() + "> | 2026-11-06 | Row only. | **substantive** |\n";
    std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), no_section).unwrap();
    let out = amend_finish(&dir);
    assert!(!out.status.success());
}

#[test]
fn amend_refuses_a_removed_list_item_blank_line_or_rule_in_the_decision() {
    let decided = "---\nStatus: Open\n---\n# NOTE-1 — N\n\n## Decision\n\n- keep A\n- keep B\n\n----\n\nWe do it this way.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n";
    let amendment =
        "\n## Amendment (2026-11-06): a correction\n\nThe wording was wrong.\n\n> **Revision log**";
    // Each removal is a change to the Decision text that the amendment must not hide.
    for (case, from, to) in [
        ("list-item", "- keep A\n- keep B", "- keep B"),
        ("blank-line", "- keep B\n\n----", "- keep B\n----"),
        ("rule", "----\n\nWe do", "We do"),
    ] {
        let dir = installed(&format!("amend-removed-{case}"), BASE);
        std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), decided).unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "a decided note"]);
        let edited = decided.replacen(from, to, 1);
        assert_ne!(edited, decided, "{case}: the fixture must remove something");
        let edited = edited.replace("\n> **Revision log**", amendment)
            + "> | 2026-11-06 | Amendment added: a correction. | **substantive** |\n";
        std::fs::write(dir.join("docs/notes/NOTE-1-n.md"), edited).unwrap();
        let out = amend_finish(&dir);
        assert!(!out.status.success(), "{case} was accepted");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("refusing") && stderr.contains("changed or removed"),
            "{case}: {stderr}"
        );
    }
}
