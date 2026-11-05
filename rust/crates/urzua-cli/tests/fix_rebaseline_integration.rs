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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
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
        rows[0].contains("Re-baselined (attested by ana): re-pinned"),
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
            "r",
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
            "r",
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
            "batch",
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
            "r",
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
            "--reason",
            "r",
        ],
        Some("gh-login"),
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let a = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(a.contains("(attested by gh-login)"), "{a}");
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
                "r",
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
            "r",
        ],
        None,
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "not-run", "{parsed}");
    assert_eq!(std::fs::read_to_string(dir.join(A)).unwrap(), before);
}
