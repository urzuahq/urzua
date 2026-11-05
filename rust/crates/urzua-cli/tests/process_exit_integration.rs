//! Process-level exit behaviour (MILE-176): a panic exits 2, a closed stdout
//! does not panic.
//!
//! Both run in a small fixture repository, not this repository's corpus, so
//! the outcome does not move with the records being edited.

mod common;

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// One clean ADR under a config that enables one rule: `check` exits 0.
fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-process-exit-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn a_panic_exits_2_with_a_not_run_result() {
    let dir = fixture("panic");
    let out = bin()
        .arg("check")
        .env("URZUA_TEST_PANIC", "1")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout must be one JSON object ({e}): {out:?}"));
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(
        report["panic"]
            .as_str()
            .is_some_and(|p| p.contains("URZUA_TEST_PANIC set")),
        "the report names the panic: {report}"
    );
}

#[test]
fn a_closed_stdout_does_not_panic() {
    let dir = fixture("closed-stdout");
    // The same run with stdout open exits 0, which is what the closed-stdout
    // run must still do: the exit code is the check's verdict, not a failure
    // to print it.
    let open = bin().arg("check").current_dir(&dir).output().unwrap();
    assert_eq!(open.status.code(), Some(0), "{open:?}");

    let mut child = bin()
        .arg("check")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the read end before the child writes.
    drop(child.stdout.take());
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(!err.contains("panicked"), "{err}");
    assert_eq!(status.code(), Some(0), "must keep the verdict: {err}");
}
