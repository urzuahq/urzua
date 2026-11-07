//! MILE-206, ADR-92: `discovery.include_untracked` makes discovery include
//! untracked, non-ignored record files under declared record directories; without
//! it a plain `check` names the untracked records it skipped. Real git-repo
//! fixtures, real binary; all output is JSON.
//!
//! Verifies: SPEC-2, SPEC-3

mod common;

use std::path::Path;
use std::process::Command;

const SKIPPED: &str = "discovery.untracked-skipped";
const NOT_JUDGED: &str = "discovery.untracked-not-drift-judged";
const NUDGE: &str = "embodiment.locator-on-back-pointer-path";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let v = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {} / {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), v)
}

/// `include`: the `discovery` object, or none. `fail_on: warning` throughout, so
/// a notice that moved the exit code would show.
fn config(include: Option<bool>) -> String {
    let discovery = include
        .map(|b| format!("discovery:\n  include_untracked: {b}\n"))
        .unwrap_or_default();
    format!(
        "schema_version: 2
fail_on: warning
embodiment:
  drift: Drift detected
  tiers:
    - {{state: Verified, evidence: test, claims_built: true}}
    - {{state: Implemented, evidence: code, claims_built: true}}
    - {{state: Specified, evidence: spec}}
  none: Not started
{discovery}rules:
  header.required-fields: error
  embodiment.consistency: warn
  {NUDGE}:
    level: warn
    back_pointer_paths: [src]
    scope: changed
record_types:
  adr:
    dir: docs/adr
    prefix: ADR
    required_fields: [Status]
    known_fields: [Embodiment, Realized-by]
    header_shape: yaml-frontmatter
    companion_fields:
      - [embodiment_state, embodiment_locator]
"
    )
}

fn record(n: u32, realized_by: Option<&str>) -> String {
    let locator = realized_by
        .map(|r| format!("Embodiment: Implemented\nRealized-by: {r}\n"))
        .unwrap_or_default();
    format!("---\nStatus: Accepted\n{locator}---\n# ADR-{n}: T\n")
}

/// One committed ADR citing `src/a.rs`, a `.gitignore` for one record and a
/// scratch directory. Nothing is untracked.
fn fixture(name: &str, include: Option<bool>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-untracked-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(include)).unwrap();
    std::fs::write(dir.join(".gitignore"), "docs/adr/ADR-9-ignored.md\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        record(1, Some("code:src/a.rs")),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// Two drafts, an ignored record, a record one level too deep, and a record-shaped
/// file outside every declared directory.
fn plant(dir: &Path) {
    std::fs::write(dir.join("docs/adr/ADR-2-draft.md"), record(2, None)).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-3-draft.md"), record(3, None)).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-9-ignored.md"), record(9, None)).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr/deeper")).unwrap();
    std::fs::write(dir.join("docs/adr/deeper/ADR-4-deep.md"), record(4, None)).unwrap();
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes/ADR-5-scratch.md"), record(5, None)).unwrap();
}

fn notices<'a>(v: &'a serde_json::Value, subject: &str) -> Vec<&'a serde_json::Value> {
    v.get("notices")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .filter(|n| n["subject"] == subject)
        .collect()
}

fn nudged(v: &serde_json::Value) -> Vec<String> {
    let mut files: Vec<String> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == NUDGE)
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    files.sort();
    files
}

#[test]
fn without_the_option_a_plain_check_names_the_untracked_records_it_skipped() {
    let dir = fixture("off-names", None);
    plant(&dir);
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(v["files_examined"], 1, "{v}");
    let found = notices(&v, SKIPPED);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["severity"], "info", "{v}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("docs/adr/ADR-2-draft.md"), "{message}");
    assert!(message.contains("docs/adr/ADR-3-draft.md"), "{message}");
    assert!(message.contains("discovery"), "names the option: {message}");
    // Not an ignored record, not one below the directory, not one outside it.
    for not_named in ["ADR-9-ignored", "ADR-4-deep", "ADR-5-scratch"] {
        assert!(!message.contains(not_named), "{not_named}: {message}");
    }
    assert!(message.contains('2'), "the count: {message}");
    // Info does not gate, even under fail_on: warning.
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["scope"]["include_untracked"], false, "{v}");
    assert_eq!(v["scope"]["untracked_included"], 0, "{v}");
}

#[test]
fn the_skipped_notice_lists_at_most_twenty_paths_and_the_count() {
    let dir = fixture("off-cap", None);
    for n in 10..35 {
        std::fs::write(dir.join(format!("docs/adr/ADR-{n}-d.md")), record(n, None)).unwrap();
    }
    let (_, v) = urzua(&dir, &["check"]);
    let found = notices(&v, SKIPPED);
    assert_eq!(found.len(), 1, "{v}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("25"), "the count: {message}");
    assert_eq!(
        message.matches("docs/adr/ADR-").count(),
        20,
        "twenty paths, not twenty-five: {message}"
    );
}

#[test]
fn without_the_option_no_notice_when_nothing_is_untracked_or_the_file_was_named() {
    let dir = fixture("off-quiet", None);
    let (_, v) = urzua(&dir, &["check"]);
    assert!(notices(&v, SKIPPED).is_empty(), "nothing untracked: {v}");
    // The ignored record and a stray non-record file are not untracked records.
    std::fs::write(dir.join("docs/adr/ADR-9-ignored.md"), record(9, None)).unwrap();
    std::fs::write(dir.join("docs/adr/scratch.txt"), "x").unwrap();
    let (_, v) = urzua(&dir, &["check"]);
    assert!(notices(&v, SKIPPED).is_empty(), "{v}");
    // A record named on argv is examined (`BUG-24`), so it was not skipped.
    std::fs::write(dir.join("docs/adr/ADR-2-draft.md"), record(2, None)).unwrap();
    let (_, v) = urzua(&dir, &["check", "docs/adr/ADR-2-draft.md"]);
    assert_eq!(v["files_examined"], 1, "{v}");
    assert_eq!(v["scope"]["source"], "argv", "{v}");
    assert!(notices(&v, SKIPPED).is_empty(), "{v}");
}

#[test]
fn without_the_option_a_directory_argument_still_does_not_examine_untracked_records() {
    let dir = fixture("off-dir", None);
    plant(&dir);
    let (_, v) = urzua(&dir, &["check", "docs/adr"]);
    assert_eq!(v["files_examined"], 1, "{v}");
    assert_eq!(v["scope"]["source"], "tracked-sweep", "{v}");
}

#[test]
fn with_the_option_untracked_non_ignored_records_under_a_declared_dir_are_examined() {
    let dir = fixture("on-sweep", Some(true));
    plant(&dir);
    let (code, v) = urzua(&dir, &["check"]);
    // ADR-1 plus the two drafts: not the ignored record, not the one below the
    // directory, not the file outside every record directory.
    assert_eq!(v["files_examined"], 3, "{v}");
    assert_eq!(v["scope"]["include_untracked"], true, "{v}");
    assert_eq!(v["scope"]["untracked_included"], 2, "{v}");
    assert_eq!(v["scope"]["source"], "tracked-sweep", "{v}");
    assert!(notices(&v, SKIPPED).is_empty(), "nothing is skipped: {v}");
    assert_eq!(code, 0, "{v}");
}

#[test]
fn with_the_option_a_malformed_draft_is_found_before_it_is_staged() {
    let dir = fixture("on-finds", Some(true));
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        "---\nDate: 2026-01-01\n---\n# ADR-2: T\n",
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check"]);
    let rules: Vec<(&str, &str)> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["rule"].as_str().unwrap(), f["file"].as_str().unwrap()))
        .collect();
    assert!(
        rules.contains(&("header.required-fields", "docs/adr/ADR-2-draft.md")),
        "{v}"
    );
    assert_eq!(code, 1, "{v}");
}

#[test]
fn with_the_option_a_directory_argument_examines_the_untracked_records_below_it() {
    let dir = fixture("on-dir", Some(true));
    plant(&dir);
    let (_, v) = urzua(&dir, &["check", "docs/adr"]);
    assert_eq!(v["files_examined"], 3, "{v}");
    let (_, v) = urzua(&dir, &["check", "docs/adr/ADR-2-draft.md"]);
    assert_eq!(v["files_examined"], 1, "{v}");
    assert_eq!(
        v["scope"]["source"], "argv",
        "a named file keeps BUG-24: {v}"
    );
}

#[test]
fn a_file_the_option_does_not_include_stays_ungoverned_under_a_directory_argument() {
    let dir = fixture("on-outside", Some(true));
    plant(&dir);
    let (_, v) = urzua(&dir, &["check", "notes"]);
    assert_eq!(v["files_examined"], 0, "{v}");
}

#[test]
fn base_with_the_option_reports_an_untracked_record_as_added_and_judges_it() {
    let dir = fixture("base-on", Some(true));
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        record(2, Some("code:src/a.rs")),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    let changed: Vec<&str> = v["scope"]["changed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(changed, vec!["docs/adr/ADR-2-draft.md"], "{v}");
    assert_eq!(v["scope"]["source"], "git-diff", "{v}");
    // The diff rule judges the draft as it judges a staged addition; the
    // committed ADR-1 is unchanged and so deferred.
    assert_eq!(
        nudged(&v),
        vec!["docs/adr/ADR-2-draft.md".to_string()],
        "{v}"
    );
    assert_eq!(
        code, 1,
        "the nudge is a warning under fail_on: warning: {v}"
    );
    assert_eq!(v["scope"]["untracked_included"], 1, "{v}");
}

#[test]
fn base_without_the_option_is_unchanged_and_the_draft_is_only_named() {
    let dir = fixture("base-off", None);
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        record(2, Some("code:src/a.rs")),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["scope"]["changed_paths"], serde_json::json!([]), "{v}");
    assert!(nudged(&v).is_empty(), "{v}");
    assert_eq!(notices(&v, SKIPPED).len(), 1, "{v}");
}

#[test]
fn base_refuses_a_named_untracked_file_only_when_the_option_would_not_include_it() {
    // Option off: the BUG-24 refusal stands.
    let dir = fixture("base-refuse-off", None);
    std::fs::write(dir.join("docs/adr/ADR-2-draft.md"), record(2, None)).unwrap();
    let (code, v) = urzua(
        &dir,
        &["check", "--base", "HEAD", "docs/adr/ADR-2-draft.md"],
    );
    assert_eq!(code, 2, "{v}");
    assert!(
        v["error"]
            .as_str()
            .or(v["message"].as_str())
            .unwrap_or("")
            .contains("--base cannot be combined"),
        "{v}"
    );

    // Option on: a draft under a declared directory is what the option includes.
    let dir = fixture("base-refuse-on", Some(true));
    std::fs::write(dir.join("docs/adr/ADR-2-draft.md"), record(2, None)).unwrap();
    std::fs::write(dir.join("notes-scratch.md"), "x").unwrap();
    let (code, v) = urzua(
        &dir,
        &["check", "--base", "HEAD", "docs/adr/ADR-2-draft.md"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["files_examined"], 1, "{v}");
    // A file outside every declared directory is still refused.
    let (code, _) = urzua(&dir, &["check", "--base", "HEAD", "notes-scratch.md"]);
    assert_eq!(code, 2);
}

/// A committed ADR citing `src/a.rs`, drifted by a later commit to that file.
fn drift_fixture(name: &str, include: Option<bool>) -> common::TempDir {
    let dir = fixture(name, include);
    std::fs::write(dir.join("src/a.rs"), "fn a() { }\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "change the cited file"]);
    dir
}

fn drift_findings(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn drift_does_not_judge_an_untracked_record_and_says_so() {
    let dir = drift_fixture("drift", Some(true));
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        record(2, Some("code:src/a.rs")),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-3-draft.md"), record(3, None)).unwrap();
    let (_, v) = urzua(&dir, &["check"]);
    assert_eq!(v["scope"]["untracked_included"], 2, "{v}");
    // The committed record is judged as before; the draft with a locator is not.
    assert_eq!(
        drift_findings(&v),
        vec!["docs/adr/ADR-1-a.md".to_string()],
        "{v}"
    );
    let found = notices(&v, NOT_JUDGED);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["severity"], "info", "{v}");
    let message = found[0]["message"].as_str().unwrap();
    // One draft carries a locator; the other carries none and is not counted.
    assert!(message.starts_with('1'), "{message}");
}

#[test]
fn drift_judges_the_record_once_it_has_a_commit_and_the_notice_goes() {
    let dir = drift_fixture("drift-committed", Some(true));
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        record(2, Some("code:src/a.rs")),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "commit the draft"]);
    let (_, v) = urzua(&dir, &["check"]);
    assert!(notices(&v, NOT_JUDGED).is_empty(), "{v}");
}

#[test]
fn the_not_judged_notice_is_absent_when_the_option_is_off() {
    let dir = drift_fixture("drift-off", None);
    std::fs::write(
        dir.join("docs/adr/ADR-2-draft.md"),
        record(2, Some("code:src/a.rs")),
    )
    .unwrap();
    let (_, v) = urzua(&dir, &["check"]);
    assert!(notices(&v, NOT_JUDGED).is_empty(), "{v}");
    assert_eq!(drift_findings(&v), vec!["docs/adr/ADR-1-a.md".to_string()]);
}

#[test]
fn an_unknown_key_under_discovery_is_a_load_error() {
    let dir = fixture("unknown-key", None);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config(Some(true)).replace(
            "include_untracked: true",
            "include_untracked: true\n  nope: 1",
        ),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 2, "{v}");
    assert!(v.to_string().contains("nope"), "{v}");
}

#[test]
fn a_non_boolean_include_untracked_is_a_load_error() {
    let dir = fixture("not-bool", None);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config(Some(true)).replace("include_untracked: true", "include_untracked: \"yes\""),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 2, "{v}");
}
