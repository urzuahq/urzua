//! No integration test reaches the machine's `gh` or the network.
//!
//! `urzua fix --apply` and `urzua new` run `gh api user` to learn who is
//! attesting. A test that runs the binary with the machine's own `PATH` makes
//! that call: a live API request on a developer machine with a logged-in `gh`
//! (up to five seconds when offline), and a verified login that replaces the
//! `--by` the test passed, so the recorded actor differed by machine. Tests run
//! the binary through `common::urzua()`, which puts a failing `gh` first on
//! `PATH`.

mod common;

use std::path::Path;

fn test_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            out.push((name, std::fs::read_to_string(&path).unwrap()));
        }
    }
    out.sort();
    out
}

/// A line that is code, not a comment, so this file's own prose may name the
/// forbidden spelling.
fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .map(|(i, l)| (i + 1, l))
}

#[test]
fn no_test_spawns_the_binary_with_the_machines_path_observed_failing() {
    let needle = concat!("Command::new(env!(\"CARGO_BIN_EXE_", "urzua\"))");
    let mut offenders = Vec::new();
    for (name, source) in test_sources() {
        if name == "no_real_gh_integration.rs" {
            continue;
        }
        for (line, text) in code_lines(&source) {
            if text.contains(needle) {
                offenders.push(format!("{name}:{line}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "run the binary through `common::urzua()` so `gh` is stubbed; found {offenders:?}"
    );
}

#[test]
fn a_test_that_hands_the_binary_to_a_script_also_stubs_gh() {
    // A script (`URZUA`/`URZUA_BIN` in the environment) runs the binary itself,
    // so the test must give it the stubbed `PATH` too.
    let mut offenders = Vec::new();
    for (name, source) in test_sources() {
        if name == "no_real_gh_integration.rs" {
            continue;
        }
        let hands_over = code_lines(&source).any(|(_, t)| {
            t.contains(".env(\"URZUA\", env!(\"CARGO_BIN_EXE_urzua\"))")
                && !t.contains("URZUA_BIN")
        });
        if hands_over && !source.contains("path_with_stub_gh") {
            offenders.push(name);
        }
    }
    assert!(
        offenders.is_empty(),
        "set `.env(\"PATH\", common::path_with_stub_gh())` beside `URZUA`; found {offenders:?}"
    );
}

#[test]
fn the_stub_gh_reports_not_logged_in() {
    let out = std::process::Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .env("PATH", common::path_with_stub_gh())
        .output()
        .expect("the stub gh should run");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}
