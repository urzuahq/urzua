//! MILE-187: `urzua check --stdin-path` judges one record's proposed content
//! against every enabled rule without writing. Real git-repo fixtures, real
//! binary; all output is JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn run_stdin(dir: &Path, args: &[&str], stdin: &str) -> std::process::Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("urzua binary should run");
    // A refusal exits before reading stdin, so a broken pipe is not a failure.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error, section.required: error, embodiment.consistency: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    sections:\n      from: headings\n      required:\n        - {name: Context}\n";
const VALID: &str =
    "---\nStatus: Proposed\nDate: 2026-01-01\n---\n# ADR-1: T\n\n## Context\n\nwhy\n";
const PATH: &str = "docs/adr/ADR-1-a.md";

/// A clean repo: ADR-1 is valid; ADR-2 is already broken on disk (no Date).
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-check-stdin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH), VALID).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Proposed\n---\n# ADR-2: B\n\n## Context\n\nwhy\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn rules_of(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn proposed_content_is_judged_against_every_enabled_rule_and_nothing_is_written() {
    let dir = fixture("judge");
    let before = std::fs::read_to_string(dir.join(PATH)).unwrap();
    // No Date (header), no Context section, an Embodiment claim the evidence
    // does not support: three different rules, one edit.
    let proposed = "---\nStatus: Proposed\nEmbodiment: Verified\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";
    let out = run_stdin(&dir, &["check", "--stdin-path", PATH], proposed);
    let v = json(&out);
    let rules = rules_of(&v);
    for expected in [
        "header.required-fields",
        "section.required",
        "embodiment.consistency",
    ] {
        assert!(rules.iter().any(|r| r == expected), "{expected} in {v}");
    }
    assert_eq!(v["scope"]["source"], "stdin", "{v}");
    assert_eq!(v["files_examined"], 1, "{v}");
    assert_eq!(out.status.code(), Some(1), "an error finding exits 1: {v}");
    assert_eq!(std::fs::read_to_string(dir.join(PATH)).unwrap(), before);
}

#[test]
fn the_same_record_made_valid_reports_nothing() {
    let dir = fixture("valid");
    let out = run_stdin(&dir, &["check", "--stdin-path", PATH], VALID);
    let v = json(&out);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(rules_of(&v).is_empty(), "{v}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn only_the_proposed_record_is_reported_on() {
    let dir = fixture("only-one");
    // ADR-2 on disk lacks Date; the proposal for ADR-1 does not.
    let v = json(&run_stdin(&dir, &["check", "--stdin-path", PATH], VALID));
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["file"] == PATH),
        "{v}"
    );
}

#[test]
fn the_proposal_replaces_the_disk_copy_for_cross_record_rules() {
    let dir = fixture("replaces");
    // A new record that does not exist on disk yet is judged from the proposal.
    let new_path = "docs/adr/ADR-3-c.md";
    let broken = "---\nStatus: Proposed\n---\n# ADR-3: C\n\n## Context\n\nwhy\n";
    let v = json(&run_stdin(
        &dir,
        &["check", "--stdin-path", new_path],
        broken,
    ));
    assert_eq!(rules_of(&v), vec!["header.required-fields"], "{v}");
    assert_eq!(v["findings"][0]["file"], new_path, "{v}");
    assert!(!dir.join(new_path).exists());
}

#[test]
fn a_path_no_record_type_governs_is_a_could_not_run_naming_it() {
    let dir = fixture("ungoverned");
    let out = run_stdin(&dir, &["check", "--stdin-path", "src/a.rs"], "fn a() {}\n");
    let v = json(&out);
    assert_eq!(v["status"], "not-run", "{v}");
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .contains("no record type governs src/a.rs"),
        "{v}"
    );
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn it_is_refused_with_paths_or_base() {
    let dir = fixture("refused");
    for extra in [vec![PATH], vec!["--base", "HEAD"]] {
        let mut args = vec!["check", "--stdin-path", PATH];
        args.extend(extra.iter());
        let out = run_stdin(&dir, &args, VALID);
        let v = json(&out);
        assert_eq!(v["status"], "not-run", "{args:?}: {v}");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            v["error"].as_str().unwrap().contains("cannot be combined"),
            "{args:?}: {v}"
        );
    }
}

#[test]
fn a_rule_that_cannot_report_on_a_record_is_not_applicable_and_not_reported() {
    let dir = fixture("anchors");
    // type.no-declared-spec is anchored to the configuration: the fixture's
    // `adr` type declares no spec, so a whole-corpus run reports it.
    let config = CONFIG.replace("rules: {", "rules: {type.no-declared-spec: warn, ");
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    let whole = json(
        &Command::new(env!("CARGO_BIN_EXE_urzua"))
            .arg("check")
            .current_dir(&dir)
            .output()
            .unwrap(),
    );
    assert!(
        rules_of(&whole)
            .iter()
            .any(|r| r == "type.no-declared-spec"),
        "{whole}"
    );

    let v = json(&run_stdin(&dir, &["check", "--stdin-path", PATH], VALID));
    assert!(
        !rules_of(&v).iter().any(|r| r == "type.no-declared-spec"),
        "{v}"
    );
    let exec = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == "type.no-declared-spec")
        .unwrap_or_else(|| panic!("the rule is listed: {v}"));
    assert_eq!(exec["status"], "not-applicable", "{v}");
}
