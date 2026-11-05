//! `fail_on` (config) and `--fail-on` (flag): the exit code's threshold. The
//! JSON on stdout never changes; only the exit code does. Real binary.

mod common;

use std::path::Path;
use std::process::Command;

const WARN_ONLY: &str = "---\nDeciders: Ana\nReviewers:\n---\n# ADR-1: First\n";
const CLEAN: &str = "---\nDeciders: Ana\nReviewers: Bo\n---\n# ADR-1: First\n";

fn run_check(dir: &Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .args(extra)
        .arg("docs/")
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
    )
}

fn fixture(name: &str, top_level: &str, record: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fail-on-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{top_level}rules:\n  field.quality: warn\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Deciders, Reviewers]\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-first.md"), record).unwrap();
    for args in [
        &["add", "-A"][..],
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "fixture",
        ][..],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

#[test]
fn a_warning_exits_0_when_nothing_is_configured_observed_failing() {
    let dir = fixture("default", "", WARN_ONLY);
    let (code, out) = run_check(&dir, &[]);
    assert!(out.contains("\"warning\""), "{out}");
    assert_eq!(code, 0);
}

#[test]
fn config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing() {
    let default = fixture("cfg-default", "", WARN_ONLY);
    let strict = fixture("cfg-strict", "fail_on: warning\n", WARN_ONLY);
    let (_, baseline) = run_check(&default, &[]);
    let (code, out) = run_check(&strict, &[]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(out, baseline, "stdout must not change");
    assert!(out.contains("\"blocking\": false"), "{out}");
}

#[test]
fn fail_on_warning_still_exits_0_on_a_clean_corpus() {
    let dir = fixture("clean", "fail_on: warning\n", CLEAN);
    let (code, out) = run_check(&dir, &[]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_flag_overrides_the_config_in_both_directions_observed_failing() {
    let strict = fixture("flag-strict", "fail_on: warning\n", WARN_ONLY);
    assert_eq!(run_check(&strict, &["--fail-on", "error"]).0, 0);
    let lax = fixture("flag-lax", "", WARN_ONLY);
    assert_eq!(run_check(&lax, &["--fail-on", "warning"]).0, 1);
}

#[test]
fn an_unknown_fail_on_value_is_refused() {
    let dir = fixture("bad-flag", "", WARN_ONLY);
    let (code, _) = run_check(&dir, &["--fail-on", "info"]);
    assert_eq!(code, 2, "clap usage error");
}

#[test]
fn an_invalid_config_fail_on_is_a_could_not_run() {
    let dir = fixture("bad-cfg", "fail_on: info\n", WARN_ONLY);
    let (code, _) = run_check(&dir, &[]);
    assert_eq!(code, 2);
}
