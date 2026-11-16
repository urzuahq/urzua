//! MILE-181: `urzua fix --rebaseline` lists stale `@sha256:` pins and, on
//! review, re-pins the chosen records with an attributed revision row.
//! Real git-repo fixtures, real binary; all output is JSON.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// Runs urzua with a `gh` on PATH that behaves as given, so the identity tier
/// that reads the authenticated login does not depend on the machine's own
/// `gh`. `None` is a `gh` that fails (not logged in).
fn run_urzua_with_gh(dir: &Path, args: &[&str], login: Option<&str>) -> std::process::Output {
    use std::os::unix::fs::PermissionsExt;
    let stub = common::TempDir::new("urzua-rebase-stub-gh");
    let body = match login {
        Some(l) => format!("#!/bin/sh\necho '{l}'\n"),
        None => "#!/bin/sh\nexit 1\n".to_string(),
    };
    std::fs::write(stub.join("gh"), body).unwrap();
    std::fs::set_permissions(stub.join("gh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    common::urzua()
        .args(args)
        .env("PATH", path)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn sha256(content: &str) -> String {
    use std::io::Write;
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const V1: &str = "fn a() {}\n";
const V2: &str = "fn a() { let _ = 1; }\n";
const A: &str = "docs/adr/ADR-1-a.md";
const B: &str = "docs/adr/ADR-2-b.md";

fn record(n: u32) -> String {
    format!(
        "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{}, test:t.rs'\n---\n# ADR-{n}: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n",
        sha256(V1)
    )
}

/// Two records pin `src/a.rs`; the file then changes.
fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-rebase-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(dir.join(A), record(1)).unwrap();
    std::fs::write(dir.join(B), record(2)).unwrap();
    commit_all(&dir);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn detect_lists_each_stale_pin_with_both_hashes_and_writes_nothing_observed_failing() {
    let dir = fixture("detect");
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    let out = run_urzua(&dir, &["fix", "--rebaseline"]);
    let parsed = json(&out);
    assert_eq!(parsed["status"], "repairs-available", "{parsed}");
    assert_eq!(out.status.code(), Some(0));
    let rb = parsed["rebaselines"].as_array().unwrap();
    assert_eq!(rb.len(), 2, "{parsed}");
    assert_eq!(rb[0]["pins"][0]["recorded_hash"], sha256(V1));
    assert_eq!(rb[0]["pins"][0]["current_hash"], sha256(V2));
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}

#[test]
fn apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing() {
    let dir = fixture("apply");
    let b_before = std::fs::read_to_string(dir.join(B)).unwrap();
    let out = run_urzua_with_gh(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
        None,
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed} {out:?}");
    let a = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(a.contains(&format!(
        "Realized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{}, test:t.rs'",
        sha256(V2)
    )));
    // Exactly one new row, three cells, naming who attested and why.
    let rows: Vec<&str> = a.lines().filter(|l| l.contains("Re-baselined")).collect();
    assert_eq!(rows.len(), 1, "{a}");
    assert!(
        rows[0].starts_with("> | 20") && rows[0].ends_with("| **structural** |"),
        "{}",
        rows[0]
    );
    assert_eq!(rows[0].matches('|').count(), 4, "{}", rows[0]);
    assert!(
        rows[0].contains(
            "Re-baselined (attested by ana, identity: asserted, actor: unspecified): re-pinned"
        ),
        "{}",
        rows[0]
    );
    assert!(
        rows[0].contains("Reason: Read the diff; the claim holds."),
        "{}",
        rows[0]
    );
    assert_eq!(std::fs::read_to_string(dir.join(B)).unwrap(), b_before);

    // The re-pinned record clears; the other one is still listed.
    let again = json(&run_urzua(&dir, &["fix", "--rebaseline"]));
    assert_eq!(again["rebaselines"].as_array().unwrap().len(), 1, "{again}");
    let second = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    );
    assert_eq!(json(&second)["status"], "partial-failure");
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), a);
}

#[test]
fn apply_refuses_without_a_selector_or_a_reason_and_force_is_explicit_observed_failing() {
    let dir = fixture("gates");
    let a_before = std::fs::read_to_string(dir.join(A)).unwrap();
    for args in [
        vec![
            "fix",
            "--rebaseline",
            "--apply",
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
        vec!["fix", "--rebaseline", "--apply", "--ids", A, "--by", "ana"],
    ] {
        let out = run_urzua(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        assert_eq!(json(&out)["status"], "not-run");
    }
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), a_before);

    let forced = json(&run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--force",
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    ));
    assert_eq!(forced["status"], "applied", "{forced}");
    let none = json(&run_urzua(&dir, &["fix", "--rebaseline"]));
    assert_eq!(none["status"], "ok", "{none}");
}

#[test]
fn an_ids_entry_with_no_stale_pin_fails_naming_it_observed_failing() {
    let dir = fixture("unmatched");
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            "docs/adr/ADR-9-nope.md",
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(parsed["failed"][0]["error"]
        .as_str()
        .unwrap()
        .contains("ADR-9"));
}

#[test]
fn rebaseline_cannot_be_combined_with_stdin_path() {
    let dir = fixture("stdin");
    let out = run_urzua(&dir, &["fix", "--rebaseline", "--stdin-path", A]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn an_authenticated_gh_login_is_recorded_over_by_and_the_override_is_disclosed() {
    let dir = fixture("gh-wins");
    let out = run_urzua_with_gh(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--actor",
            "agent",
            "--reason",
            "Read the diff; the claim holds.",
        ],
        Some("gh-login"),
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let a = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        a.contains("(attested by gh-login, identity: gh-verified, actor: agent)"),
        "{a}"
    );
    assert!(!a.contains("attested by ana"), "{a}");
    assert!(
        parsed["notices"]
            .to_string()
            .contains("differs from the authenticated gh login"),
        "{parsed}"
    );
}

#[test]
fn an_identity_that_would_break_the_revision_row_is_refused_and_writes_nothing() {
    for by in ["ana | bob", "ana\nbob"] {
        let dir = fixture("by-pipe");
        let before = std::fs::read_to_string(dir.join(A)).unwrap();
        let out = run_urzua_with_gh(
            &dir,
            &[
                "fix",
                "--rebaseline",
                "--apply",
                "--ids",
                A,
                "--by",
                by,
                "--reason",
                "Read the diff; the claim holds.",
            ],
            None,
        );
        assert_eq!(out.status.code(), Some(2), "{by:?}: {out:?}");
        let parsed = json(&out);
        assert_eq!(parsed["status"], "not-run", "{parsed}");
        assert!(
            parsed["error"].as_str().unwrap().contains("--by"),
            "{parsed}"
        );
        assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
    }
}

#[test]
fn a_git_user_name_with_a_pipe_is_refused_for_the_row_too() {
    let dir = fixture("name-pipe");
    git(&dir, &["config", "user.name", "ana | bob"]);
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    let out = run_urzua_with_gh(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--reason",
            "Read the diff; the claim holds.",
        ],
        None,
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "not-run", "{parsed}");
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}

#[test]
fn rebaseline_detect_honours_ids_and_names_an_unknown_one_observed_failing() {
    let dir = fixture("detect-ids");
    let v = json(&run_urzua(&dir, &["fix", "--rebaseline", "--ids", B]));
    let rb = v["rebaselines"].as_array().unwrap();
    assert_eq!(rb.len(), 1, "{v}");
    assert_eq!(rb[0]["record"], B, "{v}");

    let out = run_urzua(
        &dir,
        &["fix", "--rebaseline", "--ids", "docs/adr/ADR-9-nope.md"],
    );
    let v = json(&out);
    assert_eq!(v["status"], "partial-failure", "{v}");
    assert_eq!(out.status.code(), Some(1), "{v}");
    assert!(v["failed"][0]["error"].as_str().unwrap().contains("ADR-9"));
    assert!(
        v["rebaselines"].as_array().is_none_or(|r| r.is_empty()),
        "{v}"
    );
}

#[test]
fn rebaseline_without_apply_rejects_force_by_and_actor_observed_failing() {
    let dir = fixture("detect-flags");
    for args in [
        vec!["fix", "--rebaseline", "--force"],
        vec!["fix", "--rebaseline", "--by", "ana"],
        vec!["fix", "--rebaseline", "--actor", "agent"],
    ] {
        let out = run_urzua(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
    }
}

#[test]
fn a_carriage_return_in_the_reason_is_refused_like_a_newline() {
    let dir = fixture("reason-cr");
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    for reason in ["line one\rline two", "line one\nline two"] {
        let out = run_urzua(
            &dir,
            &[
                "fix",
                "--rebaseline",
                "--apply",
                "--ids",
                A,
                "--by",
                "ana",
                "--reason",
                reason,
            ],
        );
        assert_eq!(out.status.code(), Some(2), "{reason:?}: {out:?}");
        assert!(json(&out).to_string().contains("one line"), "{reason:?}");
    }
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}

#[test]
fn a_reason_too_short_or_not_a_sentence_is_refused_and_nothing_is_written() {
    // A re-pin is an attestation: "x" or "ok" says nothing a reviewer can check.
    let dir = fixture("reason-thin");
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    for reason in ["x", "r", "ok", "...", "no change", "looks fine", "a b c d"] {
        let out = run_urzua(
            &dir,
            &[
                "fix",
                "--rebaseline",
                "--apply",
                "--ids",
                A,
                "--by",
                "ana",
                "--reason",
                reason,
            ],
        );
        assert_eq!(out.status.code(), Some(2), "{reason:?}: {out:?}");
        let message = json(&out).to_string();
        assert!(
            message.contains("at least 3 words") && message.contains("12 characters"),
            "{reason:?}: {message}"
        );
    }
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
    // Three words of 12 characters or more is accepted.
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; claim holds.",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

/// The reason minimum is configuration: `attestation.reason` lowers or raises it,
/// and the same declaration governs `reverify --apply`.
#[test]
fn the_reason_minimum_is_read_from_attestation_reason() {
    let dir = fixture("reason-configured");
    let low = format!(
        "{CONFIG}attestation:\n  identity: asserted\n  reason: {{min_words: 1, min_chars: 1}}\n"
    );
    std::fs::write(dir.join(".urzua/config.yaml"), low).unwrap();
    commit_all(&dir);
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "r",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    let dir = fixture("reason-raised");
    let high = format!(
        "{CONFIG}attestation:\n  identity: asserted\n  reason: {{min_words: 8, min_chars: 12}}\n"
    );
    std::fs::write(dir.join(".urzua/config.yaml"), high).unwrap();
    commit_all(&dir);
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; claim holds.",
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        json(&out).to_string().contains("at least 8 words"),
        "{out:?}"
    );
}

/// A declared `evidence_class` is the class of a re-pin row, so a re-pin
/// leaves a `version_bump` that does not name it alone.
#[test]
fn a_declared_evidence_class_is_the_class_of_the_re_pin_row() {
    let dir = fixture("evidence-class");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        CONFIG.replace(
            "    required_fields: []\n",
            "    required_fields: []\n    revision_log: {marker: \"**Revision log**\", classes: [substantive, structural, evidence], evidence_class: evidence}\n",
        ),
    )
    .unwrap();
    let out = run_urzua_with_gh(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
        None,
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    let a = std::fs::read_to_string(dir.join(A)).unwrap();
    let row = a.lines().find(|l| l.contains("Re-baselined")).unwrap();
    assert!(row.ends_with("| **evidence** |"), "{row}");
}

#[test]
fn a_record_under_a_path_gate_is_not_re_pinned_for_an_agent() {
    let dir = fixture("gated");
    common::gate_paths(&dir, &["docs/adr"]);
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    let out = run_urzua_with_gh(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--actor",
            "agent",
            "--reason",
            "Read the diff; the claim holds.",
        ],
        Some("lead"),
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed
            .to_string()
            .contains("only group 'leads' (lead) may change it"),
        "{parsed}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}

fn check_base(dir: &Path) -> serde_json::Value {
    json(&run_urzua(dir, &["check", "--base", "main"]))
}

fn unattested(report: &serde_json::Value) -> Vec<serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.pin-changed-unattested")
        .cloned()
        .collect()
}

/// `fixture` on `main`, then a branch to change pins on.
fn branched(name: &str) -> common::TempDir {
    let dir = fixture(name);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "work"]);
    dir
}

#[test]
fn a_hand_edited_pin_fails_check_base_and_a_re_pin_with_the_tool_does_not() {
    let dir = branched("hand");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(dir.join(A), text.replace(&sha256(V1), &sha256(V2))).unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    let found = unattested(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["severity"], "error", "{report}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("'src/a.rs'") && message.contains(&sha256(V1)[..12]),
        "{message}"
    );
    assert!(message.contains("--actor agent"), "{message}");
    assert!(unattested(&json(&run_urzua(&dir, &["check"]))).is_empty());

    let dir = branched("tool");
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    );
    assert_eq!(json(&out)["status"], "applied");
    commit_all(&dir);
    assert!(unattested(&check_base(&dir)).is_empty());
}

#[test]
fn a_row_for_another_change_does_not_account_for_a_hand_edited_pin() {
    let dir = branched("reuse");
    let text = std::fs::read_to_string(dir.join(B)).unwrap();
    let row = format!(
        "> | 2026-11-16 | Re-baselined (attested by ana, identity: asserted, actor: agent): re-pinned `src/a.rs` {} -> {}. Reason: r **Why:** w | **structural** |\n",
        &sha256("other")[..12],
        &sha256(V2)[..12]
    );
    std::fs::write(
        dir.join(B),
        format!("{}{row}", text.replace(&sha256(V1), &sha256(V2))),
    )
    .unwrap();
    commit_all(&dir);
    assert_eq!(unattested(&check_base(&dir)).len(), 1);
}

#[test]
fn a_removed_pin_is_reported_and_a_pin_on_a_new_record_is_not() {
    let dir = branched("removed");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(&format!("@sha256:{}", sha256(V1)), ""),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-3-c.md"),
        record(3).replace(&sha256(V1), &sha256(V2)),
    )
    .unwrap();
    commit_all(&dir);
    let found = unattested(&check_base(&dir));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], A);
    assert!(found[0]["message"].as_str().unwrap().contains("removed"));
}

#[test]
fn turning_the_rule_off_is_announced() {
    let dir = fixture("off");
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "rules: {embodiment.consistency: error}",
            "rules: {embodiment.consistency: error, diff.pin-changed-unattested: off}",
        ),
    )
    .unwrap();
    let report = json(&run_urzua(&dir, &["check"]));
    assert!(
        report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["subject"] == "diff.pin-changed-unattested"),
        "{report}"
    );
}

fn variants(report: &serde_json::Value) -> Vec<String> {
    unattested(report)
        .iter()
        .map(|f| {
            let m = f["message"].as_str().unwrap_or_default();
            if m.contains("was added") {
                "pin-added-unattested"
            } else if m.contains("removed") {
                "pin-removed"
            } else {
                "pin-changed-unattested"
            }
            .to_string()
        })
        .collect()
}

#[test]
fn renaming_a_record_does_not_take_a_hand_edited_pin_past_the_rule() {
    let dir = fixture("rename");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replacen("---\n", "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\n", 1),
    )
    .unwrap();
    commit_all(&dir);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "work"]);
    git(&dir, &["mv", A, "docs/adr/ADR-1-renamed.md"]);
    let moved = dir.join("docs/adr/ADR-1-renamed.md");
    let text = std::fs::read_to_string(&moved).unwrap();
    std::fs::write(&moved, text.replace(&sha256(V1), &sha256(V2))).unwrap();
    commit_all(&dir);
    let found = unattested(&check_base(&dir));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-1-renamed.md");
}

#[test]
fn a_hand_added_first_pin_is_reported_and_fix_pin_writes_the_row_that_clears_it() {
    let dir = branched("first-hand");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace("test:t.rs", &format!("test:t.rs@sha256:{}", sha256("t\n"))),
    )
    .unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    let found = unattested(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("urzua fix --pin --apply --ids"),
        "{report}"
    );

    let dir = branched("first-tool");
    let listed = json(&run_urzua(&dir, &["fix", "--pin", "--ids", A]));
    assert_eq!(listed["status"], "pins-available", "{listed}");
    assert!(
        listed["next_action"]
            .as_str()
            .unwrap()
            .contains("--actor agent"),
        "{listed}"
    );
    assert_eq!(
        listed["next_action_inputs"][0]["flag"], "--reason",
        "{listed}"
    );
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "t.rs is the test this claim rests on.",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    assert!(std::fs::read_to_string(dir.join(A))
        .unwrap()
        .contains(&format!("test:t.rs@sha256:{}", sha256("t\n"))));
    commit_all(&dir);
    assert!(unattested(&check_base(&dir)).is_empty());
}

#[test]
fn moving_a_pin_to_a_renamed_file_needs_fix_pin_replace() {
    let dir = branched("move-hand");
    git(&dir, &["mv", "src/a.rs", "src/b.rs"]);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(
            &format!("src/a.rs@sha256:{}", sha256(V1)),
            &format!("src/b.rs@sha256:{}", sha256(V2)),
        ),
    )
    .unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    let mut found = variants(&report);
    found.sort();
    assert_eq!(found, ["pin-added-unattested", "pin-removed"], "{report}");

    let dir = branched("move-tool");
    git(&dir, &["mv", "src/a.rs", "src/b.rs"]);
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--replace",
            "src/a.rs=src/b.rs",
            "--by",
            "ana",
            "--reason",
            "a.rs was renamed to b.rs; the code the claim rests on moved with it.",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        text.contains(&format!("code:src/b.rs@sha256:{}", sha256(V2))),
        "{text}"
    );
    assert!(text.contains("Pin moved to another locator"), "{text}");
    commit_all(&dir);
    let report = check_base(&dir);
    assert!(unattested(&report).is_empty(), "{report}");
}

#[test]
fn removing_a_pinned_locator_needs_fix_pin_remove() {
    let dir = branched("remove-tool");
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--remove",
            "src/a.rs",
            "--by",
            "ana",
            "--reason",
            "the claim no longer rests on a.rs.",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    let field = text
        .lines()
        .find(|l| l.starts_with("Realized-by:"))
        .unwrap();
    assert_eq!(field, "Realized-by: 'spec:docs/s.md, test:t.rs'", "{text}");
    commit_all(&dir);
    assert!(unattested(&check_base(&dir)).is_empty());
}

#[test]
fn a_pull_request_that_turns_the_rule_off_is_still_judged_at_the_base_level() {
    let dir = branched("off-in-head");
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "rules: {embodiment.consistency: error}",
            "rules: {embodiment.consistency: error, diff.pin-changed-unattested: off}",
        ),
    )
    .unwrap();
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(dir.join(A), text.replace(&sha256(V1), &sha256(V2))).unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    let found = unattested(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["severity"], "error", "{report}");
    assert!(
        report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["subject"] == "diff.pin-changed-unattested"
                && n["message"]
                    .as_str()
                    .unwrap()
                    .contains("the base's level applies")),
        "{report}"
    );
}

/// `fixture` with `A`'s text changed on main by `edit` first, then a branch.
fn branched_with(name: &str, edit: impl Fn(&str) -> String) -> common::TempDir {
    let dir = fixture(name);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(dir.join(A), edit(&text)).unwrap();
    commit_all(&dir);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "work"]);
    dir
}

fn with_sid(text: &str) -> String {
    text.replacen("---\n", "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\n", 1)
}

/// Renames `A` to `to` on the branch, applies `edit` and hand-types the
/// current hash over the stale pin.
fn rename_and_hand_pin(dir: &Path, to: &str, edit: impl Fn(&str) -> String) {
    git(dir, &["mv", A, to]);
    let text = std::fs::read_to_string(dir.join(to)).unwrap();
    std::fs::write(dir.join(to), edit(&text).replace(&sha256(V1), &sha256(V2))).unwrap();
    commit_all(dir);
}

fn messages(report: &serde_json::Value) -> Vec<String> {
    unattested(report)
        .iter()
        .map(|f| f["message"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn a_rename_with_no_stable_id_field_is_paired_by_record_id() {
    let dir = branched("rename-no-sid");
    rename_and_hand_pin(&dir, "docs/adr/ADR-1-renamed.md", |t| t.to_string());
    let report = check_base(&dir);
    assert_eq!(messages(&report).len(), 1, "{report}");
}

#[test]
fn a_rename_that_changes_or_removes_the_stable_id_is_paired_and_reported() {
    for (name, edit) in [
        (
            "sid-changed",
            Box::new(|t: &str| {
                t.replace("01AAAAAAAAAAAAAAAAAAAAAA01", "01BBBBBBBBBBBBBBBBBBBBBB02")
            }) as Box<dyn Fn(&str) -> String>,
        ),
        (
            "sid-removed",
            Box::new(|t: &str| t.replace("Stable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\n", "")),
        ),
    ] {
        let dir = branched_with(name, with_sid);
        rename_and_hand_pin(&dir, "docs/adr/ADR-1-renamed.md", &edit);
        let report = check_base(&dir);
        let m = messages(&report);
        assert_eq!(m.len(), 2, "{name}: {report}");
        assert!(m.iter().any(|m| m.contains("Stable-Id")), "{name}: {m:?}");
    }
}

#[test]
fn a_renumbered_record_with_the_same_stable_id_is_paired() {
    let dir = branched_with("renumbered", with_sid);
    rename_and_hand_pin(&dir, "docs/adr/ADR-7-a.md", |t| {
        t.replace("# ADR-1:", "# ADR-7:")
    });
    let report = check_base(&dir);
    assert_eq!(messages(&report).len(), 1, "{report}");
}

#[test]
fn an_unpinned_citation_inserted_above_a_pinned_one_is_no_change() {
    let dir = branched("inserted-citation");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(
            "spec:docs/s.md, code:src/a.rs@",
            "spec:docs/s.md, code:src/a.rs, code:src/a.rs@",
        ),
    )
    .unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    assert!(unattested(&report).is_empty(), "{report}");
}

#[test]
fn reordering_two_pins_of_one_file_is_no_change_and_copying_one_over_the_other_is() {
    let two = |t: &str| {
        t.replace(
            &format!("code:src/a.rs@sha256:{}", sha256(V1)),
            &format!(
                "code:src/a.rs@sha256:{}, code:src/a.rs@sha256:{}",
                sha256(V1),
                sha256(V2)
            ),
        )
    };
    let dir = branched_with("reorder", two);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    let (p1, p2) = (
        format!("src/a.rs@sha256:{}", sha256(V1)),
        format!("src/a.rs@sha256:{}", sha256(V2)),
    );
    std::fs::write(
        dir.join(A),
        text.replace(&p1, "@X@")
            .replace(&p2, &p1)
            .replace("@X@", &p2),
    )
    .unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    assert!(unattested(&report).is_empty(), "{report}");

    let dir = branched_with("copy-over", two);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(dir.join(A), text.replace(&p1, &p2)).unwrap();
    commit_all(&dir);
    let report = check_base(&dir);
    assert_eq!(unattested(&report).len(), 1, "{report}");
}

#[test]
fn a_replaced_pin_is_listed_in_a_notice() {
    let dir = branched("replace-notice");
    std::fs::write(dir.join("src/zz.rs"), "fn zz() {}\n").unwrap();
    commit_all(&dir);
    let out = run_urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--replace",
            "src/a.rs=src/zz.rs",
            "--by",
            "ana",
            "--reason",
            "zz.rs now carries the claim.",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
    commit_all(&dir);
    let report = check_base(&dir);
    assert!(unattested(&report).is_empty(), "{report}");
    let notice = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "pins-replaced-or-removed")
        .unwrap_or_else(|| panic!("{report}"));
    let m = notice["message"].as_str().unwrap();
    assert!(
        m.contains("src/a.rs") && m.contains("src/zz.rs") && m.contains(A),
        "{m}"
    );
}

#[test]
fn a_rename_that_changes_both_ids_is_paired_by_git_rename_detection() {
    let dir = branched("git-rename");
    rename_and_hand_pin(&dir, "docs/adr/ADR-9-other.md", |t| t.to_string());
    let report = check_base(&dir);
    assert_eq!(messages(&report).len(), 1, "{report}");
}

#[test]
fn moving_the_config_and_passing_config_does_not_lift_the_base_level() {
    let dir = branched("moved-config");
    std::fs::create_dir_all(dir.join("cfg")).unwrap();
    git(&dir, &["mv", ".urzua/config.yaml", "cfg/u.yaml"]);
    let cfg = std::fs::read_to_string(dir.join("cfg/u.yaml")).unwrap();
    std::fs::write(
        dir.join("cfg/u.yaml"),
        cfg.replace(
            "rules: {embodiment.consistency: error}",
            "rules: {embodiment.consistency: error, diff.pin-changed-unattested: off}",
        ),
    )
    .unwrap();
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(dir.join(A), text.replace(&sha256(V1), &sha256(V2))).unwrap();
    commit_all(&dir);
    let report = json(&run_urzua(
        &dir,
        &["--config", "cfg/u.yaml", "check", "--base", "main"],
    ));
    assert_eq!(unattested(&report).len(), 1, "{report}");
}

#[test]
fn fix_pin_lists_a_locator_it_cannot_pin_and_names_an_uncommitted_pinned_file() {
    let dir = branched("pin-skips");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace("test:t.rs", "test:t.rs, test:gone.rs"),
    )
    .unwrap();
    commit_all(&dir);
    std::fs::write(dir.join("t.rs"), "t edited\n").unwrap();
    let listed = json(&run_urzua(&dir, &["fix", "--pin", "--ids", A]));
    assert!(listed.to_string().contains("gone.rs"), "{listed}");
    let notice = listed["notices"]
        .as_array()
        .and_then(|n| {
            n.iter()
                .find(|n| n["message"].as_str().unwrap_or_default().contains("t.rs"))
                .cloned()
        })
        .unwrap_or_else(|| panic!("{listed}"));
    assert!(
        notice["message"].as_str().unwrap().contains("uncommitted"),
        "{notice}"
    );
}

/// One record pinning `src/a.rs` with `body` below its header and no revision
/// log, committed on `main`; the pinned file then changes on a branch.
fn logless(name: &str, config: &str, body: &str) -> common::TempDir {
    logless_with(name, config, "", body)
}

/// `logless` with `header` lines added to the record's header.
fn logless_with(name: &str, config: &str, header: &str, body: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-rebase-logless-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(
        dir.join(A),
        format!(
            "---\n{header}Embodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{}, test:t.rs'\n---\n{body}",
            sha256(V1)
        ),
    )
    .unwrap();
    commit_all(&dir);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "work"]);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir);
    dir
}

fn repin_a(dir: &Path) -> std::process::Output {
    run_urzua(
        dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    )
}

#[test]
fn a_re_pin_on_a_record_with_no_log_creates_it_with_the_row_and_passes_check_base() {
    let dir = logless("create", CONFIG, "# ADR-1: T\n\n## What\n\nBody.\n");
    let listed = json(&run_urzua(&dir, &["fix", "--rebaseline", "--ids", A]));
    assert_eq!(listed["status"], "repairs-available", "{listed}");
    assert_eq!(
        listed["rebaselines"][0]["creates_revision_log"], true,
        "{listed}"
    );

    let out = repin_a(&dir);
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert_eq!(
        parsed["rebaselines"][0]["creates_revision_log"], true,
        "{parsed}"
    );
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        text.contains(
            "Body.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | "
        ),
        "{text}"
    );
    assert_eq!(
        text.matches("> | ").count(),
        2,
        "one header, one row: {text}"
    );
    assert!(text.contains(&sha256(V2)), "{text}");

    commit_all(&dir);
    let report = check_base(&dir);
    assert!(unattested(&report).is_empty(), "{report}");
}

#[test]
fn a_created_log_takes_the_declared_heading_and_columns() {
    let config = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    revision_log: {heading: History, date_column: When, class_column: Kind, classes: [substantive, structural, evidence], evidence_class: evidence}\n";
    let dir = logless("declared", config, "# ADR-1: T\n\nBody.\n");
    let parsed = json(&repin_a(&dir));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        text.contains("Body.\n\n## History\n\n| When | Change | Kind |\n|---|---|---|\n| "),
        "{text}"
    );
    assert!(text.trim_end().ends_with("| **evidence** |"), "{text}");
    commit_all(&dir);
    assert!(unattested(&check_base(&dir)).is_empty());
}

#[test]
fn a_created_log_copies_the_types_template_section() {
    let dir = logless("template", CONFIG, "# ADR-1: T\n\nBody.\n");
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    std::fs::write(
        dir.join(".urzua/templates/adr.md"),
        "---\n---\n# N -- T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |:---|:---|:---:|\n> | YYYY-MM-DD | Drafted. | **structural** |\n",
    )
    .unwrap();
    let parsed = json(&repin_a(&dir));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(text.contains("> |:---|:---|:---:|\n> | "), "{text}");
    assert!(!text.contains("YYYY-MM-DD"), "{text}");
}

#[test]
fn a_record_with_no_body_section_is_refused_with_a_next_action_that_adds_one() {
    let dir = logless("nobody", CONFIG, "");
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    let mut command = String::new();
    for args in [
        vec!["fix", "--rebaseline", "--ids", A],
        vec![
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    ] {
        let parsed = json(&run_urzua(&dir, &args));
        assert_eq!(parsed["status"], "partial-failure", "{parsed}");
        let error = parsed["failed"][0]["error"].as_str().unwrap();
        assert!(error.contains("no body section"), "{error}");
        command = parsed["next_action"].as_str().unwrap().to_string();
        assert!(
            command.starts_with("printf") && command.ends_with(A),
            "{parsed}"
        );
        assert_eq!(
            parsed["next_action_inputs"],
            serde_json::json!([]),
            "{parsed}"
        );
        assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
    }
    let ran = Command::new("sh")
        .args(["-c", &command])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(ran.success());
    assert!(std::fs::read_to_string(dir.join(A))
        .unwrap()
        .ends_with("\n# ADR-1-a\n"));
    assert_eq!(json(&repin_a(&dir))["status"], "applied");
}

#[test]
fn a_log_deleted_by_hand_and_committed_is_refused_not_recreated() {
    let with_log = "# ADR-1: T\n\nBody.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Decided the thing. | **substantive** |\n";
    let dir = logless("laundered", CONFIG, with_log);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    let cut = text.find("\n> **Revision log**").unwrap();
    std::fs::write(dir.join(A), &text[..cut]).unwrap();
    commit_all(&dir);
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    for args in [
        vec!["fix", "--rebaseline", "--ids", A],
        vec![
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    ] {
        let parsed = json(&run_urzua(&dir, &args));
        assert_eq!(parsed["status"], "partial-failure", "{parsed}");
        let error = parsed["failed"][0]["error"].as_str().unwrap();
        assert!(
            error.contains("has a Revision log") && error.contains("restore"),
            "{error}"
        );
        let next = parsed["next_action"].as_str().unwrap();
        assert!(
            next.starts_with("git show ") && next.contains(&format!(":{A}")),
            "{next}"
        );
        assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
    }
    let shown = Command::new("sh")
        .args([
            "-c",
            json(&run_urzua(&dir, &["fix", "--rebaseline", "--ids", A]))["next_action"]
                .as_str()
                .unwrap(),
        ])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&shown.stdout).contains("Decided the thing."));
}

#[test]
fn an_uncommitted_log_deletion_is_refused_and_a_new_record_still_gets_a_log() {
    let with_log = "# ADR-1: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Decided. | **substantive** |\n";
    let dir = logless("uncommitted", CONFIG, with_log);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        &text[..text.find("\n> **Revision log**").unwrap()],
    )
    .unwrap();
    let parsed = json(&repin_a(&dir));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("HEAD:"),
        "{parsed}"
    );

    let dir = logless("new-record", CONFIG, "# ADR-1: T\n\nBody.\n");
    std::fs::write(
        dir.join(B),
        format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{}, test:t.rs'\n---\n# ADR-2: New\n\nBody.\n",
            sha256(V1)
        ),
    )
    .unwrap();
    git(&dir, &["add", B]);
    let parsed = json(&run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            B,
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    ));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert_eq!(
        parsed["rebaselines"][0]["creates_revision_log"], true,
        "{parsed}"
    );
}

#[test]
fn a_created_log_sits_where_the_template_orders_it() {
    let config = "schema_version: 2\nrules: {embodiment.consistency: error, section.order: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    sections:\n      from: headings\n      order: {from: template}\n    revision_log: {heading: History}\n";
    let dir = logless(
        "order",
        config,
        "# ADR-1: T\n\n## What\n\nA.\n\n## Notes\n\nB.\n",
    );
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    std::fs::write(
        dir.join(".urzua/templates/adr.md"),
        "---\n---\n# N -- T\n\n## What\n\nw\n\n## History\n\n| Date | Change | Class |\n|---|---|---|\n\n## Notes\n\nn\n",
    )
    .unwrap();
    commit_all(&dir);
    let parsed = json(&repin_a(&dir));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        text.contains("A.\n\n## History\n\n| Date | Change | Class |\n|---|---|---|\n| "),
        "{text}"
    );
    assert!(text.ends_with("|\n\n## Notes\n\nB.\n"), "{text}");
    commit_all(&dir);
    let report = check_base(&dir);
    let order: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "section.order" || f["rule"] == "diff.pin-changed-unattested")
        .collect();
    assert!(order.is_empty(), "{report}");
}

#[test]
fn a_record_ending_inside_an_open_fence_is_refused_in_detect_and_apply() {
    let dir = logless("fence", CONFIG, "# ADR-1: T\n\n```\ncode\n");
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    let listed = json(&run_urzua(&dir, &["fix", "--rebaseline", "--ids", A]));
    assert_eq!(listed["status"], "partial-failure", "{listed}");
    assert!(
        listed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("code fence opened on line"),
        "{listed}"
    );
    assert_eq!(json(&repin_a(&dir))["status"], "partial-failure");
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}

#[test]
fn a_created_log_names_the_declared_narrative_column() {
    let config = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    revision_log: {narrative_column: What changed}\n";
    let dir = logless("narrative", config, "# ADR-1: T\n\nBody.\n");
    assert_eq!(json(&repin_a(&dir))["status"], "applied");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(text.contains("> | Date | What changed | Class |"), "{text}");
    commit_all(&dir);
    assert!(unattested(&check_base(&dir)).is_empty());
}

#[test]
fn a_first_pin_on_a_record_with_no_log_creates_it_with_the_row() {
    let dir = logless("first-pin", CONFIG, "# ADR-1: T\n\nBody.\n");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(", test:t.rs", ", test:t.rs, code:docs/s.md"),
    )
    .unwrap();
    let parsed = json(&run_urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "The spec text carries the claim.",
        ],
    ));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert_eq!(
        parsed["records"][0]["creates_revision_log"], true,
        "{parsed}"
    );
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(text.contains("Body.\n\n> **Revision log**\n"), "{text}");
}

#[test]
fn fix_pin_lists_a_log_it_would_create_and_refuses_a_record_with_no_body() {
    let dir = logless("pin-detect", CONFIG, "# ADR-1: T\n\nBody.\n");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(", test:t.rs", ", test:t.rs, code:docs/s.md"),
    )
    .unwrap();
    let listed = json(&run_urzua(&dir, &["fix", "--pin", "--ids", A]));
    assert_eq!(listed["status"], "pins-available", "{listed}");
    assert_eq!(
        listed["records"][0]["creates_revision_log"], true,
        "{listed}"
    );

    let dir = logless("pin-nobody", CONFIG, "");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace(", test:t.rs", ", test:t.rs, code:docs/s.md"),
    )
    .unwrap();
    let before = std::fs::read_to_string(dir.join(A)).unwrap();
    for args in [
        vec!["fix", "--pin", "--ids", A],
        vec![
            "fix",
            "--pin",
            "--apply",
            "--ids",
            A,
            "--by",
            "ana",
            "--reason",
            "The spec text carries the claim.",
        ],
    ] {
        let parsed = json(&run_urzua(&dir, &args));
        assert_eq!(parsed["status"], "partial-failure", "{parsed}");
        assert!(
            parsed["failed"][0]["error"]
                .as_str()
                .unwrap()
                .contains("no body section"),
            "{parsed}"
        );
        assert!(
            parsed["next_action"]
                .as_str()
                .unwrap()
                .starts_with("printf"),
            "{parsed}"
        );
        assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
    }
}

const LOGGED: &str = "# ADR-1: T\n\nBody.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Decided the thing. | **substantive** |\n";

/// A record with a substantive log on `main`, the log cut on a branch.
fn laundering(name: &str, header: &str) -> common::TempDir {
    let dir = logless_with(name, CONFIG, header, LOGGED);
    cut_log(&dir, A);
    dir
}

fn cut_log(dir: &Path, record: &str) {
    let text = std::fs::read_to_string(dir.join(record)).unwrap();
    let cut = text.find("\n> **Revision log**").unwrap();
    std::fs::write(dir.join(record), &text[..cut]).unwrap();
}

/// Re-pins `record` against `--base main`, asserts it is refused naming a base
/// copy and leaves the record and `check --base main` as they were.
fn assert_restore_refused(dir: &Path, record: &str) {
    assert_restore_refused_then(dir, record, true);
}

fn assert_restore_refused_then(dir: &Path, record: &str, check: bool) {
    let before = std::fs::read_to_string(dir.join(record)).unwrap();
    let parsed = json(&run_urzua(
        dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            record,
            "--base",
            "main",
            "--by",
            "ana",
            "--reason",
            "Read the diff; the claim holds.",
        ],
    ));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    let error = parsed["failed"][0]["error"].as_str().unwrap();
    assert!(
        error.contains("has a Revision log") && error.contains("restore"),
        "{error}"
    );
    assert!(
        parsed["next_action"]
            .as_str()
            .unwrap()
            .starts_with("git show "),
        "{parsed}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(record)).unwrap(), before);
    if check {
        let report = check_base(dir);
        assert!(unattested(&report).is_empty(), "{report}");
    }
}

#[test]
fn a_deleted_log_behind_many_marker_toggles_is_refused_against_the_base() {
    let dir = laundering("toggles", "");
    commit_all(&dir);
    for i in 0..21 {
        let text = std::fs::read_to_string(dir.join(A)).unwrap();
        let next = if i % 2 == 0 {
            format!("{text}\nSee **Revision log**.\n")
        } else {
            text.replace("\nSee **Revision log**.\n", "")
        };
        std::fs::write(dir.join(A), next).unwrap();
        commit_all(&dir);
    }
    assert_restore_refused(&dir, A);
}

#[test]
fn a_deleted_log_then_a_rename_is_refused_against_the_base() {
    let dir = laundering("moved", "");
    commit_all(&dir);
    let moved = "docs/adr/ADR-1-moved.md";
    git(&dir, &["mv", A, moved]);
    commit_all(&dir);
    assert_restore_refused(&dir, moved);
}

#[test]
fn a_record_deleted_and_re_added_elsewhere_is_paired_by_its_stable_id() {
    let dir = laundering("readded", "Stable-Id: 01TESTSTABLEID0000000000AA\n");
    git(&dir, &["rm", "-q", "-f", A]);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    commit_all(&dir);
    let new = "docs/adr/ADR-7-other.md";
    std::fs::write(
        dir.join(new),
        format!(
            "---\nStable-Id: 01TESTSTABLEID0000000000AA\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{}, test:t.rs'\n---\n# ADR-7: Entirely rewritten\n\nNothing alike.\n",
            sha256(V1)
        ),
    )
    .unwrap();
    commit_all(&dir);
    assert_restore_refused(&dir, new);
}

#[test]
fn a_deleted_log_beyond_a_shallow_clones_depth_is_refused_against_the_base_ref() {
    let origin = laundering("shallow-origin", "");
    commit_all(&origin);
    for i in 0..3 {
        std::fs::write(origin.join(format!("n{i}.txt")), "n\n").unwrap();
        commit_all(&origin);
    }
    let dir = common::TempDir::new("urzua-rebase-shallow-clone");
    let url = format!("file://{}", origin.display());
    let cloned = Command::new("git")
        .args(["clone", "-q", "--depth", "1", "--branch", "work", &url, "."])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(cloned.success());
    git(
        &dir,
        &["fetch", "-q", "--depth", "1", "origin", "main:main"],
    );
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    // `check --base` cannot run where the merge base is hidden; the record is
    // left as it was, which is what it would judge.
    assert_restore_refused_then(&dir, A, false);
}

#[test]
fn the_cases_refused_without_a_base_stay_refused_with_one() {
    // Deleted and committed.
    let dir = laundering("base-commit", "");
    commit_all(&dir);
    assert_restore_refused(&dir, A);
    // The marker kept in prose.
    let dir = laundering("base-prose", "");
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        format!("{text}\nThe **Revision log** moved.\n"),
    )
    .unwrap();
    commit_all(&dir);
    assert_restore_refused(&dir, A);
    // The marker changed slightly.
    let dir = logless("base-marker", CONFIG, LOGGED);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    std::fs::write(
        dir.join(A),
        text.replace("**Revision log**", "**Revision Log**"),
    )
    .unwrap();
    commit_all(&dir);
    assert_restore_refused(&dir, A);
    // Squashed into the branch's one commit.
    let dir = laundering("base-squash", "");
    git(&dir, &["reset", "-q", "--soft", "main"]);
    commit_all(&dir);
    assert_restore_refused(&dir, A);
    // Deleted, then re-added at the same path.
    let dir = logless("base-readd", CONFIG, LOGGED);
    let text = std::fs::read_to_string(dir.join(A)).unwrap();
    git(&dir, &["rm", "-q", "-f", A]);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    commit_all(&dir);
    std::fs::write(
        dir.join(A),
        &text[..text.find("\n> **Revision log**").unwrap()],
    )
    .unwrap();
    commit_all(&dir);
    assert_restore_refused(&dir, A);
    // Not committed at all.
    let dir = laundering("base-uncommitted", "");
    assert_restore_refused(&dir, A);
}

#[test]
fn a_run_with_no_base_that_creates_a_log_says_so_and_an_unknown_base_is_refused() {
    let dir = logless("no-base", CONFIG, "# ADR-1: T\n\nBody.\n");
    let parsed = json(&run_urzua(&dir, &["fix", "--rebaseline", "--ids", A]));
    assert!(
        parsed["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["subject"] == "fix.revision-log-base"),
        "{parsed}"
    );
    let with_base = json(&run_urzua(
        &dir,
        &["fix", "--rebaseline", "--ids", A, "--base", "main"],
    ));
    assert!(with_base.get("notices").is_none(), "{with_base}");
    let unknown = json(&run_urzua(&dir, &["fix", "--rebaseline", "--base", "nope"]));
    assert_eq!(unknown["status"], "not-run", "{unknown}");
}
