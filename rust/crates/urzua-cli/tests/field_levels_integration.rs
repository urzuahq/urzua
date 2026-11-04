//! MILE-140: per-field severity within one rule. Real binary, JSON output.
//! The gate overrides every finding's severity with the rule level unless the
//! rule declared a level for that finding's own field.

use std::path::Path;
use std::process::Command;

fn run_check(dir: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap_or(-1), v)
}

fn fixture(name: &str, options: &str, record: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("urzua-field-levels-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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
            "schema_version: 2\nrules:\n  field.quality:\n    level: warn\n{options}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Deciders, Reviewers]\n"
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

fn severities(v: &serde_json::Value) -> Vec<(String, String)> {
    v["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .filter(|f| f["rule"] == "field.quality")
        .map(|f| {
            (
                f["message"].as_str().unwrap().to_string(),
                f["severity"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

const BOTH_BLANK: &str = "---\nDeciders:\nReviewers:\n---\n# ADR-1: First\n";
const ONLY_REVIEWERS_BLANK: &str = "---\nDeciders: Ana\nReviewers:\n---\n# ADR-1: First\n";
const OPTION: &str = "    field_levels: {Deciders: error, Reviewers: warn}\n";

#[test]
fn a_blank_deciders_is_an_error_and_a_blank_reviewers_a_warning_observed_failing() {
    let dir = fixture("both", OPTION, BOTH_BLANK);
    let (code, v) = run_check(&dir);
    let s = severities(&v);
    let sev = |f: &str| s.iter().find(|(m, _)| m.contains(f)).unwrap().1.clone();
    assert_eq!(sev("'Deciders'"), "error", "{v}");
    assert_eq!(sev("'Reviewers'"), "warning", "{v}");
    assert_ne!(code, 0, "an error finding blocks");
}

#[test]
fn only_a_blank_reviewers_warns_and_does_not_block_observed_failing() {
    let dir = fixture("only", OPTION, ONLY_REVIEWERS_BLANK);
    let (code, v) = run_check(&dir);
    assert_eq!(severities(&v).len(), 1, "{v}");
    assert_eq!(severities(&v)[0].1, "warning", "{v}");
    assert_eq!(code, 0, "a warning alone does not block: {v}");
}

#[test]
fn an_unlisted_field_takes_the_rule_level() {
    let dir = fixture(
        "unlisted",
        "    field_levels: {Deciders: error}\n",
        BOTH_BLANK,
    );
    let (_, v) = run_check(&dir);
    let s = severities(&v);
    let sev = |f: &str| s.iter().find(|(m, _)| m.contains(f)).unwrap().1.clone();
    assert_eq!(sev("'Deciders'"), "error", "{v}");
    assert_eq!(
        sev("'Reviewers'"),
        "warning",
        "rule level warn applies: {v}"
    );
}

#[test]
fn without_the_option_every_finding_takes_the_rule_level() {
    let dir = fixture("none", "", BOTH_BLANK);
    let (code, v) = run_check(&dir);
    assert!(severities(&v).iter().all(|(_, s)| s == "warning"), "{v}");
    assert_eq!(code, 0, "{v}");
}
